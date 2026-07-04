use crate::app::{GpuStatus, TerrafierApp};

impl eframe::App for TerrafierApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New World").clicked() {
                    self.show_new_world = true;
                }
                if ui.button("Open").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder()
                {
                    match terrafier_core::io::import::import(&path) {
                        Ok(world) => {
                            self.world = Some(world);
                            self.selected_tile = None;
                            self.undo_stack.clear();
                            self.redo_stack.clear();
                            self.status_message = format!("Opened world from {}", path.display());
                        }
                        Err(e) => {
                            self.status_message = format!("Open error: {}", e);
                        }
                    }
                }
                if ui.button("Export").clicked() {
                    self.show_export = true;
                }
                ui.separator();
                if ui.button("Undo").clicked() {
                    self.undo();
                }
                if ui.button("Redo").clicked() {
                    self.redo();
                }
            });
        });

        egui::SidePanel::left("tools")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                crate::view::show_tools_panel(ui, self);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            crate::view::show_viewport(ui, self);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                if let Some((tx, tz)) = self.selected_tile {
                    ui.separator();
                    ui.label(format!("Tile ({}, {})", tx, tz));
                }
                if self.world.is_some() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("World loaded");
                    });
                }
                if matches!(self.gpu_status, GpuStatus::FallbackCpu(_)) {
                    ui.separator();
                    ui.label(" CPU fallback");
                }
            });
        });

        if let Some(ref mut renderer) = self.renderer
            && let Some(ref world) = self.world
            && let Some(tile) = world.dimensions[0].tiles.values().next()
        {
            renderer.render(
                &tile.heightmap.as_slice().try_into().unwrap_or([0; 16384]),
                &tile.terrain.as_slice().try_into().unwrap_or([0; 16384]),
                self.show_heightmap,
            );

            if let Some(render_state) = frame.wgpu_render_state() {
                let mut renderer_lock = render_state.renderer.write();
                if renderer.texture_id.is_none() {
                    renderer.register_texture(&mut renderer_lock);
                } else {
                    renderer.update_texture(&mut renderer_lock);
                }
            }
        }

        crate::dialogs::handle_dialogs(self, ctx);
    }
}
