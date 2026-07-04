use crate::renderer::GpuRenderer;
use crate::renderer::Uniforms;

fn as_u8_slice<T>(data: &[T]) -> &[u8] {
    // SAFETY: Reinterprets `&[T]` as `&[u8]` for GPU buffer upload.
    // T is a primitive type with no padding, and the byte layout is stable.
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data)) }
}

fn as_u8_value<T>(data: &T) -> &[u8] {
    // SAFETY: Reinterprets a single `&T` as `&[u8]` for GPU buffer upload.
    // T is a primitive type with no padding, and the byte layout is stable.
    unsafe { std::slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of::<T>()) }
}

impl GpuRenderer {
    pub fn render(
        &mut self,
        heightmap: &[i16; 16384],
        terrain: &[u8; 16384],
        show_heightmap: bool,
    ) {
        if !self.needs_upload
            && heightmap == &self.last_heightmap[..]
            && terrain == &self.last_terrain[..]
            && show_heightmap == self.last_show_heightmap
        {
            return;
        }

        self.last_heightmap.copy_from_slice(heightmap);
        self.last_terrain.copy_from_slice(terrain);
        self.last_show_heightmap = show_heightmap;
        self.needs_upload = false;

        let min_height = *heightmap.iter().min().unwrap_or(&0);
        let max_height = *heightmap.iter().max().unwrap_or(&255);

        let packed_heightmap = Self::pack_heightmap(heightmap);
        self.queue
            .write_buffer(&self.heightmap_buffer, 0, as_u8_slice(&packed_heightmap));

        let packed_terrain = Self::pack_terrain(terrain);
        self.queue
            .write_buffer(&self.terrain_buffer, 0, as_u8_slice(&packed_terrain));

        let uniforms = Uniforms {
            min_height: min_height as i32,
            max_height: max_height as i32,
            show_heightmap: if show_heightmap { 1 } else { 0 },
            tile_size: 128,
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, as_u8_value(&uniforms));

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("terrain_compute_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.bind_group, &[]);
            cpass.dispatch_workgroups(8, 8, 1);
        }

        let bytes_per_row = self.width * 4;
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));
    }

    pub fn register_texture(&mut self, renderer: &mut egui_wgpu::Renderer) {
        let id = renderer.register_native_texture(
            &self.device,
            &self.output_texture_view,
            wgpu::FilterMode::Linear,
        );
        self.texture_id = Some(id);
    }

    pub fn update_texture(&mut self, renderer: &mut egui_wgpu::Renderer) {
        if let Some(id) = self.texture_id {
            renderer.update_egui_texture_from_wgpu_texture(
                &self.device,
                &self.output_texture_view,
                wgpu::FilterMode::Linear,
                id,
            );
        }
    }

    pub fn mark_dirty(&mut self) {
        self.needs_upload = true;
    }
}
