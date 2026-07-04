use std::path::Path;

use rhai::{Dynamic, Engine, EvalAltResult};

use terrafier_core::World;
use terrafier_core::ops::heightmap::{FlatHeightMap, HeightMapSource, NoiseHeightMap};
use terrafier_core::ops::layers::generate_all_layers;

pub fn run_script(path: &Path) -> anyhow::Result<()> {
    let script = std::fs::read_to_string(path)?;
    let mut engine = ScriptEngine::new();
    engine
        .run(&script)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("Script error: {}", e))
}

pub struct ScriptEngine {
    engine: Engine,
}

fn new_world_fn(name: &str, seed: i64) -> World {
    World::new(name, seed as u64)
}

fn save_world_fn(world: &mut World, path: &str) -> Result<(), Box<EvalAltResult>> {
    terrafier_core::io::save_binary(world, Path::new(path)).map_err(|e| e.to_string().into())
}

fn info_fn(world: &mut World) -> String {
    let tile_count: usize = world.dimensions.iter().map(|d| d.tiles.len()).sum();
    let dim_names: Vec<&str> = world.dimensions.iter().map(|d| d.name.as_str()).collect();
    format!(
        "World: {} seed: {} dimensions: [{}] tiles: {}",
        world.name,
        world.seed,
        dim_names.join(", "),
        tile_count
    )
}

fn generate_heightmap_flat_fn(
    world: &mut World,
    height: i64,
    dim_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    let dim = world
        .dimensions
        .iter_mut()
        .find(|d| d.name == dim_name)
        .ok_or_else(|| format!("Dimension '{}' not found", dim_name))?;
    let flat = FlatHeightMap {
        height: height as i16,
    };
    for (_, tile) in &mut dim.tiles {
        flat.generate(tile, world.seed);
    }
    Ok(())
}

fn generate_heightmap_noise_fn(
    world: &mut World,
    seed: i64,
    amplitude: f64,
    _octaves: i64,
    dim_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    let dim = world
        .dimensions
        .iter_mut()
        .find(|d| d.name == dim_name)
        .ok_or_else(|| format!("Dimension '{}' not found", dim_name))?;
    let noise = NoiseHeightMap {
        base_height: 63.0,
        amplitude,
        frequency: 0.01,
        scale_x: 1.0,
        scale_z: 1.0,
    };
    for (_, tile) in &mut dim.tiles {
        noise.generate(tile, seed as u64);
    }
    Ok(())
}

fn apply_smooth_fn(
    world: &mut World,
    _iterations: i64,
    _strength: f64,
    dim_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    world
        .dimensions
        .iter_mut()
        .find(|d| d.name == dim_name)
        .ok_or_else(|| format!("Dimension '{}' not found", dim_name))?;
    Ok(())
}

fn apply_erode_fn(
    world: &mut World,
    _iterations: i64,
    _strength: f64,
    dim_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    world
        .dimensions
        .iter_mut()
        .find(|d| d.name == dim_name)
        .ok_or_else(|| format!("Dimension '{}' not found", dim_name))?;
    Ok(())
}

fn generate_layers_fn(world: &mut World, dim_name: &str) -> Result<(), Box<EvalAltResult>> {
    let dim = world
        .dimensions
        .iter_mut()
        .find(|d| d.name == dim_name)
        .ok_or_else(|| format!("Dimension '{}' not found", dim_name))?;
    for (_, tile) in &mut dim.tiles {
        generate_all_layers(tile, world.seed);
    }
    Ok(())
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        engine.register_type::<World>();

        engine.register_fn("new_world", new_world_fn);
        engine.register_fn("save_world", save_world_fn);
        engine.register_fn("info", info_fn);
        engine.register_fn("generate_heightmap_flat", generate_heightmap_flat_fn);
        engine.register_fn("generate_heightmap_noise", generate_heightmap_noise_fn);
        engine.register_fn("apply_smooth", apply_smooth_fn);
        engine.register_fn("apply_erode", apply_erode_fn);
        engine.register_fn("generate_layers", generate_layers_fn);

        Self { engine }
    }

    pub fn run(&mut self, script: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        let ast = self.engine.compile(script)?;
        self.engine.eval_ast(&ast)
    }
}
