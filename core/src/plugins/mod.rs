//! Plugin host — trait-объекты для расширения функциональности.

pub mod registry;
mod traits;

pub use registry::PluginRegistry;
pub use traits::{ExportPlugin, LayerPlugin, OperationPlugin, TileSourcePlugin};
