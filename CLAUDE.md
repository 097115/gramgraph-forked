# GramGraph - Grammar of Graphics DSL for Rust

A command-line tool for generating data visualizations from CSV data using a Grammar of Graphics DSL inspired by ggplot2.

## Overview

GramGraph implements a **Grammar of Graphics** approach to data visualization, separating concerns between:
- **Aesthetics**: Mappings from data columns to visual properties
- **Geometries**: Visual representations (line, point, bar, etc.)
- **Layers**: Independent, composable visualization layers
- **Statistics**: Data transformations (binning, smoothing, counting)
- **Scales & Coordinates**: Data mapping to visual space (log, reverse, flip)

This architecture enables powerful, declarative chart specifications with clean composition semantics.

## Features

### ✅ Implemented

- **Core Geometries**: `line()`, `point()`, `bar()`, `ribbon()`, `boxplot()`, `violin()` with full styling options
- **Statistical Geoms**: `histogram(bins: n)`, `smooth()` (linear regression), `boxplot()`, `violin()` (KDE)
- **Data-Driven Aesthetics**: Automatic grouping by color, size, shape, or alpha with legends
- **Faceting**: Multi-panel subplot grids with `facet_wrap()` and flexible axis scales
- **Layer Composition**: Multiple geometries on shared coordinate space
- **Bar/Boxplot Positioning**: Smart dodging (occupancy-based) for categorical axes
- **Statistical Transformations**: `bin`, `count`, `smooth`, `boxplot` (5-number summary + outliers)
- **Scales**: `scale_x_reverse()`, `scale_y_reverse()`, `xlim()`, `ylim()`, `scale_x_log10()`, `scale_y_log10()`
- **Coordinates**: `coord_flip()` for horizontal charts
- **Visual Customization**: `labs()` for titles/labels, `theme_minimal()` for presets
- **Hierarchical Theme System**: `element_text()`, `element_line()`, `element_rect()`, `element_blank()` with inheritance
- **Automatic Legends**: Generated for grouped visualizations
- **Color Palettes**: Category10 scheme with 10 distinct colors
- **Flexible Parsing**: Order-independent named arguments in DSL
- **Data Abstraction**: Internal `PlotData` type for flexible data input (e.g., CSV, JSON)
- **Render Options**: Configurable output dimensions (`--width`, `--height`) and format (`--format png | svg`)
- **Variable Injection**: Runtime substitution with `-D`/`--define` flags for reusable plot templates

### 🚀 Coming Soon

- More statistical methods (loess smoothing)
- Additional geometries (heatmap)
- Custom legend configuration
- Additional preset themes (theme_dark, theme_classic)

## Architecture

GramGraph employs a strict **Grammar of Graphics** pipeline, moving data through five distinct phases:

```
CSV/JSON Data → Resolution → Transformation → Scaling → Compilation → Rendering → PNG/SVG
```

### Core Principles

1. **Separation of Aesthetics and Geometries**
   - Aesthetics define WHAT data maps to visual properties
   - Geometries define HOW that data is rendered

2. **Unidirectional Data Flow**
   - Data is transformed, scaled, and compiled in strict sequence.
   - Rendering is "dumb" and only executes drawing commands.

3. **Layer Composition**
   - Multiple layers share the same coordinate space (Scales).
   - Layers are processed independently but rendered onto a shared canvas.

## DSL Syntax

### Basic Structure

```
aes(x: column, y: column) | geom() | labs() | theme() | scales()
```

### Examples

**Simple line chart:**
```bash
cat data.csv | gramgraph 'aes(x: time, y: temperature) | line()' --width 1024 --height 768 --format svg
```

**Histogram with Theme:**
```bash
cat data.csv | gramgraph 'aes(x: value) | histogram(bins: 20) | labs(title: "Distribution") | theme_minimal()' --width 800 --height 600 --format png
```

**Horizontal Bar Chart (Coord Flip):**
```bash
cat data.csv | gramgraph 'aes(x: category, y: value) | bar() | coord_flip() | labs(x: "Category", y: "Value")'
```

**Smoothing (Linear Regression):**
```bash
cat data.csv | gramgraph 'aes(x: height, y: weight) | point(alpha: 0.5) | smooth()'
```

**Boxplot:**
```bash
cat demographics.csv | gramgraph 'aes(x: gender, y: height, color: gender) | boxplot()'
```

**Violin Plot:**
```bash
cat demographics.csv | gramgraph 'aes(x: gender, y: height, color: gender) | violin(draw_quantiles: [0.25, 0.5, 0.75])'
```

