//! Golden integration test for loading a synthetic Minecraft .mca region file.
//!
//! This test creates a minimal valid Anvil world save in a temporary directory,
//! loads it via `load_save`, and verifies the parsed chunk data is correct.

use std::collections::HashMap;

use terrafier_core::io::minecraft::load_save;
use terrafier_fastanvil::io::chunk::Chunk;
use terrafier_fastanvil::io::region::Region;
use terrafier_nbt::Tag;
use terrafier_nbt::io::reader::read_gzip;
use terrafier_nbt::io::writer::to_gzip_bytes;

// ---------------------------------------------------------------------------
// Helpers: synthetic NBT and region file writer
// ---------------------------------------------------------------------------

/// Create a minimal level.dat as gzip-compressed NBT bytes.
fn make_level_dat() -> Vec<u8> {
    let mut data = HashMap::new();
    data.insert("DataVersion".into(), Tag::Int(3954));
    data.insert("LevelName".into(), Tag::String("test_world".into()));

    let mut wgs = HashMap::new();
    wgs.insert("seed".into(), Tag::Long(42));
    data.insert("WorldGenSettings".into(), Tag::Compound(wgs));

    let mut root = HashMap::new();
    root.insert("Data".into(), Tag::Compound(data));

    to_gzip_bytes(&Tag::Compound(root)).unwrap()
}

/// Build a minimal valid chunk NBT tag, gzip-compress it, and return the
/// gzip-compressed bytes ready for storage in an .mca file.
fn make_chunk_nbt_gzip(cx: i32, cz: i32) -> Vec<u8> {
    let mut chunk = HashMap::new();
    chunk.insert("xPos".into(), Tag::Int(cx));
    chunk.insert("zPos".into(), Tag::Int(cz));
    chunk.insert("DataVersion".into(), Tag::Int(3954));
    chunk.insert("Status".into(), Tag::String("full".into()));

    // Single section at Y=4 (blocks y = 64..79)
    let mut section = HashMap::new();
    section.insert("Y".into(), Tag::Byte(4));

    // block_states with a single-entry palette
    let mut palette_list: Vec<Tag> = Vec::new();
    let mut grass = HashMap::new();
    grass.insert("Name".into(), Tag::String("minecraft:grass_block".into()));
    palette_list.push(Tag::Compound(grass));
    let mut block_states = HashMap::new();
    block_states.insert("palette".into(), Tag::List(palette_list));
    section.insert("block_states".into(), Tag::Compound(block_states));

    // biomes with a single-entry palette
    let mut biome_palette: Vec<Tag> = Vec::new();
    let mut plains = HashMap::new();
    plains.insert("Name".into(), Tag::String("minecraft:plains".into()));
    biome_palette.push(Tag::Compound(plains));
    let mut biomes = HashMap::new();
    biomes.insert("palette".into(), Tag::List(biome_palette));
    section.insert("biomes".into(), Tag::Compound(biomes));

    let sections = vec![Tag::Compound(section)];
    chunk.insert("sections".into(), Tag::List(sections));

    to_gzip_bytes(&Tag::Compound(chunk)).unwrap()
}

