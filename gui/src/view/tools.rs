use std::sync::Arc;

use terrafier_core::Terrain;
use terrafier_core::model::tile::TILE_SIZE;
use terrafier_core::model::types::SymmetricBrush;
use terrafier_core::ops::operations::{
    ErodeOperation, FillOperation, FlattenOperation, HeightOperation, MultiTileOperation,
    Operation, PaintOperation, SmoothOperation,
};

use crate::app::{LAYER_NAMES, TerrafierApp, ToolMode};

pub fn show_tools_panel(ui: &mut egui::Ui, app: &mut TerrafierApp) {
    ui.heading("Tools");
    ui.separator();

    ui.label("Mode:");
    ui.radio_value(&mut app.tool_mode, ToolMode::Raise, "Raise");
    ui.radio_value(&mut app.tool_mode, ToolMode::Lower, "Lower");
    ui.radio_value(&mut app.tool_mode, ToolMode::Smooth, "Smooth");
    ui.radio_value(&mut app.tool_mode, ToolMode::Flatten, "Flatten");
    ui.radio_value(&mut app.tool_mode, ToolMode::Paint, "Paint");
    ui.radio_value(&mut app.tool_mode, ToolMode::Erode, "Erode");
    ui.radio_value(&mut app.tool_mode, ToolMode::Fill, "Fill");
    ui.radio_value(&mut app.tool_mode, ToolMode::Inspect, "Inspect");

    ui.separator();
    ui.add(egui::Slider::new(&mut app.brush_radius, 1..=64).text("Radius"));
    ui.add(egui::Slider::new(&mut app.brush_strength, 0.0..=1.0).text("Strength"));

    if app.tool_mode == ToolMode::Smooth {
        ui.add(egui::Slider::new(&mut app.smooth_iterations, 1..=20).text("Iterations"));
    }

    if app.tool_mode == ToolMode::Flatten {
        ui.add(egui::Slider::new(&mut app.target_height, -64..=320).text("Height"));
    }

    if app.tool_mode == ToolMode::Erode {
        ui.add(egui::Slider::new(&mut app.erode_iterations, 1..=20).text("Iterations"));
        ui.add(egui::Slider::new(&mut app.erode_talus_angle, 0.0..=1.0).text("Talus angle"));
    }

    if app.tool_mode == ToolMode::Paint || app.tool_mode == ToolMode::Fill {
        ui.label("Terrain:");
        egui::ComboBox::from_label("Type")
            .selected_text(app.selected_terrain.name())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.selected_terrain, Terrain::Desert, "Desert");
                ui.selectable_value(&mut app.selected_terrain, Terrain::Grass, "Grass");
                ui.selectable_value(&mut app.selected_terrain, Terrain::Forest, "Forest");
                ui.selectable_value(&mut app.selected_terrain, Terrain::Rock, "Rock");
                ui.selectable_value(&mut app.selected_terrain, Terrain::Sand, "Sand");
                ui.selectable_value(&mut app.selected_terrain, Terrain::Swamp, "Swamp");
                ui.selectable_value(&mut app.selected_terrain, Terrain::Water, "Water");
            });
    }

    ui.separator();
    ui.checkbox(&mut app.show_heightmap, "Heightmap view");

    ui.separator();
    ui.heading("Layers");
    for (i, name) in LAYER_NAMES.iter().enumerate() {
        ui.checkbox(&mut app.layer_visible[i], *name);
    }

    ui.separator();
    if ui.button("Apply to selected tile").clicked() {
        apply_tool(app);
    }
}

