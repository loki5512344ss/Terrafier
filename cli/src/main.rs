use clap::Parser;
use std::path::PathBuf;

mod cmds;
mod util;

use crate::util::output::OutputFormat;

/// Terrafier — high-performance Minecraft world painter
#[derive(Parser)]
#[command(name = "terrafier")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format: json or human
    #[arg(global = true, long, default_value = "json")]
    format: String,

    /// Dry run — validate without making changes
    #[arg(global = true, long)]
    dry_run: bool,

    /// Validate input without executing
    #[arg(global = true, long)]
    validate: bool,
}

#[derive(Parser)]
enum Commands {
    /// Create a new world from noise
    New(cmds::new::NewArgs),
    /// Export world to Minecraft save
    Export(cmds::export::ExportArgs),
    /// Import Minecraft save to Terrafier world
    Import(cmds::import::ImportArgs),
    /// Show world information
    Info(cmds::info::InfoArgs),
    /// Render world preview to an image
    Render(cmds::render::RenderArgs),
    /// Run a Terrafier script (.rhai)
    #[cfg(feature = "scripting")]
    Run {
        /// Path to .rhai script file
        script: PathBuf,
    },
}

fn main() {
    use terrafier_i18n as i18n;
    i18n::init();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();
    let format = OutputFormat::from_str(&cli.format);

    let result = match cli.command {
        Commands::New(args) => cmds::new::cmd_new(args, &format, cli.dry_run, cli.validate),
        Commands::Export(args) => {
            cmds::export::cmd_export(args, &format, cli.dry_run, cli.validate)
        }
        Commands::Import(args) => {
            cmds::import::cmd_import(args, &format, cli.dry_run, cli.validate)
        }
        Commands::Info(args) => cmds::info::cmd_info(args, &format, cli.dry_run, cli.validate),
        Commands::Render(args) => {
            cmds::render::cmd_render(args, &format, cli.dry_run, cli.validate)
        }
        #[cfg(feature = "scripting")]
        Commands::Run { script } => cmds::cmd_run(&script),
    };

    if let Err(e) = result {
        crate::util::output::print_error(&format, &e);
        std::process::exit(1);
    }
}