**Ribbon Chart (Area with range):**
```bash
cat data.csv | gramgraph 'aes(x: time, y: mean, ymin: lower, ymax: upper) | ribbon(alpha: 0.2) | line()'
```

**Reverse Scale:**
```bash
cat data.csv | gramgraph 'aes(x: depth, y: pressure) | line() | labs(title: "Depth Profile") | scale_x_reverse()'
```

**Variable Injection:**
```bash
# Variables in aesthetics and labels
cat data.csv | gramgraph 'aes(x: $xcol, y: $ycol) | line() | labs(title: $title)' -D xcol=time -D ycol=value -D title="My Chart"

# Variables in geometry styling
cat data.csv | gramgraph 'aes(x: time, y: value) | line(color: $color, width: $width)' -D color=red -D width=2
```

### Supported Commands

#### `aes(...)`
Defines global aesthetic mappings.
- **Required**: `x: col`.
- **Optional**: `y: col` (required for most geoms except histogram), `color: col`, `size: col`, `shape: col`, `alpha: col`, `ymin: col`, `ymax: col`.

#### Geometries
- `line(...)`: Line chart.
- `point(...)`: Scatter plot.
- `bar(...)`: Bar chart. Supports `position: "dodge" | "stack" | "identity"`.
- `boxplot(...)`: Box and whisker plot with automatic outlier detection.
- `violin(...)`: Violin plot using Kernel Density Estimation (KDE). Supports `draw_quantiles: [0.25, 0.5, 0.75]`.
- `ribbon(...)`: Filled area between `ymin` and `ymax`.
- `histogram(...)`: Binning bar chart. Supports `bins: n`.
- `smooth(...)`: Smoothing line (Linear Regression).

#### `labs(...)`
- `title: "..."`
- `subtitle: "..."`
- `x: "..."`
- `y: "..."`
- `caption: "..."`

#### `coord_flip()`
Swaps X and Y axes. Useful for horizontal bar charts.

#### Scales
- `scale_x_reverse()`, `scale_y_reverse()`
- `scale_x_log10()`, `scale_y_log10()`
- `xlim(min, max)`, `ylim(min, max)`

#### Themes

GramGraph implements a hierarchical theme system inspired by ggplot2, using element primitives.

**Preset Themes:**
- `theme_minimal()`: Clean, white background, no axis lines/ticks, light grid.

**Element Functions:**
- `element_text(size: n, color: "...", family: "...", face: "bold|italic", angle: n)` - Text styling
- `element_line(color: "...", width: n, linetype: "solid|dashed|dotted")` - Line styling
- `element_rect(fill: "...", color: "...", width: n)` - Rectangle styling (backgrounds)
- `element_blank()` - Remove an element entirely

**Theme Properties:**
- `plot_background`: Canvas background (element_rect)
- `panel_background`: Drawing area background (element_rect)
- `plot_title`: Title text styling (element_text)
- `panel_grid_major`: Major grid lines (element_line or element_blank)
- `panel_grid_minor`: Minor grid lines (element_line or element_blank)
- `axis_text`: Axis label styling (element_text)
- `axis_line`: Axis line styling (element_line or element_blank)
- `axis_ticks`: Tick mark styling (element_line or element_blank)
- `legend_position`: "right" | "left" | "top" | "bottom" | "upper-right" | "upper-middle" | "upper-left" | "middle-right" | "middle-middle" | "middle-left" | "lower-right" | "lower-middle" | "lower-left" | "none"

**Color Formats:**
- Named colors: "red", "blue", "gray", "white", etc.
- Hex colors: "#FF0000", "#2E86AB", "#F00"
- Gray scale: "gray0" (black) to "gray100" (white)

**Theme Merging:**
Multiple `theme()` calls are merged (ggplot2-style), allowing customization on top of presets:
```bash
theme_minimal() | theme(plot_title: element_text(size: 24, face: "bold"))
```

#### `facet_wrap(by: column, ...)`
Creates small multiples.
- `ncol: n`
- `scales: "fixed" | "free" | "free_x" | "free_y"`

#### CLI Arguments
- `--width <pixels>`: Sets the output width in pixels (default: 800).
- `--height <pixels>`: Sets the output height in pixels (default: 600).
- `--format <png|svg>`: Sets the output format (default: png).
- `-D, --define <KEY=VALUE>`: Define variables for DSL substitution. Can be used multiple times (e.g., `-D x=time -D color=red`).

#### Variable Injection

Variables use the `$name` syntax and can be substituted at runtime using `-D`/`--define` flags. This enables reusable plot templates.

