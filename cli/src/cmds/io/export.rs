use clap::Args;
use std::path::Path;
use terrafier_i18n as i18n;

use crate::util::output::{self, OutputFormat};
use crate::util::{load_world, progress};

#[derive(Args)]
pub struct ExportArgs {
    /// Path to Terrafier world directory
    pub world: String,
    /// Output directory for Minecraft save
    #[arg(short, long)]
    pub output: String,
    /// Minecraft data version (default: 3954 for 1.21)
    #[arg(long, default_value = "3954")]
    pub data_version: i32,
}

pub fn cmd_export(
    args: ExportArgs,
    format: &OutputFormat,
    dry_run: bool,
    validate: bool,
) -> anyhow::Result<()> {
    let world_path = Path::new(&args.world);
    log::info!("Exporting world from: {}", world_path.display());

    let world = load_world(world_path)?;
    terrafier_core::io::export::validate_export(&world)?;

    let tile_count: usize = world.dimensions.iter().map(|d| d.tiles.len()).sum();

    if validate || dry_run {
        if dry_run {
            log::info!("{}", i18n::translate_with("cli.dry_run", &[("action", "export")]));
        }
        output::print_result(
            format,
            &serde_json::json!({
                "status": "validated",
                "input": args.world,
                "output": args.output,
                "tiles": tile_count,
            }),
        );
        if dry_run {
            return Ok(());
        }
    }

    let exporting_msg = i18n::translate_with("cli.export.exporting", &[("path", &args.output)]);
    let bar = progress::new_bar(tile_count as u64, &exporting_msg);
    terrafier_core::io::export::export_to_save(&world, Path::new(&args.output))?;
    progress::finish_with(i18n::t!("cli.export.complete"), bar);

    log::info!("World exported to: {}", args.output);
    output::print_result(
        format,
        &serde_json::json!({
            "status": "exported",
            "input": args.world,
            "output": args.output,
            "tiles": tile_count,
        }),
    );

    Ok(())
}
