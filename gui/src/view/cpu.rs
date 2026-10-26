use crate::app::{
    LAYER_BIOME_IDX, LAYER_CAVES_IDX, LAYER_FROST_IDX, LAYER_RESOURCES_IDX, LAYER_RIVER_IDX,
    LAYER_TREES_IDX,
};
use egui::{Color32, ColorImage, TextureOptions, Vec2};
use terrafier_core::model::layers::{
    LAYER_BIOME, LAYER_CAVES, LAYER_FROST, LAYER_RESOURCES, LAYER_RIVER, LAYER_TREES,
};
use terrafier_core::model::tile::{LayerBuffer, TILE_SIZE};

const LAYER_IDS: [u32; 6] = [
    LAYER_CAVES,
    LAYER_RIVER,
    LAYER_FROST,
    LAYER_TREES,
    LAYER_BIOME,
    LAYER_RESOURCES,
];

fn layer_value(
    tile: &terrafier_core::model::tile::Tile,
    idx: usize,
    layer_idx: usize,
) -> Option<f32> {
    let layer_id = LAYER_IDS[layer_idx];
    let buf = tile.layer_data.get(&layer_id)?;
    match buf {
        LayerBuffer::Bit(data) => {
            let word = data.get(idx / 64)?;
            Some(if (word >> (idx % 64)) & 1 != 0 {
                1.0
            } else {
                0.0
            })
        }
        LayerBuffer::Nibble(data) => {
            let byte = *data.get(idx / 2)?;
            let nib = if idx.is_multiple_of(2) {
                byte & 0x0F
            } else {
                (byte >> 4) & 0x0F
            };
            Some(nib as f32 / 15.0)
        }
        LayerBuffer::Byte(data) => {
            let v = *data.get(idx)? as f32;
            Some(v / 255.0)
        }
        LayerBuffer::Int(_) => None,
    }
}

fn layer_overlay(layer_idx: usize, value: f32) -> Option<(u8, u8, u8)> {
    if value <= 0.0 {
        return None;
    }
    match layer_idx {
        LAYER_CAVES_IDX => Some(((value * 150.0) as u8, 0, 0)),
        LAYER_RIVER_IDX => Some((0, 0, (128.0 + value * 127.0) as u8)),
        LAYER_FROST_IDX => Some((255, 255, 255)),
        LAYER_TREES_IDX => Some((0, (64.0 + value * 191.0) as u8, 0)),
        LAYER_BIOME_IDX => {
            let h = (value * 255.0) as u8;
            Some((h.wrapping_mul(7), h.wrapping_mul(3), h))
        }
        LAYER_RESOURCES_IDX => Some(((value * 255.0) as u8, (value * 200.0) as u8, 0)),
        _ => None,
    }
}

fn blend_overlay(base: (u8, u8, u8), overlay: (u8, u8, u8), alpha: f32) -> (u8, u8, u8) {
    let a = alpha.clamp(0.0, 1.0);
    (
        (base.0 as f32 * (1.0 - a) + overlay.0 as f32 * a) as u8,
        (base.1 as f32 * (1.0 - a) + overlay.1 as f32 * a) as u8,
        (base.2 as f32 * (1.0 - a) + overlay.2 as f32 * a) as u8,
    )
}

const TERRAIN_COLORS: [Color32; 7] = [
    Color32::from_rgb(194, 178, 128), // Desert
    Color32::from_rgb(124, 189, 107), // Grass
    Color32::from_rgb(86, 140, 74),   // Forest
    Color32::from_rgb(128, 128, 128), // Rock
    Color32::from_rgb(227, 212, 160), // Sand
    Color32::from_rgb(72, 107, 75),   // Swamp
    Color32::from_rgb(64, 128, 255),  // Water
];

