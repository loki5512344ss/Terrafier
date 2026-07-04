use std::sync::Arc;

const TILE_SIZE: u32 = 128;
const TILE_CELLS: u32 = TILE_SIZE * TILE_SIZE;
const HEIGHTMAP_PACKED_SIZE: u64 = (TILE_CELLS / 2) as u64 * 4;
const TERRAIN_PACKED_SIZE: u64 = (TILE_CELLS / 4) as u64 * 4;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Uniforms {
    pub(crate) min_height: i32,
    pub(crate) max_height: i32,
    pub(crate) show_heightmap: u32,
    pub(crate) tile_size: u32,
}

pub struct GpuRenderer {
    pub(crate) device: Arc<wgpu::Device>,
    pub(crate) queue: Arc<wgpu::Queue>,
    pub(crate) compute_pipeline: wgpu::ComputePipeline,
    pub(crate) heightmap_buffer: wgpu::Buffer,
    pub(crate) terrain_buffer: wgpu::Buffer,
    pub(crate) uniform_buffer: wgpu::Buffer,
    pub(crate) output_buffer: wgpu::Buffer,
    pub(crate) output_texture: wgpu::Texture,
    pub(crate) output_texture_view: wgpu::TextureView,
    pub(crate) bind_group: wgpu::BindGroup,
    pub texture_id: Option<egui::TextureId>,
    pub width: u32,
    pub height: u32,
    pub(crate) last_heightmap: Vec<i16>,
    pub(crate) last_terrain: Vec<u8>,
    pub(crate) last_show_heightmap: bool,
    pub(crate) needs_upload: bool,
}

impl GpuRenderer {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        adapter: &wgpu::Adapter,
        width: u32,
        height: u32,
        shader_source: &str,
    ) -> Self {
        let limits = adapter.limits();
        let needed_storage: u64 = 65536;
        if limits.max_buffer_size < needed_storage {
            log::warn!(
                "Adapter buffer size limit ({}) < needed ({})",
                limits.max_buffer_size,
                needed_storage,
            );
        }
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("terrain_compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let heightmap_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heightmap_buffer"),
            size: HEIGHTMAP_PACKED_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let terrain_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain_buffer"),
            size: TERRAIN_PACKED_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let output_size = (width * height * 4) as u64;
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output_buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        });
        let output_texture_view = output_texture.create_view(&Default::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &heightmap_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &terrain_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &output_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        Self {
            device,
            queue,
            compute_pipeline,
            heightmap_buffer,
            terrain_buffer,
            uniform_buffer,
            output_buffer,
            output_texture,
            output_texture_view,
            bind_group,
            texture_id: None,
            width,
            height,
            last_heightmap: vec![0i16; 16384],
            last_terrain: vec![0u8; 16384],
            last_show_heightmap: false,
            needs_upload: true,
        }
    }

    pub(crate) fn pack_heightmap(heightmap: &[i16; 16384]) -> Vec<u32> {
        heightmap
            .chunks(2)
            .map(|chunk| {
                let low = chunk[0] as u32 & 0xFFFF;
                let high = chunk[1] as u32 & 0xFFFF;
                low | (high << 16)
            })
            .collect()
    }

    pub(crate) fn pack_terrain(terrain: &[u8; 16384]) -> Vec<u32> {
        terrain
            .chunks(4)
            .map(|chunk| {
                chunk[0] as u32
                    | (chunk[1] as u32) << 8
                    | (chunk[2] as u32) << 16
                    | (chunk[3] as u32) << 24
            })
            .collect()
    }
}
