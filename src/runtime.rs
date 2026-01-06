// Runtime executor for Grammar of Graphics DSL

use crate::csv_reader::{self, CsvData};
use crate::graph;
use crate::parser::ast::{Aesthetics, BarLayer, Layer, LineLayer, PlotSpec, PointLayer};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;

/// Render a plot specification to PNG bytes
pub fn render_plot(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>> {
    // Validate: must have at least one layer
    if spec.layers.is_empty() {
        anyhow::bail!("Plot requires at least one geometry layer (line, point, etc.)");
    }

    // Validate layer compatibility (no mixing categorical and continuous)
    validate_layer_compatibility(&spec.layers)?;

    // Check if we have bar charts (categorical) or continuous charts
    let has_bar = spec.layers.iter().any(|l| matches!(l, Layer::Bar(_)));

    if has_bar {
        // Handle bar charts (categorical x-axis)
        render_bar_plot(spec, csv_data)
    } else {
        // Handle continuous charts (line/point)
        render_continuous_plot(spec, csv_data)
    }
}

/// Render a plot with bar charts (categorical x-axis)
fn render_bar_plot(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>> {
    use crate::parser::ast::BarPosition;

    let mut all_y_data = Vec::new();
    let mut bar_specs: Vec<BarData> = Vec::new();

    // Extract data for each bar layer
    for layer in &spec.layers {
        if let Layer::Bar(bar_layer) = layer {
            let (x_col, y_col) = resolve_aesthetics(layer, &spec.aesthetics)?;

            // Extract categorical data
            let (categories, y_values) = extract_categorical_data(&csv_data, &x_col, &y_col)?;

            // Collect y data for range calculation
            all_y_data.extend(y_values.iter().copied());

            bar_specs.push(BarData {
                layer: bar_layer.clone(),
                categories,
                y_data: y_values,
            });
        }
    }

    // Use category indices for x-axis range
    let num_categories = if let Some(first) = bar_specs.first() {
        first.categories.len()
    } else {
        0
    };
    let all_x_data: Vec<f64> = (0..num_categories).map(|i| i as f64).collect();

    // For stacked bars, calculate cumulative y values for range
    let first_position = bar_specs.first().map(|b| &b.layer.position);
    if matches!(first_position, Some(BarPosition::Stack)) {
        // For stacked bars, sum all y values at each category position
        let mut max_stack: f64 = 0.0;
        for cat_idx in 0..num_categories {
            let stack_sum: f64 = bar_specs.iter().map(|b| b.y_data[cat_idx]).sum();
            max_stack = f64::max(max_stack, stack_sum);
        }
        all_y_data.push(max_stack); // Ensure range includes full stack height
    }

    // Create canvas with ranges
    let mut canvas = graph::Canvas::new(
        800,
        600,
        spec.labels.as_ref().and_then(|l| l.title.clone()),
        all_x_data,
        all_y_data,
    )?;

    // Check if all bars have the same position mode and x aesthetic
    let should_group = bar_specs.len() > 1
        && bar_specs
            .windows(2)
            .all(|w| w[0].layer.position == w[1].layer.position && w[0].categories == w[1].categories);

    if should_group && bar_specs.len() > 1 {
        // Group bars for dodge/stack/identity rendering
        let position = match &bar_specs[0].layer.position {
            BarPosition::Dodge => "dodge",
            BarPosition::Stack => "stack",
            BarPosition::Identity => "identity",
        };

        let categories = bar_specs[0].categories.clone();
        let series: Vec<(Vec<f64>, graph::BarStyle)> = bar_specs
            .into_iter()
            .map(|b| (b.y_data, bar_layer_to_style(&b.layer)))
            .collect();

        canvas.add_bar_group(categories, series, position)?;
    } else {
        // Render each bar layer independently
        for bar_data in bar_specs {
            canvas.add_bar_layer(
                bar_data.categories,
                bar_data.y_data,
                bar_layer_to_style(&bar_data.layer),
            )?;
        }
    }

    canvas.render()
}

/// Render a plot with continuous charts (line/point)
fn render_continuous_plot(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>> {
    let mut all_x_data = Vec::new();
    let mut all_y_data = Vec::new();
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
            Layer::Bar(_) => {
                // Should not reach here due to validation
                unreachable!()
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
        Layer::Bar(b) => (b.x.as_ref(), b.y.as_ref()),
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

/// Convert BarLayer to graph::BarStyle
fn bar_layer_to_style(layer: &BarLayer) -> graph::BarStyle {
    graph::BarStyle {
        color: layer.color.clone(),
        alpha: layer.alpha,
        width: layer.width,
    }
}

/// Extract categorical data from CSV for bar charts
/// Returns (categories, y_values) where y_values are aggregated by category (sum)
fn extract_categorical_data(
    csv_data: &CsvData,
    x_col: &str,
    y_col: &str,
) -> Result<(Vec<String>, Vec<f64>)> {
    // Find column indices
    let x_col_index = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(x_col))
        .ok_or_else(|| anyhow!("Column '{}' not found", x_col))?;

    let y_col_index = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(y_col))
        .ok_or_else(|| anyhow!("Column '{}' not found", y_col))?;

    // Extract x categories and y values, aggregating by category
    let mut category_values: HashMap<String, f64> = HashMap::new();
    let mut categories_order: Vec<String> = Vec::new();

    for (row_idx, row) in csv_data.rows.iter().enumerate() {
        let category = row[x_col_index].clone();
        let y_str = &row[y_col_index];
        let y_val = y_str.parse::<f64>().with_context(|| {
            format!(
                "Failed to parse '{}' as number in column '{}' at row {}",
                y_str,
                y_col,
                row_idx + 1
            )
        })?;

        // Track category order (first appearance)
        if !category_values.contains_key(&category) {
            categories_order.push(category.clone());
        }

        // Aggregate y values (sum)
        *category_values.entry(category).or_insert(0.0) += y_val;
    }

    // Build vectors in category order
    let y_values: Vec<f64> = categories_order
        .iter()
        .map(|cat| *category_values.get(cat).unwrap_or(&0.0))
        .collect();

    Ok((categories_order, y_values))
}

/// Validate that layers are compatible (no mixing categorical and continuous)
fn validate_layer_compatibility(layers: &[Layer]) -> Result<()> {
    let has_bar = layers.iter().any(|l| matches!(l, Layer::Bar(_)));
    let has_continuous = layers
        .iter()
        .any(|l| matches!(l, Layer::Line(_) | Layer::Point(_)));

    if has_bar && has_continuous {
        anyhow::bail!(
            "Cannot mix bar charts (categorical x-axis) with line/point charts (continuous x-axis) in the same plot"
        );
    }

    Ok(())
}

/// Helper struct to hold layer data
struct LayerData {
    layer: Layer,
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    x_label: String,
    y_label: String,
}

/// Helper struct to hold bar chart data
struct BarData {
    layer: BarLayer,
    categories: Vec<String>,
    y_data: Vec<f64>,
}
