use std::sync::{Arc, Mutex};

use crate::model::dimension::Dimension;
use crate::model::tile::TILE_SIZE;
use crate::model::types::Brush;

use crate::ops::operations::{Operation, OperationError, RestoreHeightsOperation};

/// Simulate thermal erosion — material moves from high to low when slope exceeds threshold.
pub struct ErodeOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub center_x: u32,
    pub center_z: u32,
    pub radius: u32,
    pub iterations: u32,
    pub talus_angle: f64,
    pub brush: Arc<dyn Brush>,
    pub before_snapshot: Mutex<Option<Vec<(usize, i16)>>>,
}

impl Operation for ErodeOperation {
    fn name(&self) -> &'static str {
        "Erode"
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

        // Snapshot original heights on first apply
        let mut snapshot: Vec<(usize, i16)> = Vec::new();
        let is_first_apply = self.before_snapshot.lock().unwrap().is_none();
        if is_first_apply {
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
                    snapshot.push((idx, tile.heightmap[idx]));
                }
            }
        }
        if is_first_apply {
            self.before_snapshot.lock().unwrap().replace(snapshot);
        }

        // Apply erosion for `iterations` passes
        let tile_size = TILE_SIZE;
        for _ in 0..self.iterations.max(1) {
            let mut deltas = vec![0i32; tile_size * tile_size];

            for dz in -r..=r {
                for dx in -r..=r {
                    let ax = cx + dx;
                    let az = cz + dz;
                    if ax < 0 || az < 0 || ax >= tile_size as i32 || az >= tile_size as i32 {
                        continue;
                    }
                    if self.brush.get_strength(dx as f64, dz as f64) <= 0.0 {
                        continue;
                    }

                    let idx = (az as usize) * tile_size + (ax as usize);
                    let current = tile.heightmap[idx] as i32;

                    // Find lowest neighbor that is also in brush area
                    let mut lowest_h = current;
                    let mut lowest_idx = None;
                    for (ndx, ndz) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                        let bx = ax + ndx;
                        let bz = az + ndz;
                        if bx < 0 || bx >= tile_size as i32 || bz < 0 || bz >= tile_size as i32 {
                            continue;
                        }
                        let n_dx = bx - cx;
                        let n_dz = bz - cz;
                        if self.brush.get_strength(n_dx as f64, n_dz as f64) <= 0.0 {
                            continue;
                        }
                        let nidx = (bz as usize) * tile_size + (bx as usize);
                        let nh = tile.heightmap[nidx] as i32;
                        if nh < lowest_h {
                            lowest_h = nh;
                            lowest_idx = Some(nidx);
                        }
                    }

                    if let Some(lidx) = lowest_idx {
                        let diff = current - lowest_h;
                        if (diff as f64) > self.talus_angle {
                            let excess = diff as f64 - self.talus_angle;
                            let move_amount = (excess * 0.5).round() as i32;
                            deltas[idx] -= move_amount;
                            deltas[lidx] += move_amount;
                        }
                    }
                }
            }

            // Apply deltas
            for dz in -r..=r {
                for dx in -r..=r {
                    let ax = cx + dx;
                    let az = cz + dz;
                    if ax < 0 || az < 0 || ax >= tile_size as i32 || az >= tile_size as i32 {
                        continue;
                    }
                    if self.brush.get_strength(dx as f64, dz as f64) <= 0.0 {
                        continue;
                    }
                    let idx = (az as usize) * tile_size + (ax as usize);
                    if deltas[idx] != 0 {
                        let new_h = (tile.heightmap[idx] as i32 + deltas[idx])
                            .clamp(tile.min_height as i32, tile.max_height as i32)
                            as i16;
                        tile.heightmap[idx] = new_h;
                    }
                }
            }
        }

        Ok(())
    }

    fn inverse(&self) -> Box<dyn Operation> {
        let snapshot = self
            .before_snapshot
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default();
        Box::new(RestoreHeightsOperation {
            tile_x: self.tile_x,
            tile_z: self.tile_z,
            snapshot,
        })
    }
}
