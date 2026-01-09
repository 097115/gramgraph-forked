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

- **Core Geometries**: `line()`, `point()`, `bar()`, `ribbon()` with full styling options
- **Statistical Geoms**: `histogram(bins: n)`, `smooth()` (linear regression)
- **Data-Driven Aesthetics**: Automatic grouping by color, size, shape, or alpha with legends
- **Faceting**: Multi-panel subplot grids with `facet_wrap()` and flexible axis scales
- **Layer Composition**: Multiple geometries on shared coordinate space
- **Bar Charts**: Categorical x-axis with dodge, stack, and identity positioning
- **Statistical Transformations**: `bin`, `count`, `smooth`
- **Scales**: `scale_x_reverse()`, `scale_y_reverse()`, `xlim()`, `ylim()`, `scale_x_log10()`, `scale_y_log10()`
- **Coordinates**: `coord_flip()` for horizontal charts
- **Visual Customization**: `labs()` for titles/labels and `theme_minimal()` for styling
- **Automatic Legends**: Generated for grouped visualizations
- **Color Palettes**: Category10 scheme with 10 distinct colors
- **Flexible Parsing**: Order-independent named arguments in DSL
- **Data Abstraction**: Internal `PlotData` type for flexible data input (e.g., CSV, JSON)
- **Render Options**: Configurable output dimensions (`--width`, `--height`) and format (`--format png | svg`)

### 🚀 Coming Soon

- More statistical methods (loess smoothing)
- Additional geometries (boxplot, violin, heatmap)
- Custom legend configuration
- More themes

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

**Ribbon Chart (Area with range):**
```bash
cat data.csv | gramgraph 'aes(x: time, y: mean, ymin: lower, ymax: upper) | ribbon(alpha: 0.2) | line()'
```

**Reverse Scale:**
```bash
cat data.csv | gramgraph 'aes(x: depth, y: pressure) | line() | labs(title: "Depth Profile") | scale_x_reverse()'
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
- `theme_minimal()`: Clean, white background theme.
- `theme(legend_position: "right" | "bottom" | "none")`

#### `facet_wrap(by: column, ...)`
Creates small multiples.
- `ncol: n`
- `scales: "fixed" | "free" | "free_x" | "free_y"`

#### CLI Arguments
- `--width <pixels>`: Sets the output width in pixels (default: 800).
- `--height <pixels>`: Sets the output height in pixels (default: 600).
- `--format <png|svg>`: Sets the output format (default: png).

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
├── palette.rs           # Color/size/shape palettes
├── runtime.rs           # Pipeline Coordinator
└── parser/              # Grammar of Graphics parser
    ├── mod.rs           # Public API exports
    ├── ast.rs           # AST types
    ├── lexer.rs         # Token parsing
    ├── aesthetics.rs    # Parse aes()
    ├── geom.rs          # Parse geom(), histogram(), smooth()
    ├── facet.rs         # Parse facet_wrap()
    ├── coord.rs         # Parse coord_flip()
    ├── labels.rs        # Parse labs()
    ├── scale.rs         # Parse scale_*()
    ├── theme.rs         # Parse theme()
    └── pipeline.rs      # Parse full pipeline
```

## Contributing

See `src/parser/` for DSL additions and `src/transform.rs` for new statistical capabilities.

## License

[Add your license here]
