// PlotPipe DSL Parser Module - Grammar of Graphics

pub mod aesthetics;
pub mod ast;
pub mod geom;
pub mod lexer;
pub mod pipeline;

// Public API re-exports
pub use ast::{Aesthetics, Layer, LineLayer, PlotSpec, PointLayer};
pub use pipeline::parse_plot_spec;
