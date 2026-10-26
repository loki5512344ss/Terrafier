use wasmtime::{Caller, Linker};

use crate::context::WorldContext;

const TILE_SIZE: usize = 128;
const HEIGHTMAP_BYTES: usize = TILE_SIZE * TILE_SIZE * 2;
const TERRAIN_BYTES: usize = TILE_SIZE * TILE_SIZE;
const WATER_BYTES: usize = TILE_SIZE * TILE_SIZE;
const TOTAL_BUFFER: usize = HEIGHTMAP_BYTES + TERRAIN_BYTES + WATER_BYTES;

pub fn register_host_functions(linker: &mut Linker<WorldContext>) -> anyhow::Result<()> {
    linker.func_wrap("env", "host_log", host_log)?;
    linker.func_wrap("env", "host_get_seed", host_get_seed)?;
    linker.func_wrap("env", "host_tile_size", host_tile_size)?;

    linker.func_wrap("env", "host_get_height", host_get_height)?;
    linker.func_wrap("env", "host_set_height", host_set_height)?;
    linker.func_wrap("env", "host_get_terrain", host_get_terrain)?;
    linker.func_wrap("env", "host_set_terrain", host_set_terrain)?;
    linker.func_wrap("env", "host_get_water", host_get_water)?;
    linker.func_wrap("env", "host_set_water", host_set_water)?;

    linker.func_wrap("env", "host_alloc_buffers", host_alloc_buffers)?;
    linker.func_wrap("env", "host_load_heightmap", host_load_heightmap)?;
    linker.func_wrap("env", "host_load_terrain", host_load_terrain)?;
    linker.func_wrap("env", "host_load_water", host_load_water)?;
    linker.func_wrap("env", "host_store_heightmap", host_store_heightmap)?;
    linker.func_wrap("env", "host_store_terrain", host_store_terrain)?;
    linker.func_wrap("env", "host_store_water", host_store_water)?;
    Ok(())
}

fn host_log(mut caller: Caller<'_, WorldContext>, level: i32, ptr: i32, len: i32) {
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return;
    };
    let mut buf = vec![0u8; len as usize];
    let _ = mem.read(&caller, ptr as usize, &mut buf);
    let msg = String::from_utf8_lossy(&buf);
    match level {
        0 => log::error!("[wasm] {}", msg),
        1 => log::warn!("[wasm] {}", msg),
        2 => log::info!("[wasm] {}", msg),
        _ => log::debug!("[wasm] {}", msg),
    }
}

fn host_get_seed(caller: Caller<'_, WorldContext>) -> i64 {
    caller.data().seed as i64
}

fn host_tile_size(_caller: Caller<'_, WorldContext>) -> i32 {
    TILE_SIZE as i32
}

fn host_get_height(caller: Caller<'_, WorldContext>, x: i32, z: i32) -> i32 {
    if x < 0 || z < 0 || x >= TILE_SIZE as i32 || z >= TILE_SIZE as i32 {
        return 0;
    }
    caller.data().tile.get_height(x as usize, z as usize) as i32
}

fn host_set_height(mut caller: Caller<'_, WorldContext>, x: i32, z: i32, val: i32) {
    if x < 0 || z < 0 || x >= TILE_SIZE as i32 || z >= TILE_SIZE as i32 {
        return;
    }
    caller
        .data_mut()
        .tile
        .set_height(x as usize, z as usize, val as i16);
}

fn host_get_terrain(caller: Caller<'_, WorldContext>, x: i32, z: i32) -> i32 {
    if x < 0 || z < 0 || x >= TILE_SIZE as i32 || z >= TILE_SIZE as i32 {
        return 0;
    }
    caller.data().tile.get_terrain(x as usize, z as usize) as i32
}

fn host_set_terrain(mut caller: Caller<'_, WorldContext>, x: i32, z: i32, val: i32) {
    if x < 0 || z < 0 || x >= TILE_SIZE as i32 || z >= TILE_SIZE as i32 {
        return;
    }
    caller
        .data_mut()
        .tile
        .set_terrain(x as usize, z as usize, val as u8);
}

fn host_get_water(caller: Caller<'_, WorldContext>, x: i32, z: i32) -> i32 {
    if x < 0 || z < 0 || x >= TILE_SIZE as i32 || z >= TILE_SIZE as i32 {
        return 0;
    }
    caller.data().tile.water_level[(z as usize) * TILE_SIZE + (x as usize)] as i32
}

fn host_set_water(mut caller: Caller<'_, WorldContext>, x: i32, z: i32, val: i32) {
    if x < 0 || z < 0 || x >= TILE_SIZE as i32 || z >= TILE_SIZE as i32 {
        return;
    }
    caller.data_mut().tile.water_level[(z as usize) * TILE_SIZE + (x as usize)] = val as u8;
}

fn host_alloc_buffers(mut caller: Caller<'_, WorldContext>) -> i32 {
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return -1;
    };
    let needed_bytes = TOTAL_BUFFER;
    let current_size = mem.data_size(&caller);
    let pages_needed = (needed_bytes + current_size).div_ceil(65536) - current_size / 65536;
    if mem.grow(&mut caller, pages_needed as u64).is_err() {
        return -1;
    }
    let hm_offset = current_size;
    let ter_offset = hm_offset + HEIGHTMAP_BYTES;
    let wat_offset = ter_offset + TERRAIN_BYTES;
    let ctx = caller.data_mut();
    ctx.hm_offset = hm_offset;
    ctx.ter_offset = ter_offset;
    ctx.wat_offset = wat_offset;
    0
}

fn host_load_heightmap(mut caller: Caller<'_, WorldContext>) -> i32 {
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return -1;
    };
    let offset = caller.data().hm_offset;
    let bytes: Vec<u8> = caller
        .data()
        .tile
        .heightmap
        .iter()
        .flat_map(|h| h.to_le_bytes())
        .collect();
    let _ = mem.write(&mut caller, offset, &bytes);
    offset as i32
}

fn host_load_terrain(mut caller: Caller<'_, WorldContext>) -> i32 {
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return -1;
    };
    let offset = caller.data().ter_offset;
    let terrain = caller.data().tile.terrain;
    let _ = mem.write(&mut caller, offset, &terrain);
    offset as i32
}

fn host_load_water(mut caller: Caller<'_, WorldContext>) -> i32 {
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return -1;
    };
    let offset = caller.data().wat_offset;
    let water = caller.data().tile.water_level;
    let _ = mem.write(&mut caller, offset, &water);
    offset as i32
}

fn host_store_heightmap(mut caller: Caller<'_, WorldContext>) {
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return;
    };
    let offset = caller.data().hm_offset;
    let mut buf = vec![0u8; HEIGHTMAP_BYTES];
    let _ = mem.read(&caller, offset, &mut buf);
    let ctx = caller.data_mut();
    let (chunks, _) = buf.as_chunks::<2>();
    for (i, chunk) in chunks.iter().enumerate() {
        ctx.tile.heightmap[i] = i16::from_le_bytes(*chunk);
    }
}

fn host_store_terrain(mut caller: Caller<'_, WorldContext>) {
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return;
    };
    let offset = caller.data().ter_offset;
    let mut buf = [0u8; TERRAIN_BYTES];
    let _ = mem.read(&caller, offset, &mut buf);
    caller.data_mut().tile.terrain = buf;
}

fn host_store_water(mut caller: Caller<'_, WorldContext>) {
    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
        return;
    };
    let offset = caller.data().wat_offset;
    let mut buf = [0u8; WATER_BYTES];
    let _ = mem.read(&caller, offset, &mut buf);
    caller.data_mut().tile.water_level = buf;
}
