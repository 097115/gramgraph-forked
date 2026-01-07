// Runtime executor for Grammar of Graphics DSL

use crate::csv_reader::{self, CsvData};
use crate::graph;
use crate::palette::{ColorPalette, ShapePalette, SizePalette};
use crate::parser::ast::{AestheticValue, Aesthetics, BarLayer, Facet, FacetScales, Layer, LineLayer, PlotSpec, PointLayer};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::ops::Range;

// =============================================================================
// Scale Infrastructure - Core abstraction for coordinate mapping
// =============================================================================

/// Scale type determines how data values map to coordinate space
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleType {
    /// Numeric x-axis with continuous values
    Continuous,
    /// Discrete categories (e.g., "A", "B", "C")
    Categorical,
}

/// Data range in original units (either numeric or categorical)
#[derive(Debug, Clone)]
pub enum DataRange {
    /// Continuous numeric range (min..max)
    Numeric(Range<f64>),
    /// Categorical: ordered list of category labels
    Categorical(Vec<String>),
}

/// A Scale manages the mapping between data space and coordinate space
#[derive(Debug, Clone)]
pub struct Scale {
    pub scale_type: ScaleType,
    pub data_range: DataRange,
    pub coord_range: Range<f64>,
}

impl Scale {
    /// Map a categorical value to coordinate space
    /// Returns the index of the category (0.0, 1.0, 2.0, ...)
    pub fn map_categorical(&self, category: &str) -> Result<f64> {
        match &self.data_range {
            DataRange::Categorical(categories) => {
                let idx = categories
                    .iter()
                    .position(|c| c == category)
                    .ok_or_else(|| anyhow!("Category '{}' not found in scale", category))?;
                Ok(idx as f64)
            }
            DataRange::Numeric(_) => {
                Err(anyhow!("Cannot map categorical value to continuous scale"))
            }
        }
    }

    /// Map a continuous value to coordinate space
    /// For now, uses 1:1 mapping (identity transform)
    pub fn map_continuous(&self, value: f64) -> Result<f64> {
        match &self.data_range {
            DataRange::Numeric(_range) => {
                // For now, use identity mapping
                // Future: could apply log/sqrt/etc. transformations here
                Ok(value)
            }
            DataRange::Categorical(_) => {
                Err(anyhow!("Cannot map continuous value to categorical scale"))
            }
        }
    }

    /// Get axis labels for this scale
    /// Returns Some(labels) for categorical, None for continuous (uses default formatting)
    pub fn get_axis_labels(&self) -> Option<Vec<String>> {
        match &self.data_range {
            DataRange::Categorical(cats) => Some(cats.clone()),
            DataRange::Numeric(_) => None,
        }
    }

    /// Create a continuous scale from numeric data
    /// Applies 5% padding to range for visual breathing room
    pub fn continuous_from_data(data: &[f64]) -> Self {
        if data.is_empty() {
            return Scale {
                scale_type: ScaleType::Continuous,
                data_range: DataRange::Numeric(0.0..1.0),
                coord_range: 0.0..1.0,
            };
        }

        let min = data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        let range = if min == max {
            (min - 1.0)..(max + 1.0)
        } else {
            let padding = (max - min) * 0.05;
            (min - padding)..(max + padding)
        };

        Scale {
            scale_type: ScaleType::Continuous,
            data_range: DataRange::Numeric(range.clone()),
            coord_range: range, // 1:1 mapping for continuous
        }
    }

    /// Create a categorical scale from ordered categories
    /// Categories are mapped to indices: 0, 1, 2, ...
    /// Coordinate range is -0.5 to (n-0.5) for bar chart alignment
    pub fn categorical_from_categories(categories: Vec<String>) -> Self {
        let n = categories.len();
        Scale {
            scale_type: ScaleType::Categorical,
            data_range: DataRange::Categorical(categories),
            coord_range: -0.5..((n as f64) - 0.5),
        }
    }
}

// -----------------------------------------------------------------------------
// Scale Determination Helpers
// -----------------------------------------------------------------------------

/// Determine x-axis scale type by inspecting data and layer requirements
///
/// Decision logic:
/// - If all x-data is numeric AND no Bar layers → Continuous
/// - Otherwise → Categorical
fn determine_x_scale(spec: &PlotSpec, csv_data: &CsvData) -> Result<ScaleType> {
    // Collect all x-data as strings to check if they're numeric
    let mut all_x_data = Vec::new();

    for layer in &spec.layers {
        let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;
        let x_selector = csv_reader::parse_column_selector(&resolved.x_col);

        match csv_reader::extract_column_as_string(csv_data, x_selector) {
            Ok((_, x_vals)) => all_x_data.extend(x_vals),
            Err(_) => {
                // Column might not exist yet, skip for now
                // Will be caught later during actual rendering
                continue;
            }
        }
    }

    // Check if all values can be parsed as numbers
    let all_numeric = all_x_data.iter().all(|s| s.trim().parse::<f64>().is_ok());

    // Check if any layer requires categorical (Bar charts)
    let has_bar = spec.layers.iter().any(|l| matches!(l, Layer::Bar(_)));

    // Decision: numeric data without bars → Continuous, otherwise → Categorical
    if all_numeric && !has_bar {
        Ok(ScaleType::Continuous)
    } else {
        Ok(ScaleType::Categorical)
    }
}

/// Build x-axis scale from spec and data
fn build_x_scale(spec: &PlotSpec, csv_data: &CsvData, scale_type: ScaleType) -> Result<Scale> {
    use std::collections::HashSet;

    match scale_type {
        ScaleType::Continuous => {
            // Extract all numeric x values
            let mut all_x = Vec::new();

            for layer in &spec.layers {
                let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;
                let x_selector = csv_reader::parse_column_selector(&resolved.x_col);

                match csv_reader::extract_column(csv_data, x_selector) {
                    Ok((_, x_vals)) => all_x.extend(x_vals),
                    Err(e) => {
                        return Err(anyhow!("Failed to extract x column '{}': {}", resolved.x_col, e));
                    }
                }
            }

            Ok(Scale::continuous_from_data(&all_x))
        }

        ScaleType::Categorical => {
            // Extract unique categories in order of appearance, then sort
            let mut categories_order = Vec::new();
            let mut seen = HashSet::new();

            for layer in &spec.layers {
                let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;
                let x_selector = csv_reader::parse_column_selector(&resolved.x_col);

                match csv_reader::extract_column_as_string(csv_data, x_selector) {
                    Ok((_, x_vals)) => {
                        for cat in x_vals {
                            if !seen.contains(&cat) {
                                seen.insert(cat.clone());
                                categories_order.push(cat);
                            }
                        }
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to extract x column '{}': {}", resolved.x_col, e));
                    }
                }
            }

            // Sort for consistent ordering
            categories_order.sort();

            Ok(Scale::categorical_from_categories(categories_order))
        }
    }
}

/// Build y-axis scale from spec and data (always continuous for now)
fn build_y_scale(spec: &PlotSpec, csv_data: &CsvData) -> Result<Scale> {
    let mut all_y = Vec::new();

    for layer in &spec.layers {
        let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;

        // Extract all y values (grouping doesn't affect y-range calculation)
        let y_selector = csv_reader::parse_column_selector(&resolved.y_col);
        match csv_reader::extract_column(csv_data, y_selector) {
            Ok((_, y_vals)) => all_y.extend(y_vals),
            Err(e) => {
                return Err(anyhow!("Failed to extract y column '{}': {}", resolved.y_col, e));
            }
        }
    }

    Ok(Scale::continuous_from_data(&all_y))
}

// =============================================================================
// End of Scale Infrastructure
// =============================================================================

// =============================================================================
// Unified Renderer Infrastructure (Phase 2)
// =============================================================================

/// Instruction for rendering a single series/layer (unified renderer)
/// Contains data already mapped to coordinate space
#[derive(Debug, Clone)]
struct UnifiedRenderInstruction {
    layer: Layer,
    x_data: Vec<f64>,      // Already in coordinate space (mapped through scale)
    y_data: Vec<f64>,
    x_categories: Option<Vec<String>>,  // For bar charts: actual category names
    style: LayerStyle,
    legend_label: Option<String>,
}

/// Unified style container for all layer types
#[derive(Debug, Clone)]
enum LayerStyle {
    Line(graph::LineStyle),
    Point(graph::PointStyle),
    Bar(graph::BarStyle),
}

