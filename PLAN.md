# Terrafier

> Rust-native Minecraft world painter — быстрее, легче, современнее.

Нативная замена [WorldPainter](https://www.worldpainter.net) на Rust.
Рендеринг карт через GPU, параллельный экспорт, CLI-first архитектура.

## Архитектура

```
┌────────────────────────────────────────────────────┐
│  terrafier-cli    terrafier-gui    Библиотека       │
│  (clap + json)    (egui + eframe)  (crates.io)     │
├───────────────────┬────────────────────────────────┤
│  terrafier-core   │  модель мира, I/O, операции    │
│  ┌──────┬──────┬──┴─────┬────────┐                 │
│  │World │Tile  │Export  │Import  │                 │
│  │Model │Ops   │Pipeline│Anvil   │                 │
│  └──────┴──────┴────────┴────────┘                 │
├──────────────────────┬─────────────────────────────┤
│  wasm-host / rhai    │  Плагины и скриптинг         │
├──────────────────────┴─────────────────────────────┤
│  Foundation crates                                  │
│  nbt │ fastanvil │ noise │ palette-compress         │
│  biome-db                                          │
└────────────────────────────────────────────────────┘
```

## Текущий статус (v0.1.0)

### Реализовано
- **Модель мира**: World, Dimension, Tile (128×128), Terrain (7 типов), Layer trait, Platform
- **Система координат**: явные конвертации BlockCoords → ChunkCoords → RegionCoords → TileCoords
- **NBT**: full read/write, все типы тегов, gzip support
- **Anvil**: чтение/запись .mca регионов, разбор чанков, секций, палитры
- **Экспорт**: Terrafier → Java Edition 1.18+ save (секции, block_states, биомы)
- **Импорт**: Java Edition save → Terrafier world (level.dat, регионы, surface)
- **CLI**: new, import, export, info, render (с прогресс-барами, dry-run)
- **GUI**: редактор на egui (viewport, инструменты, undo/redo)
- **Операции**: Raise, Lower, Smooth, Flatten, Paint — с MultiTile поддержкой
- **Heightmap**: noise-based генерация (OpenSimplex), flat, combined

### В работе / не реализовано
- **WASM плагины** — wasmtime-хост, ABI, загрузка .wasm файлов
- **Скриптинг** — Rhai-движок для world-генераторов
- **i18n** — локализация CLI/GUI
- **GUI polish** — pan/zoom GPU viewport, brush overlay, layer panel

## Стек

| Компонент | Технология |
|-----------|-----------|
| Язык | Rust (edition 2024) |
| GUI | egui + eframe |
| GPU | (план) wgpu |
| NBT | Самописный `nbt` crate |
| Anvil | Самописный `fastanvil` crate |
| Шум | `noise-rs` |
| Параллелизм | rayon |
| CLI | clap + indicatif |
| Изображения | `image` crate |
| Сериализация | serde + bincode |

### Почему egui, а не Tauri

egui выбран за:
- Immediate mode — нет оверхэда как у React, подходит для тулов
- Нативная интеграция с Rust (без bridge)
- wgpu-рендеринг напрямую (когда будет реализован)
- Бинарник 2–3 MB vs Tauri 5–10 MB + WebView

## Roadmap

### Phase 0 (done) — Foundation
NBT парсер, Anvil reader, workspace setup

### Phase 1 (done) — Core Model
Tile, Dimension, World, Terrain, Brush, Operation

### Phase 2 (done) — Minecraft I/O + CLI
Импорт/экспорт Java Edition, CLI команды, импорт .world

### Phase 3 (done) — Operations & Layers
Инструменты (raise, erode, smooth, flatten, fill, paint)
Слои (caves, river, frost, trees, biome, resources)
Экспортёры слоёв, фильтры, MultiTile

### Phase 4 (done) — GUI (MVP)
GPU-рендеринг через wgpu (базовый), панель инструментов
Диалоги (New World, Export), undo/redo

### Phase 5 — WASM Plugins + Scripting
WASM-хост на wasmtime, ABI для плагинов
Rhai-скриптинг для world-генераторов
Загрузка плагинов из .wasm файлов, registry

### Phase 6 — i18n + Инфраструктура
Локализация CLI/GUI (rust-i18n / Fluent)
CI/CD, cargo-dist, AppImage/msi/app
Бенчмарки, оптимизация, polish

### Будущее
Bedrock Edition, GUI pan/zoom/brush-overlay, GUI layer panel

## Модель данных

```rust
struct Tile {                    // 128×128 блоков
    heightmap: [i16; 16384],     // карта высот
    terrain: [u8; 16384],        // тип террейна
    water_level: [u8; 16384],    // уровень воды
    layer_data: HashMap<u32, LayerBuffer>,
}

struct Dimension {
    tiles: HashMap<(i32, i32), Tile>,
    min_height, max_height: i16,
    seed: u64,
}

struct World {
    name: String,
    dimensions: Vec<Dimension>,
    platform: Platform,
    seed: u64,
}
```

## Известные проблемы (требуют фикса)

### Критические
1. **GPU рендерит только 1 тайл** — `GpuRenderer` 128×128, а вьюпорт показывает мульти-тайловый мир. `gpu.rs` рисует один тайл, остальные пустые.
2. **Undo/redo сломан** — `OnceLock` в операциях не позволяет повторно захватить снепшот при redo. После undo→redo мир ломается.
3. **Импорт высот неверный** — `reader.rs:122` всегда ставит surface_y = section_y * 16 + 15, игнорируя реальную высоту поверхности.
4. **Brush strength не влияет на Paint** — `apply_brush_terrain()` игнорирует силу кисти, кроме проверки на 0.
5. **Макрос `t!()` сломан** — `i18n/src/lib.rs:61`: невалидный каст, макрос с переменными не компилируется.

### Существенные
6. **level.dat при экспорте неполный** — нет `GameRules`, `Difficulty`, `Spawn`, `Time`, `RandomSeed`.
7. **Нет поддержки Nether/End** — экспорт всегда в `region/`, импорт создаёт только `"overworld"`.
8. **`FloodOperation` — мёртвый код** — дубликат `PaintOperation`, нигде не используется.
9. **`Dimension` сериализация хрупкая** — строковые ключи `"{tx},{tz}"`, битые ключи молча дропаются.
10. **CPU-вьюпорт рендерит всё каждый кадр** — полный перерендер при каждом `update()`.
11. **`wasm-host` `host_get_seed` бессмысленный** — разбивает u64 на lo/hi 32-bit и собирает обратно.

### Архитектурные
12. **Только 1 тест на 93 .rs файла** — операции, импорт, экспорт, слои без покрытия.
13. **Экспорт блоков примитивный** — 9 типов блоков, без свойств (snowy, waterlogged).
14. **`data_version` хардкод** — всегда экспортит 3954, игнорируя версию исходного мира.
15. **GPU buffer sizes хардкод** — `8192 * 4` и `4096 * 4` без Named constants.
