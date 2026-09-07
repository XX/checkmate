//! Сохранение текущих параметров игры в файл конфигурации.
//!
//! Источник истины во время игры — компоненты параметров, поэтому сохранение — это снимок мира,
//! а не запись ресурса [`Config`]: за основу берётся загруженный конфиг (в нём структурные
//! параметры, которых нет в компонентах: пути моделей, пресеты камеры, логирование, `remote`),
//! а секции, для которых в мире нашлись параметры, переписываются их текущими значениями.
//! Поэтому сохранение из ангара не обнуляет секции самолёта и ландшафта, которых в этот момент
//! в мире нет.
//!
//! Рантайм-состояние (текущая тяга, положение самолёта) параметром не является и в снимок
//! не попадает.
//!
//! Сохранение запускается двумя способами: клавишей F5 и методом BRP `checkmate.save_config`
//! (см. [`crate::remote`]). Оба ведут в [`save`].
//!
//! Рабочий `Config.toml` не перезаписывается, пока путь не задан явно: сериализация не сохраняет
//! комментарии, и закомментированные варианты настроек были бы молча потеряны. По умолчанию
//! снимок пишется в [`DEFAULT_SAVE_FILE`] в текущем каталоге, и получившийся файл самодостаточен:
//! его можно передать как `checkmate --config <файл>`.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::{fs, io};

use bevy::app::{App, Last, Plugin, Update};
use bevy::ecs::hierarchy::Children;
use bevy::ecs::message::{Message, MessageWriter, Messages};
use bevy::ecs::query::With;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::schedule::common_conditions::on_message;
use bevy::ecs::system::{Query, Res, SystemState};
use bevy::ecs::world::World;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::log::{error, info};

use crate::camera::CameraParams;
use crate::config::{Config, JetFireSettings};
use crate::environment::{AmbientParams, SunParams};
use crate::state::ingame::aircraft::{Aircraft, AircraftParams, ThrustParams};
use crate::state::ingame::engine::JetFireParams;
use crate::state::ingame::terrain::TerrainParams;

/// Файл, в который сохраняются параметры, если путь не задан явно.
pub const DEFAULT_SAVE_FILE: &str = "Config.saved.toml";

/// Клавиша сохранения параметров в [`DEFAULT_SAVE_FILE`].
pub const SAVE_KEY: KeyCode = KeyCode::F5;

/// Запрос на сохранение текущих параметров.
#[derive(Message, Debug, Default, Clone)]
pub struct SaveConfig {
    /// `None` — сохранить в [`DEFAULT_SAVE_FILE`].
    pub path: Option<PathBuf>,
}

pub struct SaveConfigPlugin;

impl Plugin for SaveConfigPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SaveConfig>()
            .add_systems(Update, request_save_on_hotkey)
            // Обработчик эксклюзивный: ему нужен весь мир. Поэтому он вынесен в `Last`
            // и выполняется только при наличии запроса.
            .add_systems(Last, handle_save_requests.run_if(on_message::<SaveConfig>));
    }
}

pub fn request_save_on_hotkey(input: Res<ButtonInput<KeyCode>>, mut requests: MessageWriter<SaveConfig>) {
    if input.just_pressed(SAVE_KEY) {
        requests.write(SaveConfig::default());
    }
}

pub fn handle_save_requests(world: &mut World) {
    let requests: Vec<SaveConfig> = world.resource_mut::<Messages<SaveConfig>>().drain().collect();

    for request in requests {
        match save(world, request.path) {
            Ok(path) => info!("params saved to {}", path.display()),
            Err(err) => error!("params save failed: {err}"),
        }
    }
}

/// Сохраняет снимок текущих параметров и возвращает путь, по которому он записан.
pub fn save(world: &mut World, path: Option<PathBuf>) -> Result<PathBuf, Box<dyn Error>> {
    let path = path.unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_FILE));
    // `to_string`, а не `to_string_pretty`: последний разворачивает каждый массив
    // в несколько строк, и векторы позиций становятся нечитаемыми.
    let text = toml::to_string(&snapshot(world))?;

    write_atomically(&path, &text)?;

    Ok(path.canonicalize().unwrap_or(path))
}

/// Собирает настройки, отражающие текущее состояние параметров игры.
pub fn snapshot(world: &mut World) -> Config {
    let mut state = SystemState::<(
        Res<Config>,
        Res<AmbientParams>,
        Res<CameraParams>,
        Query<&SunParams>,
        Query<&TerrainParams>,
        Query<(&AircraftParams, &ThrustParams, Option<&Children>), With<Aircraft>>,
        Query<&JetFireParams>,
    )>::new(world);

    // Все параметры запроса читающие, ошибку валидации давать нечему.
    let (config, ambient_params, camera_params, suns, terrains, aircrafts, jet_fires) =
        state.get(world).expect("read-only params must be valid");

    let mut snapshot = config.clone();

    snapshot.environment.ambient = (&*ambient_params).into();

    if let Some(sun) = suns.iter().next() {
        snapshot.environment.sun = sun.into();
    }

    if let Some(terrain) = terrains.iter().next() {
        snapshot.game.terrain = terrain.to_settings(&config.game.terrain);
    }

    if let Some((aircraft, thrust, children)) = aircrafts.iter().next() {
        snapshot.game.aircraft = aircraft.to_settings(thrust);

        // Порядок сопел должен совпадать с порядком в конфиге: `Children` хранит потомков
        // в порядке добавления, а порядок обхода запросом не гарантирован.
        let jet_fires: Vec<JetFireSettings> = children
            .into_iter()
            .flatten()
            .filter_map(|child| jet_fires.get(*child).ok())
            .map(JetFireSettings::from)
            .collect();

        if !jet_fires.is_empty() {
            snapshot.game.flying_model.jet_fires = jet_fires;
        }
    }

    camera_params.write_to(&mut snapshot);

    snapshot
}

/// Пишет файл через временный и переименование: прерванная запись не должна оставить
/// обрезанный конфиг. Временный файл кладётся рядом с целевым, чтобы переименование
/// происходило в пределах одной файловой системы.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    let mut name = path
        .file_name()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("not a file path: {}", path.display()),
            )
        })?
        .to_os_string();
    name.push(".tmp");
    let tmp_path = path.with_file_name(name);

    fs::write(&tmp_path, text)?;
    fs::rename(&tmp_path, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_serializable_to_toml() {
        let text = toml::to_string(&Config::default()).expect("config must be serializable");

        assert!(text.contains("[game]"), "unexpected config dump:\n{text}");
        assert!(text.contains("[camera.bloom]"), "unexpected config dump:\n{text}");
        // Цвет — массив sRGB с альфой, а не развёрнутая таблица цветового пространства.
        assert!(
            text.contains("color = [1.0, 1.0, 1.0, 1.0]"),
            "colors must be plain arrays:\n{text}"
        );
        // Векторы должны остаться однострочными, как в рабочем конфиге.
        assert!(
            text.contains("position = [0.0, 0.0, -5.5]"),
            "vectors must stay inline:\n{text}"
        );
    }
}
