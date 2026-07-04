use std::path::Path;

use terrafier_core::World;
use terrafier_scripting::ScriptEngine;

#[test]
fn test_new_world() {
    let mut engine = ScriptEngine::new();
    let result = engine
        .run(
            r#"
        let world = new_world("test_world", 12345);
        let i = info(world);
        i.contains("test_world") && i.contains("12345")
    "#,
        )
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn test_generate_flat_heightmap() {
    let mut engine = ScriptEngine::new();
    let _ = engine
        .run(
            r#"
        let world = new_world("flat_world", 42);
        generate_heightmap_flat(world, 64, "overworld");
        save_world(world, "/tmp/test_flat.tfw");
    "#,
        )
        .unwrap();
    let loaded: World = terrafier_core::io::load_binary(Path::new("/tmp/test_flat.tfw")).unwrap();
    if let Some(dim) = loaded.dimensions.first() {
        for (_, tile) in &dim.tiles {
            for z in 0..128 {
                for x in 0..128 {
                    assert_eq!(tile.get_height(x, z), 64);
                }
            }
        }
    }
    let _ = std::fs::remove_file("/tmp/test_flat.tfw");
}

#[test]
fn test_script_error_reporting() {
    let mut engine = ScriptEngine::new();
    let result = engine.run(
        r#"
        let world = new_world("bad", 1);
        generate_heightmap_flat(world, 64, "nonexistent");
    "#,
    );
    assert!(result.is_err());
}
