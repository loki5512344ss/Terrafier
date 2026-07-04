//! Minecraft save writing — orchestrate world-to-region export.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use crate::io::layer_export::LayerExport;
use crate::io::minecraft::{Result, data_version};
use crate::model::tile::Tile;
use crate::model::world::World;

use super::chunk::{build_chunk_nbt, build_chunk_nbt_with_layers};

#[allow(clippy::type_complexity)]
type RegionTiles<'a> = BTreeMap<(i32, i32), Vec<(&'a (i32, i32), &'a Tile)>>;

/// Save a Terrafier World to a Minecraft save directory.
pub fn save_world(world: &World, output_path: &Path) -> Result<()> {
    fs::create_dir_all(output_path.join("region"))?;

    let level_tag = build_level_dat(world)?;
    let level_bytes = terrafier_nbt::io::writer::to_gzip_bytes(&level_tag)?;
    fs::write(output_path.join("level.dat"), &level_bytes)?;

    let dim = match world.dimensions.first() {
        Some(d) => d,
        None => return Ok(()),
    };

    let mut regions: RegionTiles = BTreeMap::new();
    for (key, tile) in &dim.tiles {
        let (tx, tz) = key;
        let rx = tx >> 2;
        let rz = tz >> 2;
        regions.entry((rx, rz)).or_default().push((key, tile));
    }

    for ((rx, rz), tile_refs) in &regions {
        let mut region = terrafier_fastanvil::io::region::Region::new(*rx, *rz);

        for (_key, tile) in tile_refs {
            for chunk_lx in 0..8usize {
                for chunk_lz in 0..8usize {
                    let chunk_x = tile.x * 8 + chunk_lx as i32;
                    let chunk_z = tile.z * 8 + chunk_lz as i32;

                    let region_local_x = (chunk_x & 31) as u8;
                    let region_local_z = (chunk_z & 31) as u8;

                    let chunk_data = build_chunk_nbt(chunk_x, chunk_z, tile, chunk_lx, chunk_lz)?;

                    if chunk_data.is_empty() {
                        continue;
                    }

                    region.set_chunk_data(region_local_x, region_local_z, chunk_data);
                }
            }
        }

        let region_bytes = region.to_bytes()?;
        let file_name = format!("r.{}.{}.mca", rx, rz);
        fs::write(output_path.join("region").join(&file_name), &region_bytes)?;
    }

    Ok(())
}

/// Save a Terrafier World to a Minecraft save directory, applying layer exporters.
pub fn save_world_with_layers(
    world: &World,
    output_path: &Path,
    layer_exporters: &[&dyn LayerExport],
) -> Result<()> {
    fs::create_dir_all(output_path.join("region"))?;

    let level_tag = build_level_dat(world)?;
    let level_bytes = terrafier_nbt::io::writer::to_gzip_bytes(&level_tag)?;
    fs::write(output_path.join("level.dat"), &level_bytes)?;

    let dim = match world.dimensions.first() {
        Some(d) => d,
        None => return Ok(()),
    };

    let mut regions: RegionTiles = BTreeMap::new();
    for (key, tile) in &dim.tiles {
        let (tx, tz) = key;
        let rx = tx >> 2;
        let rz = tz >> 2;
        regions.entry((rx, rz)).or_default().push((key, tile));
    }

    for ((rx, rz), tile_refs) in &regions {
        let mut region = terrafier_fastanvil::io::region::Region::new(*rx, *rz);

        for (_key, tile) in tile_refs {
            for chunk_lx in 0..8usize {
                for chunk_lz in 0..8usize {
                    let chunk_x = tile.x * 8 + chunk_lx as i32;
                    let chunk_z = tile.z * 8 + chunk_lz as i32;

                    let region_local_x = (chunk_x & 31) as u8;
                    let region_local_z = (chunk_z & 31) as u8;

                    let chunk_data = build_chunk_nbt_with_layers(
                        chunk_x,
                        chunk_z,
                        tile,
                        chunk_lx,
                        chunk_lz,
                        layer_exporters,
                    )?;

                    if chunk_data.is_empty() {
                        continue;
                    }

                    region.set_chunk_data(region_local_x, region_local_z, chunk_data);
                }
            }
        }

        let region_bytes = region.to_bytes()?;
        let file_name = format!("r.{}.{}.mca", rx, rz);
        fs::write(output_path.join("region").join(&file_name), &region_bytes)?;
    }

    Ok(())
}

fn build_level_dat(world: &World) -> Result<terrafier_nbt::Tag> {
    use terrafier_nbt::Tag;
    let mut data = HashMap::new();
    data.insert("LevelName".into(), Tag::String(world.name.clone()));
    data.insert("DataVersion".into(), Tag::Int(data_version()));
    data.insert("version".into(), Tag::Int(19133));

    let mut world_gen = HashMap::new();
    world_gen.insert("seed".into(), Tag::Long(world.seed as i64));
    data.insert("WorldGenSettings".into(), Tag::Compound(world_gen));

    let mut root = HashMap::new();
    root.insert("Data".into(), Tag::Compound(data));

    Ok(Tag::Compound(root))
}
