use anyhow::Result;
use std::path::PathBuf;

pub fn cmd_run(script: &PathBuf) -> Result<()> {
    terrafier_scripting::run_script(script)?;
    println!("Script executed successfully.");
    Ok(())
}