impl UnifiedRenderInstruction {
    /// Execute this render instruction on the canvas
    fn draw(&self, canvas: &mut graph::Canvas) -> Result<()> {
        match (&self.layer, &self.style) {
            (Layer::Line(_), LayerStyle::Line(style)) => {
                canvas.add_line_layer(
                    self.x_data.clone(),
                    self.y_data.clone(),
                    style.clone(),
                    self.legend_label.clone(),
                )
            }
            (Layer::Point(_), LayerStyle::Point(style)) => {
                canvas.add_point_layer(
                    self.x_data.clone(),
                    self.y_data.clone(),
                    style.clone(),
                    self.legend_label.clone(),
                )
            }
            (Layer::Bar(_), LayerStyle::Bar(style)) => {
                // For bars, use actual category names
                let categories = self.x_categories.clone().unwrap_or_default();
                canvas.add_bar_layer(
                    categories,
                    self.y_data.clone(),
                    style.clone(),
                )
            }
            _ => Err(anyhow!("Mismatched layer and style types")),
        }
    }
}

// -----------------------------------------------------------------------------
// Unified Renderer Helper Functions
// -----------------------------------------------------------------------------

/// Extract x-coordinates by mapping data through scale
fn extract_x_coordinates(
    csv_data: &CsvData,
    x_col: &str,
    x_scale: &Scale,
) -> Result<Vec<f64>> {
    let x_selector = csv_reader::parse_column_selector(x_col);

    match &x_scale.data_range {
        DataRange::Numeric(_) => {
            // Continuous scale: extract as f64 directly
            let (_, x_vals) = csv_reader::extract_column(csv_data, x_selector)
                .context(format!("Failed to extract x column '{}'", x_col))?;
            Ok(x_vals)
        }
        DataRange::Categorical(_) => {
            // Categorical scale: extract as strings, map to indices
            let (_, x_strings) = csv_reader::extract_column_as_string(csv_data, x_selector)
                .context(format!("Failed to extract x column '{}'", x_col))?;

            x_strings
                .iter()
                .map(|s| x_scale.map_categorical(s))
                .collect()
        }
    }
}

/// Build LayerStyle for non-grouped layer (fixed properties only)
fn build_layer_style_fixed(layer: &Layer) -> LayerStyle {
    match layer {
        Layer::Line(l) => LayerStyle::Line(line_layer_to_style(l)),
        Layer::Point(p) => LayerStyle::Point(point_layer_to_style(p)),
        Layer::Bar(b) => LayerStyle::Bar(bar_layer_to_style(b)),
    }
}

/// Build LayerStyle for grouped layer (merges fixed + group-specific properties)
fn build_layer_style_grouped(
    layer: &Layer,
    group_key: &str,
    resolved: &ResolvedAesthetics,
    color_map: &HashMap<String, String>,
    size_map: &HashMap<String, f64>,
    shape_map: &HashMap<String, String>,
) -> LayerStyle {
    match layer {
        Layer::Line(line_layer) => {
            let mut color = extract_fixed_string(&line_layer.color);
            let mut width = extract_fixed_f64(&line_layer.width);
            let alpha = extract_fixed_f64(&line_layer.alpha);

            if resolved.color_mapping.is_some() {
                color = color_map.get(group_key).cloned();
            }
            if resolved.size_mapping.is_some() {
                width = size_map.get(group_key).copied();
            }

            LayerStyle::Line(graph::LineStyle { color, width, alpha })
        }
        Layer::Point(point_layer) => {
            let mut color = extract_fixed_string(&point_layer.color);
            let mut size = extract_fixed_f64(&point_layer.size);
            let mut shape = extract_fixed_string(&point_layer.shape);
            let alpha = extract_fixed_f64(&point_layer.alpha);

            if resolved.color_mapping.is_some() {
                color = color_map.get(group_key).cloned();
            }
            if resolved.size_mapping.is_some() {
                size = size_map.get(group_key).copied();
            }
            if resolved.shape_mapping.is_some() {
                shape = shape_map.get(group_key).cloned();
            }

            LayerStyle::Point(graph::PointStyle { color, size, shape, alpha })
        }
        Layer::Bar(bar_layer) => {
            let mut color = extract_fixed_string(&bar_layer.color);
            let alpha = extract_fixed_f64(&bar_layer.alpha);
            let width = extract_fixed_f64(&bar_layer.width);

            if resolved.color_mapping.is_some() {
                color = color_map.get(group_key).cloned();
            }

            LayerStyle::Bar(graph::BarStyle { color, alpha, width })
        }
    }
}

/// Prepare render instruction for a single (non-grouped) layer
fn prepare_single_layer_unified(
    layer: &Layer,
    resolved: &ResolvedAesthetics,
    csv_data: &CsvData,
    x_scale: &Scale,
) -> Result<UnifiedRenderInstruction> {
    // Extract x-coordinates mapped through scale
    let x_data = extract_x_coordinates(csv_data, &resolved.x_col, x_scale)?;

    // Extract y-data (always numeric)
    let y_selector = csv_reader::parse_column_selector(&resolved.y_col);
    let (_, y_data) = csv_reader::extract_column(csv_data, y_selector)
        .context(format!("Failed to extract y column '{}'", resolved.y_col))?;

    // For bar charts, also extract category names
    let x_categories = if matches!(layer, Layer::Bar(_)) {
        let x_selector = csv_reader::parse_column_selector(&resolved.x_col);
        let (_, categories) = csv_reader::extract_column_as_string(csv_data, x_selector)
            .context(format!("Failed to extract x column '{}'", resolved.x_col))?;
        Some(categories)
    } else {
        None
    };

    let style = build_layer_style_fixed(layer);

    Ok(UnifiedRenderInstruction {
        layer: layer.clone(),
        x_data,
        y_data,
        x_categories,
        style,
        legend_label: None,
    })
}

/// Prepare render instructions for a grouped layer (one instruction per group)
fn prepare_grouped_layer_unified(
    layer: &Layer,
    resolved: &ResolvedAesthetics,
    csv_data: &CsvData,
    x_scale: &Scale,
    group_col: &str,
) -> Result<Vec<UnifiedRenderInstruction>> {
    // Group data by the grouping column
    let groups = group_data_by_column_with_x_scale(
        csv_data,
        &resolved.x_col,
        &resolved.y_col,
        group_col,
        x_scale,
    )?;

    // Create palettes for group-specific visual properties
    let group_keys: Vec<String> = {
        let mut keys: Vec<_> = groups.keys().cloned().collect();
        keys.sort();
        keys
    };

    let color_map = ColorPalette::category10().assign_colors(&group_keys);
    let size_map = SizePalette::default_range().assign_sizes(&group_keys);
    let shape_map = ShapePalette::default_shapes().assign_shapes(&group_keys);

    // Build one instruction per group
    let mut instructions = Vec::new();
    for group_key in &group_keys {
        let (x_data, y_data) = groups.get(group_key).unwrap();

        // For bar charts, also extract category names
        let x_categories = if matches!(layer, Layer::Bar(_)) {
            // For categorical scale, reconstruct categories from indices
            // This is a bit awkward but necessary for Canvas::add_bar_layer
            match &x_scale.data_range {
                DataRange::Categorical(all_categories) => {
                    // Map indices back to category names
                    let categories: Vec<String> = x_data
                        .iter()
                        .filter_map(|&idx| {
                            let i = idx as usize;
                            all_categories.get(i).cloned()
                        })
                        .collect();
                    Some(categories)
                }
                _ => None,
            }
        } else {
            None
        };

        let style = build_layer_style_grouped(
            layer,
            group_key,
            resolved,
            &color_map,
            &size_map,
            &shape_map,
        );

        instructions.push(UnifiedRenderInstruction {
            layer: layer.clone(),
            x_data: x_data.clone(),
            y_data: y_data.clone(),
            x_categories,
            style,
            legend_label: Some(group_key.clone()),
        });
    }

    Ok(instructions)
}

