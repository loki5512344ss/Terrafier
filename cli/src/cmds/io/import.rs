use clap::Args;
use std::path::Path;
use terrafier_i18n as i18n;

use crate::util::output::{self, OutputFormat};
use crate::util::progress;

#[derive(Args)]
pub struct ImportArgs {
    /// Path to Minecraft save directory
    pub input: String,
    /// Output Terrafier world path
    pub output: String,
}

pub fn cmd_import(
    args: ImportArgs,
    format: &OutputFormat,
    dry_run: bool,
    validate: bool,
) -> anyhow::Result<()> {
    let input_path = Path::new(&args.input);
    log::info!("Importing Minecraft save from: {}", input_path.display());

    if validate || dry_run {
        if dry_run {
            log::info!(
                "{}",
                i18n::translate_with("cli.dry_run", &[("action", "import")])
            );
        }
        terrafier_core::io::import::validate_save(input_path)?;
        output::print_result(
            format,
            &serde_json::json!({
                "status": "validated",
                "input": args.input,
            }),
        );
        if dry_run {
            return Ok(());
        }
    }

    terrafier_core::io::import::validate_save(input_path)?;

    // Count regions for progress display
    let region_dir = input_path.join("region");
    let region_count = if region_dir.is_dir() {
        std::fs::read_dir(&region_dir)
            .map(|e| e.count())
            .unwrap_or(0)
    } else {
        0
    };
    let importing_msg = i18n::translate_with("cli.import.importing", &[("path", &args.input)]);
    let bar = progress::new_bar(region_count.max(1) as u64, &importing_msg);
    let world = terrafier_core::io::import::import(input_path)?;
    let tile_count: usize = world.dimensions.iter().map(|d| d.tiles.len()).sum();
    let import_msg = i18n::translate_with(
        "cli.import.complete",
        &[
            ("tiles", &tile_count.to_string()),
            ("dimensions", &world.dimensions.len().to_string()),
        ],
    );
    progress::finish_with(&import_msg, bar);

    let output_path = Path::new(&args.output);
    std::fs::create_dir_all(output_path)?;

    let world_json = serde_json::to_string_pretty(&world)?;
    std::fs::write(output_path.join("world.tfw"), &world_json)?;

    log::info!(
        "Import complete: {} dimensions, {} tiles",
        world.dimensions.len(),
        tile_count
    );
    output::print_result(
        format,
        &serde_json::json!({
            "status": "imported",
            "name": world.name,
            "seed": world.seed,
            "dimensions": world.dimensions.len(),
            "tiles": tile_count,
            "output": output_path.display().to_string(),
        }),
    );

    Ok(())
}
