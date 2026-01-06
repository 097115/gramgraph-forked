// Abstract Syntax Tree for Grammar of Graphics DSL

/// Complete plot specification
#[derive(Debug, Clone, PartialEq)]
pub struct PlotSpec {
    pub aesthetics: Option<Aesthetics>,
    pub layers: Vec<Layer>,
    pub labels: Option<Labels>,
}

/// Global aesthetic mappings (data columns → visual properties)
#[derive(Debug, Clone, PartialEq)]
pub struct Aesthetics {
    /// Column name for x-axis
    pub x: String,
    /// Column name for y-axis
    pub y: String,
    // Future: color, size, shape, alpha (can map to columns for grouping/faceting)
}

/// Individual visualization layer
#[derive(Debug, Clone, PartialEq)]
pub enum Layer {
    Line(LineLayer),
    Point(PointLayer),
    // Future: Bar, Area, Ribbon, Histogram, etc.
}

/// Line geometry layer
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineLayer {
    // Aesthetic overrides (None = inherit from global)
    pub x: Option<String>,
    pub y: Option<String>,

    // Fixed visual properties (not data-driven)
    pub color: Option<String>,
    pub width: Option<f64>,
    pub alpha: Option<f64>,
    // Future: linetype (solid, dashed, dotted)
}

/// Point geometry layer
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PointLayer {
    // Aesthetic overrides
    pub x: Option<String>,
    pub y: Option<String>,

    // Fixed visual properties
    pub color: Option<String>,
    pub size: Option<f64>,
    pub shape: Option<String>,
    pub alpha: Option<f64>,
}

/// Plot labels (title, axes)
#[derive(Debug, Clone, PartialEq)]
pub struct Labels {
    pub title: Option<String>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
}
