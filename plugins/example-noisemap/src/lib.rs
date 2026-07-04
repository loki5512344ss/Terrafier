#![no_std]

// Allocator for WASM — bump allocator
// SAFETY: WASM is single-threaded; no concurrent access to BUMP is possible.
// The allocator is only called during plugin initialization before tile generation.
static mut BUMP: usize = 4096; // start after string data

#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: i32) -> i32 {
    // SAFETY: WASM is single-threaded; no concurrent access to BUMP is possible.
    // The allocator is only called during plugin initialization before tile generation.
    unsafe {
        let ptr = BUMP as i32;
        BUMP += size as usize;
        ptr
    }
}

// Host imports
// SAFETY: These host functions are guaranteed by the Terrafier WASM host ABI
// to be callable from within generate_tile. The host validates all coordinates
// (0..127) internally. All set functions are idempotent and panic-free.
unsafe extern "C" {
    fn host_log(level: i32, ptr: i32, len: i32);
    fn host_get_seed() -> i64;
    fn host_tile_size() -> i32;
    fn host_get_height(x: i32, z: i32) -> i32;
    fn host_set_height(x: i32, z: i32, val: i32);
    fn host_get_terrain(x: i32, z: i32) -> i32;
    fn host_set_terrain(x: i32, z: i32, val: i32);
    fn host_get_water(x: i32, z: i32) -> i32;
    fn host_set_water(x: i32, z: i32, val: i32);
    // Bulk transfer (faster)
    fn host_alloc_buffers() -> i32;
    fn host_load_heightmap() -> i32;
    fn host_load_terrain() -> i32;
    fn host_load_water() -> i32;
    fn host_store_heightmap();
    fn host_store_terrain();
    fn host_store_water();
}

// Simple pseudo-random number generator (LCG)
// SAFETY: WASM execution is single-threaded. RNG_STATE is only accessed
// during tile generation which runs sequentially per tile.
static mut RNG_STATE: u64 = 0;

// SAFETY: WASM execution is single-threaded. RNG_STATE is only accessed
// during tile generation which runs sequentially per tile.
unsafe fn rng() -> u32 {
    // SAFETY: Same single-thread guarantee as RNG_STATE declaration.
    unsafe {
        RNG_STATE = RNG_STATE.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (RNG_STATE >> 32) as u32
    }
}

unsafe fn rng_range(min: f64, max: f64) -> f64 {
    // SAFETY: rng() only accesses RNG_STATE, which is safe under single-threaded WASM.
    let r = unsafe { rng() } as f64 / u32::MAX as f64;
    min + r * (max - min)
}

// Simple 2D noise function (value noise)
unsafe fn noise_2d(x: f64, z: f64, seed: u64) -> f64 {
    let xi = x as i32;
    let zi = z as i32;
    let xf = x - xi as f64;
    let zf = z - zi as f64;

    fn hash(x: i32, z: i32, seed: u64) -> f64 {
        let h = (x as u64).wrapping_mul(374761393)
            .wrapping_add((z as u64).wrapping_mul(668265263))
            .wrapping_add(seed)
            .wrapping_mul(1274126177);
        ((h ^ (h >> 16)) as u32) as f64 / u32::MAX as f64
    }

    let n00 = hash(xi, zi, seed);
    let n10 = hash(xi + 1, zi, seed);
    let n01 = hash(xi, zi + 1, seed);
    let n11 = hash(xi + 1, zi + 1, seed);

    let sx = xf * xf * (3.0 - 2.0 * xf);
    let sz = zf * zf * (3.0 - 2.0 * zf);

    let nx0 = n00 + (n10 - n00) * sx;
    let nx1 = n01 + (n11 - n01) * sx;
    nx0 + (nx1 - nx0) * sz
}

