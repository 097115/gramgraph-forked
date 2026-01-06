// Runtime executor for Grammar of Graphics DSL

use crate::csv_reader::{self, CsvData};
use crate::graph;
use crate::parser::ast::{Aesthetics, Layer, LineLayer, PlotSpec, PointLayer};
use anyhow::{Context, Result};

/// Render a plot specification to PNG bytes
pub fn render_plot(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>> {
    // Validate: must have at least one layer
    if spec.layers.is_empty() {
        anyhow::bail!("Plot requires at least one geometry layer (line, point, etc.)");
    }

    // Prepare to collect all data for range calculation
    let mut all_x_data = Vec::new();
    let mut all_y_data = Vec::new();

    // Collect layer specifications with resolved aesthetics and data
    let mut layer_specs: Vec<LayerData> = Vec::new();

    for layer in &spec.layers {
        let (x_col, y_col) = resolve_aesthetics(layer, &spec.aesthetics)?;

        // Extract data for this layer
        let x_selector = csv_reader::parse_column_selector(&x_col);
        let (x_col_name, x_values) = csv_reader::extract_column(&csv_data, x_selector)
            .context(format!("Failed to extract x column '{}'", x_col))?;

        let y_selector = csv_reader::parse_column_selector(&y_col);
        let (y_col_name, y_values) = csv_reader::extract_column(&csv_data, y_selector)
            .context(format!("Failed to extract y column '{}'", y_col))?;

        // Collect for global range
        all_x_data.extend(x_values.iter().copied());
        all_y_data.extend(y_values.iter().copied());

        layer_specs.push(LayerData {
            layer: layer.clone(),
            x_data: x_values,
            y_data: y_values,
            x_label: x_col_name,
            y_label: y_col_name,
        });
    }

    // Create canvas with global data ranges
    let mut canvas = graph::Canvas::new(
        800,
        600,
        spec.labels.as_ref().and_then(|l| l.title.clone()),
        all_x_data,
        all_y_data,
    )?;

    // Render each layer
    for layer_data in layer_specs {
        match layer_data.layer {
            Layer::Line(line_layer) => {
                canvas.add_line_layer(
                    layer_data.x_data,
                    layer_data.y_data,
                    line_layer_to_style(&line_layer),
                )?;
            }
            Layer::Point(point_layer) => {
                canvas.add_point_layer(
                    layer_data.x_data,
                    layer_data.y_data,
                    point_layer_to_style(&point_layer),
                )?;
            }
        }
    }

    // Finalize and encode
    canvas.render()
}

/// Resolve aesthetics for a layer (layer override or global)
fn resolve_aesthetics(layer: &Layer, global_aes: &Option<Aesthetics>) -> Result<(String, String)> {
    let (x_override, y_override) = match layer {
        Layer::Line(l) => (l.x.as_ref(), l.y.as_ref()),
        Layer::Point(p) => (p.x.as_ref(), p.y.as_ref()),
    };

    // Get x column
    let x_col = if let Some(x) = x_override {
        x.clone()
    } else if let Some(ref aes) = global_aes {
        aes.x.clone()
    } else {
        anyhow::bail!("No x aesthetic specified (use aes(x: ..., y: ...) or layer-level x: ...)");
    };

    // Get y column
    let y_col = if let Some(y) = y_override {
        y.clone()
    } else if let Some(ref aes) = global_aes {
        aes.y.clone()
    } else {
        anyhow::bail!("No y aesthetic specified (use aes(x: ..., y: ...) or layer-level y: ...)");
    };

    Ok((x_col, y_col))
}

/// Convert LineLayer to graph::LineStyle
fn line_layer_to_style(layer: &LineLayer) -> graph::LineStyle {
    graph::LineStyle {
        color: layer.color.clone(),
        width: layer.width,
        alpha: layer.alpha,
    }
}

/// Convert PointLayer to graph::PointStyle
fn point_layer_to_style(layer: &PointLayer) -> graph::PointStyle {
    graph::PointStyle {
        color: layer.color.clone(),
        size: layer.size,
        shape: layer.shape.clone(),
        alpha: layer.alpha,
    }
}

/// Helper struct to hold layer data
struct LayerData {
    layer: Layer,
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    x_label: String,
    y_label: String,
}
