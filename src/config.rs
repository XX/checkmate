use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use bevy::color::Color;
use bevy::core_pipeline::auto_exposure::AutoExposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::resource::Resource;
use bevy::math::{Quat, Vec3};
use bevy::pbr::AmbientLight;
use bevy::pbr::light_consts::lux;
use bevy::transform::components::Transform;
use config_load::config::builder::DefaultState;
use config_load::config::{ConfigBuilder, Environment};
use config_load::{ConfigLoader, FileLocation, Load};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::follow::Follower;

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
    pub flying_model: FlyingModelSettings,

    #[serde(default = "GameSettings::default_flight_altitude")]
    pub flight_altitude: f32,

    #[serde(default)]
    pub terrain: TerrainSettings,

    #[serde(default)]
    pub state: AppState,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            lang: Self::default_lang(),
            assets_root: Self::default_assets_root(),
            hangar_model: Default::default(),
            flying_model: Default::default(),
            flight_altitude: Self::default_flight_altitude(),
            terrain: Default::default(),
            state: Default::default(),
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

    pub const fn default_flight_altitude() -> f32 {
        1000.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct FlyingModelSettings {
    #[serde(default)]
    pub path: String,

    #[serde(default = "FlyingModelSettings::default_jet_fires")]
    pub jet_fires: Vec<JetFireSettings>,
}

impl Default for FlyingModelSettings {
    fn default() -> Self {
        Self {
            path: Default::default(),
            jet_fires: Self::default_jet_fires(),
        }
    }
}

impl FlyingModelSettings {
    pub fn default_jet_fires() -> Vec<JetFireSettings> {
        vec![JetFireSettings::default()]
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct JetFireSettings {
    #[serde(default = "JetFireSettings::default_intensity")]
    pub intensity: f32,

    #[serde(default = "JetFireSettings::default_color")]
    pub color: [f32; 3],

    #[serde(default = "JetFireSettings::default_radius")]
    pub radius: f32,

    #[serde(default = "JetFireSettings::default_range")]
    pub range: f32,

    #[serde(default = "JetFireSettings::default_position")]
    pub position: [f32; 3],

    #[serde(default)]
    pub flickering: FlickeringSettings,
}

impl Default for JetFireSettings {
    fn default() -> Self {
        Self {
            intensity: Self::default_intensity(),
            color: Self::default_color(),
            radius: Self::default_radius(),
            range: Self::default_range(),
            position: Self::default_position(),
            flickering: Default::default(),
        }
    }
}

impl JetFireSettings {
    pub const fn default_intensity() -> f32 {
        3000000.0
    }

    pub const fn default_color() -> [f32; 3] {
        [1.0, 0.5, 0.1]
    }

    pub const fn default_radius() -> f32 {
        0.05
    }

    pub const fn default_range() -> f32 {
        5.0
    }

    pub const fn default_position() -> [f32; 3] {
        [0.0, 0.0, -5.5]
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct FlickeringSettings {
    #[serde(default = "FlickeringSettings::default_variation")]
    pub variation: f32,

    #[serde(default = "FlickeringSettings::default_frequency")]
    pub frequency: f32,
}

impl Default for FlickeringSettings {
    fn default() -> Self {
        Self {
            variation: Self::default_variation(),
            frequency: Self::default_frequency(),
        }
    }
}

impl FlickeringSettings {
    pub const fn default_variation() -> f32 {
        600000.0
    }

    pub const fn default_frequency() -> f32 {
        0.03
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Rotation {
    pub from: [f32; 3],
    pub to: [f32; 3],
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TerrainSettings {
    #[serde(default)]
    pub model: String,

    #[serde(default)]
    pub position: [f32; 3],

    #[serde(default)]
    pub rotation: Option<Rotation>,

    #[serde(default = "TerrainSettings::default_scale")]
    pub scale: f32,
}

impl Default for TerrainSettings {
    fn default() -> Self {
        Self {
            model: Default::default(),
            position: Default::default(),
            rotation: None,
            scale: Self::default_scale(),
        }
    }
}

impl TerrainSettings {
    pub const fn default_scale() -> f32 {
        1.0
    }

    pub fn get_transform(&self) -> Transform {
        if let Some(rotation) = self.rotation {
            Transform::from_rotation(Quat::from_rotation_arc(
                Vec3::from(rotation.from).normalize(),
                Vec3::from(rotation.to).normalize(),
            ))
        } else {
            Transform::default()
        }
        .with_translation(self.position.into())
        .with_scale(Vec3::splat(self.scale))
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
    pub sun: SunSettings,

    #[serde(default)]
    pub ambient: AmbientSettings,

    #[serde(default)]
    pub atmosphere: AtmosphereSettings,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct SunSettings {
    #[serde(default = "SunSettings::default_illuminance")]
    pub illuminance: f32,

    #[serde(default = "SunSettings::default_shadows_enabled")]
    pub shadows_enabled: bool,

    #[serde(default = "SunSettings::default_position")]
    pub position: [f32; 3],

    #[serde(default)]
    pub target: [f32; 3],
}

impl Default for SunSettings {
    fn default() -> Self {
        Self {
            illuminance: Self::default_illuminance(),
            shadows_enabled: Self::default_shadows_enabled(),
            position: Self::default_position(),
            target: Default::default(),
        }
    }
}

impl SunSettings {
    pub const fn default_illuminance() -> f32 {
        lux::AMBIENT_DAYLIGHT
    }

    pub const fn default_shadows_enabled() -> bool {
        true
    }

    pub const fn default_position() -> [f32; 3] {
        [20000.0, 10000., 50000.0]
    }
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

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct AtmosphereSettings {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CameraSettings {
    #[serde(default)]
    pub exposure: f32,

    #[serde(default)]
    pub presets: Vec<CameraPresetSettings>,

    #[serde(default)]
    pub auto_exposure: AutoExposureSettings,

    #[serde(default)]
    pub tonemap: Tonemap,

    #[serde(default)]
    pub follow: CameraFollowSettings,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CameraPresetSettings {
    #[serde(default)]
    pub position: [f32; 3],

    #[serde(default)]
    pub target: [f32; 3],
}

impl CameraPresetSettings {
    pub fn to_vec3s(&self) -> (Vec3, Vec3) {
        let position = Vec3::from(self.position);
        let target = Vec3::from(self.target);
        (position, target)
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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub enum Tonemap {
    #[default]
    None,
    Reinhard,
    ReinhardLuminance,
    AcesFitted,
    AgX,
    SomewhatBoringDisplayTransform,
    TonyMcMapface,
    BlenderFilmic,
}

impl Tonemap {
    pub fn to_tonemapping(&self) -> Tonemapping {
        match self {
            Self::None => Tonemapping::None,
            Self::Reinhard => Tonemapping::Reinhard,
            Self::ReinhardLuminance => Tonemapping::ReinhardLuminance,
            Self::AcesFitted => Tonemapping::AcesFitted,
            Self::AgX => Tonemapping::AgX,
            Self::SomewhatBoringDisplayTransform => Tonemapping::SomewhatBoringDisplayTransform,
            Self::TonyMcMapface => Tonemapping::TonyMcMapface,
            Self::BlenderFilmic => Tonemapping::BlenderFilmic,
        }
    }
}

impl From<Tonemap> for Tonemapping {
    fn from(value: Tonemap) -> Self {
        value.to_tonemapping()
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CameraFollowSettings {
    #[serde(default = "CameraFollowSettings::default_distance")]
    pub distance: f32,

    #[serde(default = "CameraFollowSettings::default_height")]
    pub height: f32,

    #[serde(default)]
    pub turn_towards: bool,
}

impl Default for CameraFollowSettings {
    fn default() -> Self {
        Self {
            distance: Self::default_distance(),
            height: Self::default_height(),
            turn_towards: false,
        }
    }
}

impl CameraFollowSettings {
    pub const fn default_distance() -> f32 {
        15.0
    }

    pub const fn default_height() -> f32 {
        5.0
    }

    pub fn to_follower(&self) -> Follower {
        Follower {
            turn_towards: self.turn_towards,
            ..Default::default()
        }
    }
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
    pub camera: CameraSettings,

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
