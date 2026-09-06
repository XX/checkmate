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

С небом теперь всё нормально, но солнце стало не яркое, просто как белый кружок с размытыми краями, нет ощущения яркого света, когда поворачиваешь на него камеру. Почему так?

---

● Разобрался, причина найдена и подтверждена экспериментально. К атмосфере она отношения не имеет.

  Причина

  В Bevy 0.19 изменили одну функцию в шейдере bloom — karis_average (подавление «светлячков» на первом downsample). PR #22565 «Fix bloom luma calc in sRGB» (https://github.com/bevyengine/bevy/pull/22565). Это
  единственное функциональное изменение во всём модуле bloom между 0.18 и 0.19 — остальное там чисто рефакторинг render graph.

  wgsl
  // 0.18: яркость считалась после гамма-коррекции
  let luma = tonemapping_luminance(pow(color, vec3(1.0/2.2))) / 4.0;
  // 0.19: яркость считается в линейном пространстве
  let luma = tonemapping_luminance(color) / 4.0;

  Вес 1/(1 + luma) в линейном пространстве асимптотически зажимает вклад пикселя: group * 1/(1 + group/4) → ≈4, каким бы ярким ни был источник. Раньше из-за гаммы вклад был ≈ L^0.545 и ничем не ограничивался.

  Для солнца это критично. При illuminance = 50000 и телесном угле SunDisk::EARTH (0.0093 рад → 6.8e-5 ср) радианс диска ≈ 7.4e8, после экспозиции ev100 = 12.5 ≈ 1.1e5. Считаем вклад одной группы:

  ┌──────┬───────────┬───────────────┐
  │      │ вес Karis │ вклад в bloom │
  ├──────┼───────────┼───────────────┤
  │ 0.18 │ 0.028     │ ≈ 1465        │
  ├──────┼───────────┼───────────────┤
  │ 0.19 │ 7.5e-5    │ ≈ 4.0         │
  └──────┴───────────┴───────────────┘
  
  Примерно в 370 раз меньше. Диск остаётся выжженно-белым, а свечение вокруг исчезает — ровно то, что вы описали.
  
  Bevy знает об этом побочном эффекте, гайд лежит прямо в крейте: bevy-0.19.0/_release-content/migration-guides/bloom-luma-fix.md.
  
  Проверка
  
  Я вендорнул bevy_post_process через [patch.crates-io] и вернул одну только строчку karis_average к формуле 0.18. Всё остальное — ваш код, конфиг, настройки — без изменений. Свечение вернулось полностью, ровно
  то, каким вы его помните. Затем убрал патч и вернул Cargo.toml и исходники в исходное состояние; в рабочем дереве сейчас только вчерашнее исправление атмосферы.
                                                                                           
  Что советы из миграционного гайда не решают                                              
                                                                                           
  Гайд предлагает поднять Bloom::intensity, emissive или подкрутить prefilter. Я это померил на вашей сцене:

  - intensity = 0.5 — прирост почти незаметен;                                             
  - intensity = 1.0 — EnergyConserving полностью заменяет кадр размытием, солнце исчезает; 
  - composite_mode = Additive (0.5 и 3.0) — засвечивает весь кадр, диск уходит в чёрное от переполнения;
  - SunDisk::angular_size × 4 — диск больше, ореол по-прежнему слабый.
  
  Это ожидаемо: Karis обрезает энергию по абсолютной величине, поэтому глобальные множители тянут вверх и небо тоже, а соотношение не меняется. Отключить Karis из пользовательского кода нельзя — он безусловный
  под #ifdef FIRST_DOWNSAMPLE, отдельного ключа пайплайна нет. 
  
  Варианты                                                                                 
  
  1. Форк шейдера — держать bevy_post_process (или форк bevy) с прежней karis_average. Единственное, что возвращает картинку один в один; цена — сопровождение форка.
  2. Рисовать корону самому — билборд-квад с радиальным градиентом, ориентированный на солнце, поверх атмосферы. Так делают почти все авиасимы: полный контроль, дешевле bloom, не зависит от изменений в bevy.
  3. Завести issue в bevy. Совет из гайда для физически корректного солнца не работает, а зажим по абсолютной величине выглядит как непреднамеренный побочный эффект. Разумно попросить настраиваемый порог Karis
  или флаг его отключения в Bloom.
  
  Я бы делал вариант 2: он даёт художественный контроль над видом солнца и не привязывает вас к внутренностям bevy. Скажите, какой выберете, — реализую.
  
  Sources:                                                                                 
  - PR #22565 — Fix bloom luma calc in sRGB (https://github.com/bevyengine/bevy/pull/22565)
  - Bevy 0.19 release notes (https://bevy.org/news/bevy-0-19/)
  - Bloom — Unofficial Bevy Cheat Book (https://bevy-cheatbook.github.io/graphics/bloom.html)
  - LearnOpenGL — Physically Based Bloom (https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom)