/// Build raw .mca region file bytes containing a single chunk at global
/// coordinates (cx, cz).
///
/// The chunk payload is stored as *uncompressed* (compression type 3), so
/// `Region::from_bytes` treats it as a pass-through and `load_save`'s
/// subsequent `read_gzip` call can decompress it.
fn build_region_bytes(cx: i32, cz: i32, chunk_gzip: &[u8]) -> Vec<u8> {
    let sector_size: u64 = 4096;

    // .mca chunk entry: [4B total_len][1B compression_type][payload…]
    let total_len = 1 + chunk_gzip.len();
    let mut chunk_entry = Vec::with_capacity(4 + total_len);
    chunk_entry.extend_from_slice(&(total_len as u32).to_be_bytes());
    chunk_entry.push(3); // compression type = Uncompressed
    chunk_entry.extend_from_slice(chunk_gzip);

    // Pad to sector boundary
    while chunk_entry.len() % sector_size as usize != 0 {
        chunk_entry.push(0);
    }

    let sectors_needed = (chunk_entry.len() / sector_size as usize) as u32;

    // Header tables (1024 entries each)
    let mut locations = [0u32; 1024];
    let mut timestamps = [0u32; 1024];

    let local_x = cx.rem_euclid(32) as usize;
    let local_z = cz.rem_euclid(32) as usize;
    let index = local_z * 32 + local_x;

    let offset: u32 = 2; // first sector after the 2-sector header
    locations[index] = (offset << 8) | (sectors_needed & 0xFF);
    timestamps[index] = 1;

    // Assemble output: 2 sectors of header + chunk data
    let mut output = Vec::new();

    for loc in locations.iter() {
        output.extend_from_slice(&loc.to_be_bytes());
    }
    for ts in timestamps.iter() {
        output.extend_from_slice(&ts.to_be_bytes());
    }
    while output.len() < 2 * sector_size as usize {
        output.push(0);
    }

    output.extend_from_slice(&chunk_entry);
    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_golden_mca_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let world_path = dir.path().join("test_world");
    std::fs::create_dir_all(world_path.join("region")).unwrap();

    // Write level.dat
    std::fs::write(world_path.join("level.dat"), make_level_dat()).unwrap();

    // Build and write region r.0.0.mca
    let chunk_gzip = make_chunk_nbt_gzip(0, 0);
    let region_bytes = build_region_bytes(0, 0, &chunk_gzip);
    std::fs::write(world_path.join("region/r.0.0.mca"), &region_bytes).unwrap();

    // --- 1. load_save integration ---
    let world = load_save(&world_path).expect("load_save should succeed");

    assert_eq!(world.name, "test_world");
    assert_eq!(world.seed, 42);
    assert_eq!(world.dimensions.len(), 1, "should have one dimension");

    let dim = &world.dimensions[0];
    assert_eq!(dim.name, "overworld");
    assert_eq!(dim.seed, 42);

    // Chunks are dispatched to tiles; verify we have data.
    assert!(!dim.tiles.is_empty(), "tiles should contain parsed chunks");
    let tile = dim.tiles.get(&(0, 0)).expect("tile (0,0) should exist");
    // Chunk at global (0,0) → tile (0,0), height = 4 * 16 + 15 = 79
    assert_eq!(tile.heightmap[0], 79, "first block height should be 79");

    // --- 2. Direct region + chunk parsing verification ---
    let region = Region::from_bytes(0, 0, &region_bytes).expect("region should parse");
    assert_eq!(region.chunk_count(), 1, "region should contain 1 chunk");
    let coords = region.chunk_coords();
    assert_eq!(coords, vec![(0, 0)]);

    let raw = region
        .get_chunk_data(0, 0)
        .expect("chunk (0,0) should exist");
    assert!(!raw.is_empty(), "chunk data should not be empty");

    // The stored payload is gzip-compressed NBT → decompress via read_gzip
    let chunk_tag = read_gzip(raw).expect("chunk data should be valid gzip NBT");
    let chunk = Chunk::from_nbt(&chunk_tag).expect("chunk NBT should be parseable");

    // --- 3. Field-level assertions ---
    assert_eq!(chunk.x, 0, "chunk xPos");
    assert_eq!(chunk.z, 0, "chunk zPos");
    assert_eq!(chunk.data_version, 3954, "chunk DataVersion");
    assert_eq!(chunk.status.as_deref(), Some("full"), "chunk Status");

    // Sections
    assert_eq!(chunk.sections.len(), 1, "should have 1 section");
    let section = &chunk.sections[0];
    assert_eq!(section.section_y, 4, "section Y");

    // Block palette
    assert_eq!(section.palette.len(), 1, "block palette size");
    let block_name = section.palette[0]
        .get("Name")
        .and_then(|t| match t {
            Tag::String(s) => Some(s.as_str()),
            _ => None,
        })
        .expect("block palette entry should have Name");
    assert_eq!(block_name, "minecraft:grass_block");

    // Biome palette
    assert_eq!(section.biome_palette.len(), 1, "biome palette size");
    let biome_name = section.biome_palette[0]
        .get("Name")
        .and_then(|t| match t {
            Tag::String(s) => Some(s.as_str()),
            _ => None,
        })
        .expect("biome palette entry should have Name");
    assert_eq!(biome_name, "minecraft:plains");

    // --- 4. Region round-trip via to_bytes / from_bytes ---
    let reencoded = region.to_bytes().expect("region should re-serialize");
    let region2 = Region::from_bytes(0, 0, &reencoded).expect("re-serialized region should parse");
    assert_eq!(
        region2.chunk_count(),
        1,
        "round-tripped region should still have 1 chunk"
    );
}
