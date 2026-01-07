# GramGraph - Grammar of Graphics DSL for Rust

A command-line tool for generating data visualizations from CSV data using a Grammar of Graphics DSL inspired by ggplot2.

## Overview

GramGraph implements a **Grammar of Graphics** approach to data visualization, separating concerns between:
- **Aesthetics**: Mappings from data columns to visual properties
- **Geometries**: Visual representations (line, point, bar, etc.)
- **Layers**: Independent, composable visualization layers

This architecture enables powerful, declarative chart specifications with clean composition semantics.

## Features

### ✅ Implemented

- **Core Geometries**: `line()`, `point()`, `bar()` with full styling options
- **Data-Driven Aesthetics**: Automatic grouping by color, size, shape, or alpha with legends
- **Faceting**: Multi-panel subplot grids with `facet_wrap()` and flexible axis scales
- **Layer Composition**: Multiple geometries on shared coordinate space
- **Bar Charts**: Categorical x-axis with dodge, stack, and identity positioning
- **Automatic Legends**: Generated for grouped visualizations
- **Color Palettes**: Category10 scheme with 10 distinct colors
- **Flexible Parsing**: Order-independent named arguments in DSL

### 🚀 Coming Soon

- Scale transformations (log, sqrt, etc.)
- Statistical transformations (count, bin, smooth, etc.)
- Additional geometries (area, ribbon, histogram, boxplot, violin, heatmap)
- Custom labels and themes
- Coordinate system transformations

## Architecture

```
CSV Data (stdin) → Parser → PlotSpec → Runtime → Canvas → PNG (stdout)
```

### Core Principles

1. **Separation of Aesthetics and Geometries**
   - Aesthetics define WHAT data maps to visual properties
   - Geometries define HOW that data is rendered

2. **Layer Composition**
   - Each geometry creates an independent layer
   - Layers are rendered in sequence, composing naturally
   - Multiple layers share coordinate space and ranges

3. **Aesthetic Inheritance**
   - Global aesthetics defined once with `aes()`
   - Individual layers inherit global aesthetics
   - Layers can override aesthetics locally

## DSL Syntax

### Basic Structure

```
aes(x: column, y: column) | geom() | geom() | ...
```

### Examples

**Simple line chart:**
```bash
cat data.csv | gramgraph 'aes(x: time, y: temperature) | line()'
```

**Styled line:**
```bash
cat data.csv | gramgraph 'aes(x: time, y: temp) | line(color: "red", width: 2)'
```

**Multiple layers (line + points):**
```bash
cat data.csv | gramgraph 'aes(x: time, y: temp) | line(color: "blue") | point(size: 5)'
```

**Per-layer aesthetic override:**
```bash
cat data.csv | gramgraph 'aes(x: time, y: temp) | line(y: high, color: "red") | line(y: low, color: "blue")'
```

**Bar chart:**
```bash
cat data.csv | gramgraph 'aes(x: category, y: value) | bar()'
```

**Side-by-side (dodged) bars:**
```bash
cat data.csv | gramgraph 'aes(x: region, y: q1) | bar(position: "dodge", color: "blue") | bar(y: q2, position: "dodge", color: "green")'
```

**Stacked bars:**
```bash
cat data.csv | gramgraph 'aes(x: month, y: product_a) | bar(position: "stack", color: "blue") | bar(y: product_b, position: "stack", color: "orange")'
```

### Supported Commands

#### `aes(x: col, y: col, ...)`
Defines global aesthetic mappings from data columns to visual properties.

**Required parameters:**
- `x:` - Column name for x-axis
- `y:` - Column name for y-axis

**Optional parameters (data-driven aesthetics):**
- `color: column` - Map column values to colors (creates grouped visualization with legend)
- `size: column` - Map column values to sizes
- `shape: column` - Map column values to shapes
- `alpha: column` - Map column values to transparency

#### `line(...)`
Renders data as a line series.

Optional parameters:
- `x: column` - Override x aesthetic for this layer
- `y: column` - Override y aesthetic for this layer
- `color: "red"` - Line color (red, green, blue, black, yellow, cyan, magenta)
- `width: 2` - Line width in pixels
- `alpha: 0.5` - Transparency (0.0-1.0)

