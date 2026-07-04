use std::collections::VecDeque;

use terrafier_core::World;
use terrafier_core::model::types::Terrain;
use terrafier_core::ops::operations::Operation;

use crate::renderer::GpuRenderer;

mod ops;
mod update;

pub enum GpuStatus {
    Enabled,
    FallbackCpu(&'static str),
}

#[derive(Clone, Copy, PartialEq)]
pub enum ToolMode {
    Raise,
    Lower,
    Smooth,
    Flatten,
    Paint,
    Erode,
    Fill,
    Inspect,
}

impl ToolMode {
    pub fn name(&self) -> &'static str {
        match self {
            ToolMode::Raise => "Raise",
            ToolMode::Lower => "Lower",
            ToolMode::Smooth => "Smooth",
            ToolMode::Flatten => "Flatten",
            ToolMode::Paint => "Paint",
            ToolMode::Erode => "Erode",
            ToolMode::Fill => "Fill",
            ToolMode::Inspect => "Inspect",
        }
    }
}

pub const LAYER_CAVES_IDX: usize = 0;
pub const LAYER_RIVER_IDX: usize = 1;
pub const LAYER_FROST_IDX: usize = 2;
pub const LAYER_TREES_IDX: usize = 3;
pub const LAYER_BIOME_IDX: usize = 4;
pub const LAYER_RESOURCES_IDX: usize = 5;
pub const LAYER_NAMES: [&str; 6] = ["Caves", "River", "Frost", "Trees", "Biome", "Resources"];

pub struct TerrafierApp {
    pub world: Option<World>,
    pub selected_tile: Option<(i32, i32)>,
    pub tool_mode: ToolMode,
    pub brush_radius: u32,
    pub brush_strength: f64,
    pub selected_terrain: Terrain,
    pub target_height: i16,
    pub smooth_iterations: u32,
    pub erode_iterations: u32,
    pub erode_talus_angle: f64,
    pub undo_stack: VecDeque<Box<dyn Operation>>,
    pub redo_stack: Vec<Box<dyn Operation>>,
    pub brush_local_x: Option<u32>,
    pub brush_local_z: Option<u32>,
    pub show_heightmap: bool,
    pub show_new_world: bool,
    pub show_export: bool,
    pub status_message: String,
    pub world_name: String,
    pub world_seed: String,
    pub export_path: String,
    pub renderer: Option<GpuRenderer>,
    pub show_gpu_render: bool,
    pub gpu_status: GpuStatus,
    pub view_offset: egui::Vec2,
    pub view_zoom: f32,
    pub cpu_texture_handle: Option<egui::TextureHandle>,
    pub layer_visible: [bool; 6],
}

impl TerrafierApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let (renderer, gpu_status) = if let Some(render_state) = &cc.wgpu_render_state {
            let device: std::sync::Arc<wgpu::Device> = render_state.device.clone().into();
            let queue: std::sync::Arc<wgpu::Queue> = render_state.queue.clone().into();
            let adapter: std::sync::Arc<wgpu::Adapter> = render_state.adapter.clone().into();
            let mut gpu_renderer = GpuRenderer::new(
                device,
                queue,
                &adapter,
                128,
                128,
                include_str!("../renderer/shaders.wgsl"),
            );
            let mut renderer_lock = render_state.renderer.write();
            gpu_renderer.register_texture(&mut renderer_lock);
            (Some(gpu_renderer), GpuStatus::Enabled)
        } else {
            (
                None,
                GpuStatus::FallbackCpu("GPU init failed — no wgpu adapter found"),
            )
        };

        Self {
            world: None,
            selected_tile: None,
            tool_mode: ToolMode::Raise,
            brush_radius: 16,
            brush_strength: 0.5,
            selected_terrain: Terrain::Grass,
            target_height: 64,
            smooth_iterations: 3,
            erode_iterations: 3,
            erode_talus_angle: 0.5,
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            brush_local_x: None,
            brush_local_z: None,
            show_heightmap: false,
            show_new_world: false,
            show_export: false,
            status_message: "Ready".to_string(),
            world_name: String::new(),
            world_seed: String::new(),
            export_path: String::new(),
            renderer,
            show_gpu_render: true,
            gpu_status,
            view_offset: egui::Vec2::ZERO,
            view_zoom: 1.0,
            cpu_texture_handle: None,
            layer_visible: [false; 6],
        }
    }
}
