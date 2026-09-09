//! Внешний API изменения параметров игры.
//!
//! Включает [Bevy Remote Protocol][brp] (JSON-RPC 2.0 поверх HTTP): сторонняя программа —
//! редактор, скрипт, отладочная утилита — читает и изменяет зарегистрированные компоненты
//! и ресурсы работающей игры. Изменённые параметры попадают в игру через системы применения
//! (см. [`crate::params`]), поэтому результат виден в том же или следующем кадре.
//!
//! Включается параметром конфигурации `remote.enabled`. Пока API выключено, ни одна система
//! и ни один поток не создаются, сокет не открывается: стоимость в кадре нулевая.
//!
//! Помимо `port`, при включённом API открывается второй порт для рендер-подприложения
//! (15703, `DEFAULT_RENDER_PORT`); в bevy 0.19 он не настраивается.
//!
//! По умолчанию сервер слушает только петлевой интерфейс (127.0.0.1). Протокол не имеет
//! ни аутентификации, ни шифрования, поэтому менять адрес на внешний стоит только в доверенной
//! сети: любой, кто дотянется до порта, сможет менять состояние игры.
//!
//! # Примеры запросов
//!
//! Найти сущность солнца и её параметры:
//!
//! ```text
//! curl -s http://127.0.0.1:15702 -d '{
//!   "jsonrpc": "2.0", "id": 1, "method": "world.query",
//!   "params": {"data": {"components": ["checkmate::environment::SunParams"]}}
//! }'
//! ```
//!
//! Изменить освещённость (`entity` — из ответа выше):
//!
//! ```text
//! curl -s http://127.0.0.1:15702 -d '{
//!   "jsonrpc": "2.0", "id": 2, "method": "world.mutate_components",
//!   "params": {
//!     "entity": 4294967299,
//!     "component": "checkmate::environment::SunParams",
//!     "path": ".illuminance",
//!     "value": 20000.0
//!   }
//! }'
//! ```
//!
//! Параметры камеры — ресурс, а не компонент:
//!
//! ```text
//! curl -s http://127.0.0.1:15702 -d '{
//!   "jsonrpc": "2.0", "id": 3, "method": "world.mutate_resources",
//!   "params": {
//!     "resource": "checkmate::camera::CameraParams",
//!     "path": ".bloom_intensity",
//!     "value": 0.5
//!   }
//! }'
//! ```
//!
//! Сохранить текущие параметры в файл (путь необязателен, по умолчанию `Config.saved.toml`):
//!
//! ```text
//! curl -s http://127.0.0.1:15702 -d '{
//!   "jsonrpc": "2.0", "id": 4, "method": "checkmate.save_config",
//!   "params": {"path": "Config.tuned.toml"}
//! }'
//! ```
//!
//! Список доступных методов — `rpc.discover`, схема зарегистрированных типов —
//! `registry.schema`: по ней редактор строит UI, не зная о типах игры заранее.
//!
//! [brp]: https://docs.rs/bevy_remote

use std::path::PathBuf;

use bevy::app::{App, Plugin};
use bevy::ecs::system::In;
use bevy::ecs::world::World;
use bevy::log;
use bevy::remote::http::RemoteHttpPlugin;
use bevy::remote::{BrpError, BrpResult, RemotePlugin};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::RemoteSettings;
use crate::save;

/// Метод BRP, сохраняющий текущие параметры в файл конфигурации.
pub const SAVE_CONFIG_METHOD: &str = "checkmate.save_config";

pub struct AppRemotePlugin {
    settings: RemoteSettings,
}

impl AppRemotePlugin {
    pub fn new(settings: &RemoteSettings) -> Self {
        Self {
            settings: settings.clone(),
        }
    }
}

impl Plugin for AppRemotePlugin {
    fn build(&self, app: &mut App) {
        let RemoteSettings { enabled, address, port } = self.settings;

        if !enabled {
            return;
        }

        app.add_plugins((
            RemotePlugin::default().with_method_main(SAVE_CONFIG_METHOD, save_config),
            RemoteHttpPlugin::default().with_address(address).with_port(port),
        ));

        log::warn!("remote params API is enabled and listening on http://{address}:{port}");
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SaveConfigParams {
    /// Куда сохранять. По умолчанию — [`save::DEFAULT_SAVE_FILE`].
    #[serde(default)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct SaveConfigResult {
    /// Путь, по которому файл действительно записан.
    pub path: PathBuf,
}

/// Обработчик [`SAVE_CONFIG_METHOD`]: сохраняет снимок параметров и отвечает путём к файлу.
pub fn save_config(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let params: SaveConfigParams = match params {
        Some(params) => serde_json::from_value(params).map_err(BrpError::internal)?,
        None => Default::default(),
    };

    let path = save::save(world, params.path).map_err(|err| BrpError::internal(err.to_string()))?;

    serde_json::to_value(SaveConfigResult { path }).map_err(BrpError::internal)
}
