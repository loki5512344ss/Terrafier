//! WorldPainter .world file import.
//!
//! Reads WorldPainter `.world` files, which use Java serialization format.
//! Implements a minimal Java ObjectInputStream parser sufficient for extracting
//! tile heightmap, terrain, and water level data from WorldPainter worlds.

mod extract;
mod java_reader;
mod types;

use std::io::Read;
use std::path::{Path, PathBuf};

pub use extract::*;
pub use types::WpImportError;

use crate::model::dimension::Dimension;
use crate::model::tile::Tile;
use crate::model::types::Platform;
use crate::model::world::World;

use java_reader::JvmStream;
use types::Result;

pub struct WpImporter {
    path: PathBuf,
}

impl WpImporter {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn import(&self) -> Result<World> {
        import_world_file(&self.path)
    }
}

pub fn is_world_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "world")
        || path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(".world"))
}

pub fn import_world_file(path: &Path) -> Result<World> {
    let raw = std::fs::read(path).map_err(WpImportError::Io)?;

    if raw.is_empty() {
        return Err(WpImportError::InvalidFormat("file is empty".into()));
    }

    let data = if raw.len() >= 2 && raw[0] == 0x1F && raw[1] == 0x8B {
        let mut decoder = flate2::read::GzDecoder::new(&raw[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| WpImportError::Compression(e.to_string()))?;
        decompressed
    } else {
        raw
    };

    if data.len() < 4 {
        return Err(WpImportError::InvalidFormat(
            "file too small after decompression".into(),
        ));
    }
    let magic = u16::from_be_bytes([data[0], data[1]]);
    if magic != 0xACED {
        return Err(WpImportError::InvalidFormat(format!(
            "not a Java serialization stream (expected magic ACED, got {:04X})",
            magic
        )));
    }
    let stream_version = u16::from_be_bytes([data[2], data[3]]);
    if stream_version != 5 {
        return Err(WpImportError::InvalidFormat(format!(
            "unsupported Java serialization stream version: {}",
            stream_version
        )));
    }

    let mut stream = JvmStream::new(data);

    let root = stream
        .read_content()
        .map_err(|e| WpImportError::InvalidFormat(e.to_string()))?;

    let world_data = extract::extract_world_data(&root)?;

    if world_data.tiles.is_empty() && !world_data.name.is_empty() {
        log::warn!("WorldPainter world '{}' has no tiles", world_data.name);
    }

    let platform = Platform::java_1_18();

    let mut tiles = std::collections::HashMap::new();
    for wp_tile in &world_data.tiles {
        let mut tile = Tile::new(
            wp_tile.x,
            wp_tile.z,
            platform.min_height,
            platform.max_height,
        );

        if let Some(hm) = &wp_tile.heightmap {
            tile.heightmap = *hm;
        }
        if let Some(ter) = &wp_tile.terrain {
            tile.terrain = *ter;
        }
        if let Some(wl) = &wp_tile.water_level {
            tile.water_level = *wl;
        }

        tiles.insert((wp_tile.x, wp_tile.z), tile);
    }

    let dimension = Dimension {
        name: "overworld".to_string(),
        tiles,
        min_height: platform.min_height,
        max_height: platform.max_height,
        seed: world_data.seed,
    };

    Ok(World {
        name: world_data.name,
        platform,
        dimensions: vec![dimension],
        seed: world_data.seed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_no_file() {
        let result = import_world_file(Path::new("/nonexistent/magic.world"));
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.world");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"not a world file").unwrap();
        let result = import_world_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_not_world() {
        let result = import_world_file(Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[test]
    fn test_is_world_file() {
        assert!(is_world_file(Path::new("test.world")));
        assert!(!is_world_file(Path::new("test.mca")));
        assert!(!is_world_file(Path::new("level.dat")));
    }
}
