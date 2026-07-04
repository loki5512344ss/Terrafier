//! Minecraft save writing — orchestrate world-to-region export.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use crate::io::layer_export::LayerExport;
use crate::io::minecraft::{Result, data_version};
use crate::model::tile::Tile;
use crate::model::world::World;

use super::chunk::{build_chunk_nbt, build_chunk_nbt_with_layers};

type RegionTiles<'a> = BTreeMap<(i32, i32), Vec<(&'a (i32, i32), &'a Tile)>>;

fn dim_region_path(output_path: &Path, dim_name: &str) -> std::path::PathBuf {
    match dim_name {
        "the_nether" => output_path.join("DIM-1").join("region"),
        "the_end" => output_path.join("DIM1").join("region"),
        _ => output_path.join("region"),
    }
}

fn write_dimension(
    dim: &crate::model::dimension::Dimension,
    output_path: &Path,
    build_chunk: &dyn Fn(i32, i32, &Tile, usize, usize) -> Result<Vec<u8>>,
) -> Result<()> {
    let region_path = dim_region_path(output_path, &dim.name);
    fs::create_dir_all(&region_path)?;

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

                    let chunk_data = build_chunk(chunk_x, chunk_z, tile, chunk_lx, chunk_lz)?;

                    if chunk_data.is_empty() {
                        continue;
                    }

                    region.set_chunk_data(region_local_x, region_local_z, chunk_data);
                }
            }
        }

        let region_bytes = region.to_bytes()?;
        let file_name = format!("r.{}.{}.mca", rx, rz);
        fs::write(region_path.join(&file_name), &region_bytes)?;
    }

    Ok(())
}

/// Save a Terrafier World to a Minecraft save directory.
pub fn save_world(world: &World, output_path: &Path) -> Result<()> {
    let level_tag = build_level_dat(world)?;
    let level_bytes = terrafier_nbt::io::writer::to_gzip_bytes(&level_tag)?;
    fs::write(output_path.join("level.dat"), &level_bytes)?;

    for dim in &world.dimensions {
        write_dimension(dim, output_path, &|cx, cz, tile, clx, clz| {
            build_chunk_nbt(cx, cz, tile, clx, clz)
        })?;
    }

    Ok(())
}

/// Save a Terrafier World to a Minecraft save directory, applying layer exporters.
pub fn save_world_with_layers(
    world: &World,
    output_path: &Path,
    layer_exporters: &[&dyn LayerExport],
) -> Result<()> {
    let level_tag = build_level_dat(world)?;
    let level_bytes = terrafier_nbt::io::writer::to_gzip_bytes(&level_tag)?;
    fs::write(output_path.join("level.dat"), &level_bytes)?;

    for dim in &world.dimensions {
        write_dimension(dim, output_path, &|cx, cz, tile, clx, clz| {
            build_chunk_nbt_with_layers(cx, cz, tile, clx, clz, layer_exporters)
        })?;
    }

    Ok(())
}

fn build_level_dat(world: &World) -> Result<terrafier_nbt::Tag> {
    use terrafier_nbt::Tag;
    let mut data = HashMap::new();
    data.insert("LevelName".into(), Tag::String(world.name.clone()));
    data.insert("DataVersion".into(), Tag::Int(data_version()));
    data.insert("version".into(), Tag::Int(19133));
    data.insert("RandomSeed".into(), Tag::Long(world.seed as i64));
    data.insert("GameType".into(), Tag::Int(1));
    data.insert("hardcore".into(), Tag::Byte(0));
    data.insert("Difficulty".into(), Tag::Byte(2));
    data.insert("DifficultyLocked".into(), Tag::Byte(0));
    data.insert("SpawnX".into(), Tag::Int(0));
    data.insert("SpawnY".into(), Tag::Int(64));
    data.insert("SpawnZ".into(), Tag::Int(0));
    data.insert("Time".into(), Tag::Long(0));
    data.insert("DayTime".into(), Tag::Long(1000));
    data.insert("allowCommands".into(), Tag::Byte(1));
    data.insert("initialized".into(), Tag::Byte(1));
    data.insert("LastPlayed".into(), Tag::Long(0));

    let mut data_packs = HashMap::new();
    data_packs.insert("Enabled".into(), Tag::List(vec![]));
    data_packs.insert("Disabled".into(), Tag::List(vec![]));
    data.insert("DataPacks".into(), Tag::Compound(data_packs));

    let mut world_gen = HashMap::new();
    world_gen.insert("seed".into(), Tag::Long(world.seed as i64));
    data.insert("WorldGenSettings".into(), Tag::Compound(world_gen));

    let mut root = HashMap::new();
    root.insert("Data".into(), Tag::Compound(data));

    Ok(Tag::Compound(root))
}
