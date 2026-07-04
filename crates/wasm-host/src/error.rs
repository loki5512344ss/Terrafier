use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmHostError {
    #[error("Failed to load WASM module: {0}")]
    LoadError(#[from] anyhow::Error),
    #[error("WASM plugin does not export required function: {0}")]
    MissingExport(String),
    #[error("WASM plugin returned error: {0}")]
    PluginError(String),
    #[error("Memory access error: {0}")]
    MemoryError(String),
}
