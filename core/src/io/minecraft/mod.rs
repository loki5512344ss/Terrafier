mod export;
mod reader;
mod version;

pub use export::save_world;
pub use export::save_world_with_layers;
pub use reader::discover_region_files;
pub use reader::load_save;
pub(crate) use version::data_version;
pub use version::version_from_data_version;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MinecraftIOError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("NBT parse error: {0}")]
    NbtParse(#[from] terrafier_nbt::io::reader::ReadError),
    #[error("NBT write error: {0}")]
    NbtWrite(#[from] terrafier_nbt::io::writer::WriteError),
    #[error("Region error: {0}")]
    Region(#[from] terrafier_fastanvil::io::region::RegionError),
    #[error("Not a Minecraft save directory: {0}")]
    NotASave(String),
    #[error("Missing level.dat")]
    MissingLevelDat,
    #[error("Missing region directory")]
    MissingRegionDir,
    #[error("Unsupported data version: {0}")]
    UnsupportedVersion(i32),
    #[error("Invalid chunk data: {0}")]
    InvalidChunk(String),
}

pub type Result<T> = std::result::Result<T, MinecraftIOError>;
