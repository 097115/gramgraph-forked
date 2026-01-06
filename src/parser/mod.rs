// PlotPipe DSL Parser Module

pub mod ast;
pub mod command;
pub mod lexer;
pub mod pipeline;

// Public API re-exports
pub use ast::{Command, Pipeline};
pub use pipeline::parse_pipeline;
