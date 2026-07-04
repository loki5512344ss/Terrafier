use terrafier_core::model::tile::Tile;
use terrafier_core::plugins::{PluginRegistry, TileSourcePlugin};
use terrafier_wasm_host::{WasmTileSourcePlugin, load_wasm_dir};

const MINIMAL_PLUGIN_WAT: &str = r#"
(module
  (import "env" "host_set_height" (func $host_set_height (param i32 i32 i32)))
  (import "env" "host_set_terrain" (func $host_set_terrain (param i32 i32 i32)))

  (memory (export "memory") 1)

  (data (i32.const 1024) "test_plugin")
  (data (i32.const 1040) "1.0.0")

  (func (export "name_ptr") (result i32) i32.const 1024)
  (func (export "name_len") (result i32) i32.const 11)
  (func (export "version_ptr") (result i32) i32.const 1040)
  (func (export "version_len") (result i32) i32.const 5)
  (func (export "plugin_type") (result i32) i32.const 3)

  (func (export "generate_tile")
    (param $tx i32) (param $tz i32) (param $seed_lo i32) (param $seed_hi i32)
    (result i32)
    (local $x i32) (local $z i32)

    (loop $loop_z
      (local.set $x (i32.const 0))
      (loop $loop_x
        (call $host_set_height (local.get $x) (local.get $z)
          (i32.add (local.get $x) (local.get $z)))
        (call $host_set_terrain (local.get $x) (local.get $z) (i32.const 1))
        (br_if $loop_x
          (i32.lt_u
            (local.tee $x (i32.add (local.get $x) (i32.const 1)))
            (i32.const 128)))
      )
      (br_if $loop_z
        (i32.lt_u
          (local.tee $z (i32.add (local.get $z) (i32.const 1)))
          (i32.const 128)))
    )
    (i32.const 0)
  )
)
"#;

#[test]
fn test_load_plugin_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let wasm_path = dir.path().join("test_plugin.wasm");
    let wasm_bytes = wat::parse_str(MINIMAL_PLUGIN_WAT).expect("Failed to parse WAT");
    std::fs::write(&wasm_path, &wasm_bytes).unwrap();

    let plugin = WasmTileSourcePlugin::from_file(&wasm_path).expect("Failed to load WASM plugin");

    assert_eq!(plugin.name(), "test_plugin");
    assert_eq!(plugin.version(), "1.0.0");
}

#[test]
fn test_generate_tile() {
    let dir = tempfile::tempdir().unwrap();
    let wasm_path = dir.path().join("test_plugin.wasm");
    let wasm_bytes = wat::parse_str(MINIMAL_PLUGIN_WAT).expect("Failed to parse WAT");
    std::fs::write(&wasm_path, &wasm_bytes).unwrap();

    let plugin = WasmTileSourcePlugin::from_file(&wasm_path).unwrap();
    let mut tile = Tile::new(5, 3, -64, 320);

    plugin.generate_tile(&mut tile, 42);

    assert_eq!(tile.x, 5);
    assert_eq!(tile.z, 3);

    // Plugin sets height = x + z, terrain = 1
    for z in 0..128 {
        for x in 0..128 {
            assert_eq!(tile.get_height(x, z), (x + z) as i16);
            assert_eq!(tile.get_terrain(x, z), 1);
        }
    }
}

#[test]
fn test_load_wasm_dir() {
    let dir = tempfile::tempdir().unwrap();

    // Create two .wasm files
    let wasm_bytes = wat::parse_str(MINIMAL_PLUGIN_WAT).expect("Failed to parse WAT");
    std::fs::write(dir.path().join("plugin_a.wasm"), &wasm_bytes).unwrap();
    std::fs::write(dir.path().join("plugin_b.wasm"), &wasm_bytes).unwrap();
    // Non-wasm file should be ignored
    std::fs::write(dir.path().join("readme.txt"), b"ignore me").unwrap();

    let mut registry = PluginRegistry::new();
    assert_eq!(registry.sources.len(), 0);

    let count = load_wasm_dir(&mut registry, dir.path()).unwrap();
    assert_eq!(count, 2);
    assert_eq!(registry.sources.len(), 2);
}
