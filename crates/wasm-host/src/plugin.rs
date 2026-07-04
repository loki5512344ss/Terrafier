use std::path::Path;

use terrafier_core::model::tile::Tile;
use terrafier_core::plugins::TileSourcePlugin;
use wasmtime::*;

use crate::context::WorldContext;
use crate::error::WasmHostError;
use crate::host::register_host_functions;

pub trait WasmPlugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn plugin_type(&self) -> u32;
}

pub struct WasmTileSourcePlugin {
    engine: Engine,
    module: Module,
    name: String,
    version: String,
}

impl WasmTileSourcePlugin {
    pub fn from_file(path: &Path) -> Result<Self, WasmHostError> {
        let engine = Engine::default();
        let wasm_bytes = std::fs::read(path).map_err(|e| {
            WasmHostError::LoadError(anyhow::anyhow!("Failed to read .wasm file: {}", e))
        })?;
        let module = Module::from_binary(&engine, &wasm_bytes)
            .map_err(|e| WasmHostError::LoadError(anyhow::anyhow!("Invalid WASM module: {}", e)))?;

        let mut linker = Linker::new(&engine);
        register_host_functions(&mut linker).map_err(|e| {
            WasmHostError::LoadError(anyhow::anyhow!("Failed to register host functions: {}", e))
        })?;
        let dummy_ctx = WorldContext {
            tile: Tile::new(0, 0, -64, 320),
            seed: 0,
            tile_x: 0,
            tile_z: 0,
            hm_offset: 0,
            ter_offset: 0,
            wat_offset: 0,
            error: None,
        };
        let mut store = Store::new(&engine, dummy_ctx);
        let instance = linker.instantiate(&mut store, &module).map_err(|e| {
            WasmHostError::LoadError(anyhow::anyhow!("Failed to instantiate module: {}", e))
        })?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| WasmHostError::MissingExport("memory".into()))?;

        let name = Self::read_string(&mut store, &instance, &memory, "name_ptr", "name_len")?;
        let version =
            Self::read_string(&mut store, &instance, &memory, "version_ptr", "version_len")?;

        Ok(Self {
            engine,
            module,
            name,
            version,
        })
    }

    fn read_string(
        store: &mut Store<WorldContext>,
        instance: &Instance,
        memory: &Memory,
        ptr_func: &str,
        len_func: &str,
    ) -> Result<String, WasmHostError> {
        let ptr = instance
            .get_typed_func::<(), i32>(&mut *store, ptr_func)
            .map_err(|_| WasmHostError::MissingExport(ptr_func.into()))?
            .call(&mut *store, ())
            .map_err(|e| WasmHostError::PluginError(e.to_string()))?;
        let len = instance
            .get_typed_func::<(), i32>(&mut *store, len_func)
            .map_err(|_| WasmHostError::MissingExport(len_func.into()))?
            .call(&mut *store, ())
            .map_err(|e| WasmHostError::PluginError(e.to_string()))?;

        let mut buf = vec![0u8; len as usize];
        memory
            .read(&*store, ptr as usize, &mut buf)
            .map_err(|e| WasmHostError::MemoryError(e.to_string()))?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    fn make_instance(
        &self,
        tile: &Tile,
        seed: u64,
        tile_x: i32,
        tile_z: i32,
    ) -> Result<(Store<WorldContext>, Instance), WasmHostError> {
        let mut linker = Linker::new(&self.engine);
        register_host_functions(&mut linker).map_err(|e| WasmHostError::LoadError(e.into()))?;

        let ctx = WorldContext {
            tile: tile.clone(),
            seed,
            tile_x,
            tile_z,
            hm_offset: 0,
            ter_offset: 0,
            wat_offset: 0,
            error: None,
        };
        let mut store = Store::new(&self.engine, ctx);
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| WasmHostError::LoadError(e.into()))?;
        Ok((store, instance))
    }
}

impl WasmPlugin for WasmTileSourcePlugin {
    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn plugin_type(&self) -> u32 {
        3
    }
}

impl TileSourcePlugin for WasmTileSourcePlugin {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn version(&self) -> &'static str {
        Box::leak(self.version.clone().into_boxed_str())
    }

    fn generate_tile(&self, tile: &mut Tile, seed: u64) {
        let tx = tile.x;
        let tz = tile.z;
        let tile_clone = tile.clone();
        let (mut store, instance) = match self.make_instance(&tile_clone, seed, tx, tz) {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "Failed to create WASM instance for plugin '{}': {}",
                    self.name,
                    e
                );
                return;
            }
        };

        let Ok(gen_func) =
            instance.get_typed_func::<(i32, i32, i32, i32), i32>(&mut store, "generate_tile")
        else {
            log::error!(
                "WASM plugin '{}' does not export 'generate_tile'",
                self.name
            );
            return;
        };

        let seed_lo = (seed & 0xFFFF_FFFF) as i32;
        let seed_hi = ((seed >> 32) & 0xFFFF_FFFF) as i32;

        match gen_func.call(&mut store, (tx, tz, seed_lo, seed_hi)) {
            Ok(0) => {
                *tile = store.data().tile.clone();
            }
            Ok(code) => {
                log::error!(
                    "WASM plugin '{}' generate_tile returned error code: {}",
                    self.name,
                    code
                );
            }
            Err(e) => {
                log::error!("WASM plugin '{}' generate_tile trap: {}", self.name, e);
            }
        }
    }
}
