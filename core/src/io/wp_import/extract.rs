use super::types::WpImportError;
use super::types::{JvmValue, Result};

pub struct WpTileData {
    pub x: i32,
    pub z: i32,
    pub heightmap: Option<[i16; 16384]>,
    pub terrain: Option<[u8; 16384]>,
    pub water_level: Option<[u8; 16384]>,
}

pub struct WpWorldData {
    pub name: String,
    pub seed: u64,
    pub tiles: Vec<WpTileData>,
}

pub fn extract_world_data(val: &JvmValue) -> Result<WpWorldData> {
    let obj = val.as_object().ok_or_else(|| {
        WpImportError::InvalidFormat("root object is not a compound object".into())
    })?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "WorldPainter World".to_string());

    let seed = obj.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);

    let mut tiles_data = Vec::new();

    if let Some(tiles_val) = obj.get("tiles") {
        collect_tiles(tiles_val, &mut tiles_data);
    }

    for key in &["dimensions", "tileSet", "worldTiles"] {
        if let Some(val) = obj.get(*key) {
            collect_tiles(val, &mut tiles_data);
        }
    }

    if tiles_data.is_empty() {
        for val in obj.values() {
            collect_tiles(val, &mut tiles_data);
        }
    }

    Ok(WpWorldData {
        name,
        seed,
        tiles: tiles_data,
    })
}

fn collect_tiles(val: &JvmValue, tiles: &mut Vec<WpTileData>) {
    match val {
        JvmValue::Object(map) => {
            if (map.contains_key("heightMap")
                || map.contains_key("terrain")
                || map.contains_key("waterLevel"))
                && let Some(tile) = parse_single_tile(val)
            {
                tiles.push(tile);
                return;
            }
            for sub in map.values() {
                collect_tiles(sub, tiles);
            }
        }
        JvmValue::ByteArray(_)
        | JvmValue::ShortArray(_)
        | JvmValue::IntArray(_)
        | JvmValue::LongArray(_) => {}
        JvmValue::Skipped => {}
        _ => {}
    }
}

fn parse_single_tile(val: &JvmValue) -> Option<WpTileData> {
    let map = val.as_object()?;
    let x = map.get("x").and_then(|v| v.as_i32()).unwrap_or(0);
    let z = map.get("y").and_then(|v| v.as_i32()).unwrap_or(0);

    let heightmap = map.get("heightMap").and_then(array_to_heightmap);
    let terrain = map.get("terrain").and_then(array_to_u8_16384);
    let water_level = map.get("waterLevel").and_then(array_to_u8_16384);

    Some(WpTileData {
        x,
        z,
        heightmap,
        terrain,
        water_level,
    })
}

fn array_to_heightmap(val: &JvmValue) -> Option<[i16; 16384]> {
    match val {
        JvmValue::ShortArray(arr) => {
            if arr.len() >= 16384 {
                let mut result = [0i16; 16384];
                for (i, &v) in arr.iter().enumerate().take(16384) {
                    result[i] = v;
                }
                Some(result)
            } else {
                None
            }
        }
        JvmValue::IntArray(arr) => {
            if arr.len() >= 16384 {
                let mut result = [0i16; 16384];
                for (i, &v) in arr.iter().enumerate().take(16384) {
                    result[i] = v.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                }
                Some(result)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn array_to_u8_16384(val: &JvmValue) -> Option<[u8; 16384]> {
    match val {
        JvmValue::ByteArray(arr) => {
            if arr.len() >= 16384 {
                let mut result = [0u8; 16384];
                for (i, &v) in arr.iter().enumerate().take(16384) {
                    result[i] = v as u8;
                }
                Some(result)
            } else {
                None
            }
        }
        JvmValue::IntArray(arr) => {
            if arr.len() >= 16384 {
                let mut result = [0u8; 16384];
                for (i, &v) in arr.iter().enumerate().take(16384) {
                    result[i] = v as u8;
                }
                Some(result)
            } else {
                None
            }
        }
        JvmValue::ShortArray(arr) => {
            if arr.len() >= 16384 {
                let mut result = [0u8; 16384];
                for (i, &v) in arr.iter().enumerate().take(16384) {
                    result[i] = v as u8;
                }
                Some(result)
            } else {
                None
            }
        }
        _ => None,
    }
}
