use anyhow::{Result, anyhow};
use std::collections::HashMap;
use crate::parser::ast::{PlotSpec, Layer, Aesthetics, AestheticValue, Labels,
    LineLayer, PointLayer, BarLayer, RibbonLayer, BoxplotLayer, ViolinLayer};
use crate::data::PlotData;
use crate::ir::{ResolvedSpec, ResolvedLayer, ResolvedAesthetics, ResolvedFacet};

/// Resolve all aesthetic mappings for the entire plot
pub fn resolve_plot_aesthetics(
    spec: &PlotSpec,
    _data: &PlotData,
    variables: &HashMap<String, String>,
) -> Result<ResolvedSpec> {
    // 0. Resolve variables in the global aesthetics
    let resolved_aes = if let Some(aes) = &spec.aesthetics {
        Some(resolve_aesthetics_variables(aes, variables)?)
    } else {
        None
    };

    // 1. Resolve Facet (if any)
    let facet = if let Some(f) = &spec.facet {
        Some(ResolvedFacet {
            col: resolve_string_variable(&f.by, variables)?,
            ncol: f.ncol,
            scales: f.scales.clone(),
        })
    } else {
        None
    };

    // 2. Resolve variables in layers and then resolve layer aesthetics
    let mut layers = Vec::new();
    for layer in &spec.layers {
        // First resolve variables in the layer
        let resolved_layer = resolve_layer_variables(layer, variables)?;
        // Then resolve the layer aesthetics
        let aesthetics = resolve_layer_aesthetics(&resolved_layer, &resolved_aes)?;
        layers.push(ResolvedLayer {
            original_layer: resolved_layer,
            aesthetics,
        });
    }

    // 3. Resolve labels
    let labels = resolve_labels_variables(&spec.labels.clone().unwrap_or_default(), variables)?;

    Ok(ResolvedSpec {
        layers,
        facet,
        coord: spec.coord.clone(),
        labels,
        theme: spec.theme.clone().unwrap_or_default(),
        x_scale_spec: spec.x_scale.clone(),
        y_scale_spec: spec.y_scale.clone(),
    })
}

/// Helper to resolve a string that might be a variable reference ($name)
fn resolve_string_variable(s: &str, variables: &HashMap<String, String>) -> Result<String> {
    if let Some(var_name) = s.strip_prefix('$') {
        variables
            .get(var_name)
            .cloned()
            .ok_or_else(|| anyhow!("Variable '{}' not defined. Use -D {}=value to define it.", var_name, var_name))
    } else {
        Ok(s.to_string())
    }
}

/// Helper to resolve an optional string that might be a variable reference
fn resolve_optional_string_variable(
    s: &Option<String>,
    variables: &HashMap<String, String>,
) -> Result<Option<String>> {
    match s {
        Some(val) => Ok(Some(resolve_string_variable(val, variables)?)),
        None => Ok(None),
    }
}

/// Resolve variables in Aesthetics struct
fn resolve_aesthetics_variables(
    aes: &Aesthetics,
    variables: &HashMap<String, String>,
) -> Result<Aesthetics> {
    Ok(Aesthetics {
        x: resolve_string_variable(&aes.x, variables)?,
        y: resolve_optional_string_variable(&aes.y, variables)?,
        color: resolve_optional_string_variable(&aes.color, variables)?,
        size: resolve_optional_string_variable(&aes.size, variables)?,
        shape: resolve_optional_string_variable(&aes.shape, variables)?,
        alpha: resolve_optional_string_variable(&aes.alpha, variables)?,
        ymin: resolve_optional_string_variable(&aes.ymin, variables)?,
        ymax: resolve_optional_string_variable(&aes.ymax, variables)?,
    })
}

/// Resolve variables in Labels struct
fn resolve_labels_variables(
    labels: &Labels,
    variables: &HashMap<String, String>,
) -> Result<Labels> {
    Ok(Labels {
        title: resolve_optional_string_variable(&labels.title, variables)?,
        subtitle: resolve_optional_string_variable(&labels.subtitle, variables)?,
        x: resolve_optional_string_variable(&labels.x, variables)?,
        y: resolve_optional_string_variable(&labels.y, variables)?,
        caption: resolve_optional_string_variable(&labels.caption, variables)?,
    })
}

/// Resolve AestheticValue::Variable to Fixed (for color/string types)
fn resolve_aesthetic_value_string(
    value: &Option<AestheticValue<String>>,
    variables: &HashMap<String, String>,
) -> Result<Option<AestheticValue<String>>> {
    match value {
        Some(AestheticValue::Variable(var_name)) => {
            let resolved = variables
                .get(var_name)
                .ok_or_else(|| anyhow!("Variable '{}' not defined. Use -D {}=value to define it.", var_name, var_name))?;
            Ok(Some(AestheticValue::Fixed(resolved.clone())))
        }
        Some(other) => Ok(Some(other.clone())),
        None => Ok(None),
    }
}