#### `point(...)`
Renders data as points/scatter plot.

Optional parameters:
- `x: column` - Override x aesthetic
- `y: column` - Override y aesthetic
- `color: "blue"` - Point color
- `size: 5` - Point size in pixels
- `shape: "circle"` - Point shape (future)
- `alpha: 0.8` - Transparency

#### `bar(...)`
Renders data as a bar chart (categorical x-axis).

Optional parameters:
- `x: column` - Override x aesthetic for this layer
- `y: column` - Override y aesthetic for this layer
- `color: "red"` - Bar color (red, green, blue, black, yellow, cyan, magenta)
- `alpha: 0.7` - Transparency (0.0-1.0)
- `width: 0.8` - Bar width as fraction of category space (0.0-1.0)
- `position: "dodge"` - Positioning mode:
  - `"identity"` - Bars overlap at same position (default)
  - `"dodge"` - Bars side-by-side
  - `"stack"` - Bars stacked vertically

**Note**: Bar charts use categorical x-axis and cannot be mixed with line/point charts in the same plot.

#### `facet_wrap(by: column, ...)`
Creates a grid of subplots (small multiples), one for each unique value in the specified column.

**Required parameters:**
- `by: column` - Column name to facet by (creates one subplot per unique value)

**Optional parameters:**
- `ncol: Some(n)` - Number of columns in the grid layout (auto-calculated if omitted)
- `scales: "mode"` - Axis scale sharing mode:
  - `"fixed"` - All facets share the same x and y ranges (default)
  - `"free_x"` - Independent x ranges, shared y range
  - `"free_y"` - Shared x range, independent y ranges
  - `"free"` - Independent x and y ranges for each facet

**Example:**
```bash
cat data.csv | gramgraph 'aes(x: time, y: sales) | line() | facet_wrap(by: region)'
cat data.csv | gramgraph 'aes(x: time, y: sales) | line() | facet_wrap(by: region, ncol: Some(2), scales: "free_y")'
```

## Module Structure

```
src/
├── main.rs              # CLI entry point
├── csv_reader.rs        # CSV parsing from stdin
├── graph.rs             # Canvas & rendering (Plotters backend)
├── palette.rs           # Color/size/shape palettes for grouped data
├── runtime.rs           # Execute PlotSpec → PNG
└── parser/              # Grammar of Graphics parser
    ├── mod.rs           # Public API exports
    ├── ast.rs           # AST types (PlotSpec, Aesthetics, Layer, Facet, etc.)
    ├── lexer.rs         # Token parsing (identifier, string, number)
    ├── aesthetics.rs    # Parse aes(x: col, y: col, color: col, ...)
    ├── geom.rs          # Parse line(), point(), bar() geometries
    ├── facet.rs         # Parse facet_wrap() specifications
    └── pipeline.rs      # Parse complete plot specifications
```

## Parser Architecture

### AST Structure

