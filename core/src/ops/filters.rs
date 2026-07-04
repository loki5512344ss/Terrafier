//! Filter system — restrict which cells an operation affects based on terrain, height, or slope.
use crate::model::dimension::Dimension;
use crate::model::tile::{TILE_SIZE, Tile};
use crate::model::types::Terrain;
use crate::ops::operations::{Operation, OperationError, RestoreHeightsOperation};
use std::sync::Mutex;
/// Determines whether a cell should be modified by an operation.
pub trait Filter: Send + Sync {
    fn name(&self) -> &'static str;
    fn should_apply(&self, tile: &Tile, local_x: usize, local_z: usize) -> bool;
}
/// Only apply where terrain matches a specific type.
pub struct TerrainFilter {
    pub terrain: Terrain,
}
impl Filter for TerrainFilter {
    fn name(&self) -> &'static str {
        "TerrainFilter"
    }
    fn should_apply(&self, tile: &Tile, local_x: usize, local_z: usize) -> bool {
        tile.get_terrain(local_x, local_z) == self.terrain as u8
    }
}
/// Only apply where height is within a range.
pub struct HeightFilter {
    pub min: i16,
    pub max: i16,
}
impl Filter for HeightFilter {
    fn name(&self) -> &'static str {
        "HeightFilter"
    }
    fn should_apply(&self, tile: &Tile, local_x: usize, local_z: usize) -> bool {
        let h = tile.get_height(local_x, local_z);
        h >= self.min && h <= self.max
    }
}

/// Only apply where slope (max height difference to 4 neighbors) is within a range.
pub struct SlopeFilter {
    pub min: f64,
    pub max: f64,
}

impl Filter for SlopeFilter {
    fn name(&self) -> &'static str {
        "SlopeFilter"
    }
    fn should_apply(&self, tile: &Tile, local_x: usize, local_z: usize) -> bool {
        let center = tile.get_height(local_x, local_z);
        let mut max_diff = 0i16;
        for (dx, dz) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
            let nx = local_x as i32 + dx;
            let nz = local_z as i32 + dz;
            if nx >= 0 && nx < TILE_SIZE as i32 && nz >= 0 && nz < TILE_SIZE as i32 {
                let nh = tile.get_height(nx as usize, nz as usize);
                let diff = (center - nh).abs();
                if diff > max_diff {
                    max_diff = diff;
                }
            }
        }
        let slope = max_diff as f64;
        slope >= self.min && slope <= self.max
    }
}

/// How multiple filters are combined.
pub enum FilterCombinator {
    And,
    Or,
}

/// Combine multiple filters with AND/OR logic.
pub struct CompositeFilter {
    pub filters: Vec<Box<dyn Filter>>,
    pub combinator: FilterCombinator,
}

impl Filter for CompositeFilter {
    fn name(&self) -> &'static str {
        "CompositeFilter"
    }
    fn should_apply(&self, tile: &Tile, local_x: usize, local_z: usize) -> bool {
        match self.combinator {
            FilterCombinator::And => self
                .filters
                .iter()
                .all(|f| f.should_apply(tile, local_x, local_z)),
            FilterCombinator::Or => self
                .filters
                .iter()
                .any(|f| f.should_apply(tile, local_x, local_z)),
        }
    }
}

/// Negate a filter.
pub struct InvertFilter {
    pub filter: Box<dyn Filter>,
}

impl Filter for InvertFilter {
    fn name(&self) -> &'static str {
        "InvertFilter"
    }
    fn should_apply(&self, tile: &Tile, local_x: usize, local_z: usize) -> bool {
        !self.filter.should_apply(tile, local_x, local_z)
    }
}

/// Wraps any operation and restricts its effect to cells that pass the filter.
///
/// On apply: snapshots brush area heights, runs inner op, restores filtered-out cells.
/// Inverse restores all cells from the snapshot.
pub struct FilteredOperation {
    pub operation: Box<dyn Operation>,
    pub filter: Box<dyn Filter>,
    pub tile_x: i32,
    pub tile_z: i32,
    pub center_x: u32,
    pub center_z: u32,
    pub radius: u32,
    pub before_snapshot: Mutex<Option<Vec<(usize, i16)>>>,
}

impl Operation for FilteredOperation {
    fn name(&self) -> &'static str {
        self.operation.name()
    }

    fn apply(&self, dim: &mut Dimension) -> Result<(), OperationError> {
        let is_first_apply = self.before_snapshot.lock().unwrap().is_none();

        let snapshot: Vec<(usize, i16)> = if is_first_apply {
            let tile =
                dim.tiles
                    .get(&(self.tile_x, self.tile_z))
                    .ok_or(OperationError::OutOfBounds {
                        tx: self.tile_x,
                        tz: self.tile_z,
                        x: 0,
                        z: 0,
                    })?;

            let r = self.radius as i32;
            let cx = self.center_x as i32;
            let cz = self.center_z as i32;

            let mut snap = Vec::new();
            for dz in -r..=r {
                for dx in -r..=r {
                    let ax = cx + dx;
                    let az = cz + dz;
                    if ax < 0 || az < 0 || ax >= TILE_SIZE as i32 || az >= TILE_SIZE as i32 {
                        continue;
                    }
                    let idx = (az as usize) * TILE_SIZE + (ax as usize);
                    snap.push((idx, tile.heightmap[idx]));
                }
            }
            self.before_snapshot.lock().unwrap().replace(snap.clone());
            snap
        } else {
            self.before_snapshot
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_default()
        };

        self.operation.apply(dim)?;

        let tile =
            dim.tiles
                .get_mut(&(self.tile_x, self.tile_z))
                .ok_or(OperationError::OutOfBounds {
                    tx: self.tile_x,
                    tz: self.tile_z,
                    x: 0,
                    z: 0,
                })?;

        for &(idx, original_h) in &snapshot {
            let lx = idx % TILE_SIZE;
            let lz = idx / TILE_SIZE;
            if !self.filter.should_apply(tile, lx, lz) {
                tile.heightmap[idx] = original_h;
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
