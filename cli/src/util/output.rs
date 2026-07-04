use serde_json::Value;
use terrafier_i18n as i18n;

pub enum OutputFormat {
    Json,
    Human,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s {
            "human" => Self::Human,
            _ => Self::Json,
        }
    }
}

pub fn print_result(format: &OutputFormat, result: &Value) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(result)
                    .unwrap_or_else(|e| format!("serialization error: {e}"))
            );
        }
        OutputFormat::Human => print_human(result),
    }
}

fn print_human(result: &Value) {
    if let Some(status) = result.get("status").and_then(|v| v.as_str()) {
        match status {
            "created" => {
                let name = result["name"].as_str().unwrap_or("?");
                let seed = result["seed"].as_u64().unwrap_or(0);
                let tiles = result["tiles"].as_u64().unwrap_or(0);
                println!(
                    "{}",
                    i18n::translate_with("cli.new.created", &[
                        ("name", name),
                        ("tiles", &tiles.to_string()),
                    ])
                );
                let _ = seed;
            }
            "exported" => {
                let tiles = result["tiles"].as_u64().unwrap_or(0);
                println!(
                    "{}",
                    i18n::translate_with("cli.export.complete", &[
                        ("regions", "?"),
                        ("chunks", &tiles.to_string()),
                    ])
                );
            }
            "imported" => {
                let name = result["name"].as_str().unwrap_or("?");
                let tiles = result["tiles"].as_u64().unwrap_or(0);
                let dims = result["dimensions"].as_u64().unwrap_or(0);
                println!(
                    "{}",
                    i18n::translate_with("cli.import.complete", &[
                        ("tiles", &tiles.to_string()),
                        ("dimensions", &dims.to_string()),
                    ])
                );
                let _ = name;
            }
            "rendered" => {
                let output = result["output"].as_str().unwrap_or("?");
                println!(
                    "{}",
                    i18n::translate_with("cli.render.saved", &[
                        ("path", output),
                    ])
                );
            }
            "validated" => {
                println!("{}", i18n::t!("cli.validate.ok"));
            }
            _ => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(result)
                        .unwrap_or_else(|e| format!("serialization error: {e}"))
                );
            }
        }
    } else if result.get("name").is_some() {
        // Info output
        print_human_info(result);
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(result)
                .unwrap_or_else(|e| format!("serialization error: {e}"))
        );
    }
}

fn print_human_info(result: &Value) {
    let name = result["name"].as_str().unwrap_or("?");
    println!(
        "{}",
        i18n::translate_with("cli.info.title", &[("name", name)])
    );
    if let Some(seed) = result.get("seed").and_then(|v| v.as_u64()) {
        println!(
            "  {}",
            i18n::translate_with("cli.info.seed", &[("seed", &seed.to_string())])
        );
    }
    if let Some(platform) = result.get("platform").and_then(|v| v.as_str()) {
        println!(
            "  {}",
            i18n::translate_with("cli.info.platform", &[("platform", platform)])
        );
    }
    if let Some(height) = result.get("height_range") {
        let min = height["min"].as_i64().unwrap_or(0);
        let max = height["max"].as_i64().unwrap_or(0);
        eprintln!("  Height range: {} to {}", min, max);
    }
    if let Some(dims) = result.get("dimensions").and_then(|v| v.as_array()) {
        println!(
            "  {}",
            i18n::translate_with("cli.info.dimensions", &[
                ("dimensions", &dims.len().to_string()),
            ])
        );
        for dim in dims {
            let dim_name = dim["name"].as_str().unwrap_or("?");
            let dim_tiles = dim["tiles"].as_u64().unwrap_or(0);
            println!(
                "    - {} ({} tiles)",
                dim_name, dim_tiles
            );
        }
    }
    if let Some(tiles) = result.get("total_tiles").and_then(|v| v.as_u64()) {
        println!(
            "  {}",
            i18n::translate_with("cli.info.tiles", &[("tiles", &tiles.to_string())])
        );
    }
    if let Some(path) = result.get("path").and_then(|v| v.as_str()) {
        eprintln!("  Path: {}", path);
    }
}

pub fn print_error(format: &OutputFormat, error: &anyhow::Error) {
    match format {
        OutputFormat::Json => {
            eprintln!(
                "{}",
                serde_json::to_string(&serde_json::json!({"error": error.to_string()}))
                    .unwrap_or_else(|e| format!("serialization error: {e}"))
            );
        }
        OutputFormat::Human => {
            eprintln!("{}: {}", i18n::t!("cli.status.error"), error);
        }
    }
}

#[allow(dead_code)]
pub fn print_info(format: &OutputFormat, msg: &str) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({"info": msg}))
                    .unwrap_or_else(|e| format!("serialization error: {e}"))
            );
        }
        OutputFormat::Human => {
            println!("{}", msg);
        }
    }
}