```rust
// Complete plot specification
pub struct PlotSpec {
    pub aesthetics: Option<Aesthetics>,  // Global aes()
    pub layers: Vec<Layer>,              // Geometries
    pub labels: Option<Labels>,          // Title, axis labels
    pub facet: Option<Facet>,            // Faceting specification
}

// Aesthetic mappings (data columns → visual properties)
pub struct Aesthetics {
    pub x: String,                       // Column name for x-axis
    pub y: String,                       // Column name for y-axis
    pub color: Option<String>,           // Optional: column for color grouping
    pub size: Option<String>,            // Optional: column for size grouping
    pub shape: Option<String>,           // Optional: column for shape grouping
    pub alpha: Option<String>,           // Optional: column for alpha grouping
}

// AestheticValue: distinguishes fixed values from data-driven mappings
pub enum AestheticValue<T> {
    Fixed(T),           // Literal value: line(color: "red")
    Mapped(String),     // Column mapping: aes(color: region)
}

// Individual layers
pub enum Layer {
    Line(LineLayer),
    Point(PointLayer),
    Bar(BarLayer),
}

pub struct LineLayer {
    // Aesthetic overrides
    pub x: Option<String>,
    pub y: Option<String>,

    // Visual properties (can be fixed or data-driven)
    pub color: Option<AestheticValue<String>>,
    pub width: Option<AestheticValue<f64>>,
    pub alpha: Option<AestheticValue<f64>>,
}

pub struct PointLayer {
    pub x: Option<String>,
    pub y: Option<String>,

    pub color: Option<AestheticValue<String>>,
    pub size: Option<AestheticValue<f64>>,
    pub shape: Option<AestheticValue<String>>,
    pub alpha: Option<AestheticValue<f64>>,
}

pub struct BarLayer {
    pub x: Option<String>,
    pub y: Option<String>,

    pub color: Option<AestheticValue<String>>,
    pub alpha: Option<AestheticValue<f64>>,
    pub width: Option<AestheticValue<f64>>,
    pub position: BarPosition,  // dodge, stack, identity
}

// Faceting specification
pub struct Facet {
    pub by: String,              // Column to facet by
    pub ncol: Option<usize>,     // Number of columns in grid
    pub scales: FacetScales,     // Axis sharing mode
}

pub enum FacetScales {
    Fixed,   // All facets share same ranges
    FreeX,   // Independent x, shared y
    FreeY,   // Shared x, independent y
    Free,    // Independent x and y
}
```

### Parsing Flow

1. **Lexer** (`lexer.rs`): Tokenize input
   - Identifiers: `[a-zA-Z_][a-zA-Z0-9_]*`
   - String literals: `"..."`
   - Numbers: floats/integers
   - Operators: `|`, `:`, `,`, `(`, `)`

2. **Aesthetics Parser** (`aesthetics.rs`): Parse `aes(x: col, y: col)`
   - Extracts global aesthetic mappings
   - Returns `Aesthetics` struct

3. **Geometry Parser** (`geom.rs`): Parse `line()`, `point()`, etc.
   - Parses function name
   - Parses optional named arguments
   - Builds `Layer` enum variants

4. **Pipeline Parser** (`pipeline.rs`): Combine into `PlotSpec`
   - Parse optional `aes()` (global aesthetics)
   - Parse geometries separated by `|`
   - Build complete `PlotSpec`

## Runtime Architecture

### Layer Rendering

```rust
pub fn render_plot(spec: PlotSpec, csv_data: CsvData) -> Result<Vec<u8>>
```

1. **Faceting Check**
   - If facet specified, route to `render_faceted_plot()`
   - Otherwise, continue with standard rendering

2. **Aesthetic Resolution**
   - For each layer, resolve x/y columns
   - Layer-specific aesthetics override global
   - Validate: must have x and y for each layer
   - Identify data-driven aesthetics (color, size, shape, alpha)

3. **Data Grouping** (if data-driven aesthetics present)
   - Group data by aesthetic column (e.g., color: region)
   - Create palettes (ColorPalette, SizePalette, ShapePalette)
   - Assign visual properties to each group
   - Generate legend entries

4. **Data Extraction**
   - Extract columns from CSV data
   - Convert to `Vec<f64>` for plotting
   - Accumulate all data for range calculation

5. **Canvas Creation**
   - Calculate global x/y ranges from all layers/groups
   - Add 5% padding for visual breathing room
   - Create Canvas with shared coordinate space

6. **Layer Composition**
   - Render each layer/group in sequence
   - Each layer draws on shared canvas
   - Layers compose naturally (line + points, etc.)

7. **Legend Rendering**
   - Add legend if grouped data present
   - Legend shows group labels with colors

8. **PNG Encoding**
   - Finalize drawing area
   - Encode RGB buffer as PNG
   - Return PNG bytes

### Faceted Rendering

For faceted plots (`facet_wrap()`):

1. **Data Splitting**
   - Split CSV data by facet column
   - Create separate dataset for each facet value

2. **Range Calculation**
   - Calculate ranges based on scale mode:
     - `Fixed`: Global ranges across all facets
     - `FreeX/FreeY/Free`: Independent ranges per facet

