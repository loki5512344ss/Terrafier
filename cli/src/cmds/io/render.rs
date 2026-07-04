use clap::Args;
use std::path::PathBuf;
use terrafier_i18n as i18n;

use crate::util::output::{self, OutputFormat};
use crate::util::{load_world, progress};

#[derive(Args)]
pub struct RenderArgs {
    /// Path to Terrafier world directory
    pub world: String,
    /// Output image path
    #[arg(short, long, default_value = "preview.png")]
    pub output: PathBuf,
    /// Scale factor (1 = full resolution, 2 = half, 4 = quarter)
    #[arg(long, default_value = "4")]
    pub scale: u32,
}

pub fn cmd_render(
    args: RenderArgs,
    format: &OutputFormat,
    dry_run: bool,
    validate: bool,
) -> anyhow::Result<()> {
    let world_path = std::path::Path::new(&args.world);
    log::info!("Rendering world: {}", world_path.display());

    let world = load_world(world_path)?;
    terrafier_core::io::export::validate_export(&world)?;

    let tile_count: usize = world.dimensions.iter().map(|d| d.tiles.len()).sum();

    if validate || dry_run {
        if dry_run {
            log::info!(
                "{}",
                i18n::translate_with("cli.dry_run", &[("action", "render")])
            );
        }
        output::print_result(
            format,
            &serde_json::json!({
                "status": "validated",
                "world": args.world,
                "tiles": tile_count,
            }),
        );
        if dry_run {
            return Ok(());
        }
    }

    let bar = progress::new_bar(tile_count as u64, i18n::t!("cli.render.rendering"));
    terrafier_core::io::export::render_to_image(&world, &args.output, args.scale)?;
    let saved_msg = i18n::translate_with(
        "cli.render.saved",
        &[("path", &args.output.display().to_string())],
    );
    progress::finish_with(&saved_msg, bar);

    log::info!("Preview rendered to: {}", args.output.display());
    output::print_result(
        format,
        &serde_json::json!({
            "status": "rendered",
            "world": args.world,
            "output": args.output.display().to_string(),
            "scale": args.scale,
        }),
    );

    Ok(())
}
