//! I/O module — Minecraft save import/export, rendering, and binary save/load.

pub mod export;
pub mod import;
pub mod layer_export;
pub mod minecraft;
pub mod wp_import;

use std::path::Path;

use crate::model::world::World;

#[derive(thiserror::Error, Debug)]
pub enum BinaryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Bincode serialize error: {0}")]
    Serialize(String),
    #[error("Bincode deserialize error: {0}")]
    Deserialize(String),
    #[error("Validation failed: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, BinaryError>;

/// Save a `World` to a binary file.
pub fn save_binary(world: &World, path: &Path) -> Result<()> {
    let config = bincode::config::standard();
    let bytes = bincode::serde::encode_to_vec(world, config)
        .map_err(|e| BinaryError::Serialize(e.to_string()))?;

    std::fs::write(path, &bytes)?;
    Ok(())
}

/// Load a `World` from a binary file.
pub fn load_binary(path: &Path) -> Result<World> {
    let bytes = std::fs::read(path)?;
    let config = bincode::config::standard();
    let (world, _): (World, usize) = bincode::serde::decode_from_slice(&bytes, config)
        .map_err(|e| BinaryError::Deserialize(e.to_string()))?;
    Ok(world)
}

/// Validate a binary world file by loading and discarding.
pub fn validate_binary(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(BinaryError::Validation(format!(
            "File not found: {}",
            path.display()
        )));
    }
    let _world = load_binary(path)?;
    Ok(())
}
