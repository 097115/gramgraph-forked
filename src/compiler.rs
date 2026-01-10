use anyhow::Result;
use crate::ir::{RenderData, ScaleSystem, ResolvedSpec, SceneGraph, PanelScene, DrawCommand, RenderStyle};
use crate::parser::ast::{Layer, BarPosition};
use crate::graph::{LineStyle, PointStyle, BarStyle, BoxplotStyle};
use crate::RenderOptions;

use std::collections::HashMap;

// =============================================================================
// Boxplot Geometry Helpers
// =============================================================================

/// Computed geometry for a single boxplot, expressed as primitive shapes
struct BoxplotGeometry {
    lower_whisker: Vec<(f64, f64)>,
    upper_whisker: Vec<(f64, f64)>,
    min_cap: Vec<(f64, f64)>,
    max_cap: Vec<(f64, f64)>,
    box_tl: (f64, f64),
    box_br: (f64, f64),
    median_line: Vec<(f64, f64)>,
    outlier_points: Vec<(f64, f64)>,
}

/// Calculates boxplot primitive geometry for a single boxplot
fn compute_boxplot_geometry(
    x: f64,
    width: f64,
    min: f64,
    q1: f64,
    median: f64,
    q3: f64,
    max: f64,
    outliers: &[f64],
    is_vertical: bool,
) -> BoxplotGeometry {
    let half_width = width / 2.0;
    let cap_width = width * 0.4;
    let cap_half = cap_width / 2.0;

    if is_vertical {
        BoxplotGeometry {
            lower_whisker: vec![(x, min), (x, q1)],
            upper_whisker: vec![(x, q3), (x, max)],
            min_cap: vec![(x - cap_half, min), (x + cap_half, min)],
            max_cap: vec![(x - cap_half, max), (x + cap_half, max)],
            box_tl: (x - half_width, q3),
            box_br: (x + half_width, q1),
            median_line: vec![(x - half_width, median), (x + half_width, median)],
            outlier_points: outliers.iter().map(|&v| (x, v)).collect(),
        }
    } else {
        // Horizontal orientation (coord_flip)
        BoxplotGeometry {
            lower_whisker: vec![(min, x), (q1, x)],
            upper_whisker: vec![(q3, x), (max, x)],
            min_cap: vec![(min, x - cap_half), (min, x + cap_half)],
            max_cap: vec![(max, x - cap_half), (max, x + cap_half)],
            box_tl: (q1, x - half_width),
            box_br: (q3, x + half_width),
            median_line: vec![(median, x - half_width), (median, x + half_width)],
            outlier_points: outliers.iter().map(|&v| (v, x)).collect(),
        }
    }
}

/// Converts BoxplotStyle into component styles for boxplot primitives
fn boxplot_component_styles(style: &BoxplotStyle) -> (LineStyle, BarStyle, LineStyle, PointStyle) {
    // Whisker lines - use main color
    let whisker_style = LineStyle {
        color: style.color.clone(),
        width: Some(2.0),
        alpha: style.alpha,
    };

    // Box fill
    let box_style = BarStyle {
        color: style.color.clone(),
        alpha: style.alpha,
        width: None,
    };

    // Median line - white for contrast
    let median_style = LineStyle {
        color: Some("white".to_string()),
        width: Some(2.0),
        alpha: Some(0.9),
    };

    // Outliers - use outlier-specific style or fallback to main color
    let outlier_style = PointStyle {
        color: style.outlier_color.clone().or_else(|| style.color.clone()),
        size: style.outlier_size,
        shape: style.outlier_shape.clone(),
        alpha: style.alpha,
    };

    (whisker_style, box_style, median_style, outlier_style)
}