**Supported Locations:**
- **Aesthetics**: `aes(x: $xcol, y: $ycol, color: $groupby)`
- **Geometry properties**: `line(color: $color, width: $width)`, `point(size: $size, alpha: $alpha)`
- **Labels**: `labs(title: $title, x: $xlabel, y: $ylabel)`
- **Facets**: `facet_wrap(by: $facetcol)`

**Example:**
```bash
# Reusable template
cat sales.csv | gramgraph 'aes(x: $x, y: $y, color: $group) | line() | labs(title: $title)' \
  -D x=date -D y=revenue -D group=region -D title="Sales by Region"
```

**Error Handling:**
If a variable is used but not defined, a helpful error message is shown:
```
Variable 'undefined' not defined. Use -D undefined=value to define it.
```

## Module Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library export
├── csv_reader.rs        # CSV parsing
├── data.rs              # PlotData abstraction (CSV/JSON input)
├── ir.rs                # Intermediate Representation (Data Contracts)
├── resolve.rs           # Phase 1: Aesthetic Resolution
├── transform.rs         # Phase 2: Data Transformation (Stats/Position/Sort)
├── scale.rs             # Phase 3: Scale Calculation (Ranges/Categories)
├── compiler.rs          # Phase 4: Compile to SceneGraph (Draw Commands)
├── graph.rs             # Phase 5: Rendering Backend (Plotters)
├── theme_resolve.rs     # Theme Resolution Engine (Inheritance/Defaults)
├── palette.rs           # Color/size/shape palettes
├── runtime.rs           # Pipeline Coordinator
└── parser/              # Grammar of Graphics parser
    ├── mod.rs           # Public API exports
    ├── ast.rs           # AST types (includes Theme element primitives)
    ├── lexer.rs         # Token parsing
    ├── aesthetics.rs    # Parse aes()
    ├── geom.rs          # Parse geom(), histogram(), smooth()
    ├── facet.rs         # Parse facet_wrap()
    ├── coord.rs         # Parse coord_flip()
    ├── labels.rs        # Parse labs()
    ├── scale.rs         # Parse scale_*()
    ├── theme.rs         # Parse theme(), element_*()
    └── pipeline.rs      # Parse full pipeline
```

## Contributing

See `src/parser/` for DSL additions and `src/transform.rs` for new statistical capabilities.

## Development Guidelines

### Primitive-Only Rendering Backend

The rendering backend (`graph.rs`) must only know about **primitive drawing commands**:

| Primitive | Purpose |
|-----------|---------|
| `DrawLine` | Polylines, whiskers, axes |
| `DrawRect` | Bars, boxes, filled regions |
| `DrawPoint` | Scatter points, outliers |
| `DrawPolygon` | Ribbons, filled areas |

**Never add geometry-specific commands** (e.g., `DrawBoxplot`, `DrawViolin`) to `DrawCommand` or `graph.rs`.

### Adding a New Geometry

When implementing a new geometry (e.g., violin plot), follow this pattern:

1. **Parser** (`src/parser/geom.rs`, `src/parser/ast.rs`)
   - Add AST types for the new layer
   - Parse DSL syntax into the AST

2. **Transform** (`src/transform.rs`)
   - Compute any required statistics (e.g., density estimation for violin)
   - Store results in `GroupData` fields

3. **Compiler** (`src/compiler.rs`)
   - Convert the high-level geometry into **primitive commands**
   - Handle positioning, dodging, and orientation
   - Example: A violin plot becomes `DrawPolygon` commands

4. **Rendering** (`src/graph.rs`)
   - **No changes required** - primitives are already supported

### Why This Matters

This separation follows `ggplot2`'s architecture where the Grid graphics system never knows it's drawing a boxplot - it just draws rectangles and lines. Benefits:

- **Zero backend changes** for new geometries
- **Simpler renderer** - no statistical logic in drawing code
- **Easier testing** - primitives are straightforward to verify
- **Better maintainability** - geometry logic is localized in compiler

### Phase Responsibilities

| Phase | Module | Responsibility |
|-------|--------|----------------|
| Parse | `parser/` | DSL → AST |
| Resolve | `resolve.rs` | Validate columns, merge aesthetics |
| Transform | `transform.rs` | Statistics, grouping, stacking |
| Scale | `scale.rs` | Domain/range calculation |
| Compile | `compiler.rs` | **Geometry → Primitives** |
| Render | `graph.rs` | Primitives → Pixels/SVG |

## License

[Add your license here]