3. **Grid Layout**
   - Calculate grid dimensions (nrow × ncol)
   - Auto-calculate if ncol not specified

4. **Multi-Panel Rendering**
   - Create MultiFacetCanvas with grid layout
   - Render each facet in its panel
   - Each panel has its own coordinate system

### Canvas API (`graph.rs`)

```rust
pub struct Canvas {
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    x_range: Range<f64>,
    y_range: Range<f64>,
    title: Option<String>,
    chart_initialized: bool,
    legend_entries: Vec<LegendEntry>,
}

impl Canvas {
    pub fn new(width, height, title, all_x_data, all_y_data) -> Result<Self>
    pub fn add_line_layer(&mut self, x_data, y_data, style) -> Result<()>
    pub fn add_point_layer(&mut self, x_data, y_data, style) -> Result<()>
    pub fn add_bar_layer(&mut self, categories, y_data, style) -> Result<()>
    pub fn add_legend(&mut self, entries: Vec<LegendEntry>) -> Result<()>
    pub fn render(self) -> Result<Vec<u8>>
}

pub struct MultiFacetCanvas {
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    nrow: usize,
    ncol: usize,
    panel_width: u32,
    panel_height: u32,
}

impl MultiFacetCanvas {
    pub fn new(width, height, nrow, ncol) -> Result<Self>
    pub fn render_facet(&mut self, row, col, label, x_data, y_data, ...) -> Result<()>
    pub fn render(self) -> Result<Vec<u8>>
}
```

**Key Design:**
- Canvas owns the pixel buffer
- Calculates global ranges from all data upfront
- Each `add_*_layer()` draws on the shared buffer
- Multiple layers share the same coordinate system
- Legend support for grouped visualizations
- MultiFacetCanvas for grid-based subplots

### Palette Module (`palette.rs`)

```rust
pub struct ColorPalette {
    colors: Vec<String>,
}

impl ColorPalette {
    pub fn category10() -> Self  // 10-color palette
    pub fn assign_colors(&self, keys: &[String]) -> HashMap<String, String>
}

pub struct SizePalette {
    min_size: f64,
    max_size: f64,
}

impl SizePalette {
    pub fn default_range() -> Self
    pub fn assign_sizes(&self, keys: &[String]) -> HashMap<String, f64>
}

pub struct ShapePalette {
    shapes: Vec<String>,
}

impl ShapePalette {
    pub fn default_shapes() -> Self
    pub fn assign_shapes(&self, keys: &[String]) -> HashMap<String, String>
}
```

**Purpose:**
- Automatic assignment of visual properties to groups
- Category10 color scheme (10 distinct colors, wraps for >10 groups)
- Size scaling across groups
- Shape variation for grouped data

## Dependencies

```toml
[dependencies]
clap = { version = "4.4", features = ["derive"] }  # CLI argument parsing
csv = "1.3"                                         # CSV reading
plotters = "0.3"                                    # Plotting backend
image = "0.24"                                      # PNG encoding
anyhow = "1.0"                                      # Error handling
nom = "7.1"                                         # Parser combinators
```

## Usage Examples

### Line Chart
```bash
cat data.csv | cargo run -- 'aes(x: date, y: value) | line()'
```

### Styled Line Chart
```bash
cat data.csv | cargo run -- 'aes(x: date, y: value) | line(color: "red", width: 2)'
```

### Scatter Plot
```bash
cat data.csv | cargo run -- 'aes(x: height, y: weight) | point(size: 3)'
```

### Line + Points (Layer Composition)
```bash
cat data.csv | cargo run -- 'aes(x: date, y: value) | line(color: "blue") | point(size: 5, color: "red")'
```

### Multiple Lines (Different Y Columns)
```bash
cat data.csv | cargo run -- 'aes(x: date, y: high) | line(color: "red") | line(y: low, color: "blue")'
```

### Bar Chart
```bash
cat data.csv | cargo run -- 'aes(x: category, y: value) | bar()'
```

### Side-by-Side (Dodge) Bars
```bash
cat data.csv | cargo run -- 'aes(x: region, y: q1) | bar(position: "dodge", color: "blue") | bar(y: q2, position: "dodge", color: "green")'
```

