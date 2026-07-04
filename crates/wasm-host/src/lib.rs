mod context;
mod error;
mod host;
mod plugin;

use std::path::Path;
use terrafier_core::plugins::PluginRegistry;

pub use context::WorldContext;
pub use error::WasmHostError;
pub use plugin::{WasmPlugin, WasmTileSourcePlugin};

/// Load all .wasm plugins from a directory into the registry.
pub fn load_wasm_dir(
    registry: &mut PluginRegistry,
    dir: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                match WasmTileSourcePlugin::from_file(&path) {
                    Ok(plugin) => {
                        log::info!(
                            "Loaded WASM plugin: {} v{} ({})",
                            plugin.name(),
                            plugin.version(),
                            path.display()
                        );
                        registry.sources.push(Box::new(plugin));
                        count += 1;
                    }
                    Err(e) => {
                        log::error!("Failed to load WASM plugin {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
    Ok(count)
}
