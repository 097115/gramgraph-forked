use anyhow::{Context, Result};
use image::ImageEncoder;
use plotters::prelude::*;
use std::ops::Range;

/// Style configuration for line layers
#[derive(Debug, Clone)]
pub struct LineStyle {
    pub color: Option<String>,
    pub width: Option<f64>,
    pub alpha: Option<f64>,
}

/// Style configuration for point layers
#[derive(Debug, Clone)]
pub struct PointStyle {
    pub color: Option<String>,
    pub size: Option<f64>,
    pub shape: Option<String>,
    pub alpha: Option<f64>,
}

/// Canvas for multi-layer plotting
pub struct Canvas {
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    x_range: Range<f64>,
    y_range: Range<f64>,
    title: Option<String>,
    chart_initialized: bool,
}

impl Canvas {
    /// Create a new canvas with global data ranges
    pub fn new(
        width: u32,
        height: u32,
        title: Option<String>,
        all_x_data: Vec<f64>,
        all_y_data: Vec<f64>,
    ) -> Result<Self> {
        if all_x_data.is_empty() || all_y_data.is_empty() {
            anyhow::bail!("Cannot create canvas with no data points");
        }

        // Calculate global ranges
        let x_min = all_x_data
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let x_max = all_x_data
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let y_min = all_y_data
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let y_max = all_y_data
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let x_range = if x_min == x_max {
            (x_min - 1.0)..(x_max + 1.0)
        } else {
            let padding = (x_max - x_min) * 0.05;
            (x_min - padding)..(x_max + padding)
        };

        let y_range = if y_min == y_max {
            (y_min - 1.0)..(y_max + 1.0)
        } else {
            let padding = (y_max - y_min) * 0.05;
            (y_min - padding)..(y_max + padding)
        };

        let buffer = vec![0u8; (width * height * 3) as usize];

        Ok(Canvas {
            buffer,
            width,
            height,
            x_range,
            y_range,
            title,
            chart_initialized: false,
        })
    }

    /// Add a line layer to the canvas
    pub fn add_line_layer(
        &mut self,
        x_data: Vec<f64>,
        y_data: Vec<f64>,
        style: LineStyle,
    ) -> Result<()> {
        if x_data.len() != y_data.len() {
            anyhow::bail!(
                "X and Y data must have the same length (x: {}, y: {})",
                x_data.len(),
                y_data.len()
            );
        }

        let root = BitMapBackend::with_buffer(&mut self.buffer, (self.width, self.height))
            .into_drawing_area();

        if !self.chart_initialized {
            root.fill(&WHITE).context("Failed to fill background")?;
            self.chart_initialized = true;
        }

        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .caption(self.title.as_deref().unwrap_or(""), ("sans-serif", 20))
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(self.x_range.clone(), self.y_range.clone())
            .context("Failed to build chart")?;

        chart
            .configure_mesh()
            .draw()
            .context("Failed to draw mesh")?;

        let points: Vec<(f64, f64)> = x_data.into_iter().zip(y_data).collect();

        let color = parse_color(&style.color);
        let width = style.width.unwrap_or(1.0) as u32;

        chart
            .draw_series(LineSeries::new(points, color.stroke_width(width)))
            .context("Failed to draw line series")?;

        root.present().context("Failed to present drawing")?;

        Ok(())
    }

    /// Add a point layer to the canvas
    pub fn add_point_layer(
        &mut self,
        x_data: Vec<f64>,
        y_data: Vec<f64>,
        style: PointStyle,
    ) -> Result<()> {
        if x_data.len() != y_data.len() {
            anyhow::bail!(
                "X and Y data must have the same length (x: {}, y: {})",
                x_data.len(),
                y_data.len()
            );
        }

        let root = BitMapBackend::with_buffer(&mut self.buffer, (self.width, self.height))
            .into_drawing_area();

        if !self.chart_initialized {
            root.fill(&WHITE).context("Failed to fill background")?;
            self.chart_initialized = true;
        }

        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .caption(self.title.as_deref().unwrap_or(""), ("sans-serif", 20))
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(self.x_range.clone(), self.y_range.clone())
            .context("Failed to build chart")?;

        // Only draw mesh if this is the first layer
        if !self.chart_initialized {
            chart
                .configure_mesh()
                .draw()
                .context("Failed to draw mesh")?;
        }

        let points: Vec<(f64, f64)> = x_data.into_iter().zip(y_data).collect();

        let color = parse_color(&style.color);
        let size = style.size.unwrap_or(3.0) as i32;

        chart
            .draw_series(points.iter().map(|&(x, y)| {
                Circle::new((x, y), size, color.filled())
            }))
            .context("Failed to draw point series")?;

        root.present().context("Failed to present drawing")?;

        Ok(())
    }

    /// Finalize and encode the canvas as PNG
    pub fn render(self) -> Result<Vec<u8>> {
        let mut png_bytes = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            encoder
                .write_image(
                    &self.buffer,
                    self.width,
                    self.height,
                    image::ColorType::Rgb8,
                )
                .context("Failed to encode PNG")?;
        }

        Ok(png_bytes)
    }
}

/// Parse color string to RGBColor
fn parse_color(color_str: &Option<String>) -> RGBColor {
    match color_str.as_deref() {
        Some("red") => RED,
        Some("green") => GREEN,
        Some("blue") => BLUE,
        Some("black") => BLACK,
        Some("yellow") => YELLOW,
        Some("cyan") => CYAN,
        Some("magenta") => MAGENTA,
        Some("white") => WHITE,
        _ => BLUE, // default
    }
}