fn apply_tool(app: &mut TerrafierApp) {
    let Some((tx, tz)) = app.selected_tile else {
        app.status_message = "No tile selected".to_string();
        return;
    };
    if app.world.is_none() {
        return;
    }

    let Some(ref mut world) = app.world else {
        return;
    };
    let Some(dim) = world.overworld_mut() else {
        app.status_message = "World has no overworld".to_string();
        return;
    };

    let center = app.brush_local_x.unwrap_or(64);
    let center_z = app.brush_local_z.unwrap_or(64);

    if app.tool_mode == ToolMode::Inspect {
        if let Some(tile) = dim.tiles.get(&(tx, tz)) {
            let count = tile.terrain.iter().filter(|&&t| t != 0).count();
            let h_min = tile.heightmap.iter().min().unwrap_or(&0);
            let h_max = tile.heightmap.iter().max().unwrap_or(&0);
            app.status_message = format!(
                "Tile ({},{}): {} blocks, height {}-{}",
                tx, tz, count, h_min, h_max
            );
        }
        return;
    }

    if app.tool_mode == ToolMode::Fill {
        if dim.tiles.contains_key(&(tx, tz)) {
            let op = FillOperation {
                tile_x: tx,
                tile_z: tz,
                center_x: center,
                center_z,
                terrain: app.selected_terrain,
                before_snapshot: Default::default(),
            };
            let result = op.apply(dim);
            app.save_for_undo(Box::new(op));
            match result {
                Ok(()) => {
                    app.status_message = format!("Fill applied to ({},{})", tx, tz);
                }
                Err(e) => {
                    app.status_message = format!("Fill error: {:?}", e);
                }
            }
        }
        return;
    }

    let radius = app.brush_radius as i32;
    let global_cx = tx * TILE_SIZE as i32 + center as i32;
    let global_cz = tz * TILE_SIZE as i32 + center_z as i32;

    let min_tx = ((global_cx - radius) >> 7).min(tx);
    let max_tx = ((global_cx + radius) >> 7).max(tx);
    let min_tz = ((global_cz - radius) >> 7).min(tz);
    let max_tz = ((global_cz + radius) >> 7).max(tz);

    let mut ops: Vec<Box<dyn Operation>> = Vec::new();

    for otx in min_tx..=max_tx {
        for otz in min_tz..=max_tz {
            let local_cx =
                (global_cx - otx * TILE_SIZE as i32).clamp(0, TILE_SIZE as i32 - 1) as u32;
            let local_cz =
                (global_cz - otz * TILE_SIZE as i32).clamp(0, TILE_SIZE as i32 - 1) as u32;

            if dim.tiles.contains_key(&(otx, otz)) {
                let brush = Arc::new(SymmetricBrush::new(app.brush_radius as f64));
                let op: Box<dyn Operation> = match app.tool_mode {
                    ToolMode::Raise => Box::new(HeightOperation {
                        tile_x: otx,
                        tile_z: otz,
                        center_x: local_cx,
                        center_z: local_cz,
                        radius: app.brush_radius,
                        delta: 5,
                        brush,
                        before_snapshot: Default::default(),
                    }),
                    ToolMode::Lower => Box::new(HeightOperation {
                        tile_x: otx,
                        tile_z: otz,
                        center_x: local_cx,
                        center_z: local_cz,
                        radius: app.brush_radius,
                        delta: -5,
                        brush,
                        before_snapshot: Default::default(),
                    }),
                    ToolMode::Smooth => Box::new(SmoothOperation {
                        tile_x: otx,
                        tile_z: otz,
                        center_x: local_cx,
                        center_z: local_cz,
                        radius: app.brush_radius,
                        iterations: app.smooth_iterations,
                        brush,
                        before_snapshot: Default::default(),
                    }),
                    ToolMode::Flatten => Box::new(FlattenOperation {
                        tile_x: otx,
                        tile_z: otz,
                        center_x: local_cx,
                        center_z: local_cz,
                        radius: app.brush_radius,
                        target_height: app.target_height,
                        brush,
                        before_snapshot: Default::default(),
                    }),
                    ToolMode::Paint => Box::new(PaintOperation {
                        tile_x: otx,
                        tile_z: otz,
                        center_x: local_cx,
                        center_z: local_cz,
                        radius: app.brush_radius,
                        terrain: app.selected_terrain,
                        brush,
                        before_snapshot: Default::default(),
                    }),
                    ToolMode::Erode => Box::new(ErodeOperation {
                        tile_x: otx,
                        tile_z: otz,
                        center_x: local_cx,
                        center_z: local_cz,
                        radius: app.brush_radius,
                        iterations: app.erode_iterations,
                        talus_angle: app.erode_talus_angle,
                        brush,
                        before_snapshot: Default::default(),
                    }),
                    ToolMode::Fill => unreachable!(),
                    ToolMode::Inspect => unreachable!(),
                };
                ops.push(op);
            }
        }
    }

    if ops.is_empty() {
        app.status_message = "No tiles found in brush range".to_string();
        return;
    }

    let multi_op = MultiTileOperation { operations: ops };

    let result = multi_op.apply(dim);

    app.save_for_undo(Box::new(multi_op));

    match result {
        Ok(()) => {
            app.status_message = format!("Applied {} to ({},{})", app.tool_mode.name(), tx, tz);
        }
        Err(e) => {
            app.status_message = format!("Error: {:?}", e);
        }
    }
}
