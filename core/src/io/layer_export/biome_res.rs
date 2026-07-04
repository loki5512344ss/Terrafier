use std::sync::OnceLock;

use terrafier_biome_db::BiomeDb;

use crate::coords::TILE_SIZE;
use crate::io::layer_export::LayerExport;
use crate::model::layers::{LAYER_BIOME, LAYER_RESOURCES};
use crate::model::tile::{LayerBuffer, Tile};

pub struct ResourcesLayerExport;
impl LayerExport for ResourcesLayerExport {
    fn layer_id(&self) -> u32 {
        LAYER_RESOURCES
    }

    fn modify_block(
        &self,
        tile: &Tile,
        local_x: usize,
        local_z: usize,
        y: i32,
        _surface_y: i32,
        block_name: &str,
    ) -> Option<&'static str> {
        let idx = local_z * TILE_SIZE + local_x;
        let data = tile.get_layer_data(LAYER_RESOURCES)?;
        if let LayerBuffer::Nibble(nibbles) = data {
            let byte = nibbles[idx / 2];
            let value = if idx.is_multiple_of(2) {
                byte >> 4
            } else {
                byte & 0xF
            };
            if value > 0 && block_name == "minecraft:stone" && y > -60 {
                let ore = match value {
                    1 => "minecraft:coal_ore",
                    2 => "minecraft:iron_ore",
                    3 => "minecraft:copper_ore",
                    4 => "minecraft:gold_ore",
                    5 => "minecraft:redstone_ore",
                    6 => "minecraft:lapis_ore",
                    7 => "minecraft:diamond_ore",
                    8 => "minecraft:emerald_ore",
                    _ => return None,
                };
                Some(ore)
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Look up a Minecraft biome name from a Terrafier internal biome ID,
/// using the biome-db crate as the source of truth.
fn terrafier_biome_name(id: u8) -> &'static str {
    // Terrafier internal biome IDs → actual Minecraft biome IDs
    const MC_BIOME_IDS: [i32; 11] = [1, 2, 4, 5, 6, 12, 33, 18, 0, 7, 36];

    static NAMES: OnceLock<[&'static str; 11]> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let db = BiomeDb::default();
        let mut arr = [""; 11];
        for (i, &mc_id) in MC_BIOME_IDS.iter().enumerate() {
            let full = db
                .get_by_id(mc_id)
                .map(|e| format!("minecraft:{}", e.name))
                .unwrap_or_else(|| "minecraft:plains".to_string());
            arr[i] = Box::leak(full.into_boxed_str());
        }
        arr
    });

    names
        .get(id as usize)
        .copied()
        .unwrap_or("minecraft:plains")
}

pub struct BiomeLayerExport;
impl LayerExport for BiomeLayerExport {
    fn layer_id(&self) -> u32 {
        LAYER_BIOME
    }

    fn modify_block(
        &self,
        _tile: &Tile,
        _local_x: usize,
        _local_z: usize,
        _y: i32,
        _surface_y: i32,
        _block_name: &str,
    ) -> Option<&'static str> {
        None
    }

    fn biome_name(&self, tile: &Tile, local_x: usize, local_z: usize) -> Option<&'static str> {
        let idx = local_z * TILE_SIZE + local_x;
        let data = tile.get_layer_data(LAYER_BIOME)?;
        if let LayerBuffer::Byte(bytes) = data {
            Some(terrafier_biome_name(bytes[idx]))
        } else {
            None
        }
    }
}