// Helper to log strings from WASM
// SAFETY: Address 0-1024 in WASM linear memory is reserved as a scratch buffer.
// The host guarantees this region is writable. The copy size is bounded by msg length (max 1024).
#[allow(invalid_null_arguments)]
unsafe fn wasm_log(level: i32, msg: &str) {
    let bytes = msg.as_bytes();
    let ptr = 0usize;
    let len = bytes.len().min(1024);
    // SAFETY: Address 0 is a valid scratch buffer in WASM linear memory.
    // host_log is guaranteed by the host ABI.
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, len);
        host_log(level, ptr as i32, len as i32);
    }
}

// Plugin metadata
#[unsafe(no_mangle)]
pub extern "C" fn name_ptr() -> i32 { 1024 }

#[unsafe(no_mangle)]
pub extern "C" fn name_len() -> i32 { 16 }

#[unsafe(no_mangle)]
pub extern "C" fn version_ptr() -> i32 { 1040 }

#[unsafe(no_mangle)]
pub extern "C" fn version_len() -> i32 { 5 }

// Plugin type: 3 = TileSource
#[unsafe(no_mangle)]
pub extern "C" fn plugin_type() -> i32 { 3 }

#[unsafe(no_mangle)]
pub extern "C" fn init() -> i32 {
    let name = b"Example NoiseMap";
    let version = b"1.0.0";
    // SAFETY: Addresses 1024+ and 1040+ are reserved for plugin metadata strings.
    // The source bytes are valid string literals with static lifetime.
    // Destinations are within the first memory page (64KB) guaranteed by the host.
    unsafe {
        core::ptr::copy_nonoverlapping(name.as_ptr(), 1024 as *mut u8, name.len());
        core::ptr::copy_nonoverlapping(version.as_ptr(), 1040 as *mut u8, version.len());
    }
    0
}

/// Terrain types: 0=Desert, 1=Grass, 2=Forest, 3=Rock, 4=Sand, 5=Swamp, 6=Water
const TERRAIN_GRASS: u8 = 1;
const TERRAIN_FOREST: u8 = 2;
const TERRAIN_ROCK: u8 = 3;
const TERRAIN_WATER: u8 = 6;

#[unsafe(no_mangle)]
pub extern "C" fn generate_tile(tile_x: i32, tile_z: i32, seed_lo: i32, seed_hi: i32) -> i32 {
    let seed = (seed_lo as u64) | ((seed_hi as u64) << 32);

    // SAFETY: All host_* functions are guaranteed by the Terrafier WASM host ABI
    // to be callable from within generate_tile. The host validates all coordinates
    // (0..127) internally. host_set_* functions are idempotent and panic-free.
    unsafe {
        RNG_STATE = seed;

        let tile_size = host_tile_size();
        let scale = 16.0;

        let origin_x = (tile_x as f64) * (tile_size as f64);
        let origin_z = (tile_z as f64) * (tile_size as f64);

        let water_level = 32;

        for tz in 0..tile_size {
            for tx in 0..tile_size {
                let wx = (tx as f64) + origin_x;
                let wz = (tz as f64) + origin_z;

                let mut height: f64 = 0.0;
                let mut amplitude = 1.0;
                let mut frequency = 1.0;
                let mut max_val = 0.0;

                for _octave in 0..6 {
                    let nx = wx / scale * frequency;
                    let nz = wz / scale * frequency;
                    height += noise_2d(nx, nz, seed.wrapping_add(_octave as u64)) * amplitude;
                    max_val += amplitude;
                    amplitude *= 0.5;
                    frequency *= 2.0;
                }

                height /= max_val;
                height = height * 80.0 + 20.0;

                let iheight = height as i32;
                host_set_height(tx, tz, iheight);

                let terrain = if iheight < water_level + 2 {
                    TERRAIN_WATER
                } else if iheight < water_level + 10 {
                    TERRAIN_GRASS
                } else if iheight > 85 {
                    TERRAIN_ROCK
                } else if rng_range(0.0, 1.0) < 0.3 {
                    TERRAIN_FOREST
                } else {
                    TERRAIN_GRASS
                };
                host_set_terrain(tx, tz, terrain as i32);

                let water = if iheight < water_level {
                    8u8
                } else {
                    0u8
                };
                host_set_water(tx, tz, water as i32);
            }
        }
    }
    0
}

#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
