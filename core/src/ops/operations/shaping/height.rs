use std::sync::Arc;
use std::sync::OnceLock;

use crate::model::dimension::Dimension;
use crate::model::tile::TILE_SIZE;
use crate::model::types::Brush;

use crate::ops::operations::{Operation, OperationError, RestoreHeightsOperation};

/// Raise or lower terrain height within a brush area.
pub struct HeightOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub center_x: u32,
    pub center_z: u32,
    pub radius: u32,
    pub delta: i16,
    pub brush: Arc<dyn Brush>,
    pub before_snapshot: OnceLock<Vec<(usize, i16)>>,
}

impl Operation for HeightOperation {
    fn name(&self) -> &'static str {
        if self.delta >= 0 { "Raise" } else { "Lower" }
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

        let mut heights: Vec<(usize, i16)> = Vec::new();

        for dz in -r..=r {
            for dx in -r..=r {
                let ax = cx + dx;
                let az = cz + dz;
                if ax < 0 || az < 0 || ax >= TILE_SIZE as i32 || az >= TILE_SIZE as i32 {
                    continue;
                }
                let strength = self.brush.get_strength(dx as f64, dz as f64);
                if strength <= 0.0 {
                    continue;
                }

                let idx = (az as usize) * TILE_SIZE + (ax as usize);

                if self.before_snapshot.get().is_none() {
                    heights.push((idx, tile.heightmap[idx]));
                }

                let change = (self.delta as f64 * strength).round() as i16;
                let new_height =
                    (tile.heightmap[idx] + change).clamp(tile.min_height, tile.max_height);
                tile.heightmap[idx] = new_height;
            }
        }

        if self.before_snapshot.get().is_none() {
            let _ = self.before_snapshot.set(heights);
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

/// Flatten terrain to a target height within a brush area.
pub struct FlattenOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub center_x: u32,
    pub center_z: u32,
    pub radius: u32,
    pub target_height: i16,
    pub brush: Arc<dyn Brush>,
    pub before_snapshot: OnceLock<Vec<(usize, i16)>>,
}

impl Operation for FlattenOperation {
    fn name(&self) -> &'static str {
        "Flatten"
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

        let mut heights: Vec<(usize, i16)> = Vec::new();

        for dz in -r..=r {
            for dx in -r..=r {
                let ax = cx + dx;
                let az = cz + dz;
                if ax < 0 || az < 0 || ax >= TILE_SIZE as i32 || az >= TILE_SIZE as i32 {
                    continue;
                }
                let strength = self.brush.get_strength(dx as f64, dz as f64);
                if strength <= 0.0 {
                    continue;
                }

                let idx = (az as usize) * TILE_SIZE + (ax as usize);

                if self.before_snapshot.get().is_none() {
                    heights.push((idx, tile.heightmap[idx]));
                }

                let current = tile.heightmap[idx];
                let diff = self.target_height as i32 - current as i32;
                let change = (diff as f64 * strength).round() as i32;
                let new_height = (current as i32 + change)
                    .clamp(tile.min_height as i32, tile.max_height as i32)
                    as i16;
                tile.heightmap[idx] = new_height;
            }
        }

        if self.before_snapshot.get().is_none() {
            let _ = self.before_snapshot.set(heights);
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

/// Applies multiple sub-operations, supporting brushes that span multiple tiles.
pub struct MultiTileOperation {
    pub operations: Vec<Box<dyn Operation>>,
}

impl Operation for MultiTileOperation {
    fn name(&self) -> &'static str {
        if self.operations.is_empty() {
            "MultiTile (empty)"
        } else {
            self.operations[0].name()
        }
    }

    fn apply(&self, dim: &mut Dimension) -> Result<(), OperationError> {
        for op in &self.operations {
            op.apply(dim)?;
        }
        Ok(())
    }

    fn inverse(&self) -> Box<dyn Operation> {
        let mut inverses: Vec<Box<dyn Operation>> = Vec::with_capacity(self.operations.len());
        for op in &self.operations {
            inverses.push(op.inverse());
        }
        inverses.reverse();
        Box::new(MultiTileOperation {
            operations: inverses,
        })
    }
}
