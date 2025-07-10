use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use bevy::color::Color;
use bevy::core_pipeline::auto_exposure::AutoExposure;
use bevy::ecs::resource::Resource;
use bevy::pbr::AmbientLight;
use config_load::config::builder::DefaultState;
use config_load::config::{ConfigBuilder, Environment};
use config_load::{ConfigLoader, FileLocation, Load};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GameSettings {
    #[serde(default = "GameSettings::default_lang")]
    pub lang: String,

    #[serde(default = "GameSettings::default_assets_root")]
    pub assets_root: PathBuf,

    #[serde(default)]
    pub hangar_model: String,

    #[serde(default)]
    pub flying_model: String,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            lang: Self::default_lang(),
            assets_root: Self::default_assets_root(),
            hangar_model: Default::default(),
            flying_model: Default::default(),
        }
    }
}

impl GameSettings {
    pub fn default_lang() -> String {
        "en".into()
    }

    pub fn default_assets_root() -> PathBuf {
        PathBuf::from("assets")
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GraphicsSettings {
    #[serde(default = "GraphicsSettings::default_shadow_map_size")]
    pub shadow_map_size: usize,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            shadow_map_size: Self::default_shadow_map_size(),
        }
    }
}

impl GraphicsSettings {
    pub const fn default_shadow_map_size() -> usize {
        2048
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct EnvironmentSettings {
    #[serde(default)]
    pub ambient: AmbientSettings,

    #[serde(default)]
    pub auto_exposure: AutoExposureSettings,

    #[serde(default)]
    pub atmosphere: AtmosphereSettings,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AmbientSettings {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "AmbientSettings::default_color")]
    pub color: Color,

    #[serde(default = "AmbientSettings::default_brightness")]
    pub brightness: f32,

    #[serde(default = "AmbientSettings::default_affects_lightmapped_meshes")]
    pub affects_lightmapped_meshes: bool,
}

impl Default for AmbientSettings {
    fn default() -> Self {
        Self {
            enabled: Default::default(),
            color: Self::default_color(),
            brightness: Self::default_brightness(),
            affects_lightmapped_meshes: Self::default_affects_lightmapped_meshes(),
        }
    }
}

impl AmbientSettings {
    pub const fn default_color() -> Color {
        Color::WHITE
    }

    pub const fn default_brightness() -> f32 {
        80.0
    }

    pub const fn default_affects_lightmapped_meshes() -> bool {
        true
    }

    pub fn to_ambient_light(&self) -> Option<AmbientLight> {
        if self.enabled {
            Some(AmbientLight {
                color: self.color,
                brightness: self.brightness,
                affects_lightmapped_meshes: self.affects_lightmapped_meshes,
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct AutoExposureSettings {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub range: Option<RangeInclusive<f32>>,

    #[serde(default)]
    pub speed_brighten: Option<f32>,

    #[serde(default)]
    pub speed_darken: Option<f32>,
}

impl AutoExposureSettings {
    pub fn to_auto_exposure(&self) -> Option<AutoExposure> {
        if self.enabled {
            let mut auto_exposure = AutoExposure::default();
            let Self {
                enabled: _,
                range,
                speed_brighten,
                speed_darken,
            } = self;

            if let Some(range) = range.clone() {
                auto_exposure.range = range;
            }
            if let Some(speed_brighten) = *speed_brighten {
                auto_exposure.speed_brighten = speed_brighten;
            }
            if let Some(speed_darken) = *speed_darken {
                auto_exposure.speed_darken = speed_darken;
            }

            Some(auto_exposure)
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct AtmosphereSettings {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct LoggerSettings {
    #[serde(default = "LoggerSettings::default_spec")]
    pub spec: String,

    pub path: Option<PathBuf>,

    pub duplicate_to_stdout: bool,

    #[serde(default = "LoggerSettings::default_keep_log_for_days")]
    pub keep_log_for_days: usize,
}

impl Default for LoggerSettings {
    fn default() -> Self {
        Self {
            spec: Self::default_spec(),
            path: None,
            duplicate_to_stdout: false,
            keep_log_for_days: Self::default_keep_log_for_days(),
        }
    }
}

impl LoggerSettings {
    pub fn default_spec() -> String {
        "info".into()
    }

    pub const fn default_keep_log_for_days() -> usize {
        7
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Resource)]
#[serde(default)]
pub struct Config {
    #[serde(default)]
    pub game: GameSettings,

    #[serde(default)]
    pub graphics: GraphicsSettings,

    #[serde(default)]
    pub environment: EnvironmentSettings,

    #[serde(default)]
    pub log: LoggerSettings,
}

impl Config {
    pub fn load(config_file: Option<PathBuf>) -> config_load::Result<Self> {
        ConfigLoader::default()
            .add(
                FileLocation::first_some_path()
                    .from_env("CHECKMATE_ROOT_CONFIG")
                    .from_home(Path::new(".checkmate").join("Config.toml")),
            )
            .exclude_not_exists()
            .add(
                FileLocation::first_some_path()
                    .from_file(config_file)
                    .from_cwd_and_parents_exists("Config.toml"),
            )
            .load()
    }
}

impl Load for Config {
    fn load(config_builder: ConfigBuilder<DefaultState>) -> config_load::Result<Self> {
        // Add in settings from the environment (with a prefix of CHECKMATE_)
        // Eg.. `CHECKMATE_GAME_LANG=ru checkmate` would set the `game.lang` key
        let config = config_builder
            .add_source(Environment::with_prefix("CHECKMATE").separator("_").try_parsing(true))
            .build()?;
        config.try_deserialize()
    }
}