/// Resolve AestheticValue::Variable to Fixed (for numeric types)
fn resolve_aesthetic_value_f64(
    value: &Option<AestheticValue<f64>>,
    variables: &HashMap<String, String>,
) -> Result<Option<AestheticValue<f64>>> {
    match value {
        Some(AestheticValue::Variable(var_name)) => {
            let resolved = variables
                .get(var_name)
                .ok_or_else(|| anyhow!("Variable '{}' not defined. Use -D {}=value to define it.", var_name, var_name))?;
            let parsed: f64 = resolved
                .parse()
                .map_err(|_| anyhow!("Variable '{}' value '{}' cannot be parsed as a number", var_name, resolved))?;
            Ok(Some(AestheticValue::Fixed(parsed)))
        }
        Some(other) => Ok(Some(other.clone())),
        None => Ok(None),
    }
}

/// Resolve variables in a Layer
fn resolve_layer_variables(layer: &Layer, variables: &HashMap<String, String>) -> Result<Layer> {
    match layer {
        Layer::Line(l) => {
            Ok(Layer::Line(LineLayer {
                stat: l.stat.clone(),
                x: resolve_optional_string_variable(&l.x, variables)?,
                y: resolve_optional_string_variable(&l.y, variables)?,
                color: resolve_aesthetic_value_string(&l.color, variables)?,
                width: resolve_aesthetic_value_f64(&l.width, variables)?,
                alpha: resolve_aesthetic_value_f64(&l.alpha, variables)?,
            }))
        }
        Layer::Point(p) => {
            Ok(Layer::Point(PointLayer {
                stat: p.stat.clone(),
                x: resolve_optional_string_variable(&p.x, variables)?,
                y: resolve_optional_string_variable(&p.y, variables)?,
                color: resolve_aesthetic_value_string(&p.color, variables)?,
                size: resolve_aesthetic_value_f64(&p.size, variables)?,
                shape: resolve_aesthetic_value_string(&p.shape, variables)?,
                alpha: resolve_aesthetic_value_f64(&p.alpha, variables)?,
            }))
        }
        Layer::Bar(b) => {
            Ok(Layer::Bar(BarLayer {
                stat: b.stat.clone(),
                x: resolve_optional_string_variable(&b.x, variables)?,
                y: resolve_optional_string_variable(&b.y, variables)?,
                color: resolve_aesthetic_value_string(&b.color, variables)?,
                alpha: resolve_aesthetic_value_f64(&b.alpha, variables)?,
                width: resolve_aesthetic_value_f64(&b.width, variables)?,
                position: b.position.clone(),
            }))
        }
        Layer::Ribbon(r) => {
            Ok(Layer::Ribbon(RibbonLayer {
                stat: r.stat.clone(),
                x: resolve_optional_string_variable(&r.x, variables)?,
                ymin: resolve_optional_string_variable(&r.ymin, variables)?,
                ymax: resolve_optional_string_variable(&r.ymax, variables)?,
                color: resolve_aesthetic_value_string(&r.color, variables)?,
                alpha: resolve_aesthetic_value_f64(&r.alpha, variables)?,
            }))
        }
        Layer::Boxplot(b) => {
            Ok(Layer::Boxplot(BoxplotLayer {
                stat: b.stat.clone(),
                x: resolve_optional_string_variable(&b.x, variables)?,
                y: resolve_optional_string_variable(&b.y, variables)?,
                color: resolve_aesthetic_value_string(&b.color, variables)?,
                fill: resolve_aesthetic_value_string(&b.fill, variables)?,
                alpha: resolve_aesthetic_value_f64(&b.alpha, variables)?,
                width: resolve_aesthetic_value_f64(&b.width, variables)?,
                outlier_color: b.outlier_color.clone(),
                outlier_size: b.outlier_size,
                outlier_shape: b.outlier_shape.clone(),
            }))
        }
        Layer::Violin(v) => {
            Ok(Layer::Violin(ViolinLayer {
                stat: v.stat.clone(),
                x: resolve_optional_string_variable(&v.x, variables)?,
                y: resolve_optional_string_variable(&v.y, variables)?,
                color: resolve_aesthetic_value_string(&v.color, variables)?,
                alpha: resolve_aesthetic_value_f64(&v.alpha, variables)?,
                width: resolve_aesthetic_value_f64(&v.width, variables)?,
                draw_quantiles: v.draw_quantiles.clone(),
            }))
        }
    }
}

