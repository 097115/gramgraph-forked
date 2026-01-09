use anyhow::Result;
use crate::ir::{RenderData, ScaleSystem, ResolvedSpec, SceneGraph, PanelScene, DrawCommand, RenderStyle};
use crate::parser::ast::{Layer, BarPosition};

/// Compile data and scales into a SceneGraph of drawing commands
pub fn compile_geometry(
    data: RenderData, 
    scales: ScaleSystem, 
    spec: &ResolvedSpec
) -> Result<SceneGraph> {
    let mut panels = Vec::new();
    let is_flipped = matches!(spec.coord, Some(crate::parser::ast::CoordSystem::Flip));

    // Iterate panels (zipped with scales)
    for (panel_data, panel_scales) in data.panels.into_iter().zip(scales.panels.into_iter()) {
        let mut commands = Vec::new();

        // Iterate layers
        for (layer_idx, layer_data) in panel_data.layers.into_iter().enumerate() {
            // Retrieve original layer spec for metadata (position, etc.)
            // Note: RenderData.layers aligns 1:1 with ResolvedSpec.layers
            let layer_spec = &spec.layers[layer_idx];
            
            // Handle Bar Positioning Logic
            // If Dodge, we need to know the total number of groups to calculate offsets
            let (_is_bar, position) = match &layer_spec.original_layer {
                Layer::Bar(b) => (true, b.position.clone()),
                _ => (false, BarPosition::Identity),
            };

            let num_groups = layer_data.groups.len();

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
                        // Bar Compilation
                        let bar_width_ratio = style.width.unwrap_or(0.8);
                        
                        // Calculate Dodge parameters
                        let (slot_width, x_offset_base) = if matches!(position, BarPosition::Dodge) {
                            let slot = bar_width_ratio / num_groups as f64;
                            let base = (group_idx as f64 - (num_groups as f64 - 1.0) / 2.0) * slot;
                            (slot, base)
                        } else {
                            (bar_width_ratio, 0.0)
                        };

                        // Generate Rects
                        for i in 0..group.x.len() {
                            let x_center = group.x[i];
                            let y_top = group.y[i];
                            let y_bottom = group.y_start[i]; // From transform (0.0 or stack base)
                            
                            // Apply Dodge to X
                            let x_final = x_center + x_offset_base;
                            
                            // Rect coordinates (Top-Left, Bottom-Right) in data space
                            // Note: Width is in data units. For categorical, 1 unit = 1 category.
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
                                legend: if i == 0 { Some(group.key.clone()) } else { None }, // Only legend once per group
                            });
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
        width: 800, // Default, can be overridden or passed in
        height: 600,
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
        let scene = compile_geometry(data, scales, &spec).unwrap();
        
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
