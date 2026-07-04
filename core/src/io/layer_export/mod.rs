//! Layer export system — applies layer data to modify blocks during Minecraft export.

use crate::model::tile::Tile;

/// A layer exporter knows how to modify blocks based on layer data.
pub trait LayerExport: Send + Sync {
    fn layer_id(&self) -> u32;
    fn modify_block(
        &self,
        tile: &Tile,
        local_x: usize,
        local_z: usize,
        y: i32,
        surface_y: i32,
        block_name: &str,
    ) -> Option<&'static str>;

    /// Optional biome override for this position.
    fn biome_name(&self, tile: &Tile, local_x: usize, local_z: usize) -> Option<&'static str> {
        let _ = (tile, local_x, local_z);
        None
    }
}

/// Apply all layer exporters in order. The first exporter that returns a replacement wins.
pub fn apply_layers<'a>(
    tile: &Tile,
    local_x: usize,
    local_z: usize,
    y: i32,
    surface_y: i32,
    block_name: &'a str,
    layer_exporters: &[&dyn LayerExport],
) -> &'a str {
    for exporter in layer_exporters {
        if let Some(replacement) =
            exporter.modify_block(tile, local_x, local_z, y, surface_y, block_name)
        {
            return replacement;
        }
    }
    block_name
}

/// Resolve the biome name from layer exporters, falling back to plains.
pub fn biome_name(
    tile: &Tile,
    local_x: usize,
    local_z: usize,
    layer_exporters: &[&dyn LayerExport],
) -> &'static str {
    for exporter in layer_exporters {
        if let Some(name) = exporter.biome_name(tile, local_x, local_z) {
            return name;
        }
    }
    "minecraft:plains"
}

mod biome_res;
mod caves_frost;
mod river_trees;

pub use biome_res::{BiomeLayerExport, ResourcesLayerExport};
pub use caves_frost::{CavesLayerExport, FrostLayerExport};
pub use river_trees::{RiverLayerExport, TreesLayerExport};