### Stacked Bars
```bash
cat data.csv | cargo run -- 'aes(x: month, y: product_a) | bar(position: "stack", color: "blue") | bar(y: product_b, position: "stack", color: "orange")'
```

### Data-Driven Aesthetics (Grouped Visualization)
```bash
# Grouped line chart by color - automatically creates different colored lines per region with legend
cat fixtures/multiregion_sales.csv | cargo run -- 'aes(x: time, y: sales, color: region) | line()'

# Grouped scatter plot by color - different colors per species with legend
cat fixtures/iris.csv | cargo run -- 'aes(x: sepal_length, y: sepal_width, color: species) | point()'

# Grouped by size - different point sizes per group
cat fixtures/multiregion_sales.csv | cargo run -- 'aes(x: time, y: sales, size: region) | point()'

# Multiple layers with grouping
cat fixtures/multiregion_sales.csv | cargo run -- 'aes(x: time, y: sales, color: region) | line() | point()'
```

### Faceting (Small Multiples)
```bash
# Basic faceting - creates grid of subplots, one per region
cat fixtures/multiregion_sales.csv | cargo run -- 'aes(x: time, y: sales) | line() | facet_wrap(by: region)'

# Faceted scatter plots
cat fixtures/iris.csv | cargo run -- 'aes(x: sepal_length, y: sepal_width) | point() | facet_wrap(by: species)'

# Faceting with custom grid layout (2 columns)
cat fixtures/multiregion_sales.csv | cargo run -- 'aes(x: time, y: sales) | line() | facet_wrap(by: region, ncol: Some(2))'

# Faceting with independent y-axis scales per panel
cat fixtures/multiregion_sales.csv | cargo run -- 'aes(x: time, y: sales) | line() | facet_wrap(by: region, scales: "free_y")'

# Combined: faceting + color grouping (grouped lines within each facet panel)
cat fixtures/multiregion_sales.csv | cargo run -- 'aes(x: time, y: sales, color: product) | line() | facet_wrap(by: region)'
```

## Design Decisions

### Why Grammar of Graphics?

The Grammar of Graphics approach provides:

1. **Composability**: Layers stack naturally (`line() | point()`)
2. **Reusability**: Define aesthetics once, use in multiple layers
3. **Extensibility**: Easy to add new geometries, scales, facets
4. **Declarative**: Describe WHAT you want, not HOW to draw it
5. **Intuitive**: Mirrors successful tools like ggplot2

### Why Separate Aesthetics from Geometries?

**Problem with coupled approach:**
```
chart(x: time, y: temp) | layer_line(color: "red")
```
- Chart command conflates data mapping with initialization
- Aesthetics are not reusable across layers
- Doesn't scale to complex multi-layer plots

**Grammar of Graphics solution:**
```
aes(x: time, y: temp) | line(color: "red") | point(size: 5)
```
- Clear separation: `aes()` maps data, `line()`/`point()` render
- Aesthetics defined once, inherited by all layers
- Each layer can override as needed
- Natural composition of multiple geometries

### Layer Rendering Strategy

**Challenge**: Multiple layers need to share coordinate space.

**Solution**: Two-pass approach
1. **Pass 1 (Data Collection)**:
   - Resolve aesthetics for each layer
   - Extract all data
   - Calculate global x/y ranges

2. **Pass 2 (Rendering)**:
   - Create canvas with global ranges
   - Render each layer in sequence
   - All layers share coordinate system

This ensures layers align correctly and don't clip each other.

## Implemented Features

### ✅ Data-Driven Aesthetics
Fully implemented with automatic legends and color palettes.

```bash
aes(x: time, y: temp, color: region) | line()
# Different colored lines per region (grouping) + automatic legend
```

**Supported mappings:**
- `color: column` - Automatic Category10 color palette
- `size: column` - Automatic size scaling
- `shape: column` - Automatic shape assignment
- `alpha: column` - Automatic transparency scaling

**Features:**
- Automatic legend generation
- Support for line, point, and bar charts
- Category10 color palette (10 distinct colors, wraps for >10 groups)
- Multiple layers with grouping

