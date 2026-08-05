Я обновил bevy с версии 0.18 на 0.19 и в игре после запуска небо мерцает странными фигурами с пилообразными краями: становится то чёрным, то тёмно-синим. В прошлой версии такого не было, небо было дневным и без странных мерцаний.

---

● Исправлено. Причина — изменение API атмосферы в Bevy 0.19, а не баг шейдера.

  Что произошло
  
  В 0.18 Atmosphere вешался на камеру, а масштаб сцены задавался через AtmosphereSettings::scene_units_to_m.
  
  В 0.19 (bevy_light-0.19.0/src/atmosphere.rs:24-31):
  
  ▎ Atmosphere for one planet. The entity's GlobalTransform is the planet center in world space. Add AtmosphereSettings to each 3D camera that should use it.
  
  То есть Atmosphere теперь — отдельная сущность, чей GlobalTransform задаёт центр планеты, а на камеру идёт только AtmosphereSettings. Поле scene_units_to_m удалено полностью (заменено на scale в трансформе).
  
  Код продолжал вставлять Atmosphere в камеру (src/camera.rs:189). Экстрактор extract_atmosphere берёт GlobalTransform этой сущности как центр планеты (bevy_pbr-0.19.0/src/atmosphere/mod.rs:248), поэтому камера
  оказывалась ровно в центре Земли: r ≈ 0 при inner_radius = 6 360 000. Шейдер уходил в ветку if r < atmosphere.inner_radius (functions.wgsl:396), а центр планеты ещё и ездил/вращался вместе с камерой каждый
  кадр — отсюда чёрно-синее мерцание с пилообразными краями.
  
  Хук set_default_transform (который сам ставит планету на -Y * inner_radius) тут не спасал: он срабатывает только если GlobalTransform ещё дефолтный, а у камеры он свой и перезаписывается пропагацией.
  
  Что сделано
  
  - src/camera.rs: новая система spawn_atmosphere создаёт отдельную сущность с Atmosphere и Transform::from_translation(Vec3::NEG_Y * inner_radius) — планета под началом координат, сцена на её поверхности, up =
  Vec3::Y. Scale остался 1, что эквивалентно прежнему scene_units_to_m: 1.0. 
  - spawn_panorbit теперь кладёт на камеру только AtmosphereSettings. Заодно из него и из respawn_panorbit ушёл ненужный ResMut<Assets<ScatteringMedium>> — атмосфера создаётся один раз на Startup и переживает
  пересоздание камеры при смене состояния (Hangar ↔ InGame).
  - src/state/hangar.rs, src/state/ingame.rs: убраны соответствующие параметры и импорты.
  
  Проверка
  
  cargo check чист (остались только прежние предупреждения о мёртвом коде). Запустил игру и снял четыре кадра с интервалами — небо стабильно дневное, голубой градиент с дымкой у горизонта, мерцания и чёрных
  вспышек нет.
  
  Ещё замечание не по теме: в логах много VUID-vkAcquireNextImageKHR-semaphore-01286 и VUID-VkPresentInfoKHR-pImageIndices-01430 от слоёв валидации Vulkan. Это со стороны wgpu/swapchain и к атмосфере отношения
  не имеет, но если раньше их не было — возможно, стоит отдельно посмотреть.

```rust
/// Since Bevy 0.19 the [`Atmosphere`] lives on its own entity, whose [`GlobalTransform`]
/// is the *planet center* — only [`AtmosphereSettings`] goes on the camera. Putting it on
/// the camera would place the camera in the middle of the planet and follow it around,
/// which renders as a flickering black sky.
///
/// The planet is placed `inner_radius` below the origin, so the scene sits on its surface
/// with an up vector of `Vec3::Y`. The scale stays at 1, meaning 1 scene unit = 1 meter
/// (the old `AtmosphereSettings::scene_units_to_m`, which no longer exists).
///
/// [`GlobalTransform`]: bevy::transform::components::GlobalTransform
```

---
