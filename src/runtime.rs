// Runtime executor for PlotPipe DSL

use crate::parser::ast::{Command, Pipeline};
use crate::csv_reader::{self, CsvData};
use crate::graph::{self, GraphConfig};
use anyhow::{Context, Result};

/// Runtime context for executing commands
pub struct RuntimeContext {
    pub csv_data: CsvData,
    pub chart_config: Option<ChartState>,
}

/// Chart configuration state
struct ChartState {
    x_column: String,
    y_column: String,
    title: Option<String>,
    line_color: Option<String>,
    line_stroke: Option<u32>,
    point_shape: Option<String>,
    point_size: Option<u32>,
}

/// Execute a parsed pipeline and return PNG bytes
pub fn execute_pipeline(pipeline: Pipeline, csv_data: CsvData) -> Result<Vec<u8>> {
    let mut ctx = RuntimeContext {
        csv_data,
        chart_config: None,
    };

    // Execute each command in sequence
    for command in pipeline.commands {
        execute_command(command, &mut ctx)?;
    }

    // Generate final graph
    render_chart(&ctx)
}

/// Execute a single command
fn execute_command(cmd: Command, ctx: &mut RuntimeContext) -> Result<()> {
    match cmd {
        Command::Chart { x, y, title } => {
            ctx.chart_config = Some(ChartState {
                x_column: x,
                y_column: y,
                title,
                line_color: Some("blue".to_string()), // default
                line_stroke: Some(1),
                point_shape: None,
                point_size: None,
            });
            Ok(())
        }
        Command::LayerLine { color, stroke } => {
            if let Some(ref mut config) = ctx.chart_config {
                if let Some(c) = color {
                    config.line_color = Some(c);
                }
                if let Some(s) = stroke {
                    config.line_stroke = Some(s);
                }
            }
            Ok(())
        }
        Command::LayerPoint { shape, size, color: _ } => {
            if let Some(ref mut config) = ctx.chart_config {
                config.point_shape = shape;
                config.point_size = size;
            }
            Ok(())
        }
    }
}

/// Render the final chart
fn render_chart(ctx: &RuntimeContext) -> Result<Vec<u8>> {
    let config = ctx
        .chart_config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No chart command found in pipeline"))?;

    // Extract columns using existing csv_reader
    let x_selector = csv_reader::parse_column_selector(&config.x_column);
    let (x_col_name, x_values) = csv_reader::extract_column(&ctx.csv_data, x_selector)
        .context("Failed to extract X column")?;

    let y_selector = csv_reader::parse_column_selector(&config.y_column);
    let (y_col_name, y_values) = csv_reader::extract_column(&ctx.csv_data, y_selector)
        .context("Failed to extract Y column")?;

    // Build graph config
    let graph_config = GraphConfig {
        title: config.title.clone(),
        x_label: x_col_name,
        y_label: y_col_name,
        width: 800,
        height: 600,
        line_color: config.line_color.clone(),
    };

    // Generate graph using existing function
    graph::generate_line_graph(x_values, y_values, graph_config)
        .context("Failed to generate graph")
}