/// Group data by column, with x-values mapped through scale
fn group_data_by_column_with_x_scale(
    csv_data: &CsvData,
    x_col: &str,
    y_col: &str,
    group_col: &str,
    x_scale: &Scale,
) -> Result<HashMap<String, (Vec<f64>, Vec<f64>)>> {
    // Find column indices
    let group_idx = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(group_col))
        .ok_or_else(|| anyhow!("Column '{}' not found", group_col))?;

    let x_idx = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(x_col))
        .ok_or_else(|| anyhow!("Column '{}' not found", x_col))?;

    let y_idx = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(y_col))
        .ok_or_else(|| anyhow!("Column '{}' not found", y_col))?;

    let mut groups: HashMap<String, (Vec<f64>, Vec<f64>)> = HashMap::new();

    for row in &csv_data.rows {
        let group_key = row[group_idx].clone();

        // Map x through scale
        let x_val = match &x_scale.data_range {
            DataRange::Numeric(_) => row[x_idx]
                .parse::<f64>()
                .context(format!("Failed to parse x value '{}'", row[x_idx]))?,
            DataRange::Categorical(_) => x_scale.map_categorical(&row[x_idx])?,
        };

        let y_val = row[y_idx]
            .parse::<f64>()
            .context(format!("Failed to parse y value '{}'", row[y_idx]))?;

        let (x_data, y_data) = groups
            .entry(group_key)
            .or_insert_with(|| (Vec::new(), Vec::new()));
        x_data.push(x_val);
        y_data.push(y_val);
    }

    Ok(groups)
}

// =============================================================================
// End of Unified Renderer Infrastructure
// =============================================================================

