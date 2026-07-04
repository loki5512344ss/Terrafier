//! Editing operations — raise, lower, smooth, flatten, etc.
//!
//! Each operation implements the Operation trait and supports undo.

use crate::model::dimension::Dimension;

/// An operation that can be applied and reverted.
pub trait Operation: Send + Sync {
    fn name(&self) -> &'static str;
    fn apply(&self, dim: &mut Dimension) -> Result<(), OperationError>;
    fn inverse(&self) -> Box<dyn Operation>;
}

#[derive(Debug)]
pub enum OperationError {
    InvalidParameters(String),
    OutOfBounds { tx: i32, tz: i32, x: u32, z: u32 },
}

mod brush;
mod fill;
mod shaping;

pub use brush::{FloodOperation, PaintOperation, PencilOperation};
pub use fill::FillOperation;
pub use shaping::{
    ErodeOperation, FlattenOperation, HeightOperation, MultiTileOperation, SmoothOperation,
};

/// Restores heights from a saved snapshot (used as inverse of FlattenOperation).
pub struct RestoreHeightsOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub snapshot: Vec<(usize, i16)>,
}

impl Operation for RestoreHeightsOperation {
    fn name(&self) -> &'static str {
        "RestoreHeights"
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
        for &(idx, h) in &self.snapshot {
            if idx < tile.heightmap.len() {
                tile.heightmap[idx] = h;
            }
        }
        Ok(())
    }

    fn inverse(&self) -> Box<dyn Operation> {
        Box::new(NoOpOperation)
    }
}

/// Restores terrain values from a saved snapshot (used as inverse of PaintOperation).
pub struct RestoreTerrainOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub snapshot: Vec<(usize, u8)>,
}

impl Operation for RestoreTerrainOperation {
    fn name(&self) -> &'static str {
        "RestoreTerrain"
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
        for &(idx, t) in &self.snapshot {
            if idx < tile.terrain.len() {
                tile.terrain[idx] = t;
            }
        }
        Ok(())
    }

    fn inverse(&self) -> Box<dyn Operation> {
        Box::new(NoOpOperation)
    }
}

/// No-op operation (used as fallback inverse).
pub struct NoOpOperation;

impl Operation for NoOpOperation {
    fn name(&self) -> &'static str {
        "NoOp"
    }
    fn apply(&self, _dim: &mut Dimension) -> Result<(), OperationError> {
        Ok(())
    }
    fn inverse(&self) -> Box<dyn Operation> {
        Box::new(NoOpOperation)
    }
}
