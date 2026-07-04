use super::block_name;
use super::blocks::{BLOCK_SET, BLOCK_SET_EXTENDED, bits_needed, pack_indices};
use crate::io::layer_export::{LayerExport, apply_layers, biome_name};
use crate::io::minecraft::{Result, data_version};
use crate::model::tile::Tile;
use std::collections::{BTreeSet, HashMap};

fn make_section(
    sec_y: i32,
    block_states: HashMap<String, terrafier_nbt::Tag>,
    biomes: HashMap<String, terrafier_nbt::Tag>,
) -> terrafier_nbt::Tag {
    let mut sec = HashMap::new();
    sec.insert("Y".into(), terrafier_nbt::Tag::Byte(sec_y as i8));
    sec.insert(
        "block_states".into(),
        terrafier_nbt::Tag::Compound(block_states),
    );
    sec.insert("biomes".into(), terrafier_nbt::Tag::Compound(biomes));
    terrafier_nbt::Tag::Compound(sec)
}

fn make_biomes(biome_name: &str) -> HashMap<String, terrafier_nbt::Tag> {
    let mut entry = HashMap::new();
    entry.insert("Name".into(), terrafier_nbt::Tag::String(biome_name.into()));
    let palette = vec![terrafier_nbt::Tag::Compound(entry)];
    let mut biomes = HashMap::new();
    biomes.insert("palette".into(), terrafier_nbt::Tag::List(palette));
    biomes
}

/// Build a single chunk's NBT data (raw uncompressed bytes).
/// Returns empty Vec if the chunk has no blocks at all.
pub(super) fn build_chunk_nbt(
    chunk_x: i32,
    chunk_z: i32,
    tile: &Tile,
    chunk_lx: usize,
    chunk_lz: usize,
) -> Result<Vec<u8>> {
    let mut compound = HashMap::new();
    compound.insert("xPos".into(), terrafier_nbt::Tag::Int(chunk_x));
    compound.insert("zPos".into(), terrafier_nbt::Tag::Int(chunk_z));
    compound.insert(
        "DataVersion".into(),
        terrafier_nbt::Tag::Int(data_version()),
    );
    compound.insert("Status".into(), terrafier_nbt::Tag::String("full".into()));
    let min_sec = (tile.min_height as i32 >> 4).max(-4);
    let max_sec = (tile.max_height as i32 >> 4).min(20);
    let mut sections: Vec<terrafier_nbt::Tag> = Vec::new();
    for sec_y in min_sec..=max_sec {
        let sec_base = sec_y * 16;
        let mut indices = Vec::with_capacity(4096);
        for y_rel in 0..16 {
            let global_y = sec_base + y_rel;
            for lz in 0..16 {
                for lx in 0..16 {
                    let tile_lx = chunk_lx * 16 + lx;
                    let tile_lz = chunk_lz * 16 + lz;
                    let idx = if tile_lx >= 128 || tile_lz >= 128 {
                        0u16
                    } else {
                        let surface_y = tile.heightmap[tile_lz * 128 + tile_lx] as i32;
                        let terrain_id = tile.terrain[tile_lz * 128 + tile_lx];
                        let name = block_name(terrain_id, global_y, surface_y);
                        BLOCK_SET.iter().position(|s| *s == name).unwrap_or(0) as u16
                    };
                    indices.push(idx);
                }
            }
        }
        if !indices.iter().any(|&i| i != 0) {
            continue;
        }
        let bits = bits_needed(BLOCK_SET.len());
        let packed = pack_indices(&indices, bits);
        let palette_tags: Vec<terrafier_nbt::Tag> = BLOCK_SET
            .iter()
            .map(|name| {
                let mut entry = HashMap::new();
                entry.insert("Name".into(), terrafier_nbt::Tag::String(name.to_string()));
                terrafier_nbt::Tag::Compound(entry)
            })
            .collect();
        let mut block_states = HashMap::new();
        block_states.insert("palette".into(), terrafier_nbt::Tag::List(palette_tags));
        block_states.insert("data".into(), terrafier_nbt::Tag::LongArray(packed));
        let biomes = make_biomes("minecraft:plains");
        sections.push(make_section(sec_y, block_states, biomes));
    }
    if sections.is_empty() {
        return Ok(Vec::new());
    }
    compound.insert("sections".into(), terrafier_nbt::Tag::List(sections));
    let chunk_tag = terrafier_nbt::Tag::Compound(compound);
    let bytes = terrafier_nbt::io::writer::to_bytes(&chunk_tag)?;
    Ok(bytes)
}

