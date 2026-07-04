use crate::coords::TILE_SIZE;
use crate::model::layers::{DataSize, LAYER_TREES};
use crate::model::tile::Tile;
use crate::model::types::Terrain;
use std::hash::{Hash, Hasher};

fn tile_hash(tile_x: i32, tile_z: i32, seed: u64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    tile_x.hash(&mut hasher);
    tile_z.hash(&mut hasher);
    seed.hash(&mut hasher);
    hasher.finish()
}

fn pseudo_rand(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

pub struct TreesGenerator;

impl crate::ops::layers::LayerGenerator for TreesGenerator {
    fn layer_id(&self) -> u32 {
        LAYER_TREES
    }

    fn generate(&self, tile: &mut Tile, seed: u64) {
        let terrain = tile.terrain;
        let mut rng = tile_hash(tile.x, tile.z, seed);

        let total = TILE_SIZE * TILE_SIZE;
        let mut bytes = vec![0u8; total];

        for idx in 0..total {
            let terrain_type = terrain[idx] as usize;
            bytes[idx] = match terrain_type {
                t if t == Terrain::Forest as usize => (pseudo_rand(&mut rng) % 60).max(20) as u8,
                t if t == Terrain::Grass as usize && pseudo_rand(&mut rng) % 100 < 30 => {
                    (pseudo_rand(&mut rng) % 20 + 5) as u8
                }
                _ => 0,
            };
        }

        let buf = tile.ensure_layer(LAYER_TREES, DataSize::Byte);
        if let crate::model::tile::LayerBuffer::Byte(b) = buf {
            b.copy_from_slice(&bytes);
        }
    }
}
