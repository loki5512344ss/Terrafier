use std::sync::Arc;
use std::sync::OnceLock;

use crate::model::dimension::Dimension;
use crate::model::tile::TILE_SIZE;
use crate::model::types::Brush;

use crate::ops::operations::{Operation, OperationError, RestoreHeightsOperation};

/// Smooth terrain by averaging heights in a 3x3 neighborhood.
pub struct SmoothOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub center_x: u32,
    pub center_z: u32,
    pub radius: u32,
    pub iterations: u32,
    pub brush: Arc<dyn Brush>,
    pub before_snapshot: OnceLock<Vec<(usize, i16)>>,
}

impl Operation for SmoothOperation {
    fn name(&self) -> &'static str {
        "Smooth"
    }

    fn apply(&self, dim: &mut Dimension) -> Result<(), OperationError> {
        let tile =
            dim.tiles
                .get_mut(&(self.tile_x, self.tile_z))
                .ok_or(OperationError::OutOfBounds {
                    tx: self.tile_x,
                    tz: self.tile_z,
                    x: 0,
                    z: 0,
                })?;

        let r = self.radius as i32;
        let cx = self.center_x as i32;
        let cz = self.center_z as i32;

        // Snapshot original heights
        let mut snapshot: Vec<(usize, i16)> = Vec::new();
        for dz in -r..=r {
            for dx in -r..=r {
                let ax = cx + dx;
                let az = cz + dz;
                if ax < 0 || az < 0 || ax >= TILE_SIZE as i32 || az >= TILE_SIZE as i32 {
                    continue;
                }
                if self.brush.get_strength(dx as f64, dz as f64) <= 0.0 {
                    continue;
                }
                if self.before_snapshot.get().is_none() {
                    snapshot.push((
                        (az as usize) * TILE_SIZE + (ax as usize),
                        tile.heightmap[(az as usize) * TILE_SIZE + (ax as usize)],
                    ));
                }
            }
        }
        if self.before_snapshot.get().is_none() {
            let _ = self.before_snapshot.set(snapshot);
        }

        // Apply smoothing for `iterations` passes
        for _ in 0..self.iterations.max(1) {
            let mut new_heights = tile.heightmap;

            for dz in -r..=r {
                for dx in -r..=r {
                    let ax = cx + dx;
                    let az = cz + dz;
                    if ax < 0 || az < 0 || ax >= TILE_SIZE as i32 || az >= TILE_SIZE as i32 {
                        continue;
                    }
                    if self.brush.get_strength(dx as f64, dz as f64) <= 0.0 {
                        continue;
                    }

                    let idx = (az as usize) * TILE_SIZE + (ax as usize);

                    // Average of 3x3 neighborhood (clamped to tile bounds)
                    let mut sum = 0i32;
                    let mut count = 0i32;
                    for ny in -1..=1i32 {
                        for nx in -1..=1i32 {
                            let bx = ax + nx;
                            let bz = az + ny;
                            if bx >= 0 && bx < TILE_SIZE as i32 && bz >= 0 && bz < TILE_SIZE as i32
                            {
                                sum += tile.heightmap[(bz as usize) * TILE_SIZE + (bx as usize)]
                                    as i32;
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        new_heights[idx] = (sum / count)
                            .clamp(tile.min_height as i32, tile.max_height as i32)
                            as i16;
                    }
                }
            }

            tile.heightmap = new_heights;
        }

        Ok(())
    }

    fn inverse(&self) -> Box<dyn Operation> {
        let snapshot = self.before_snapshot.get().cloned().unwrap_or_default();
        Box::new(RestoreHeightsOperation {
            tile_x: self.tile_x,
            tile_z: self.tile_z,
            snapshot,
        })
    }
}
