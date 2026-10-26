use egui::{Color32, Vec2};
use terrafier_core::model::tile::TILE_SIZE;

pub fn try_render(ui: &mut egui::Ui, app: &mut crate::app::TerrafierApp) -> bool {
    let Some(ref world) = app.world else {
        return false;
    };
    let Some(dim) = world.dimensions.first() else {
        return false;
    };

    // If world has more than one tile, fall back to CPU renderer
    if dim.tiles.len() > 1 {
        return false;
    }

    if let Some(ref renderer) = app.renderer
        && let Some(texture_id) = renderer.texture_id
    {
        let desired_size = Vec2::new(512.0, 512.0);
        let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::click_and_drag());
        let rect = response.rect;

        // Handle scroll-wheel zoom
        let scroll_delta = ui.input(|i| i.raw_scroll_delta);
        if scroll_delta.y != 0.0 {
            app.view_zoom = (app.view_zoom * (1.0 + scroll_delta.y * 0.002)).clamp(0.1, 10.0);
        }

        // Handle drag-to-pan
        if response.dragged() {
            app.view_offset += response.drag_delta();
        }

        let zoom = app.view_zoom;
        let offset = app.view_offset;

        let base_display_size = rect.width() / 4.0;
        let display_size = base_display_size * zoom;

        // Find tile bounds for positioning
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

        // Draw the GPU-rendered tile texture at the correct position
        // GPU only renders the first tile - position it correctly
        // Use a default position for the rendered tile
        let first_tile_pos = dim.tiles.keys().next().copied().unwrap_or((0, 0));
        let tex_origin_x =
            rect.min.x + offset.x + (first_tile_pos.0 - min_tx) as f32 * display_size;
        let tex_origin_y =
            rect.min.y + offset.y + (first_tile_pos.1 - min_tz) as f32 * display_size;

        let tex_rect = egui::Rect::from_min_size(
            egui::pos2(tex_origin_x, tex_origin_y),
            Vec2::splat(display_size),
        );
        painter.image(
            texture_id,
            tex_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        if let Some((sel_tx, sel_tz)) = app.selected_tile {
            let bx = rect.min.x + offset.x + (sel_tx - min_tx) as f32 * display_size;
            let bz = rect.min.y + offset.y + (sel_tz - min_tz) as f32 * display_size;
            let border_rect =
                egui::Rect::from_min_size(egui::pos2(bx, bz), Vec2::splat(display_size));
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
            let tile_x_in_pixels = (sel_tx - min_tx) as f32 * display_size;
            let tile_z_in_pixels = (sel_tz - min_tz) as f32 * display_size;
            let brush_x = rect.min.x
                + offset.x
                + tile_x_in_pixels
                + (bx as f32 * display_size / TILE_SIZE as f32);
            let brush_z = rect.min.y
                + offset.y
                + tile_z_in_pixels
                + (bz as f32 * display_size / TILE_SIZE as f32);
            let brush_radius_px = app.brush_radius as f32 * display_size / TILE_SIZE as f32;
            painter.circle_stroke(
                egui::pos2(brush_x, brush_z),
                brush_radius_px.max(2.0),
                egui::Stroke::new(1.5_f32, Color32::YELLOW),
            );
        }

        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let lx = pos.x - rect.min.x - offset.x;
            let lz = pos.y - rect.min.y - offset.y;
            if lx >= 0.0 && lz >= 0.0 {
                let tx = (lx / display_size).floor() as i32 + min_tx;
                let tz = (lz / display_size).floor() as i32 + min_tz;
                if dim.tiles.contains_key(&(tx, tz)) {
                    app.selected_tile = Some((tx, tz));
                    let local_x = ((lx as u32 % display_size as u32) * TILE_SIZE as u32
                        / display_size as u32)
                        .min(127);
                    let local_z = ((lz as u32 % display_size as u32) * TILE_SIZE as u32
                        / display_size as u32)
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

        return true;
    }
    false
}
