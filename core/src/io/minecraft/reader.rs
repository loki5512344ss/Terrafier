//! Minecraft save reading — load chunks from .mca, parse NBT, build tiles.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use terrafier_fastanvil::io::region::Region;
use terrafier_nbt::io::reader::read_gzip;

use crate::model::dimension::Dimension;
use crate::model::tile::Tile;
use crate::model::world::World;

use super::version::version_from_data_version;
use super::{MinecraftIOError, Result};

/// Load a Minecraft save directory into a Terrafier World model.
pub fn load_save(path: &Path) -> Result<World> {
    if !path.is_dir() {
        return Err(MinecraftIOError::NotASave(path.display().to_string()));
    }

    let level_dat_path = path.join("level.dat");
    if !level_dat_path.exists() {
        return Err(MinecraftIOError::MissingLevelDat);
    }
    let level_data = fs::read(&level_dat_path)?;
    let level_tag = read_gzip(&level_data)?;

    let (world_name, seed, data_version) = parse_level_dat(&level_tag);
    let platform = version_from_data_version(data_version)
        .ok_or(MinecraftIOError::UnsupportedVersion(data_version))?;

    let region_dir = path.join("region");
    if !region_dir.is_dir() {
        return Err(MinecraftIOError::MissingRegionDir);
    }

    let mut tiles: HashMap<(i32, i32), Tile> = HashMap::new();

    let region_entries = fs::read_dir(&region_dir)?;
    for entry in region_entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "mca") {
            let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let parts: Vec<&str> = file_name.split('.').collect();
            if parts.len() >= 3
                && let (Ok(rx), Ok(rz)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>())
            {
                let region_bytes = fs::read(&path)?;
                let region = Region::from_bytes(rx, rz, &region_bytes)?;
                for (local_x, local_z) in region.chunk_coords() {
                    let chunk_data = match region.get_chunk_data(local_x, local_z) {
                        Some(data) => data,
                        None => {
                            log::warn!(
                                "corrupted chunk data at ({}, {}) in region ({}, {})",
                                local_x,
                                local_z,
                                rx,
                                rz
                            );
                            continue;
                        }
                    };
                    if let Ok(chunk_tag) = read_gzip(chunk_data)
                        && let Some(chunk) =
                            terrafier_fastanvil::io::chunk::Chunk::from_nbt(&chunk_tag)
                    {
                        let tile_x = chunk.x >> 3;
                        let tile_z = chunk.z >> 3;

                        let chunk_local_x = (chunk.x & 7) as usize;
                        let chunk_local_z = (chunk.z & 7) as usize;

                        let tile = tiles.entry((tile_x, tile_z)).or_insert_with(|| {
                            Tile::new(tile_x, tile_z, platform.min_height, platform.max_height)
                        });

                        for lx in 0..16usize {
                            for lz in 0..16usize {
                                let mut surface_y = None;

                                if !chunk.sections.is_empty() {
                                    let mut sorted: Vec<_> = chunk.sections.iter().collect();
                                    sorted.sort_by_key(|b| std::cmp::Reverse(b.section_y));

                                    'section_loop: for section in &sorted {
                                        if section.palette.is_empty() {
                                            continue;
                                        }

                                        let non_air_indices: Vec<usize> = section
                                            .palette
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, p)| {
                                                p.get("Name").is_some_and(|n| {
                                                    matches!(
                                                        n,
                                                        terrafier_nbt::Tag::String(s)
                                                            if s != "minecraft:air"
                                                    )
                                                })
                                            })
                                            .map(|(i, _)| i)
                                            .collect();

                                        if non_air_indices.is_empty() {
                                            continue;
                                        }

                                        if section.block_data.is_empty() {
                                            surface_y = Some((section.section_y as i32) * 16 + 15);
                                            break;
                                        }

                                        let bpe = std::cmp::max(
                                            4,
                                            (section.palette.len() as f64).log2().ceil() as usize,
                                        );

                                        for dy in (0..16usize).rev() {
                                            let block_index = dy * 256 + lz * 16 + lx;
                                            let bit_pos = block_index * bpe;
                                            let long_idx = bit_pos / 64;
                                            let bit_off = bit_pos % 64;

                                            if let Some(&word) = section.block_data.get(long_idx) {
                                                let palette_index = if bit_off + bpe <= 64 {
                                                    let mask = (1u64 << bpe) - 1;
                                                    ((word as u64 >> bit_off) & mask) as usize
                                                } else {
                                                    let low_bits = 64 - bit_off;
                                                    let low = ((word as u64) >> bit_off) as usize;
                                                    let high_bits = bit_off + bpe - 64;
                                                    let high = section
                                                        .block_data
                                                        .get(long_idx + 1)
                                                        .map(|&w| {
                                                            (w as u64 & ((1u64 << high_bits) - 1))
                                                                as usize
                                                        })
                                                        .unwrap_or(0);
                                                    low | (high << low_bits)
                                                };

                                                if non_air_indices.contains(&palette_index) {
                                                    surface_y = Some(
                                                        (section.section_y as i32) * 16 + dy as i32,
                                                    );
                                                    break 'section_loop;
                                                }
                                            }
                                        }
                                    }
                                }

                                let tile_local_x = chunk_local_x * 16 + lx;
                                let tile_local_z = chunk_local_z * 16 + lz;

                                if tile_local_x < 128
                                    && tile_local_z < 128
                                    && let Some(y) = surface_y
                                {
                                    let clamped =
                                        (y as i16).clamp(tile.min_height, tile.max_height);
                                    tile.heightmap[tile_local_z * 128 + tile_local_x] = clamped;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let dimension = Dimension {
        name: "overworld".to_string(),
        tiles,
        min_height: platform.min_height,
        max_height: platform.max_height,
        seed,
    };

    Ok(World {
        name: world_name,
        platform,
        dimensions: vec![dimension],
        seed,
    })
}

fn parse_level_dat(root: &terrafier_nbt::Tag) -> (String, u64, i32) {
    let default = || ("Unknown".to_string(), 0u64, 3954i32);
    match root {
        terrafier_nbt::Tag::Compound(map) => {
            let data = match map.get("Data") {
                Some(terrafier_nbt::Tag::Compound(d)) => d,
                _ => return default(),
            };
            let name = match data.get("LevelName") {
                Some(terrafier_nbt::Tag::String(s)) => s.clone(),
                _ => "Unknown".to_string(),
            };
            let seed = match data.get("WorldGenSettings").and_then(|w| match w {
                terrafier_nbt::Tag::Compound(m) => m.get("seed"),
                _ => None,
            }) {
                Some(terrafier_nbt::Tag::Long(s)) => *s as u64,
                _ => 0,
            };
            let data_version = match data.get("DataVersion") {
                Some(terrafier_nbt::Tag::Int(v)) => *v,
                _ => 3954,
            };
            (name, seed, data_version)
        }
        _ => default(),
    }
}

pub fn discover_region_files(world_path: &Path) -> Result<Vec<PathBuf>> {
    let region_dir = world_path.join("region");
    if !region_dir.is_dir() {
        return Err(MinecraftIOError::MissingRegionDir);
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&region_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "mca") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}