### ✅ Faceting (Small Multiples)
Fully implemented with multi-panel grid layouts.

```bash
aes(x: time, y: temp) | line() | facet_wrap(by: region)
# Grid of subplots, one per region
```

**Supported options:**
- `by: column` - Column to facet by (required)
- `ncol: Some(n)` - Number of columns in grid (optional, auto-calculated if omitted)
- `scales: "fixed|free_x|free_y|free"` - Axis sharing mode (optional, default: "fixed")

**Features:**
- Automatic grid layout calculation
- Shared or independent axis scales
- Works with all geometry types (line, point, bar)
- Can be combined with data-driven aesthetics

## Future Extensions

The Grammar of Graphics architecture naturally supports these additional features:

### 1. Scales & Transformations
```
aes(x: time, y: temp) | line() | scale_y_log10()
# Logarithmic y-axis
```

### 2. Statistical Transformations
```
aes(x: category) | bar(stat: "count")
# Bar chart showing counts of each category
```

### 3. More Geometries
- `area()` - Filled area plots
- `ribbon()` - Confidence intervals
- `histogram()` - Frequency distributions
- `boxplot()` - Box-and-whisker plots
- `violin()` - Violin plots
- `heatmap()` - 2D density/heatmaps

### 4. Labels & Themes
```
aes(x: time, y: temp) | line() | labs(title: "Temperature", x: "Date", y: "Temp (°F)")
```

### 5. Coordinate Systems
```
aes(x: category, y: value) | bar() | coord_flip()
# Horizontal bar chart
```

## Testing

### Test Coverage

**GramGraph maintains 93%+ test coverage** across all modules. Current coverage:

```
Module                Coverage    Details
---------------------------------------------------
palette.rs           100.00%     Fully tested (new module)
parser/lexer.rs       99.25%     Near-complete coverage
parser/pipeline.rs    97.73%     Comprehensive tests
main.rs               97.87%     CLI integration tested
graph.rs              96.43%     Canvas & rendering
runtime.rs            92.95%     Core execution logic
parser/facet.rs       89.40%     Faceting parser
parser/aesthetics.rs  87.30%     Aesthetic parsing
parser/geom.rs        85.97%     Geometry parsing
csv_reader.rs         86.90%     CSV handling
---------------------------------------------------
TOTAL                 93.35%     Overall coverage
```

**Test suite:**
- 174 total tests (151 unit + 23 integration)
- All tests passing
- Coverage ensures:
  - Reliable functionality for all features
  - Early detection of regressions
  - Confidence in error handling
  - Safe refactoring

### Running Tests

Run all tests:
```bash
cargo test
```

Run unit tests only:
```bash
cargo test --lib
```

Run integration tests only:
```bash
cargo test --test '*'
```

Run parser tests:
```bash
cargo test --lib parser
```

### Generating Coverage Reports

Install cargo-llvm-cov:
```bash
cargo install cargo-llvm-cov
```

Generate HTML coverage report:
```bash
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

Generate terminal summary:
```bash
cargo llvm-cov
```

Coverage should be 100% across all modules.

### Test Data Files

The `fixtures/` directory contains CSV files for various testing scenarios:

#### Basic Test Files
- **fixtures/basic.csv** - Simple 3x3 numeric data for basic functionality
- **fixtures/timeseries.csv** - Time series data with multiple numeric columns
- **fixtures/scatter.csv** - X-Y scatter plot data
- **fixtures/bar_chart.csv** - Categorical data with multiple value columns
- **fixtures/sales.csv** - Multi-region sales data for dodge/stack testing

#### Edge Case Test Files
- **fixtures/empty.csv** - Empty file (headers only, no data rows)
- **fixtures/single_row.csv** - Single data row
- **fixtures/single_column.csv** - Single column of data
- **fixtures/large_values.csv** - Very large numeric values (1e10)
- **fixtures/small_values.csv** - Very small numeric values (1e-10)
- **fixtures/negative_values.csv** - Negative numeric values
- **fixtures/mixed_types.csv** - Mix of numeric and text (for error testing)
- **fixtures/duplicate_headers.csv** - Duplicate column names
- **fixtures/missing_values.csv** - Empty cells in data
- **fixtures/special_chars.csv** - Special characters in column names
- **fixtures/unicode.csv** - Unicode characters in data
- **fixtures/long_column_names.csv** - Very long column names
- **fixtures/many_rows.csv** - Large dataset (10,000+ rows)

#### Creating Test CSV Files

When adding new tests:
1. Create CSV files with descriptive names in `fixtures/` directory
2. Include header row with column names
3. Add at least 3-5 data rows for meaningful tests
4. Document the purpose in test comments

Example test CSV structure:
```csv
x_column,y_column,category
1.0,10.0,A
2.0,20.0,B
3.0,30.0,C
```

### Test Organization

Tests are organized as:
- **Unit tests**: Inline `#[cfg(test)]` modules in each source file
- **Integration tests**: `tests/` directory for end-to-end workflows
- **Test fixtures**: `fixtures/` directory for CSV data files

