//! Height map sources — generate height maps from noise, flat, or combined sources.

use crate::model::tile::Tile;
use crate::model::types::Terrain;

/// A source that can generate a height map for a tile.
pub trait HeightMapSource: Send + Sync {
    fn name(&self) -> &'static str;
    fn generate(&self, tile: &mut Tile, seed: u64);
}

/// Flat height map — sets all heights to a constant value.
pub struct FlatHeightMap {
    pub height: i16,
}

impl HeightMapSource for FlatHeightMap {
    fn name(&self) -> &'static str {
        "Flat"
    }

    fn generate(&self, tile: &mut Tile, _seed: u64) {
        for h in tile.heightmap.iter_mut() {
            *h = self.height;
        }
        for t in tile.terrain.iter_mut() {
            *t = Terrain::Grass as u8;
        }
    }
}

/// Noise-based height map using Simplex noise.
pub struct NoiseHeightMap {
    pub base_height: f64,
    pub amplitude: f64,
    pub frequency: f64,
    pub scale_x: f64,
    pub scale_z: f64,
}

impl Default for NoiseHeightMap {
    fn default() -> Self {
        Self {
            base_height: 63.0,
            amplitude: 30.0,
            frequency: 0.01,
            scale_x: 1.0,
            scale_z: 1.0,
        }
    }
}

impl HeightMapSource for NoiseHeightMap {
    fn name(&self) -> &'static str {
        "Noise"
    }

    fn generate(&self, tile: &mut Tile, seed: u64) {
        use terrafier_noise::NoiseFn;
        let seed_u32 = (seed ^ (seed >> 32)) as u32;
        let noise_gen = terrafier_noise::OpenSimplex::new(seed_u32);
        let tile_size = 128usize;

        for lx in 0..tile_size {
            for lz in 0..tile_size {
                let world_x = (tile.x as f64 * tile_size as f64 + lx as f64) * self.scale_x;
                let world_z = (tile.z as f64 * tile_size as f64 + lz as f64) * self.scale_z;

                let n = noise_gen.get([world_x * self.frequency, world_z * self.frequency]);
                let height = (self.base_height + n * self.amplitude)
                    .round()
                    .clamp(tile.min_height as f64, tile.max_height as f64)
                    as i16;

                let idx = lz * tile_size + lx;
                tile.heightmap[idx] = height;

                tile.terrain[idx] = if height < 0 {
                    Terrain::Water as u8
                } else if height < 5 {
                    Terrain::Sand as u8
                } else if height < 10 {
                    Terrain::Grass as u8
                } else if height < 20 {
                    Terrain::Forest as u8
                } else {
                    Terrain::Rock as u8
                };
            }
        }
    }
}

/// Combined height map — mix several height map sources.
pub struct CombinedHeightMap {
    pub sources: Vec<(Box<dyn HeightMapSource>, f64)>,
}

impl HeightMapSource for CombinedHeightMap {
    fn name(&self) -> &'static str {
        "Combined"
    }

    fn generate(&self, tile: &mut Tile, seed: u64) {
        let mut accumulated_heights = vec![0.0f64; 16384];

        for (source, weight) in &self.sources {
            let mut temp_tile = Tile::new(tile.x, tile.z, tile.min_height, tile.max_height);
            source.generate(&mut temp_tile, seed.wrapping_add(*weight as u64 * 100));

            for (i, acc) in accumulated_heights.iter_mut().enumerate() {
                *acc += temp_tile.heightmap[i] as f64 * weight;
            }
        }

        let weight_sum: f64 = self.sources.iter().map(|(_, w)| w).sum();

        if weight_sum > 0.0 {
            for (i, acc) in accumulated_heights.iter().enumerate() {
                let h = (*acc / weight_sum)
                    .round()
                    .clamp(tile.min_height as f64, tile.max_height as f64)
                    as i16;
                tile.heightmap[i] = h;
            }
        }
    }
}