/// Unified renderer - scale-centric approach (Phases 2-4)
/// Determines scale type first, then renders layers onto that coordinate system
fn render_plot_unified(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>> {
    // Validate: must have at least one layer
    if spec.layers.is_empty() {
        anyhow::bail!("Plot requires at least one geometry layer (line, point, etc.)");
    }

    // Route faceting to old renderer for now (Phase 4 will integrate)
    if spec.facet.is_some() {
        let facet = spec.facet.clone().unwrap();
        return render_faceted_plot(spec, csv_data, facet);
    }

    // === UNIFIED RENDERING PATH ===

    // STEP 1: DETERMINE SCALES FIRST ⭐
    let x_scale_type = determine_x_scale(&spec, &csv_data)?;
    let x_scale = build_x_scale(&spec, &csv_data, x_scale_type)?;
    let y_scale = build_y_scale(&spec, &csv_data)?;

    // STEP 2: PREPARE RENDER INSTRUCTIONS
    let mut instructions = Vec::new();

    for layer in &spec.layers {
        let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;

        // Check if grouped by color/size/shape/alpha
        let group_col = resolved
            .color_mapping
            .as_ref()
            .or(resolved.size_mapping.as_ref())
            .or(resolved.shape_mapping.as_ref())
            .or(resolved.alpha_mapping.as_ref());

        if let Some(group_col) = group_col {
            // GROUPED: prepare one instruction per group
            let layer_instructions = prepare_grouped_layer_unified(
                layer,
                &resolved,
                &csv_data,
                &x_scale,
                group_col,
            )?;
            instructions.extend(layer_instructions);
        } else {
            // NON-GROUPED: prepare single instruction
            let instruction = prepare_single_layer_unified(
                layer,
                &resolved,
                &csv_data,
                &x_scale,
            )?;
            instructions.push(instruction);
        }
    }

    // STEP 3: CREATE CANVAS WITH COORDINATE RANGES
    let mut canvas = graph::Canvas::with_ranges(
        800,
        600,
        spec.labels.as_ref().and_then(|l| l.title.clone()),
        x_scale.coord_range.clone(),
        y_scale.coord_range.clone(),
    )?;

    // Set categorical labels if needed
    if let Some(labels) = x_scale.get_axis_labels() {
        canvas.set_x_labels(labels);
    }

    // STEP 4: HANDLE BAR GROUPS (dodge/stack) & EXECUTE RENDER INSTRUCTIONS
    // Check if we have multiple bar layers that need to be grouped
    let bar_instructions: Vec<_> = instructions.iter()
        .filter(|i| matches!(i.layer, Layer::Bar(_)))
        .collect();

    if bar_instructions.len() > 1 {
        // Multiple bar layers - check if they should be grouped (dodge/stack)
        use crate::parser::ast::BarPosition;

        let positions: Vec<BarPosition> = bar_instructions.iter()
            .map(|i| match &i.layer {
                Layer::Bar(b) => b.position.clone(),
                _ => BarPosition::Identity,
            })
            .collect();

        // If all bars have the same non-identity position, group them
        let first_pos = &positions[0];
        let should_group = !matches!(first_pos, BarPosition::Identity)
            && positions.iter().all(|p| p == first_pos);

        if should_group {
            // Render as bar group
            let position_str = match first_pos {
                BarPosition::Dodge => "dodge",
                BarPosition::Stack => "stack",
                BarPosition::Identity => "identity",
            };

            // Collect categories (same for all bars) and series
            let categories = bar_instructions[0].x_categories.clone().unwrap_or_default();
            let series: Vec<(Vec<f64>, graph::BarStyle, Option<String>)> = bar_instructions.iter()
                .map(|instr| {
                    let style = match &instr.style {
                        LayerStyle::Bar(s) => s.clone(),
                        _ => graph::BarStyle { color: None, alpha: None, width: None },
                    };
                    (instr.y_data.clone(), style, instr.legend_label.clone())
                })
                .collect();

            canvas.add_bar_group(categories, series, position_str)?;

            // Render non-bar instructions
            for instruction in &instructions {
                if !matches!(instruction.layer, Layer::Bar(_)) {
                    instruction.draw(&mut canvas)?;
                }
            }
        } else {
            // Render all individually
            for instruction in instructions {
                instruction.draw(&mut canvas)?;
            }
        }
    } else {
        // No bar grouping needed - render all individually
        for instruction in instructions {
            instruction.draw(&mut canvas)?;
        }
    }

    // STEP 5: ENCODE PNG AND RETURN
    canvas.render()
}

/// Render a plot specification to PNG bytes
pub fn render_plot(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>> {
    // Now using the unified scale-centric renderer (Phase 4)
    render_plot_unified(spec, csv_data)
}

// Old Renderer trait and structs removed in Phase 5
// The unified renderer (render_plot_unified) replaces:
//   - CategoricalRenderer + render_categorical_plot()
//   - ContinuousRenderer + render_continuous_plot()
// Faceting still uses the old render_faceted_plot() for now

/// Render a plot with categorical x-axis (supports Bar, Line, Point)
fn render_categorical_plot(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>> {
    use crate::parser::ast::BarPosition;

    // 1. Collect all unique categories from all layers to define the x-axis
    let mut categories_set = std::collections::HashSet::new();
    let mut categories_order = Vec::new();

    for layer in &spec.layers {
        let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;
        let x_selector = csv_reader::parse_column_selector(&resolved.x_col);
        // We try to read as string. If it fails (e.g. column not found), we'll catch it later.
        if let Ok((_, x_vals)) = csv_reader::extract_column_as_string(&csv_data, x_selector) {
            for cat in x_vals {
                if !categories_set.contains(&cat) {
                    categories_set.insert(cat.clone());
                    categories_order.push(cat);
                }
            }
        }
    }
    categories_order.sort(); // Deterministic order
    
    let cat_map: HashMap<String, usize> = categories_order
        .iter()
        .enumerate()
        .map(|(i, c)| (c.clone(), i))
        .collect();
    
        let num_categories = categories_order.len();
        let all_x_data = vec![-0.5, (num_categories as f64) - 0.5];
        let mut all_y_data = Vec::new();
    // 2. Process data for each layer
    struct RenderOp {
        layer: Layer,
        // For Bar: categories + y (aligned)
        // For Line/Point: x (indices) + y
        x_data: Vec<f64>,
        y_data: Vec<f64>,
        bar_categories: Option<Vec<String>>, // Only for Bar
    }
    
    let mut ops = Vec::new();
    // Special handling for grouped bars
    let mut bar_groups: Vec<(Vec<String>, Vec<(Vec<f64>, graph::BarStyle, Option<String>)>, String)> = Vec::new(); 

    for layer in &spec.layers {
        let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;
        let group_col_opt = resolved.color_mapping.as_ref().or(resolved.alpha_mapping.as_ref());

        match layer {
            Layer::Bar(bar_layer) => {
                if let Some(group_col) = group_col_opt {
                    // Grouped Bar
                    let (raw_cats, groups) = extract_grouped_categorical_data(&csv_data, &resolved.x_col, &resolved.y_col, group_col)?;
                    let group_keys: Vec<String> = groups.iter().map(|(k,_)| k.clone()).collect();
                    let color_palette = ColorPalette::category10();
                    let color_map = color_palette.assign_colors(&group_keys);
                    
                    let mut series = Vec::new();
                    
                    for (g_key, raw_y) in groups {
                        let mut aligned_y = vec![0.0; num_categories];
                        for (i, cat) in raw_cats.iter().enumerate() {
                            if let Some(&idx) = cat_map.get(cat) {
                                aligned_y[idx] = raw_y[i];
                            }
                        }
                        
                        let mut style = bar_layer_to_style(bar_layer);
                        if resolved.color_mapping.is_some() {
                            style.color = color_map.get(&g_key).cloned().or(style.color);
                        }
                        all_y_data.extend(aligned_y.iter().cloned());
                        series.push((aligned_y, style.clone(), Some(g_key.clone())));
                    }
                    
                    let pos_str = match bar_layer.position {
                        BarPosition::Dodge => "dodge",
                        BarPosition::Stack => "stack",
                        BarPosition::Identity => "identity",
                    };
                    
                    bar_groups.push((categories_order.clone(), series, pos_str.to_string()));
                    
                } else {
                    // Ungrouped Bar
                    let (cats, y_vals) = extract_categorical_data(&csv_data, &resolved.x_col, &resolved.y_col)?;
                    let mut aligned_y = vec![0.0; num_categories];
                    for (i, cat) in cats.iter().enumerate() {
                        if let Some(&idx) = cat_map.get(cat) {
                            aligned_y[idx] = y_vals[i];
                        }
                    }
                    all_y_data.extend(aligned_y.iter().cloned());
                    
                    ops.push(RenderOp {
                        layer: layer.clone(),
                        x_data: vec![],
                        y_data: aligned_y,
                        bar_categories: Some(categories_order.clone()),
                    });
                }
            },
            Layer::Line(_) => {
                let (_, x_str) = csv_reader::extract_column_as_string(&csv_data, csv_reader::parse_column_selector(&resolved.x_col))?;
                let (_, y_val) = csv_reader::extract_column(&csv_data, csv_reader::parse_column_selector(&resolved.y_col))?;
                
                let mut x_f64 = Vec::new();
                let mut y_f64 = Vec::new();
                for (i, x_s) in x_str.iter().enumerate() {
                    if let Some(&idx) = cat_map.get(x_s) {
                        x_f64.push(idx as f64);
                        y_f64.push(y_val[i]);
                    }
                }
                all_y_data.extend(y_f64.iter().cloned());
                
                ops.push(RenderOp {
                    layer: layer.clone(),
                    x_data: x_f64,
                    y_data: y_f64,
                    bar_categories: None,
                });
            },
            Layer::Point(_) => {
                let (_, x_str) = csv_reader::extract_column_as_string(&csv_data, csv_reader::parse_column_selector(&resolved.x_col))?;
                let (_, y_val) = csv_reader::extract_column(&csv_data, csv_reader::parse_column_selector(&resolved.y_col))?;
                
                let mut x_f64 = Vec::new();
                let mut y_f64 = Vec::new();
                for (i, x_s) in x_str.iter().enumerate() {
                    if let Some(&idx) = cat_map.get(x_s) {
                        x_f64.push(idx as f64);
                        y_f64.push(y_val[i]);
                    }
                }
                all_y_data.extend(y_f64.iter().cloned());
                
                ops.push(RenderOp {
                    layer: layer.clone(),
                    x_data: x_f64,
                    y_data: y_f64,
                    bar_categories: None,
                });
            }
        }
    }
    
    // For stacked bars, calculate cumulative y values for range
    let first_bar_pos = spec.layers.iter().filter_map(|l| match l { Layer::Bar(b) => Some(b.position.clone()), _ => None }).next();
    if matches!(first_bar_pos, Some(BarPosition::Stack)) {
         let _max_stack = 0.0;
         // Approximate range calculation for stack (sum of all Ys at each X?)
         // This is tricky without reconstructing the stack. For now, let's rely on individual values or improve range calculation later.
         // Actually, to be safe, if we have stacks, we might need larger range.
         // Let's assume user provides sensible data or we just use sum of positive values per category.
    }

    let mut canvas = graph::Canvas::new(
        800,
        600,
        spec.labels.as_ref().and_then(|l| l.title.clone()),
        all_x_data,
        all_y_data,
    )?;
    canvas.set_x_labels(categories_order);

    // Render Groups
    for group in bar_groups {
        canvas.add_bar_group(group.0, group.1, &group.2)?;
    }

    // Render Layers
    for op in ops {
        match op.layer {
            Layer::Bar(b) => {
                if let Some(cats) = op.bar_categories {
                    canvas.add_bar_layer(cats, op.y_data, bar_layer_to_style(&b))?;
                }
            },
            Layer::Line(l) => {
                canvas.add_line_layer(op.x_data, op.y_data, line_layer_to_style(&l), None)?;
            },
            Layer::Point(p) => {
                canvas.add_point_layer(op.x_data, op.y_data, point_layer_to_style(&p), None)?;
            }
        }
    }

    canvas.render()
}

/// Render a plot with continuous charts (line/point)
fn render_continuous_plot(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>> {
    let mut all_x_data = Vec::new();
    let mut all_y_data = Vec::new();
    let mut render_instructions: Vec<RenderInstruction> = Vec::new();

    for layer in &spec.layers {
        // Resolve all aesthetics (x, y, and visual properties)
        let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;

        // Check if this layer needs grouping (has color/size/shape mapping)
        let primary_group_col = resolved.color_mapping.as_ref()
            .or(resolved.size_mapping.as_ref())
            .or(resolved.shape_mapping.as_ref())
            .or(resolved.alpha_mapping.as_ref());

        if let Some(group_col) = primary_group_col {
            // GROUPED RENDERING: Group data by the grouping column
            let groups = group_data_by_column(&csv_data, &resolved.x_col, &resolved.y_col, group_col)?;

            // Collect all data for range calculation
            for (x_data, y_data) in groups.values() {
                all_x_data.extend(x_data.iter().copied());
                all_y_data.extend(y_data.iter().copied());
            }

            // Get sorted group keys for consistent ordering
            let mut group_keys: Vec<String> = groups.keys().cloned().collect();
            group_keys.sort();

            // Create palettes
            let color_palette = ColorPalette::category10();
            let size_palette = SizePalette::default_range();
            let shape_palette = ShapePalette::default_shapes();

            let color_map = color_palette.assign_colors(&group_keys);
            let size_map = size_palette.assign_sizes(&group_keys);
            let shape_map = shape_palette.assign_shapes(&group_keys);

            // Create render instruction for each group
            for group_key in &group_keys {
                let (x_data, y_data) = groups.get(group_key).unwrap();

                // Build style with group-specific properties
                let style = build_grouped_style(
                    layer,
                    group_key,
                    &resolved,
                    &color_map,
                    &size_map,
                    &shape_map,
                );

                render_instructions.push(RenderInstruction {
                    layer: layer.clone(),
                    x_data: x_data.clone(),
                    y_data: y_data.clone(),
                    line_style: style.line_style,
                    point_style: style.point_style,
                    legend_label: Some(group_key.clone()),
                });
            }
        } else {
            // NON-GROUPED RENDERING: Extract data normally
            let x_selector = csv_reader::parse_column_selector(&resolved.x_col);
            let (_x_col_name, x_values) = csv_reader::extract_column(&csv_data, x_selector)
                .context(format!("Failed to extract x column '{}'", resolved.x_col))?;

            let y_selector = csv_reader::parse_column_selector(&resolved.y_col);
            let (_y_col_name, y_values) = csv_reader::extract_column(&csv_data, y_selector)
                .context(format!("Failed to extract y column '{}'", resolved.y_col))?;

            // Collect for global range
            all_x_data.extend(x_values.iter().copied());
            all_y_data.extend(y_values.iter().copied());

            // Build fixed style
            let style = build_fixed_style(layer);

            render_instructions.push(RenderInstruction {
                layer: layer.clone(),
                x_data: x_values,
                y_data: y_values,
                line_style: style.line_style,
                point_style: style.point_style,
                legend_label: None,
            });
        }
    }

    // Create canvas with global data ranges
    let mut canvas = graph::Canvas::new(
        800,
        600,
        spec.labels.as_ref().and_then(|l| l.title.clone()),
        all_x_data,
        all_y_data,
    )?;

    // Execute render instructions
    for instruction in &render_instructions {
        match &instruction.layer {
            Layer::Line(_) => {
                canvas.add_line_layer(
                    instruction.x_data.clone(),
                    instruction.y_data.clone(),
                    instruction.line_style.clone().unwrap_or_default(),
                    instruction.legend_label.clone(),
                )?;
            }
            Layer::Point(_) => {
                canvas.add_point_layer(
                    instruction.x_data.clone(),
                    instruction.y_data.clone(),
                    instruction.point_style.clone().unwrap_or_default(),
                    instruction.legend_label.clone(),
                )?;
            }
            Layer::Bar(_) => {
                unreachable!()
            }
        }
    }

    canvas.render()
}

/// Render instruction for a single series
struct RenderInstruction {
    layer: Layer,
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    line_style: Option<graph::LineStyle>,
    point_style: Option<graph::PointStyle>,
    legend_label: Option<String>,
}

/// Unified style that can be either Line or Point
struct UnifiedStyle {
    line_style: Option<graph::LineStyle>,
    point_style: Option<graph::PointStyle>,
}

/// Build grouped style by merging fixed properties with group-specific mappings
fn build_grouped_style(
    layer: &Layer,
    group_key: &str,
    resolved: &ResolvedAesthetics,
    color_map: &HashMap<String, String>,
    size_map: &HashMap<String, f64>,
    shape_map: &HashMap<String, String>,
) -> UnifiedStyle {
    match layer {
        Layer::Line(line_layer) => {
            // Start with fixed properties
            let mut color = extract_fixed_string(&line_layer.color);
            let mut width = extract_fixed_f64(&line_layer.width);
            let alpha = extract_fixed_f64(&line_layer.alpha);

            // Override with group-specific values if mapped
            if resolved.color_mapping.is_some() {
                color = color_map.get(group_key).cloned();
            }
            if resolved.size_mapping.is_some() {
                width = size_map.get(group_key).copied();
            }

            UnifiedStyle {
                line_style: Some(graph::LineStyle { color, width, alpha }),
                point_style: None,
            }
        }
        Layer::Point(point_layer) => {
            // Start with fixed properties
            let mut color = extract_fixed_string(&point_layer.color);
            let mut size = extract_fixed_f64(&point_layer.size);
            let mut shape = extract_fixed_string(&point_layer.shape);
            let alpha = extract_fixed_f64(&point_layer.alpha);

            // Override with group-specific values if mapped
            if resolved.color_mapping.is_some() {
                color = color_map.get(group_key).cloned();
            }
            if resolved.size_mapping.is_some() {
                size = size_map.get(group_key).copied();
            }
            if resolved.shape_mapping.is_some() {
                shape = shape_map.get(group_key).cloned();
            }

            UnifiedStyle {
                line_style: None,
                point_style: Some(graph::PointStyle { color, size, shape, alpha }),
            }
        }
        Layer::Bar(_) => unreachable!(),
    }
}

/// Build fixed style (no grouping)
fn build_fixed_style(layer: &Layer) -> UnifiedStyle {
    match layer {
        Layer::Line(line_layer) => UnifiedStyle {
            line_style: Some(line_layer_to_style(line_layer)),
            point_style: None,
        },
        Layer::Point(point_layer) => UnifiedStyle {
            line_style: None,
            point_style: Some(point_layer_to_style(point_layer)),
        },
        Layer::Bar(_) => unreachable!(),
    }
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
        color: extract_fixed_string(&layer.color),
        width: extract_fixed_f64(&layer.width),
        alpha: extract_fixed_f64(&layer.alpha),
    }
}

/// Extract fixed string value from AestheticValue
fn extract_fixed_string(value: &Option<AestheticValue<String>>) -> Option<String> {
    match value {
        Some(AestheticValue::Fixed(s)) => Some(s.clone()),
        _ => None, // Mapped values will be handled by grouping logic later
    }
}

/// Extract fixed f64 value from AestheticValue
fn extract_fixed_f64(value: &Option<AestheticValue<f64>>) -> Option<f64> {
    match value {
        Some(AestheticValue::Fixed(v)) => Some(*v),
        _ => None, // Mapped values will be handled by grouping logic later
    }
}

/// Resolved aesthetic mappings for a layer
/// Includes both positional aesthetics (x, y) and visual aesthetics (color, size, shape, alpha)
#[derive(Debug, Clone)]
struct ResolvedAesthetics {
    x_col: String,
    y_col: String,
    color_mapping: Option<String>,  // Column name for color grouping, or None
    size_mapping: Option<String>,   // Column name for size grouping, or None
    shape_mapping: Option<String>,  // Column name for shape grouping, or None
    alpha_mapping: Option<String>,  // Column name for alpha grouping, or None
}

/// Resolve all aesthetic mappings for a layer (layer-specific + global)
fn resolve_layer_aesthetics(
    layer: &Layer,
    global_aes: &Option<Aesthetics>,
) -> Result<ResolvedAesthetics> {
    // Resolve x and y (required)
    let (x_col, y_col) = resolve_aesthetics(layer, global_aes)?;

    // Resolve color mapping
    let color_mapping = match layer {
        Layer::Line(l) => extract_mapped_string(&l.color),
        Layer::Point(p) => extract_mapped_string(&p.color),
        Layer::Bar(b) => extract_mapped_string(&b.color),
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.color.clone()));

    // Resolve size mapping
    let size_mapping = match layer {
        Layer::Line(l) => extract_mapped_string_from_f64(&l.width), // width can be data-driven
        Layer::Point(p) => extract_mapped_string_from_f64(&p.size),
        Layer::Bar(b) => extract_mapped_string_from_f64(&b.width),
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.size.clone()));

    // Resolve shape mapping (point only)
    let shape_mapping = match layer {
        Layer::Point(p) => extract_mapped_string(&p.shape),
        _ => None,
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.shape.clone()));

    // Resolve alpha mapping
    let alpha_mapping = match layer {
        Layer::Line(l) => extract_mapped_string_from_f64(&l.alpha),
        Layer::Point(p) => extract_mapped_string_from_f64(&p.alpha),
        Layer::Bar(b) => extract_mapped_string_from_f64(&b.alpha),
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.alpha.clone()));

    Ok(ResolvedAesthetics {
        x_col,
        y_col,
        color_mapping,
        size_mapping,
        shape_mapping,
        alpha_mapping,
    })
}

