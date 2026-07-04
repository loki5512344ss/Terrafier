use terrafier_core::model::tile::Tile;

pub struct WorldContext {
    pub tile: Tile,
    pub seed: u64,
    pub tile_x: i32,
    pub tile_z: i32,
    pub hm_offset: usize,
    pub ter_offset: usize,
    pub wat_offset: usize,
    pub error: Option<String>,
}