/// Resolve all aesthetic mappings for a single layer (layer-specific + global)
fn resolve_layer_aesthetics(
    layer: &Layer,
    global_aes: &Option<Aesthetics>,
) -> Result<ResolvedAesthetics> {
    // Resolve x and y (required)
    let (x_col, y_col) = resolve_positional(layer, global_aes)?;

    // Resolve color mapping
    let color = match layer {
        Layer::Line(l) => extract_mapped_string(&l.color),
        Layer::Point(p) => extract_mapped_string(&p.color),
        Layer::Bar(b) => extract_mapped_string(&b.color),
        Layer::Ribbon(r) => extract_mapped_string(&r.color),
        Layer::Boxplot(b) => extract_mapped_string(&b.color),
        Layer::Violin(v) => extract_mapped_string(&v.color),
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.color.clone()));

    // Resolve size mapping
    let size = match layer {
        Layer::Line(l) => extract_mapped_string_from_f64(&l.width), // width can be data-driven
        Layer::Point(p) => extract_mapped_string_from_f64(&p.size),
        Layer::Bar(b) => extract_mapped_string_from_f64(&b.width),
        Layer::Ribbon(_) => None,
        Layer::Boxplot(b) => extract_mapped_string_from_f64(&b.width),
        Layer::Violin(v) => extract_mapped_string_from_f64(&v.width),
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.size.clone()));

    // Resolve shape mapping (point only)
    let shape = match layer {
        Layer::Point(p) => extract_mapped_string(&p.shape),
        _ => None,
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.shape.clone()));

    // Resolve alpha mapping
    let alpha = match layer {
        Layer::Line(l) => extract_mapped_string_from_f64(&l.alpha),
        Layer::Point(p) => extract_mapped_string_from_f64(&p.alpha),
        Layer::Bar(b) => extract_mapped_string_from_f64(&b.alpha),
        Layer::Ribbon(r) => extract_mapped_string_from_f64(&r.alpha),
        Layer::Boxplot(b) => extract_mapped_string_from_f64(&b.alpha),
        Layer::Violin(v) => extract_mapped_string_from_f64(&v.alpha),
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.alpha.clone()));

    // Resolve ymin/ymax
    let ymin_col = match layer {
        Layer::Ribbon(r) => r.ymin.clone(),
        _ => None,
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.ymin.clone()));

    let ymax_col = match layer {
        Layer::Ribbon(r) => r.ymax.clone(),
        _ => None,
    }
    .or_else(|| global_aes.as_ref().and_then(|a| a.ymax.clone()));

    Ok(ResolvedAesthetics {
        x_col,
        y_col,
        ymin_col,
        ymax_col,
        color,
        size,
        shape,
        alpha,
    })
}