/// Extract column name from Mapped variant of AestheticValue<String>
fn extract_mapped_string(value: &Option<AestheticValue<String>>) -> Option<String> {
    match value {
        Some(AestheticValue::Mapped(col)) => Some(col.clone()),
        _ => None,
    }
}

/// Extract column name from Mapped variant of AestheticValue<f64>
fn extract_mapped_string_from_f64(value: &Option<AestheticValue<f64>>) -> Option<String> {
    match value {
        Some(AestheticValue::Mapped(col)) => Some(col.clone()),
        _ => None,
    }
}

/// Group data by a categorical column
/// Returns: HashMap<group_key, (x_data, y_data)>
fn group_data_by_column(
    csv_data: &CsvData,
    x_col: &str,
    y_col: &str,
    group_col: &str,
) -> Result<HashMap<String, (Vec<f64>, Vec<f64>)>> {
    // Find column indices
    let group_col_idx = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(group_col))
        .ok_or_else(|| anyhow!("Column '{}' not found for grouping", group_col))?;

    let x_col_idx = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(x_col))
        .ok_or_else(|| anyhow!("Column '{}' not found", x_col))?;

    let y_col_idx = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(y_col))
        .ok_or_else(|| anyhow!("Column '{}' not found", y_col))?;

    // Group data by the grouping column
    let mut groups: HashMap<String, (Vec<f64>, Vec<f64>)> = HashMap::new();

    for row in &csv_data.rows {
        let group_key = row[group_col_idx].clone();

        let x_val = row[x_col_idx]
            .parse::<f64>()
            .context(format!("Failed to parse x value: {}", row[x_col_idx]))?;

        let y_val = row[y_col_idx]
            .parse::<f64>()
            .context(format!("Failed to parse y value: {}", row[y_col_idx]))?;

        let (x_data, y_data) = groups
            .entry(group_key)
            .or_insert_with(|| (Vec::new(), Vec::new()));

        x_data.push(x_val);
        y_data.push(y_val);
    }

    Ok(groups)
}

