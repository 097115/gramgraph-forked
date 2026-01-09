use anyhow::{anyhow, Context, Result};
use std::collections::{HashMap, HashSet};
use crate::data::PlotData;
use crate::ir::{RenderData, PanelData, LayerData, GroupData, FacetLayout, RenderStyle};
use crate::ir::{ResolvedSpec, ResolvedLayer, ResolvedAesthetics, ResolvedFacet};
use crate::parser::ast::{Layer, BarPosition, Stat};
use crate::graph::{LineStyle, PointStyle, BarStyle, RibbonStyle};
use crate::palette::{ColorPalette, SizePalette, ShapePalette};

/// Main entry point: Transform resolved spec and CSV data into renderable data
pub fn apply_transformations(spec: &ResolvedSpec, data: &PlotData) -> Result<RenderData> {
    // 1. Partition Data (Faceting)
    let partitions = partition_data(spec, data)?;
    
    // 2. Calculate Layout info
    let (nrow, ncol) = calculate_grid_dimensions(partitions.len(), spec.facet.as_ref());
    let facet_layout = FacetLayout {
        nrow,
        ncol,
        panel_titles: partitions.iter().map(|p| p.title.clone()).collect(),
    };

    // 3. Process each partition into a Panel
    let mut panels = Vec::new();
    for (idx, partition) in partitions.into_iter().enumerate() {
        let panel = process_partition(idx, partition, spec)?;
        panels.push(panel);
    }

    Ok(RenderData {
        panels,
        facet_layout,
    })
}

struct DataPartition {
    title: String,
    data: PlotData,
}

/// Split CSV data based on facet configuration
fn partition_data(spec: &ResolvedSpec, data: &PlotData) -> Result<Vec<DataPartition>> {
    if let Some(facet) = &spec.facet {
        // Find facet column index
        let col_idx = data.headers.iter()
            .position(|h| h.eq_ignore_ascii_case(&facet.col))
            .ok_or_else(|| anyhow!("Facet column '{}' not found", facet.col))?;

        // Group rows
        let mut groups: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for row in &data.rows {
            if let Some(val) = row.get(col_idx) {
                groups.entry(val.clone()).or_default().push(row.clone());
            }
        }

        // Sort keys
        let mut keys: Vec<String> = groups.keys().cloned().collect();
        keys.sort();

        let mut partitions = Vec::new();
        for key in keys {
            let rows = groups.remove(&key).unwrap();
            partitions.push(DataPartition {
                title: key,
                data: PlotData {
                    headers: data.headers.clone(),
                    rows,
                },
            });
        }
        Ok(partitions)
    } else {
        // No facet, single partition
        Ok(vec![DataPartition {
            title: "".to_string(),
            data: data.clone(), // Clone is expensive but safe for now
        }])
    }
}

fn calculate_grid_dimensions(n_panels: usize, facet: Option<&ResolvedFacet>) -> (usize, usize) {
    if let Some(f) = facet {
        if let Some(cols) = f.ncol {
            let rows = (n_panels as f64 / cols as f64).ceil() as usize;
            return (rows, cols);
        }
    }
    // Default: square-ish
    let cols = (n_panels as f64).sqrt().ceil() as usize;
    let rows = (n_panels as f64 / cols as f64).ceil() as usize;
    (rows, cols)
}

/// Process a single data partition (Panel)
fn process_partition(index: usize, partition: DataPartition, spec: &ResolvedSpec) -> Result<PanelData> {
    let mut layers = Vec::new();

    for layer_spec in &spec.layers {
        let layer_data = process_layer(layer_spec, &partition.data)?;
        layers.push(layer_data);
    }

    Ok(PanelData {
        index,
        layers,
    })
}