/// Compile data and scales into a SceneGraph of drawing commands
pub fn compile_geometry(
    data: RenderData, 
    scales: ScaleSystem, 
    spec: &ResolvedSpec,
    options: &RenderOptions,
) -> Result<SceneGraph> {
    let mut panels = Vec::new();
    let is_flipped = matches!(spec.coord, Some(crate::parser::ast::CoordSystem::Flip));

    // Iterate panels (zipped with scales)
    for (panel_data, panel_scales) in data.panels.into_iter().zip(scales.panels.into_iter()) {
        let mut commands = Vec::new();

        // Iterate layers
        for (layer_idx, layer_data) in panel_data.layers.into_iter().enumerate() {
            // Retrieve original layer spec for metadata (position, etc.)
            let layer_spec = &spec.layers[layer_idx];
            
            // Handle Positioning Logic
            let (_is_bar, position) = match &layer_spec.original_layer {
                Layer::Bar(b) => (true, b.position.clone()),
                Layer::Boxplot(_) => (true, BarPosition::Dodge),
                _ => (false, BarPosition::Identity),
            };

            // Smart Dodging: Calculate occupancy per X coordinate
            // Map: Quantized X -> List of Group Indices present at that X
            let mut x_occupancy: HashMap<i64, Vec<usize>> = HashMap::new();
            
            if matches!(position, BarPosition::Dodge) {
                for (g_idx, group) in layer_data.groups.iter().enumerate() {
                    for &x in &group.x {
                        // Quantize X to integer for categorical grouping logic
                        // (Use round() to handle float imprecision)
                        let key = x.round() as i64; 
                        x_occupancy.entry(key).or_default().push(g_idx);
                    }
                }
                // Sort groups at each X to ensure deterministic order (usually sorted by group key anyway)
                for groups_at_x in x_occupancy.values_mut() {
                    groups_at_x.sort(); 
                    groups_at_x.dedup(); // Handle multiple points per group at same X (if any)
                }
            }

            for (group_idx, group) in layer_data.groups.into_iter().enumerate() {
                match &group.style {
                    RenderStyle::Line(style) => {
                        let points: Vec<(f64, f64)> = group.x.iter().zip(group.y.iter())
                            .map(|(&x, &y)| if is_flipped { (y, x) } else { (x, y) })
                            .collect();
                        commands.push(DrawCommand::DrawLine {
                            points,
                            style: style.clone(),
                            legend: Some(group.key.clone()),
                        });
                    }
                    RenderStyle::Point(style) => {
                        let points: Vec<(f64, f64)> = group.x.iter().zip(group.y.iter())
                            .map(|(&x, &y)| if is_flipped { (y, x) } else { (x, y) })
                            .collect();
                        commands.push(DrawCommand::DrawPoint {
                            points,
                            style: style.clone(),
                            legend: Some(group.key.clone()),
                        });
                    }
                    RenderStyle::Bar(style) => {
                        let bar_width_ratio = style.width.unwrap_or(0.8);
                        
                        for i in 0..group.x.len() {
                            let x_center = group.x[i];
                            let y_top = group.y[i];
                            let y_bottom = group.y_start[i];
                            
                            // Calculate Dodge Offset for this specific point
                            let (slot_width, x_offset) = if matches!(position, BarPosition::Dodge) {
                                let key = x_center.round() as i64;
                                if let Some(occupants) = x_occupancy.get(&key) {
                                    let num_at_x = occupants.len();
                                    if let Some(rank) = occupants.iter().position(|&g| g == group_idx) {
                                        let slot = bar_width_ratio / num_at_x as f64;
                                        let offset = (rank as f64 - (num_at_x as f64 - 1.0) / 2.0) * slot;
                                        (slot, offset)
                                    } else {
                                        (bar_width_ratio, 0.0) // Should not happen
                                    }
                                } else {
                                    (bar_width_ratio, 0.0)
                                }
                            } else {
                                (bar_width_ratio, 0.0)
                            };

                            let x_final = x_center + x_offset;
                            let half_width = slot_width / 2.0;
                            
                            let tl = (x_final - half_width, y_top);
                            let br = (x_final + half_width, y_bottom);
                            
                            let (tl, br) = if is_flipped {
                                ((tl.1, tl.0), (br.1, br.0))
                            } else {
                                (tl, br)
                            };

                            commands.push(DrawCommand::DrawRect {
                                tl,
                                br,
                                style: style.clone(),
                                legend: if i == 0 { Some(group.key.clone()) } else { None },
                            });
                        }
                    }
                    RenderStyle::Boxplot(style) => {
                        let width_ratio = style.width.unwrap_or(0.5);
                        let (whisker_style, box_style, median_style, outlier_style) =
                            boxplot_component_styles(style);

                        for i in 0..group.x.len() {
                            let x_center = group.x[i];

                            // Calculate Dodge Offset for this specific point
                            let (slot_width, x_offset) = if matches!(position, BarPosition::Dodge) {
                                let key = x_center.round() as i64;
                                if let Some(occupants) = x_occupancy.get(&key) {
                                    let num_at_x = occupants.len();
                                    if let Some(rank) = occupants.iter().position(|&g| g == group_idx) {
                                        let slot = width_ratio / num_at_x as f64;
                                        let offset = (rank as f64 - (num_at_x as f64 - 1.0) / 2.0) * slot;
                                        (slot, offset)
                                    } else {
                                        (width_ratio, 0.0)
                                    }
                                } else {
                                    (width_ratio, 0.0)
                                }
                            } else {
                                (width_ratio, 0.0)
                            };

                            let x_final = x_center + x_offset;
                            let is_vertical = !is_flipped;

                            let geom = compute_boxplot_geometry(
                                x_final,
                                slot_width,
                                group.y_min[i],
                                group.y_q1[i],
                                group.y_median[i],
                                group.y_q3[i],
                                group.y_max[i],
                                &group.outliers[i],
                                is_vertical,
                            );

                            // Emit primitive commands in correct z-order

                            // 1. Whiskers (lines from min/max to box edges)
                            commands.push(DrawCommand::DrawLine {
                                points: geom.lower_whisker,
                                style: whisker_style.clone(),
                                legend: None,
                            });
                            commands.push(DrawCommand::DrawLine {
                                points: geom.upper_whisker,
                                style: whisker_style.clone(),
                                legend: None,
                            });

                            // 2. Whisker caps
                            commands.push(DrawCommand::DrawLine {
                                points: geom.min_cap,
                                style: whisker_style.clone(),
                                legend: None,
                            });
                            commands.push(DrawCommand::DrawLine {
                                points: geom.max_cap,
                                style: whisker_style.clone(),
                                legend: None,
                            });

                            // 3. Box (rectangle) - legend attached here
                            commands.push(DrawCommand::DrawRect {
                                tl: geom.box_tl,
                                br: geom.box_br,
                                style: box_style.clone(),
                                legend: if i == 0 { Some(group.key.clone()) } else { None },
                            });

                            // 4. Median line (white for contrast)
                            commands.push(DrawCommand::DrawLine {
                                points: geom.median_line,
                                style: median_style.clone(),
                                legend: None,
                            });

                            // 5. Outliers (if any)
                            if !geom.outlier_points.is_empty() {
                                commands.push(DrawCommand::DrawPoint {
                                    points: geom.outlier_points,
                                    style: outlier_style.clone(),
                                    legend: None,
                                });
                            }
                        }
                    }
                    RenderStyle::Ribbon(style) => {
                        // Construct Polygon: Trace y_max forward, then y_min backward
                        let mut points = Vec::with_capacity(group.x.len() * 2);
                        
                        // Forward pass: y_max
                        for i in 0..group.x.len() {
                            let x = group.x[i];
                            let y = group.y_max[i];
                            points.push(if is_flipped { (y, x) } else { (x, y) });
                        }
                        
                        // Backward pass: y_min
                        for i in (0..group.x.len()).rev() {
                            let x = group.x[i];
                            let y = group.y_min[i];
                            points.push(if is_flipped { (y, x) } else { (x, y) });
                        }

                        commands.push(DrawCommand::DrawPolygon {
                            points,
                            style: style.clone(),
                            legend: Some(group.key.clone()),
                        });
                    }
                }
            }
        }

        // Determine Panel Title
        let title = data.facet_layout.panel_titles.get(panel_data.index).cloned()
            .filter(|s| !s.is_empty())
            .map(|s| format!("{} = {}", spec.facet.as_ref().unwrap().col, s));

        // Determine Row/Col
        let row = panel_data.index / data.facet_layout.ncol;
        let col = panel_data.index % data.facet_layout.ncol;
        
        let (x_scale, y_scale) = if is_flipped {
            (panel_scales.y, panel_scales.x)
        } else {
            (panel_scales.x, panel_scales.y)
        };

        panels.push(PanelScene {
            row,
            col,
            title,
            x_label: spec.labels.x.clone(),
            y_label: spec.labels.y.clone(),
            x_scale,
            y_scale,
            commands,
        });
    }

    Ok(SceneGraph {
        width: options.width,
        height: options.height,
        panels,
        labels: spec.labels.clone(),
        theme: spec.theme.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{PanelData, LayerData, GroupData, FacetLayout, RenderStyle, PanelScales, Scale, ResolvedLayer, ResolvedAesthetics};
    use crate::graph::LineStyle;
    use crate::parser::ast::{Layer, LineLayer};

    fn make_test_data() -> (RenderData, ScaleSystem, ResolvedSpec) {
        let render_data = RenderData {
            panels: vec![PanelData {
                index: 0,
                layers: vec![LayerData {
                    groups: vec![GroupData {
                        key: "A".to_string(),
                        x: vec![0.0, 1.0],
                        y: vec![10.0, 20.0],
                        y_start: vec![0.0, 0.0],
                        y_min: vec![0.0, 0.0],
                        y_max: vec![10.0, 20.0],
                        y_q1: vec![],
                        y_median: vec![],
                        y_q3: vec![],
                        outliers: vec![],
                        x_categories: None,
                        style: RenderStyle::Line(LineStyle::default()),
                    }],
                }],
            }],
            facet_layout: FacetLayout { nrow: 1, ncol: 1, panel_titles: vec![] },
        };

        let scales = ScaleSystem {
            panels: vec![PanelScales {
                x: Scale { domain: (0.0, 1.0), range: (0.0, 1.0), is_categorical: false, categories: vec![] },
                y: Scale { domain: (0.0, 20.0), range: (0.0, 20.0), is_categorical: false, categories: vec![] },
            }],
        };

        let spec = ResolvedSpec {
            layers: vec![ResolvedLayer {
                original_layer: Layer::Line(LineLayer::default()),
                aesthetics: ResolvedAesthetics {
                    x_col: "x".to_string(),
                    y_col: Some("y".to_string()),
                    ymin_col: None, ymax_col: None,
                    color: None, size: None, shape: None, alpha: None
                },
            }],
            facet: None,
            coord: None,
            labels: crate::parser::ast::Labels::default(),
            theme: crate::parser::ast::Theme::default(),
            x_scale_spec: None,
            y_scale_spec: None,
        };
        
        (render_data, scales, spec)
    }

    #[test]
    fn test_compile_line() {
        let (data, scales, spec) = make_test_data();
        let options = RenderOptions::default();
        let scene = compile_geometry(data, scales, &spec, &options).unwrap();
        
        assert_eq!(scene.panels.len(), 1);
        let panel = &scene.panels[0];
        assert_eq!(panel.commands.len(), 1);
        
        if let DrawCommand::DrawLine { points, .. } = &panel.commands[0] {
            assert_eq!(points.len(), 2);
            assert_eq!(points[0], (0.0, 10.0));
        } else {
            panic!("Expected DrawLine");
        }
    }
}