### Manual Testing Examples

Line and point charts:
```bash
cat fixtures/timeseries.csv | cargo run -- 'aes(x: date, y: temperature) | line()'
cat fixtures/timeseries.csv | cargo run -- 'aes(x: date, y: temperature) | line(color: "red") | point(size: 5)'
cat fixtures/scatter.csv | cargo run -- 'aes(x: height, y: weight) | point()'
```

Bar charts:
```bash
cat fixtures/bar_chart.csv | cargo run -- 'aes(x: category, y: value1) | bar()'
cat fixtures/bar_chart.csv | cargo run -- 'aes(x: category, y: value1) | bar(color: "red")'
```

Side-by-side (dodge) bars:
```bash
cat fixtures/bar_chart.csv | cargo run -- 'aes(x: category, y: value1) | bar(position: "dodge", color: "blue") | bar(y: value2, position: "dodge", color: "red")'
cat fixtures/sales.csv | cargo run -- 'aes(x: region, y: q1) | bar(position: "dodge", color: "blue") | bar(y: q2, position: "dodge", color: "green")'
```

Stacked bars:
```bash
cat fixtures/bar_chart.csv | cargo run -- 'aes(x: category, y: value1) | bar(position: "stack", color: "blue") | bar(y: value2, position: "stack", color: "green") | bar(y: value3, position: "stack", color: "red")'
```

Overlapping bars (identity):
```bash
cat fixtures/bar_chart.csv | cargo run -- 'aes(x: category, y: value1) | bar(alpha: 0.5, color: "blue") | bar(y: value2, alpha: 0.5, color: "red")'
```

## Implementation Notes

### Parser Choice: nom

**Why nom?**
- Parser combinator library for Rust
- Type-safe, zero-copy parsing
- Composable parsers (match architecture)
- Excellent error messages with `context()`
- No separate lexer needed

### Rendering Backend: Plotters

**Why Plotters?**
- Pure Rust plotting library
- Multiple backends (bitmap, SVG, HTML canvas)
- Clean API for programmatic chart construction
- Supports complex multi-layer compositions
- Good performance for static chart generation

### CSV Parsing

Uses the `csv` crate for robust CSV handling:
- Automatic header detection
- Column selection by name or index
- Type conversion to `f64` for numeric plotting
- Clear error messages for invalid data

## Contributing

When adding new features:

1. **New Geometry Types**: Add to `ast.rs` Layer enum, implement parser in `geom.rs`, add rendering in `runtime.rs` and `graph.rs`

2. **New Aesthetics**: Extend `Aesthetics` struct, update `aesthetics.rs` parser, handle in runtime resolution

3. **Statistical Transformations**: Add transformation stage between data extraction and rendering

4. **Scales**: Implement scale transformations in Canvas coordinate mapping

## License

[Add your license here]

## Credits

Inspired by:
- **ggplot2** (Hadley Wickham) - Grammar of Graphics for R
- **The Grammar of Graphics** (Leland Wilkinson) - Theoretical foundation
- **Plotters** - Rust plotting library
- **nom** - Parser combinator library