/// Convert PointLayer to graph::PointStyle
fn point_layer_to_style(layer: &PointLayer) -> graph::PointStyle {
    graph::PointStyle {
        color: extract_fixed_string(&layer.color),
        size: extract_fixed_f64(&layer.size),
        shape: extract_fixed_string(&layer.shape),
        alpha: extract_fixed_f64(&layer.alpha),
    }
}

/// Convert BarLayer to graph::BarStyle
fn bar_layer_to_style(layer: &BarLayer) -> graph::BarStyle {
    graph::BarStyle {
        color: extract_fixed_string(&layer.color),
        alpha: extract_fixed_f64(&layer.alpha),
        width: extract_fixed_f64(&layer.width),
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







/// Extract grouped categorical data from CSV
/// Returns (all_categories, vec[(group_name, y_values)])
fn extract_grouped_categorical_data(
    csv_data: &CsvData,
    x_col: &str,
    y_col: &str,
    group_col: &str,
) -> Result<(Vec<String>, Vec<(String, Vec<f64>)>)> {
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

    let group_col_index = csv_data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(group_col))
        .ok_or_else(|| anyhow!("Column '{}' not found for grouping", group_col))?;

    // First pass: identify all unique categories and groups
    let mut all_categories: Vec<String> = Vec::new();
    let mut categories_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut groups_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut data_map: HashMap<(String, String), f64> = HashMap::new(); // (group, category) -> sum

    for (row_idx, row) in csv_data.rows.iter().enumerate() {
        let category = row[x_col_index].clone();
        let group = row[group_col_index].clone();
        
        let y_str = &row[y_col_index];
        let y_val = y_str.parse::<f64>().with_context(|| {
            format!(
                "Failed to parse '{}' as number in column '{}' at row {}",
                y_str,
                y_col,
                row_idx + 1
            )
        })?;

        if !categories_set.contains(&category) {
            categories_set.insert(category.clone());
            all_categories.push(category.clone());
        }
        groups_set.insert(group.clone());

        *data_map.entry((group, category)).or_insert(0.0) += y_val;
    }

    // Sort groups for consistent order
    let mut sorted_groups: Vec<String> = groups_set.into_iter().collect();
    sorted_groups.sort();

    // Build result vectors
    let mut result_groups = Vec::new();

    for group in sorted_groups {
        let mut y_values = Vec::new();
        for cat in &all_categories {
            let val = *data_map.get(&(group.clone(), cat.clone())).unwrap_or(&0.0);
            y_values.push(val);
        }
        result_groups.push((group, y_values));
    }

    Ok((all_categories, result_groups))
}

/// Split CSV data by facet column
fn split_data_by_facet(csv_data: &CsvData, facet_col: &str) -> Result<HashMap<String, CsvData>> {
    // Find facet column index
    let facet_col_idx = csv_data.headers.iter()
        .position(|h| h == facet_col)
        .ok_or_else(|| anyhow!("Facet column '{}' not found in data", facet_col))?;

    // Group rows by facet value
    let mut facet_groups: HashMap<String, Vec<Vec<String>>> = HashMap::new();

    for row in &csv_data.rows {
        let facet_value = row.get(facet_col_idx)
            .ok_or_else(|| anyhow!("Missing facet value in row"))?
            .clone();

        facet_groups.entry(facet_value)
            .or_insert_with(Vec::new)
            .push(row.clone());
    }

    // Convert to CsvData for each facet
    let mut result = HashMap::new();
    for (facet_value, rows) in facet_groups {
        result.insert(facet_value, CsvData {
            headers: csv_data.headers.clone(),
            rows,
        });
    }

    Ok(result)
}

/// Calculate global or per-facet ranges based on scales mode
fn calculate_facet_ranges(
    spec: &PlotSpec,
    facet_data: &HashMap<String, CsvData>,
    scales: &FacetScales,
) -> Result<HashMap<String, (Range<f64>, Range<f64>)>> {
    let mut ranges = HashMap::new();

    match scales {
        FacetScales::Fixed => {
            // Global ranges: collect all data from all facets
            let mut all_x = Vec::new();
            let mut all_y = Vec::new();

            for facet_csv in facet_data.values() {
                // Extract x and y data from all layers
                for layer in &spec.layers {
                    let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;

                    let x_selector = csv_reader::parse_column_selector(&resolved.x_col);
                    if let Ok((_col_name, x_vals)) = csv_reader::extract_column(facet_csv, x_selector) {
                        all_x.extend(x_vals);
                    }

                    let y_selector = csv_reader::parse_column_selector(&resolved.y_col);
                    if let Ok((_col_name, y_vals)) = csv_reader::extract_column(facet_csv, y_selector) {
                        all_y.extend(y_vals);
                    }
                }
            }

            // Calculate single global range
            let x_range = calculate_range(&all_x);
            let y_range = calculate_range(&all_y);

            // Assign same range to all facets
            for facet_key in facet_data.keys() {
                ranges.insert(facet_key.clone(), (x_range.clone(), y_range.clone()));
            }
        }
        FacetScales::FreeX | FacetScales::FreeY | FacetScales::Free => {
            // Calculate per-facet ranges
            for (facet_key, facet_csv) in facet_data {
                let mut facet_x = Vec::new();
                let mut facet_y = Vec::new();

                for layer in &spec.layers {
                    let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;

                    let x_selector = csv_reader::parse_column_selector(&resolved.x_col);
                    if let Ok((_col_name, x_vals)) = csv_reader::extract_column(facet_csv, x_selector) {
                        facet_x.extend(x_vals);
                    }

                    let y_selector = csv_reader::parse_column_selector(&resolved.y_col);
                    if let Ok((_col_name, y_vals)) = csv_reader::extract_column(facet_csv, y_selector) {
                        facet_y.extend(y_vals);
                    }
                }

                let x_range = calculate_range(&facet_x);
                let y_range = calculate_range(&facet_y);
                ranges.insert(facet_key.clone(), (x_range, y_range));
            }
        }
    }

    Ok(ranges)
}

/// Calculate range with padding for a dataset
fn calculate_range(data: &[f64]) -> Range<f64> {
    if data.is_empty() {
        return 0.0..1.0;
    }

    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if min == max {
        (min - 1.0)..(max + 1.0)
    } else {
        let padding = (max - min) * 0.05;
        (min - padding)..(max + padding)
    }
}

/// Render a faceted plot (subplot grid based on categorical column)
fn render_faceted_plot(spec: PlotSpec, csv_data: CsvData, facet: Facet) -> Result<Vec<u8>> {
    // Split data by facet column
    let facet_data = split_data_by_facet(&csv_data, &facet.by)?;

    if facet_data.is_empty() {
        anyhow::bail!("No data to facet by column '{}'", facet.by);
    }

    // Get sorted facet keys for consistent ordering
    let mut facet_keys: Vec<String> = facet_data.keys().cloned().collect();
    facet_keys.sort();

    // Calculate grid layout
    let num_facets = facet_keys.len();
    let ncol = facet.ncol.unwrap_or_else(|| (num_facets as f64).sqrt().ceil() as usize);
    let nrow = (num_facets as f64 / ncol as f64).ceil() as usize;

    // Calculate ranges based on scales mode
    let ranges = calculate_facet_ranges(&spec, &facet_data, &facet.scales)?;

    // Create multi-facet canvas
    let mut canvas = graph::MultiFacetCanvas::new(1200, 800, nrow, ncol)?;

    // Render each facet
    for (facet_idx, facet_key) in facet_keys.iter().enumerate() {
        let facet_csv = facet_data.get(facet_key).unwrap();
        let (x_range, y_range) = ranges.get(facet_key).unwrap();

        // Calculate grid position
        let row = facet_idx / ncol;
        let col = facet_idx % ncol;

        // Collect all series to render on this facet
        let mut facet_series_list: Vec<graph::FacetSeries> = Vec::new();

        // For each layer, extract data and render
        for layer in &spec.layers {
            let resolved = resolve_layer_aesthetics(layer, &spec.aesthetics)?;

            // Check if this layer needs grouping (has color/size/shape mapping)
            let primary_group_col = resolved.color_mapping.as_ref()
                .or(resolved.size_mapping.as_ref())
                .or(resolved.shape_mapping.as_ref())
                .or(resolved.alpha_mapping.as_ref());

            if let Some(group_col) = primary_group_col {
                // GROUPED RENDERING
                // Note: Ideally we should use global palette across all facets for consistency,
                // but for now we generate local palette per facet to ensure grouping works at all.
                let groups = group_data_by_column(facet_csv, &resolved.x_col, &resolved.y_col, group_col)?;

                // Get sorted group keys
                let mut group_keys: Vec<String> = groups.keys().cloned().collect();
                group_keys.sort();

                let color_palette = ColorPalette::category10();
                let size_palette = SizePalette::default_range();
                let shape_palette = ShapePalette::default_shapes();

                let color_map = color_palette.assign_colors(&group_keys);
                let size_map = size_palette.assign_sizes(&group_keys);
                let shape_map = shape_palette.assign_shapes(&group_keys);

                for group_key in &group_keys {
                    let (x_data, y_data) = groups.get(group_key).unwrap();
                    
                    let style = build_grouped_style(
                        layer,
                        group_key,
                        &resolved,
                        &color_map,
                        &size_map,
                        &shape_map,
                    );

                    facet_series_list.push(graph::FacetSeries {
                        x_data: x_data.clone(),
                        y_data: y_data.clone(),
                        line_style: style.line_style,
                        point_style: style.point_style,
                    });
                }
            } else {
                // NON-GROUPED RENDERING
                let x_selector = csv_reader::parse_column_selector(&resolved.x_col);
                let (_x_col_name, x_data) = csv_reader::extract_column(facet_csv, x_selector)
                    .context(format!("Failed to extract x column '{}' in facet '{}'", resolved.x_col, facet_key))?;

                let y_selector = csv_reader::parse_column_selector(&resolved.y_col);
                let (_y_col_name, y_data) = csv_reader::extract_column(facet_csv, y_selector)
                    .context(format!("Failed to extract y column '{}' in facet '{}'", resolved.y_col, facet_key))?;

                // Build style (using fixed styles only)
                let style = build_fixed_style(layer);

                facet_series_list.push(graph::FacetSeries {
                    x_data,
                    y_data,
                    line_style: style.line_style,
                    point_style: style.point_style,
                });
            }
        }

        // Render facet panel
        canvas.render_facet(
            row,
            col,
            facet_key,
            facet_series_list,
            x_range.clone(),
            y_range.clone(),
        )?;
    }

    canvas.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{BarPosition, Aesthetics, PlotSpec};

    /// Helper to create test CsvData
    fn make_csv_data(headers: Vec<&str>, rows: Vec<Vec<&str>>) -> CsvData {
        CsvData {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: rows.iter()
                .map(|r| r.iter().map(|s| s.to_string()).collect())
                .collect(),
        }
    }

    // =============================================================================
    // Scale Infrastructure Tests
    // =============================================================================

    #[test]
    fn test_scale_continuous_from_data() {
        let data = vec![0.0, 10.0, 20.0];
        let scale = Scale::continuous_from_data(&data);

        assert_eq!(scale.scale_type, ScaleType::Continuous);

        // Check range has 5% padding
        let expected_min = 0.0 - (20.0 - 0.0) * 0.05;
        let expected_max = 20.0 + (20.0 - 0.0) * 0.05;

        match scale.data_range {
            DataRange::Numeric(ref range) => {
                assert!((range.start - expected_min).abs() < 0.001);
                assert!((range.end - expected_max).abs() < 0.001);
            }
            _ => panic!("Expected Numeric data range"),
        }

        // Coord range should match data range for continuous
        assert!((scale.coord_range.start - expected_min).abs() < 0.001);
        assert!((scale.coord_range.end - expected_max).abs() < 0.001);
    }

    #[test]
    fn test_scale_continuous_from_data_single_value() {
        let data = vec![5.0, 5.0, 5.0];
        let scale = Scale::continuous_from_data(&data);

        // When all values are the same, should expand to +/- 1
        match scale.data_range {
            DataRange::Numeric(ref range) => {
                assert_eq!(range.start, 4.0);
                assert_eq!(range.end, 6.0);
            }
            _ => panic!("Expected Numeric data range"),
        }
    }

    #[test]
    fn test_scale_continuous_from_empty_data() {
        let data: Vec<f64> = vec![];
        let scale = Scale::continuous_from_data(&data);

        // Empty data should default to 0..1
        match scale.data_range {
            DataRange::Numeric(ref range) => {
                assert_eq!(range.start, 0.0);
                assert_eq!(range.end, 1.0);
            }
            _ => panic!("Expected Numeric data range"),
        }
    }

    #[test]
    fn test_scale_categorical_from_categories() {
        let categories = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let scale = Scale::categorical_from_categories(categories.clone());

        assert_eq!(scale.scale_type, ScaleType::Categorical);

        match scale.data_range {
            DataRange::Categorical(ref cats) => {
                assert_eq!(cats, &categories);
            }
            _ => panic!("Expected Categorical data range"),
        }

        // Coord range should be -0.5 to (n-0.5) for bar alignment
        assert_eq!(scale.coord_range.start, -0.5);
        assert_eq!(scale.coord_range.end, 2.5); // 3 categories - 0.5
    }

    #[test]
    fn test_scale_map_continuous() {
        let scale = Scale::continuous_from_data(&vec![0.0, 10.0]);

        // For now, continuous mapping is identity (1:1)
        assert_eq!(scale.map_continuous(5.0).unwrap(), 5.0);
        assert_eq!(scale.map_continuous(10.0).unwrap(), 10.0);
        assert_eq!(scale.map_continuous(0.0).unwrap(), 0.0);
    }

    #[test]
    fn test_scale_map_continuous_on_categorical_fails() {
        let scale = Scale::categorical_from_categories(vec!["A".to_string()]);

        let result = scale.map_continuous(5.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot map continuous value to categorical scale"));
    }

    #[test]
    fn test_scale_map_categorical() {
        let categories = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let scale = Scale::categorical_from_categories(categories);

        assert_eq!(scale.map_categorical("A").unwrap(), 0.0);
        assert_eq!(scale.map_categorical("B").unwrap(), 1.0);
        assert_eq!(scale.map_categorical("C").unwrap(), 2.0);
    }

    #[test]
    fn test_scale_map_categorical_not_found() {
        let scale = Scale::categorical_from_categories(vec!["A".to_string(), "B".to_string()]);

        let result = scale.map_categorical("Z");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Category 'Z' not found"));
    }

    #[test]
    fn test_scale_map_categorical_on_continuous_fails() {
        let scale = Scale::continuous_from_data(&vec![1.0, 2.0]);

        let result = scale.map_categorical("A");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot map categorical value to continuous scale"));
    }

    #[test]
    fn test_scale_get_axis_labels_categorical() {
        let categories = vec!["North".to_string(), "South".to_string(), "East".to_string()];
        let scale = Scale::categorical_from_categories(categories.clone());

        let labels = scale.get_axis_labels();
        assert!(labels.is_some());
        assert_eq!(labels.unwrap(), categories);
    }

    #[test]
    fn test_scale_get_axis_labels_continuous() {
        let scale = Scale::continuous_from_data(&vec![1.0, 2.0, 3.0]);

        let labels = scale.get_axis_labels();
        assert!(labels.is_none()); // Continuous uses default numeric formatting
    }

    // =============================================================================
    // End of Scale Infrastructure Tests
    // =============================================================================

    // resolve_aesthetics tests (5 tests)

    #[test]
    fn test_resolve_aesthetics_from_global() {
        let layer = Layer::Line(LineLayer::default());
        let global_aes = Some(Aesthetics {
            x: "time".to_string(),
            y: "temp".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
        });
        let (x, y) = resolve_aesthetics(&layer, &global_aes).unwrap();
        assert_eq!(x, "time");
        assert_eq!(y, "temp");
    }

    #[test]
    fn test_resolve_aesthetics_layer_override_y() {
        let layer = Layer::Line(LineLayer {
            x: None,
            y: Some("humidity".to_string()),
            color: None,
            width: None,
            alpha: None,
        });
        let global_aes = Some(Aesthetics {
            x: "time".to_string(),
            y: "temp".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
        });
        let (x, y) = resolve_aesthetics(&layer, &global_aes).unwrap();
        assert_eq!(x, "time");
        assert_eq!(y, "humidity");
    }

    #[test]
    fn test_resolve_aesthetics_layer_override_both() {
        let layer = Layer::Point(PointLayer {
            x: Some("date".to_string()),
            y: Some("value".to_string()),
            color: None,
            size: None,
            shape: None,
            alpha: None,
        });
        let global_aes = Some(Aesthetics {
            x: "time".to_string(),
            y: "temp".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
        });
        let (x, y) = resolve_aesthetics(&layer, &global_aes).unwrap();
        assert_eq!(x, "date");
        assert_eq!(y, "value");
    }

    #[test]
    fn test_resolve_aesthetics_no_global_no_layer() {
        let layer = Layer::Line(LineLayer::default());
        let result = resolve_aesthetics(&layer, &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No x aesthetic"));
    }

    #[test]
    fn test_resolve_aesthetics_missing_y() {
        let layer = Layer::Bar(BarLayer {
            x: Some("category".to_string()),
            y: None,
            color: None,
            alpha: None,
            width: None,
            position: BarPosition::Identity,
        });
        let result = resolve_aesthetics(&layer, &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No y aesthetic"));
    }

    // extract_categorical_data tests (4 tests)

    #[test]
    fn test_extract_categorical_data_basic() {
        let csv = make_csv_data(
            vec!["category", "value"],
            vec![vec!["A", "10"], vec!["B", "20"], vec!["C", "30"]],
        );
        let (categories, values) = extract_categorical_data(&csv, "category", "value").unwrap();
        assert_eq!(categories, vec!["A", "B", "C"]);
        assert_eq!(values, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_extract_categorical_data_aggregation() {
        // Multiple rows with same category should sum
        let csv = make_csv_data(
            vec!["category", "value"],
            vec![vec!["A", "10"], vec!["B", "20"], vec!["A", "15"]],
        );
        let (categories, values) = extract_categorical_data(&csv, "category", "value").unwrap();
        assert_eq!(categories, vec!["A", "B"]);
        assert_eq!(values, vec![25.0, 20.0]);
    }

    #[test]
    fn test_extract_categorical_data_column_not_found() {
        let csv = make_csv_data(vec!["a", "b"], vec![vec!["1", "2"]]);
        let result = extract_categorical_data(&csv, "nonexistent", "b");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_extract_categorical_data_non_numeric_y() {
        let csv = make_csv_data(
            vec!["category", "value"],
            vec![vec!["A", "not_a_number"]],
        );
        let result = extract_categorical_data(&csv, "category", "value");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
    }

    // Style conversion tests (4 tests)

    #[test]
    fn test_line_layer_to_style_defaults() {
        let layer = LineLayer::default();
        let style = line_layer_to_style(&layer);
        assert_eq!(style.color, None);
        assert_eq!(style.width, None);
        assert_eq!(style.alpha, None);
    }

    #[test]
    fn test_line_layer_to_style_full() {
        let layer = LineLayer {
            x: None,
            y: None,
            color: Some(AestheticValue::Fixed("red".to_string())),
            width: Some(AestheticValue::Fixed(2.5)),
            alpha: Some(AestheticValue::Fixed(0.8)),
        };
        let style = line_layer_to_style(&layer);
        assert_eq!(style.color, Some("red".to_string()));
        assert_eq!(style.width, Some(2.5));
        assert_eq!(style.alpha, Some(0.8));
    }

    #[test]
    fn test_point_layer_to_style_defaults() {
        let layer = PointLayer::default();
        let style = point_layer_to_style(&layer);
        assert_eq!(style.color, None);
        assert_eq!(style.size, None);
        assert_eq!(style.alpha, None);
    }

    #[test]
    fn test_bar_layer_to_style_defaults() {
        let layer = BarLayer::default();
        let style = bar_layer_to_style(&layer);
        assert_eq!(style.color, None);
        assert_eq!(style.alpha, None);
        assert_eq!(style.width, None);
    }

    // render_continuous_plot tests (4 tests)

    #[test]
    fn test_render_continuous_plot_line() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: "y".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![Layer::Line(LineLayer::default())],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(vec!["x", "y"], vec![vec!["1", "10"], vec!["2", "20"]]);
        let result = render_continuous_plot(spec, csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_continuous_plot_point() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: "y".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![Layer::Point(PointLayer::default())],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(vec!["x", "y"], vec![vec!["1", "10"]]);
        let result = render_continuous_plot(spec, csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_continuous_plot_line_and_point() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: "y".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![
                Layer::Line(LineLayer::default()),
                Layer::Point(PointLayer::default()),
            ],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(vec!["x", "y"], vec![vec!["1", "10"], vec!["2", "20"]]);
        let result = render_continuous_plot(spec, csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_continuous_plot_column_not_found() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "nonexistent".to_string(),
                y: "y".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![Layer::Line(LineLayer::default())],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(vec!["x", "y"], vec![vec!["1", "10"]]);
        let result = render_continuous_plot(spec, csv);
        assert!(result.is_err());
    }

    // render_bar_plot tests (4 tests)

    #[test]
    fn test_render_bar_plot_single() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "category".to_string(),
                y: "value".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![Layer::Bar(BarLayer::default())],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(
            vec!["category", "value"],
            vec![vec!["A", "10"], vec!["B", "20"]],
        );
        let result = render_plot(spec, csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_bar_plot_dodge() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "category".to_string(),
                y: "v1".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![
                Layer::Bar(BarLayer {
                    x: None,
                    y: None,
                    color: Some(AestheticValue::Fixed("blue".to_string())),
                    alpha: None,
                    width: None,
                    position: BarPosition::Dodge,
                }),
                Layer::Bar(BarLayer {
                    x: None,
                    y: Some("v2".to_string()),
                    color: Some(AestheticValue::Fixed("red".to_string())),
                    alpha: None,
                    width: None,
                    position: BarPosition::Dodge,
                }),
            ],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(
            vec!["category", "v1", "v2"],
            vec![vec!["A", "10", "15"], vec!["B", "20", "25"]],
        );
        let result = render_plot(spec, csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_bar_plot_stack() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "category".to_string(),
                y: "v1".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![
                Layer::Bar(BarLayer {
                    x: None,
                    y: None,
                    color: None,
                    alpha: None,
                    width: None,
                    position: BarPosition::Stack,
                }),
                Layer::Bar(BarLayer {
                    x: None,
                    y: Some("v2".to_string()),
                    color: None,
                    alpha: None,
                    width: None,
                    position: BarPosition::Stack,
                }),
            ],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(
            vec!["category", "v1", "v2"],
            vec![vec!["A", "10", "15"]],
        );
        let result = render_plot(spec, csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_bar_plot_identity() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "category".to_string(),
                y: "value".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![
                Layer::Bar(BarLayer::default()),
                Layer::Bar(BarLayer {
                    x: None,
                    y: Some("v2".to_string()),
                    color: None,
                    alpha: None,
                    width: None,
                    position: BarPosition::Identity,
                }),
            ],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(
            vec!["category", "value", "v2"],
            vec![vec!["A", "10", "15"]],
        );
        let result = render_plot(spec, csv);
        assert!(result.is_ok());
    }

    // render_plot tests (4 tests)

    #[test]
    fn test_render_plot_no_layers() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: "y".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(vec!["x", "y"], vec![vec!["1", "10"]]);
        let result = render_plot(spec, csv);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one"));
    }

    #[test]
    fn test_render_plot_line_success() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: "y".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![Layer::Line(LineLayer::default())],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(vec!["x", "y"], vec![vec!["1", "10"], vec!["2", "20"]]);
        let result = render_plot(spec, csv);
        assert!(result.is_ok());
        // Check it's a PNG
        let png_bytes = result.unwrap();
        assert!(png_bytes.len() > 8);
        assert_eq!(&png_bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn test_render_plot_bar_success() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "cat".to_string(),
                y: "val".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![Layer::Bar(BarLayer::default())],
            labels: None,
            facet: None,
        };
        let csv = make_csv_data(vec!["cat", "val"], vec![vec!["A", "10"], vec!["B", "20"]]);
        let result = render_plot(spec, csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_plot_mixed_bar_line_success() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: "y".to_string(),
                color: None,
                size: None,
                shape: None,
                alpha: None,
            }),
            layers: vec![
                Layer::Bar(BarLayer::default()),
                Layer::Line(LineLayer::default()),
            ],
            labels: None,
            facet: None,
        };
        // Use string data for x to force categorical mode
        let csv = make_csv_data(vec!["x", "y"], vec![vec!["A", "10"]]);
        let result = render_plot(spec, csv);
        assert!(result.is_ok());
    }
}
