//! Слой изменяемых параметров игры.
//!
//! Параметры вынесены из [`Config`](crate::config::Config) в отдельные `Reflect`-компоненты
//! и ресурсы, зарегистрированные в реестре типов. Конфиг остаётся источником начальных
//! значений и форматом сохранения, а компоненты параметров — источником истины во время игры.
//!
//! Системы применения ([`ApplyParamsSet`]) переносят значения параметров в компоненты движка
//! и выполняются только при фактическом изменении параметров (`Changed<T>` / `Res::is_changed`),
//! поэтому в кадре без правок их стоимость сводится к проверке тиков изменений.
//!
//! Регистрация типов делает параметры доступными как встроенному инспектору
//! (`WorldInspectorPlugin`), так и любому внешнему редактору, работающему через reflect.

use bevy::app::{App, Plugin, PreUpdate};
use bevy::ecs::schedule::{IntoScheduleConfigs, SystemSet};

use crate::camera::panorbit::{PanOrbitCamera, PanOrbitCameraTarget};
use crate::camera::simple::SimpleCamera;
use crate::camera::{self, CameraParams};
use crate::environment::{self, AmbientParams, Sun, SunParams};
use crate::follow::{Followee, Follower, PreviousTransform};
use crate::state::ingame::aircraft::{Aircraft, AircraftParams, Movement, Thrust, ThrustParams};
use crate::state::ingame::engine::{self, FlickeringLight, FlickeringParams, JetFireParams};
use crate::state::ingame::terrain::{self, Terrain, TerrainParams};

/// Набор систем, переносящих параметры в компоненты движка.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ApplyParamsSet;

pub struct ParamsPlugin;

impl Plugin for ParamsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmbientParams>()
            .init_resource::<CameraParams>()
            // Параметры
            .register_type::<AircraftParams>()
            .register_type::<AmbientParams>()
            .register_type::<CameraParams>()
            .register_type::<FlickeringParams>()
            .register_type::<JetFireParams>()
            .register_type::<SunParams>()
            .register_type::<TerrainParams>()
            .register_type::<ThrustParams>()
            // Прочие компоненты игры: нужны редактору, чтобы находить сущности и видеть состояние
            .register_type::<Aircraft>()
            .register_type::<FlickeringLight>()
            .register_type::<Followee>()
            .register_type::<Follower>()
            .register_type::<Movement>()
            .register_type::<PanOrbitCamera>()
            .register_type::<PanOrbitCameraTarget>()
            .register_type::<PreviousTransform>()
            .register_type::<SimpleCamera>()
            .register_type::<Sun>()
            .register_type::<Terrain>()
            .register_type::<Thrust>()
            .add_systems(
                PreUpdate,
                (
                    environment::apply_ambient,
                    environment::apply_sun,
                    camera::apply_camera_params,
                    engine::apply_jet_fire,
                    terrain::apply_terrain,
                )
                    .in_set(ApplyParamsSet),
            );
    }
}