/// Process a single layer: Extract, Group, Stack
fn process_layer(layer_spec: &ResolvedLayer, data: &PlotData) -> Result<LayerData> {
    let aes = &layer_spec.aesthetics;
    
    // 1. Identify Grouping Column
    let group_col = aes.color.as_ref()
        .or(aes.size.as_ref())
        .or(aes.shape.as_ref())
        .or(aes.alpha.as_ref());

    // 2. Extract Data (Grouped)
    // We return a map: GroupKey -> (RawX, RawY, RawYMin, RawYMax)
    // RawX is String to handle both numeric and categorical initially
    let mut raw_groups: HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)> = HashMap::new();
    
    // Column Indices
    let x_idx = find_col_index(&data.headers, &aes.x_col)?;
    let y_idx = if let Some(y) = &aes.y_col { Some(find_col_index(&data.headers, y)?) } else { None };
    let ymin_idx = if let Some(col) = &aes.ymin_col { Some(find_col_index(&data.headers, col)?) } else { None };
    let ymax_idx = if let Some(col) = &aes.ymax_col { Some(find_col_index(&data.headers, col)?) } else { None };

    let group_idx = if let Some(g) = group_col {
        Some(find_col_index(&data.headers, g)?)
    } else {
        None
    };

    for row in &data.rows {
        let x_str = row[x_idx].clone();
        let y_val = if let Some(idx) = y_idx { 
            row[idx].parse::<f64>().context(format!("Failed to parse Y value '{}'", row[idx]))?
        } else { 
            0.0 // Default for histogram if not provided
        };
        let ymin_val = if let Some(idx) = ymin_idx { row[idx].parse::<f64>().unwrap_or(0.0) } else { 0.0 };
        let ymax_val = if let Some(idx) = ymax_idx { row[idx].parse::<f64>().unwrap_or(0.0) } else { 0.0 };
        
        let group_key = if let Some(idx) = group_idx {
            row[idx].clone()
        } else {
            "default".to_string()
        };

        let entry = raw_groups.entry(group_key).or_insert_with(|| (Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        entry.0.push(x_str);
        entry.1.push(y_val);
        entry.2.push(ymin_val);
        entry.3.push(ymax_val);
    }

    // Apply Statistics
    let raw_groups = apply_statistics(raw_groups, layer_spec.original_layer.stat())?;

    // 3. Determine X-Axis Type (Numeric vs Categorical)
    // Logic: If ALL x values in this layer can be parsed as float, it's numeric.
    // UNLESS it's a Bar chart, which forces categorical.
    let is_bar = matches!(layer_spec.original_layer, Layer::Bar(_));
    let all_x_strings: Vec<&String> = raw_groups.values().flat_map(|(x, _, _, _)| x.iter()).collect();
    let all_numeric = all_x_strings.iter().all(|s| s.parse::<f64>().is_ok());
    
    let use_categorical = is_bar || !all_numeric;

    // 4. Normalize X Values
    // If categorical, we need a unified mapping for stacking/grouping
    let mut x_category_map = HashMap::new();
    let mut category_order = Vec::new();
    
    if use_categorical {
        // Collect all unique categories to assign indices
        let mut unique_cats: HashSet<String> = HashSet::new();
        // Preserve order of appearance if possible, or sort? 
        // GoG usually sorts unless factor provided. Let's sort for determinism.
        for s in &all_x_strings {
            unique_cats.insert((*s).clone());
        }
        category_order = unique_cats.into_iter().collect();
        
        // Try to sort numerically if possible
        let all_numeric_cats = category_order.iter().all(|s| s.parse::<f64>().is_ok());
        if all_numeric_cats {
            category_order.sort_by(|a, b| {
                let fa = a.parse::<f64>().unwrap();
                let fb = b.parse::<f64>().unwrap();
                fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            category_order.sort();
        }
        
        for (i, cat) in category_order.iter().enumerate() {
            x_category_map.insert(cat.clone(), i as f64);
        }
    }

    // 5. Build Groups (Styles & Coordinates)
    let mut groups = Vec::new();
    let sorted_group_keys = get_sorted_keys(&raw_groups);
    
    // Assign Palettes
    let color_map = ColorPalette::category10().assign_colors(&sorted_group_keys);
    let size_map = SizePalette::default_range().assign_sizes(&sorted_group_keys);
    let shape_map = ShapePalette::default_shapes().assign_shapes(&sorted_group_keys);

    // Prepare for Stacking (if needed)
    let mut stack_offsets: HashMap<String, f64> = HashMap::new(); // Map "X_Key" -> Current Height
    let is_stacked = match &layer_spec.original_layer {
        Layer::Bar(b) => matches!(b.position, BarPosition::Stack),
        _ => false,
    };

    // Iterate groups in defined order (important for stacking order)
    for key in sorted_group_keys {
        let (raw_x, raw_y, raw_ymin, raw_ymax) = raw_groups.get(&key).unwrap();
        
        let mut x_floats = Vec::with_capacity(raw_x.len());
        let mut y_starts = Vec::with_capacity(raw_x.len());
        let mut y_ends = Vec::with_capacity(raw_x.len());
        let mut y_mins = Vec::with_capacity(raw_x.len());
        let mut y_maxs = Vec::with_capacity(raw_x.len());

        for (i, x_s) in raw_x.iter().enumerate() {
            let y_val = raw_y[i];
            let raw_min = raw_ymin[i];
            let raw_max = raw_ymax[i];
            
            // Resolve X
            let x_val = if use_categorical {
                *x_category_map.get(x_s).unwrap() // Should exist
            } else {
                x_s.parse::<f64>().unwrap() // Verified numeric earlier
            };
            x_floats.push(x_val);

            // Resolve Y (Stacking and Min/Max)
            let stack_key = if use_categorical { x_s.clone() } else { x_val.to_string() };
            
            let (y_start, y_end, y_min, y_max) = if is_stacked {
                let start = *stack_offsets.get(&stack_key).unwrap_or(&0.0);
                let end = start + y_val;
                stack_offsets.insert(stack_key, end);
                (start, end, start, end)
            } else if matches!(layer_spec.original_layer, Layer::Ribbon(_)) {
                // Ribbon uses raw ymin/ymax
                (raw_min, raw_max, raw_min, raw_max)
            } else {
                // Line/Point/Bar(unstacked)
                (0.0, y_val, 0.0, y_val)
            };
            
            y_starts.push(y_start);
            y_ends.push(y_end);
            y_mins.push(y_min);
            y_maxs.push(y_max);
        }

        // Build Style
        let style = build_style(key.clone(), &layer_spec.original_layer, aes, &color_map, &size_map, &shape_map);

        groups.push(GroupData {
            key: key.clone(),
            x: x_floats,
            y: y_ends, // Main value
            y_start: y_starts,
            y_min: y_mins,
            y_max: y_maxs,
            x_categories: if use_categorical { Some(category_order.clone()) } else { None },
            style,
        });
    }

    Ok(LayerData { groups })
}
fn find_col_index(headers: &[String], name: &str) -> Result<usize> {
    headers.iter()
        .position(|h| h.eq_ignore_ascii_case(name))
        .ok_or_else(|| anyhow!("Column '{}' not found", name))
}

fn get_sorted_keys<V>(map: &HashMap<String, V>) -> Vec<String> {
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort();
    keys
}

fn build_style(
    group_key: String,
    layer: &Layer,
    aes: &ResolvedAesthetics,
    color_map: &HashMap<String, String>,
    size_map: &HashMap<String, f64>,
    shape_map: &HashMap<String, String>,
) -> RenderStyle {
    // Helper to pick color: GroupMapped ?? Fixed ?? Default
    let pick_color = |l_color: &Option<crate::parser::ast::AestheticValue<String>>| -> Option<String> {
        if aes.color.is_some() && color_map.contains_key(&group_key) {
             color_map.get(&group_key).cloned()
        } else {
            // Check fixed
            match l_color {
                Some(crate::parser::ast::AestheticValue::Fixed(c)) => Some(c.clone()),
                _ => None,
            }
        }
    };
    
    // Helper to pick size/width
    let pick_size = |l_val: &Option<crate::parser::ast::AestheticValue<f64>>| -> Option<f64> {
         if aes.size.is_some() && size_map.contains_key(&group_key) {
             size_map.get(&group_key).copied()
         } else {
             match l_val {
                 Some(crate::parser::ast::AestheticValue::Fixed(v)) => Some(*v),
                 _ => None,
             }
         }
    };

    // Helper to pick alpha
    let pick_alpha = |l_val: &Option<crate::parser::ast::AestheticValue<f64>>| -> Option<f64> {
        // Alpha mapping not fully implemented in palettes yet, usually fixed
        // If we added alpha palette, we would check aes.alpha.is_some() here
         match l_val {
             Some(crate::parser::ast::AestheticValue::Fixed(v)) => Some(*v),
             _ => None,
         }
    };

    match layer {
        Layer::Line(l) => RenderStyle::Line(LineStyle {
            color: pick_color(&l.color),
            width: pick_size(&l.width),
            alpha: pick_alpha(&l.alpha),
        }),
        Layer::Point(p) => RenderStyle::Point(PointStyle {
            color: pick_color(&p.color),
            size: pick_size(&p.size),
            shape: if aes.shape.is_some() && shape_map.contains_key(&group_key) {
                shape_map.get(&group_key).cloned()
            } else {
                match &p.shape {
                    Some(crate::parser::ast::AestheticValue::Fixed(s)) => Some(s.clone()),
                    _ => None,
                }
            },
            alpha: pick_alpha(&p.alpha),
        }),
        Layer::Bar(b) => RenderStyle::Bar(BarStyle {
            color: pick_color(&b.color),
            width: pick_size(&b.width),
            alpha: pick_alpha(&b.alpha),
        }),
        Layer::Ribbon(r) => RenderStyle::Ribbon(RibbonStyle {
            color: pick_color(&r.color),
            alpha: pick_alpha(&r.alpha),
        }),
    }
}

fn apply_statistics(
    groups: HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)>, 
    stat: &Stat
) -> Result<HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)>> {
    match stat {
        Stat::Identity => Ok(groups),
        Stat::Bin { bins } => compute_bin_stat(groups, *bins),
        Stat::Count => compute_count_stat(groups),
        Stat::Smooth { method } => compute_smooth_stat(groups, method),
    }
}

fn compute_count_stat(
    groups: HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)>
) -> Result<HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)>> {
    let mut new_groups = HashMap::new();
    
    for (key, (x_strs, _, _, _)) in groups {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for s in x_strs {
            *counts.entry(s).or_default() += 1;
        }
        
        let mut keys: Vec<String> = counts.keys().cloned().collect();
        keys.sort();
        
        let mut new_x = Vec::new();
        let mut new_y = Vec::new();
        let mut new_ymin = Vec::new();
        let mut new_ymax = Vec::new();
        
        for k in keys {
            let count = counts[&k] as f64;
            new_x.push(k);
            new_y.push(count);
            new_ymin.push(0.0);
            new_ymax.push(count);
        }
        
        new_groups.insert(key, (new_x, new_y, new_ymin, new_ymax));
    }
    
    Ok(new_groups)
}