pub fn render(ui: &mut egui::Ui, app: &mut crate::app::TerrafierApp) {
    let Some(ref world) = app.world else {
        return;
    };
    let dim = &world.dimensions[0];

    let mut min_tx = i32::MAX;
    let mut max_tx = i32::MIN;
    let mut min_tz = i32::MAX;
    let mut max_tz = i32::MIN;
    for &(tx, tz) in dim.tiles.keys() {
        min_tx = min_tx.min(tx);
        max_tx = max_tx.max(tx);
        min_tz = min_tz.min(tz);
        max_tz = max_tz.max(tz);
    }

    let base_size = 32u32;
    let zoom = app.view_zoom;

    let grid_w = ((max_tx - min_tx + 1) * base_size as i32) as usize;
    let grid_h = ((max_tz - min_tz + 1) * base_size as i32) as usize;

    let mut img_data = vec![0u8; grid_w * grid_h * 4];

    for (&(tx, tz), tile) in &dim.tiles {
        let px = ((tx - min_tx) * base_size as i32) as usize;
        let pz = ((tz - min_tz) * base_size as i32) as usize;

        for ly in 0..base_size as usize {
            for lx in 0..base_size as usize {
                let sx = (lx * TILE_SIZE / base_size as usize).min(TILE_SIZE - 1);
                let sz = (ly * TILE_SIZE / base_size as usize).min(TILE_SIZE - 1);
                let idx = sz * TILE_SIZE + sx;

                let terrain_type = tile.terrain[idx] as usize;
                let base = TERRAIN_COLORS[terrain_type.min(6)];

                let h = tile.heightmap[idx];
                let height_factor = 0.7 + 0.3 * ((h + 64) as f32 / 384.0);

                let (mut r, mut g, mut b) = if app.show_heightmap {
                    let norm = ((h + 64) as f32 / 384.0).clamp(0.0, 1.0);
                    if norm < 0.25 {
                        let t = norm / 0.25;
                        ((t * 255.0) as u8, (t * 128.0) as u8, 255)
                    } else if norm < 0.5 {
                        let t = (norm - 0.25) / 0.25;
                        (0, (128.0 + t * 127.0) as u8, ((1.0 - t) * 255.0) as u8)
                    } else if norm < 0.75 {
                        let t = (norm - 0.5) / 0.25;
                        ((t * 255.0) as u8, 255, 0)
                    } else {
                        let t = (norm - 0.75) / 0.25;
                        (255, ((1.0 - t) * 255.0) as u8, 0)
                    }
                } else {
                    let r = (base.r() as f32 * height_factor).min(255.0) as u8;
                    let g = (base.g() as f32 * height_factor).min(255.0) as u8;
                    let b = (base.b() as f32 * height_factor).min(255.0) as u8;
                    (r, g, b)
                };

                for li in 0..6 {
                    if app.layer_visible[li]
                        && let Some(val) = layer_value(tile, idx, li)
                        && let Some(over) = layer_overlay(li, val)
                    {
                        let alpha = if val > 0.0 {
                            (val * 0.6 + 0.2).min(0.8)
                        } else {
                            0.0
                        };
                        (r, g, b) = blend_overlay((r, g, b), over, alpha);
                    }
                }

                let di = (pz + ly) * grid_w + (px + lx);
                img_data[di * 4] = r;
                img_data[di * 4 + 1] = g;
                img_data[di * 4 + 2] = b;
                img_data[di * 4 + 3] = 255;
            }
        }
    }

    let color_image = ColorImage::from_rgba_unmultiplied([grid_w, grid_h], &img_data);
    let texture_id = if let Some(ref mut handle) = app.cpu_texture_handle {
        handle.set(color_image, TextureOptions::LINEAR);
        handle.id()
    } else {
        let handle = ui
            .ctx()
            .load_texture("world_map", color_image, TextureOptions::LINEAR);
        let id = handle.id();
        app.cpu_texture_handle = Some(handle);
        id
    };

    // Viewport: allocate a large area for the world with scroll
    let viewport_size = Vec2::new(grid_w as f32 * zoom, grid_h as f32 * zoom);
    let (response, painter) = ui.allocate_painter(viewport_size, egui::Sense::click_and_drag());

    // Handle scroll-wheel zoom
    let scroll_delta = ui.input(|i| i.raw_scroll_delta);
    if scroll_delta.y != 0.0 {
        app.view_zoom = (app.view_zoom * (1.0 + scroll_delta.y * 0.002)).clamp(0.1, 10.0);
    }

    // Handle drag-to-pan
    if response.dragged() {
        app.view_offset += response.drag_delta();
    }

    // Recalculate scaled size after potential zoom change
    let scaled_size = base_size as f32 * app.view_zoom;
    let offset = app.view_offset;

    let rect = response.rect;
    painter.image(
        texture_id,
        egui::Rect::from_min_max(
            egui::pos2(rect.min.x + offset.x, rect.min.y + offset.y),
            egui::pos2(
                rect.min.x + offset.x + grid_w as f32 * app.view_zoom,
                rect.min.y + offset.y + grid_h as f32 * app.view_zoom,
            ),
        ),
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    if let Some((sel_tx, sel_tz)) = app.selected_tile
        && sel_tx >= min_tx
        && sel_tx <= max_tx
        && sel_tz >= min_tz
        && sel_tz <= max_tz
    {
        let bx = rect.min.x + offset.x + (sel_tx - min_tx) as f32 * scaled_size;
        let bz = rect.min.y + offset.y + (sel_tz - min_tz) as f32 * scaled_size;
        let border_rect = egui::Rect::from_min_size(egui::pos2(bx, bz), Vec2::splat(scaled_size));
        painter.rect_stroke(
            border_rect,
            0.0,
            egui::Stroke::new(3.0_f32, Color32::WHITE),
            egui::StrokeKind::Middle,
        );
    }

    if let (Some((sel_tx, sel_tz)), Some(bx), Some(bz)) =
        (app.selected_tile, app.brush_local_x, app.brush_local_z)
    {
        let tile_x_in_pixels = (sel_tx - min_tx) as f32 * scaled_size;
        let tile_z_in_pixels = (sel_tz - min_tz) as f32 * scaled_size;
        let brush_x =
            rect.min.x + offset.x + tile_x_in_pixels + (bx as f32 * scaled_size / TILE_SIZE as f32);
        let brush_z =
            rect.min.y + offset.y + tile_z_in_pixels + (bz as f32 * scaled_size / TILE_SIZE as f32);
        let brush_radius_px = app.brush_radius as f32 * scaled_size / TILE_SIZE as f32;
        painter.circle_stroke(
            egui::pos2(brush_x, brush_z),
            brush_radius_px.max(2.0),
            egui::Stroke::new(1.5_f32, Color32::YELLOW),
        );
    }

    // Click-to-select with inverse transform
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let lx = pos.x - rect.min.x - offset.x;
        let lz = pos.y - rect.min.y - offset.y;
        if lx >= 0.0 && lz >= 0.0 {
            let tx = (lx / scaled_size).floor() as i32 + min_tx;
            let tz = (lz / scaled_size).floor() as i32 + min_tz;
            if dim.tiles.contains_key(&(tx, tz)) {
                app.selected_tile = Some((tx, tz));
                let local_x = ((lx as u32 % scaled_size as u32) * TILE_SIZE as u32
                    / scaled_size as u32)
                    .min(127);
                let local_z = ((lz as u32 % scaled_size as u32) * TILE_SIZE as u32
                    / scaled_size as u32)
                    .min(127);
                app.brush_local_x = Some(local_x);
                app.brush_local_z = Some(local_z);
                app.status_message = format!(
                    "Selected tile ({}, {}) at local ({}, {})",
                    tx, tz, local_x, local_z
                );
            }
        }
    }
}
