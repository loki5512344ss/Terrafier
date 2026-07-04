//! NBT I/O — binary reader and writer for Java Edition (Big Endian).

pub mod read;
pub mod write;

pub use read::read_tag;
pub use read::reader;
pub use write::write_tag;
pub use write::writer;
