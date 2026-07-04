//! Terrafier Core — world model, I/O, operations.
//!
//! The core library provides the data model (World, Dimension, Tile, Terrain),
//! Minecraft save import/export, editing operations, and height map generation.

pub mod coords;
pub mod io;
pub mod model;
pub mod ops;
pub mod plugins;

// Re-export key types for convenience
pub use coords::{BlockCoords, ChunkCoords, RegionCoords, TileCoords};
pub use io::export;
pub use io::import;
pub use model::dimension::Dimension;
pub use model::tile::Tile;
pub use model::types::{Platform, Terrain};
pub use model::world::World;

pub use terrafier_biome_db as biome_db;
