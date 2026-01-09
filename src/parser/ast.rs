// Abstract Syntax Tree for Grammar of Graphics DSL

#[derive(Debug, Clone, PartialEq)]
pub enum CoordSystem {
    Cartesian,
    Flip,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LegendPosition {
    Right,
    Left,
    Top,
    Bottom,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub background_color: Option<String>,
    pub grid_visible: bool,
    pub font_family: Option<String>,
    pub legend_position: LegendPosition,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background_color: None,
            grid_visible: true,
            font_family: None,
            legend_position: LegendPosition::Right,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScaleType {
    Linear,
    Log10,
    Sqrt,
    Reverse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisScale {
    pub scale_type: ScaleType,
    pub limits: Option<(f64, f64)>, // Custom min/max
}

impl Default for AxisScale {
    fn default() -> Self {
        AxisScale {
            scale_type: ScaleType::Linear,
            limits: None,
        }
    }
}

/// Complete plot specification
#[derive(Debug, Clone, PartialEq)]
pub struct PlotSpec {
    pub aesthetics: Option<Aesthetics>,
    pub layers: Vec<Layer>,
    pub labels: Option<Labels>,
    pub facet: Option<Facet>,
    pub coord: Option<CoordSystem>,
    pub theme: Option<Theme>,
    pub x_scale: Option<AxisScale>,
    pub y_scale: Option<AxisScale>,
}

impl PlotSpec {
    /// Returns true if any layer in the plot requires a categorical x-axis
    pub fn requires_categorical_x(&self) -> bool {
        self.layers.iter().any(|l| l.requires_categorical_x())
    }
}

/// Global aesthetic mappings (data columns → visual properties)
#[derive(Debug, Clone, PartialEq)]
pub struct Aesthetics {
    /// Column name for x-axis
    pub x: String,
    /// Column name for y-axis
    pub y: Option<String>,
    /// Optional column name for color grouping
    pub color: Option<String>,
    /// Optional column name for size grouping
    pub size: Option<String>,
    /// Optional column name for shape grouping
    pub shape: Option<String>,
    /// Optional column name for alpha grouping
    pub alpha: Option<String>,
    /// Optional column name for ymin
    pub ymin: Option<String>,
    /// Optional column name for ymax
    pub ymax: Option<String>,
}

/// Represents either a fixed literal value or a data-driven column mapping
#[derive(Debug, Clone, PartialEq)]
pub enum AestheticValue<T> {
    /// Fixed literal value (e.g., line(color: "red"))
    Fixed(T),
    /// Column name for data-driven mapping (e.g., aes(color: region))
    Mapped(String),
}

/// Statistical transformation to apply
#[derive(Debug, Clone, PartialEq)]
pub enum Stat {
    Identity,
    Bin { bins: usize },
    Count,
    Smooth { method: String },
}

impl Default for Stat {
    fn default() -> Self {
        Stat::Identity
    }
}

/// Individual visualization layer
#[derive(Debug, Clone, PartialEq)]
pub enum Layer {
    Line(LineLayer),
    Point(PointLayer),
    Bar(BarLayer),
    Ribbon(RibbonLayer),
    // Future: Area, Histogram, etc.
}

impl Layer {
    /// Returns true if this layer type requires a categorical x-axis (e.g., Bar charts)
    pub fn requires_categorical_x(&self) -> bool {
        matches!(self, Layer::Bar(_))
    }

    pub fn stat(&self) -> &Stat {
        match self {
            Layer::Line(l) => &l.stat,
            Layer::Point(p) => &p.stat,
            Layer::Bar(b) => &b.stat,
            Layer::Ribbon(r) => &r.stat,
        }
    }
}

/// Line geometry layer
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineLayer {
    pub stat: Stat,
    // Aesthetic overrides (None = inherit from global)
    pub x: Option<String>,
    pub y: Option<String>,

    // Visual properties (can be fixed or data-driven)
    pub color: Option<AestheticValue<String>>,
    pub width: Option<AestheticValue<f64>>,
    pub alpha: Option<AestheticValue<f64>>,
    // Future: linetype (solid, dashed, dotted)
}

/// Point geometry layer
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PointLayer {
    pub stat: Stat,
    // Aesthetic overrides
    pub x: Option<String>,
    pub y: Option<String>,

    // Visual properties (can be fixed or data-driven)
    pub color: Option<AestheticValue<String>>,
    pub size: Option<AestheticValue<f64>>,
    pub shape: Option<AestheticValue<String>>,
    pub alpha: Option<AestheticValue<f64>>,
}

/// Bar geometry layer
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarLayer {
    pub stat: Stat,
    // Aesthetic overrides
    pub x: Option<String>,
    pub y: Option<String>,

    // Visual properties (can be fixed or data-driven)
    pub color: Option<AestheticValue<String>>,
    pub alpha: Option<AestheticValue<f64>>,
    pub width: Option<AestheticValue<f64>>, // Bar width (0.0-1.0, relative to category spacing)

    // Positioning strategy
    pub position: BarPosition,
}

/// Ribbon geometry layer
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RibbonLayer {
    pub stat: Stat,
    // Aesthetic overrides
    pub x: Option<String>,
    pub ymin: Option<String>,
    pub ymax: Option<String>,

    // Visual properties
    pub color: Option<AestheticValue<String>>, // Used for fill
    pub alpha: Option<AestheticValue<f64>>,
}

/// Bar positioning modes (how bars are arranged)
#[derive(Debug, Clone, PartialEq)]
pub enum BarPosition {
    Identity, // Bars overlap at same x position
    Dodge,    // Bars side-by-side
    Stack,    // Bars stacked vertically
}

impl Default for BarPosition {
    fn default() -> Self {
        BarPosition::Identity
    }
}

/// Plot labels (title, axes)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Labels {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub x: Option<String>, // Renamed from x_label for ggplot2 parity
    pub y: Option<String>, // Renamed from y_label
    pub caption: Option<String>,
}

/// Facet specification for creating subplot grids
#[derive(Debug, Clone, PartialEq)]
pub struct Facet {
    /// Column name to facet by (creates one subplot per unique value)
    pub by: String,
    /// Number of columns in the grid layout (auto-calculated if None)
    pub ncol: Option<usize>,
    /// Axis scale sharing mode
    pub scales: FacetScales,
}

/// Facet axis scale sharing modes
#[derive(Debug, Clone, PartialEq)]
pub enum FacetScales {
    /// All facets share the same x and y ranges (default)
    Fixed,
    /// Independent x ranges, shared y range
    FreeX,
    /// Shared x range, independent y ranges
    FreeY,
    /// Independent x and y ranges for each facet
    Free,
}

impl Default for FacetScales {
    fn default() -> Self {
        FacetScales::Fixed
    }
}
