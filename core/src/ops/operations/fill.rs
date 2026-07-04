use std::collections::VecDeque;
use std::sync::OnceLock;

use crate::model::dimension::Dimension;
use crate::model::tile::TILE_SIZE;
use crate::model::types::Terrain;

use super::{Operation, OperationError, RestoreTerrainOperation};

/// Fill a contiguous area (flood-fill from center) with a terrain type.
pub struct FillOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub center_x: u32,
    pub center_z: u32,
    pub terrain: Terrain,
    pub before_snapshot: OnceLock<Vec<(usize, u8)>>,
}

impl Operation for FillOperation {
    fn name(&self) -> &'static str {
        "Fill"
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

        let cx = self.center_x as usize;
        let cz = self.center_z as usize;
        if cx >= TILE_SIZE || cz >= TILE_SIZE {
            return Err(OperationError::OutOfBounds {
                tx: self.tile_x,
                tz: self.tile_z,
                x: self.center_x,
                z: self.center_z,
            });
        }

        let idx = cz * TILE_SIZE + cx;
        let original_terrain = tile.terrain[idx];
        let target_terrain = self.terrain as u8;

        if original_terrain == target_terrain {
            return Ok(());
        }

        let mut snapshot: Vec<(usize, u8)> = Vec::new();
        let mut visited = vec![false; TILE_SIZE * TILE_SIZE];
        let mut queue = VecDeque::new();
        queue.push_back((cx, cz));
        visited[idx] = true;

        while let Some((x, z)) = queue.pop_front() {
            let i = z * TILE_SIZE + x;
            if self.before_snapshot.get().is_none() {
                snapshot.push((i, tile.terrain[i]));
            }
            tile.terrain[i] = target_terrain;

            for (ndx, ndz) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                let nx = x as i32 + ndx;
                let nz = z as i32 + ndz;
                if nx >= 0 && nx < TILE_SIZE as i32 && nz >= 0 && nz < TILE_SIZE as i32 {
                    let nxu = nx as usize;
                    let nzu = nz as usize;
                    let ni = nzu * TILE_SIZE + nxu;
                    if !visited[ni] && tile.terrain[ni] == original_terrain {
                        visited[ni] = true;
                        queue.push_back((nxu, nzu));
                    }
                }
            }
        }

        if self.before_snapshot.get().is_none() {
            let _ = self.before_snapshot.set(snapshot);
        }

        Ok(())
    }

    fn inverse(&self) -> Box<dyn Operation> {
        let snapshot = self.before_snapshot.get().cloned().unwrap_or_default();
        Box::new(RestoreTerrainOperation {
            tile_x: self.tile_x,
            tile_z: self.tile_z,
            snapshot,
        })
    }
}