fn compute_smooth_stat(
    groups: HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)>,
    _method: &str
) -> Result<HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)>> {
    let mut new_groups = HashMap::new();
    
    for (key, (x_strs, y_vals, _, _)) in groups {
        // Simple Linear Regression
        let mut x_floats = Vec::new();
        for s in &x_strs {
            x_floats.push(s.parse::<f64>().map_err(|_| anyhow!("Stat 'smooth' requires numeric x data"))?);
        }
        
        if x_floats.len() < 2 { continue; }
        
        let n = x_floats.len() as f64;
        let sum_x: f64 = x_floats.iter().sum();
        let sum_y: f64 = y_vals.iter().sum();
        let sum_xx: f64 = x_floats.iter().map(|&x| x * x).sum();
        let sum_xy: f64 = x_floats.iter().zip(y_vals.iter()).map(|(&x, &y)| x * y).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;
        
        // Generate trend line points (min and max X)
        let min_x = x_floats.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_x = x_floats.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        let new_x = vec![min_x.to_string(), max_x.to_string()];
        let new_y = vec![slope * min_x + intercept, slope * max_x + intercept];
        
        new_groups.insert(key, (new_x, new_y.clone(), new_y.clone(), new_y));
    }
    
    Ok(new_groups)
}

fn compute_bin_stat(
    groups: HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)>,
    bin_count: usize
) -> Result<HashMap<String, (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>)>> {
    // 1. Collect all X values to determine range
    let mut all_values = Vec::new();
    for (x_strs, _, _, _) in groups.values() {
        for s in x_strs {
            let v = s.parse::<f64>().map_err(|_| anyhow!("Stat 'bin' requires numeric x data"))?;
            all_values.push(v);
        }
    }
    
    if all_values.is_empty() { return Ok(groups); }

    let min = all_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = all_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    
    // Add small buffer or handle 0 range
    let range = max - min;
    let width = if range == 0.0 { 1.0 } else { range / bin_count as f64 };
    
    let mut new_groups = HashMap::new();
    
    for (key, (x_strs, _, _, _)) in groups {
        let mut bins: HashMap<isize, usize> = HashMap::new();
        
        for s in x_strs {
             // We already checked they are numeric
             let v = s.parse::<f64>().unwrap();
             let bin_idx = ((v - min) / width).floor() as isize;
             *bins.entry(bin_idx).or_default() += 1;
        }
        
        // Convert bins back to (X, Y)
        let mut bin_indices: Vec<isize> = bins.keys().cloned().collect();
        bin_indices.sort();
        
        let mut new_x = Vec::new();
        let mut new_y = Vec::new();
        let mut new_ymin = Vec::new();
        let mut new_ymax = Vec::new();
        
        for idx in bin_indices {
            let center = min + (idx as f64 * width) + (width / 2.0);
            let count = bins[&idx] as f64;
            new_x.push(format!("{:.2}", center));
            new_y.push(count);
            new_ymin.push(0.0);
            new_ymax.push(count);
        }
        
        new_groups.insert(key, (new_x, new_y, new_ymin, new_ymax));
    }
    
    Ok(new_groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Layer, LineLayer};

    fn make_data() -> PlotData {
        PlotData {
            headers: vec!["x".to_string(), "y".to_string(), "cat".to_string()],
            rows: vec![
                vec!["1.0".to_string(), "10.0".to_string(), "A".to_string()],
                vec!["2.0".to_string(), "20.0".to_string(), "A".to_string()],
                vec!["1.0".to_string(), "15.0".to_string(), "B".to_string()],
            ],
        }
    }

    fn make_spec() -> ResolvedSpec {
        ResolvedSpec {
            layers: vec![ResolvedLayer {
                original_layer: Layer::Line(LineLayer::default()),
                aesthetics: ResolvedAesthetics {
                    x_col: "x".to_string(),
                    y_col: Some("y".to_string()),
                    ymin_col: None,
                    ymax_col: None,
                    color: Some("cat".to_string()),
                    size: None,
                    shape: None,
                    alpha: None,
                },
            }],
            facet: None,
            coord: None,
            labels: crate::parser::ast::Labels::default(),
            theme: crate::parser::ast::Theme::default(),
            x_scale_spec: None,
            y_scale_spec: None,
        }
    }

    #[test]
    fn test_transform_grouping() {
        let csv = make_data();
        let spec = make_spec();
        let render_data = apply_transformations(&spec, &csv).unwrap();
        
        assert_eq!(render_data.panels.len(), 1);
        let panel = &render_data.panels[0];
        assert_eq!(panel.layers.len(), 1);
        let layer = &panel.layers[0];
        assert_eq!(layer.groups.len(), 2); // A and B

        // Check group A
        let group_a = layer.groups.iter().find(|g| g.key == "A").unwrap();
        assert_eq!(group_a.x.len(), 2);
        assert_eq!(group_a.y, vec![10.0, 20.0]);
    }

    #[test]
    fn test_transform_facet() {
        let mut spec = make_spec();
        spec.facet = Some(ResolvedFacet {
            col: "cat".to_string(),
            ncol: None,
            scales: crate::parser::ast::FacetScales::Fixed,
        });
        
        let csv = make_data();
        let render_data = apply_transformations(&spec, &csv).unwrap();
        
        assert_eq!(render_data.panels.len(), 2); // A and B panels
        assert_eq!(render_data.facet_layout.panel_titles.len(), 2);
        assert!(render_data.facet_layout.panel_titles.contains(&"A".to_string()));
    }
}