/// Build a single chunk's NBT data, applying layer exporters for block/biome overrides.
pub(super) fn build_chunk_nbt_with_layers(
    chunk_x: i32,
    chunk_z: i32,
    tile: &Tile,
    chunk_lx: usize,
    chunk_lz: usize,
    layer_exporters: &[&dyn LayerExport],
) -> Result<Vec<u8>> {
    let mut compound = HashMap::new();
    compound.insert("xPos".into(), terrafier_nbt::Tag::Int(chunk_x));
    compound.insert("zPos".into(), terrafier_nbt::Tag::Int(chunk_z));
    compound.insert(
        "DataVersion".into(),
        terrafier_nbt::Tag::Int(data_version()),
    );
    compound.insert("Status".into(), terrafier_nbt::Tag::String("full".into()));
    let min_sec = (tile.min_height as i32 >> 4).max(-4);
    let max_sec = (tile.max_height as i32 >> 4).min(20);
    let mut sections: Vec<terrafier_nbt::Tag> = Vec::new();
    for sec_y in min_sec..=max_sec {
        let sec_base = sec_y * 16;
        let mut indices = Vec::with_capacity(4096);
        for y_rel in 0..16 {
            let global_y = sec_base + y_rel;
            for lz in 0..16 {
                for lx in 0..16 {
                    let tile_lx = chunk_lx * 16 + lx;
                    let tile_lz = chunk_lz * 16 + lz;
                    let idx = if tile_lx >= 128 || tile_lz >= 128 {
                        0u16
                    } else {
                        let surface_y = tile.heightmap[tile_lz * 128 + tile_lx] as i32;
                        let terrain_id = tile.terrain[tile_lz * 128 + tile_lx];
                        let base_name = block_name(terrain_id, global_y, surface_y);
                        let name = apply_layers(
                            tile,
                            tile_lx,
                            tile_lz,
                            global_y,
                            surface_y,
                            base_name,
                            layer_exporters,
                        );
                        BLOCK_SET_EXTENDED
                            .iter()
                            .position(|s| *s == name)
                            .unwrap_or(0) as u16
                    };
                    indices.push(idx);
                }
            }
        }
        if !indices.iter().any(|&i| i != 0) {
            continue;
        }
        let used_blocks: Vec<&str> = {
            let mut seen = BTreeSet::new();
            let mut names = Vec::new();
            for &i in &indices {
                if i < BLOCK_SET_EXTENDED.len() as u16 && seen.insert(i) {
                    names.push(BLOCK_SET_EXTENDED[i as usize]);
                }
            }
            names
        };
        let bits = bits_needed(used_blocks.len());
        let mut palette_map: HashMap<&str, u16> = HashMap::new();
        for (pi, name) in used_blocks.iter().enumerate() {
            palette_map.insert(name, pi as u16);
        }
        let remapped: Vec<u16> = indices
            .iter()
            .map(|&i| {
                if i < BLOCK_SET_EXTENDED.len() as u16 {
                    palette_map[BLOCK_SET_EXTENDED[i as usize]]
                } else {
                    0
                }
            })
            .collect();
        let packed = pack_indices(&remapped, bits);
        let palette_tags: Vec<terrafier_nbt::Tag> = used_blocks
            .iter()
            .map(|name| {
                let mut entry = HashMap::new();
                entry.insert("Name".into(), terrafier_nbt::Tag::String(name.to_string()));
                terrafier_nbt::Tag::Compound(entry)
            })
            .collect();
        let mut block_states = HashMap::new();
        block_states.insert("palette".into(), terrafier_nbt::Tag::List(palette_tags));
        block_states.insert("data".into(), terrafier_nbt::Tag::LongArray(packed));
        let biome_at = biome_name(tile, chunk_lx * 16, chunk_lz * 16, layer_exporters);
        let biomes = make_biomes(biome_at);
        sections.push(make_section(sec_y, block_states, biomes));
    }
    if sections.is_empty() {
        return Ok(Vec::new());
    }
    compound.insert("sections".into(), terrafier_nbt::Tag::List(sections));
    let chunk_tag = terrafier_nbt::Tag::Compound(compound);
    let bytes = terrafier_nbt::io::writer::to_bytes(&chunk_tag)?;
    Ok(bytes)
}