/// Resolve x and y aesthetics
fn resolve_positional(layer: &Layer, global_aes: &Option<Aesthetics>) -> Result<(String, Option<String>)> {
    let (x_override, y_override) = match layer {
        Layer::Line(l) => (l.x.as_ref(), l.y.as_ref()),
        Layer::Point(p) => (p.x.as_ref(), p.y.as_ref()),
        Layer::Bar(b) => (b.x.as_ref(), b.y.as_ref()),
        Layer::Ribbon(r) => (r.x.as_ref(), None), // Ribbon uses ymin/ymax primarily
        Layer::Boxplot(b) => (b.x.as_ref(), b.y.as_ref()),
        Layer::Violin(v) => (v.x.as_ref(), v.y.as_ref()),
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
        Some(y.clone())
    } else if let Some(ref aes) = global_aes {
        aes.y.clone()
    } else {
        // y is optional for some layers (e.g. histogram)
        None
    };
    
    // Validation: Check if y is required but missing
    if y_col.is_none() {
        match layer {
            Layer::Bar(b) if matches!(b.stat, crate::parser::ast::Stat::Bin { .. } | crate::parser::ast::Stat::Count) => {
                // Allowed
            },
            Layer::Ribbon(_) => {
                // Allowed (uses ymin/ymax)
            },
            _ => {
                 anyhow::bail!("No y aesthetic specified (use aes(x: ..., y: ...) or layer-level y: ...)");
            }
        }
    }

    Ok((x_col, y_col))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Aesthetics, Layer, LineLayer, PointLayer, PlotSpec};
    use crate::data::PlotData;

    fn make_data() -> PlotData {
        PlotData {
            headers: vec!["x".to_string(), "y".to_string(), "g".to_string()],
            rows: vec![],
        }
    }

    fn empty_vars() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn test_resolve_simple() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: Some("y".to_string()),
                color: None,
                size: None,
                shape: None,
                alpha: None,
                ymin: None,
                ymax: None,
            }),
            layers: vec![Layer::Line(LineLayer::default())],
            labels: Some(crate::parser::ast::Labels::default()),
            facet: None,
            coord: None,
            theme: None,
            x_scale: None,
            y_scale: None,
        };
        let data = make_data();
        let resolved = resolve_plot_aesthetics(&spec, &data, &empty_vars()).unwrap();
        assert_eq!(resolved.layers.len(), 1);
        assert_eq!(resolved.layers[0].aesthetics.x_col, "x");
        assert_eq!(resolved.layers[0].aesthetics.y_col, Some("y".to_string()));
    }

    #[test]
    fn test_resolve_override() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: Some("y".to_string()),
                color: None,
                size: None,
                shape: None,
                alpha: None,
                ymin: None,
                ymax: None,
            }),
            layers: vec![Layer::Point(PointLayer {
                x: None,
                y: Some("g".to_string()),
                ..Default::default()
            })],
            labels: Some(crate::parser::ast::Labels::default()),
            facet: None,
            coord: None,
            theme: None,
            x_scale: None,
            y_scale: None,
        };
        let data = make_data();
        let resolved = resolve_plot_aesthetics(&spec, &data, &empty_vars()).unwrap();
        assert_eq!(resolved.layers[0].aesthetics.y_col, Some("g".to_string()));
    }

    #[test]
    fn test_resolve_missing_aes() {
        let spec = PlotSpec {
            aesthetics: None,
            layers: vec![Layer::Line(LineLayer::default())],
            labels: None,
            facet: None,
            coord: None,
            theme: None,
            x_scale: None,
            y_scale: None,
        };
        let data = make_data();
        let res = resolve_plot_aesthetics(&spec, &data, &empty_vars());
        assert!(res.is_err());
    }

    #[test]
    fn test_resolve_facet() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: Some("y".to_string()),
                color: None,
                size: None,
                shape: None,
                alpha: None,
                ymin: None,
                ymax: None,
            }),
            layers: vec![],
            labels: Some(crate::parser::ast::Labels::default()),
            facet: Some(crate::parser::ast::Facet {
                by: "g".to_string(),
                ncol: None,
                scales: crate::parser::ast::FacetScales::Fixed,
            }),
            coord: None,
            theme: None,
            x_scale: None,
            y_scale: None,
        };
        let data = make_data();
        let resolved = resolve_plot_aesthetics(&spec, &data, &empty_vars()).unwrap();
        assert!(resolved.facet.is_some());
        assert_eq!(resolved.facet.unwrap().col, "g");
    }

    #[test]
    fn test_resolve_with_variables() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "$xcol".to_string(),
                y: Some("$ycol".to_string()),
                color: None,
                size: None,
                shape: None,
                alpha: None,
                ymin: None,
                ymax: None,
            }),
            layers: vec![Layer::Line(LineLayer::default())],
            labels: Some(crate::parser::ast::Labels::default()),
            facet: None,
            coord: None,
            theme: None,
            x_scale: None,
            y_scale: None,
        };
        let data = make_data();
        let mut vars = HashMap::new();
        vars.insert("xcol".to_string(), "x".to_string());
        vars.insert("ycol".to_string(), "y".to_string());
        let resolved = resolve_plot_aesthetics(&spec, &data, &vars).unwrap();
        assert_eq!(resolved.layers[0].aesthetics.x_col, "x");
        assert_eq!(resolved.layers[0].aesthetics.y_col, Some("y".to_string()));
    }

    #[test]
    fn test_resolve_undefined_variable() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "$undefined".to_string(),
                y: Some("y".to_string()),
                color: None,
                size: None,
                shape: None,
                alpha: None,
                ymin: None,
                ymax: None,
            }),
            layers: vec![Layer::Line(LineLayer::default())],
            labels: None,
            facet: None,
            coord: None,
            theme: None,
            x_scale: None,
            y_scale: None,
        };
        let data = make_data();
        let res = resolve_plot_aesthetics(&spec, &data, &empty_vars());
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("undefined"));
    }

    #[test]
    fn test_resolve_variable_in_layer() {
        let spec = PlotSpec {
            aesthetics: Some(Aesthetics {
                x: "x".to_string(),
                y: Some("y".to_string()),
                color: None,
                size: None,
                shape: None,
                alpha: None,
                ymin: None,
                ymax: None,
            }),
            layers: vec![Layer::Line(LineLayer {
                color: Some(AestheticValue::Variable("line_color".to_string())),
                ..Default::default()
            })],
            labels: None,
            facet: None,
            coord: None,
            theme: None,
            x_scale: None,
            y_scale: None,
        };
        let data = make_data();
        let mut vars = HashMap::new();
        vars.insert("line_color".to_string(), "red".to_string());
        let resolved = resolve_plot_aesthetics(&spec, &data, &vars).unwrap();
        match &resolved.layers[0].original_layer {
            Layer::Line(l) => {
                assert_eq!(l.color, Some(AestheticValue::Fixed("red".to_string())));
            }
            _ => panic!("Expected Line layer"),
        }
    }
}
