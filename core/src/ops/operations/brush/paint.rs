use crate::model::dimension::Dimension;
use crate::model::tile::TILE_SIZE;
use crate::model::types::Brush;
use crate::model::types::Terrain;
use crate::ops::operations::{Operation, OperationError, RestoreTerrainOperation};
use std::sync::{Arc, Mutex};

struct BrushApply<'a> {
    tile: &'a mut crate::model::tile::Tile,
    cx: i32,
    cz: i32,
    radius: u32,
    terrain_id: u8,
    brush: &'a dyn Brush,
    snapshot: &'a Mutex<Option<Vec<(usize, u8)>>>,
    snapshot_buf: &'a mut Vec<(usize, u8)>,
}

fn apply_brush_terrain(args: BrushApply) {
    let r = args.radius as i32;
    for dz in -r..=r {
        for dx in -r..=r {
            let ax = args.cx + dx;
            let az = args.cz + dz;
            if ax < 0 || az < 0 || ax >= TILE_SIZE as i32 || az >= TILE_SIZE as i32 {
                continue;
            }
            if args.brush.get_strength(dx as f64, dz as f64) < 0.3 {
                continue;
            }
            let idx = (az as usize) * TILE_SIZE + (ax as usize);
            if args.snapshot.lock().unwrap().is_none() {
                args.snapshot_buf.push((idx, args.tile.terrain[idx]));
            }
            args.tile.terrain[idx] = args.terrain_id;
        }
    }
}

/// Paint terrain type within a brush area.
pub struct PaintOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub center_x: u32,
    pub center_z: u32,
    pub radius: u32,
    pub terrain: Terrain,
    pub brush: Arc<dyn Brush>,
    pub before_snapshot: Mutex<Option<Vec<(usize, u8)>>>,
}

impl Operation for PaintOperation {
    fn name(&self) -> &'static str {
        "Paint"
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

        let mut snapshot_buf = Vec::new();
        apply_brush_terrain(BrushApply {
            tile,
            cx: self.center_x as i32,
            cz: self.center_z as i32,
            radius: self.radius,
            terrain_id: self.terrain as u8,
            brush: self.brush.as_ref(),
            snapshot: &self.before_snapshot,
            snapshot_buf: &mut snapshot_buf,
        });

        if self.before_snapshot.lock().unwrap().is_none() {
            self.before_snapshot.lock().unwrap().replace(snapshot_buf);
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
        Box::new(RestoreTerrainOperation {
            tile_x: self.tile_x,
            tile_z: self.tile_z,
            snapshot,
        })
    }
}

/// Single-point terrain paint (radius 1, no brush falloff).
pub struct PencilOperation {
    pub tile_x: i32,
    pub tile_z: i32,
    pub center_x: u32,
    pub center_z: u32,
    pub terrain: Terrain,
    pub before_snapshot: Mutex<Option<Vec<(usize, u8)>>>,
}

impl Operation for PencilOperation {
    fn name(&self) -> &'static str {
        "Pencil"
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
        if self.before_snapshot.lock().unwrap().is_none() {
            self.before_snapshot
                .lock()
                .unwrap()
                .replace(vec![(idx, tile.terrain[idx])]);
        }
        tile.terrain[idx] = self.terrain as u8;

        Ok(())
    }

    fn inverse(&self) -> Box<dyn Operation> {
        let snapshot = self
            .before_snapshot
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default();
        Box::new(RestoreTerrainOperation {
            tile_x: self.tile_x,
            tile_z: self.tile_z,
            snapshot,
        })
    }
}
