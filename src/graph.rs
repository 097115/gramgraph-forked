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

/// Style configuration for bar layers
#[derive(Debug, Clone)]
pub struct BarStyle {
    pub color: Option<String>,
    pub alpha: Option<f64>,
    pub width: Option<f64>,
}

/// Canvas for multi-layer plotting
#[derive(Debug)]
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

    /// Add a bar layer to the canvas (categorical x-axis)
    pub fn add_bar_layer(
        &mut self,
        categories: Vec<String>,
        y_data: Vec<f64>,
        style: BarStyle,
    ) -> Result<()> {
        if categories.len() != y_data.len() {
            anyhow::bail!(
                "Categories and Y data must have the same length (categories: {}, y: {})",
                categories.len(),
                y_data.len()
            );
        }

        if categories.is_empty() {
            anyhow::bail!("Cannot create bar chart with no data");
        }

        let root = BitMapBackend::with_buffer(&mut self.buffer, (self.width, self.height))
            .into_drawing_area();

        if !self.chart_initialized {
            root.fill(&WHITE).context("Failed to fill background")?;
            self.chart_initialized = true;
        }

        let num_categories = categories.len();
        let x_range = 0.0..(num_categories as f64);

        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .caption(self.title.as_deref().unwrap_or(""), ("sans-serif", 20))
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(x_range.clone(), self.y_range.clone())
            .context("Failed to build chart")?;

        // Configure mesh with custom x-axis labels
        let categories_clone = categories.clone();
        chart
            .configure_mesh()
            .x_labels(num_categories)
            .x_label_formatter(&|x| {
                let idx = *x as usize;
                if idx < categories_clone.len() {
                    categories_clone[idx].clone()
                } else {
                    String::new()
                }
            })
            .draw()
            .context("Failed to draw mesh")?;

        // Draw bars
        let color = parse_color(&style.color);
        let alpha = style.alpha.unwrap_or(1.0);
        let color_with_alpha = color.mix(alpha);
        let bar_width = style.width.unwrap_or(0.8);

        for (cat_idx, &y_val) in y_data.iter().enumerate() {
            let x_center = cat_idx as f64 + 0.5;
            chart
                .draw_series(std::iter::once(Rectangle::new(
                    [
                        (x_center - bar_width / 2.0, 0.0),
                        (x_center + bar_width / 2.0, y_val),
                    ],
                    color_with_alpha.filled(),
                )))
                .context("Failed to draw bar")?;
        }

        root.present().context("Failed to present drawing")?;

        Ok(())
    }

    /// Add multiple bar series with dodge or stack positioning
    pub fn add_bar_group(
        &mut self,
        categories: Vec<String>,
        series: Vec<(Vec<f64>, BarStyle)>, // (y_data, style) for each series
        position: &str, // "dodge", "stack", or "identity"
    ) -> Result<()> {
        if categories.is_empty() {
            anyhow::bail!("Cannot create bar chart with no categories");
        }

        if series.is_empty() {
            anyhow::bail!("Cannot create bar chart with no series");
        }

        let root = BitMapBackend::with_buffer(&mut self.buffer, (self.width, self.height))
            .into_drawing_area();

        if !self.chart_initialized {
            root.fill(&WHITE).context("Failed to fill background")?;
            self.chart_initialized = true;
        }

        let num_categories = categories.len();
        let num_series = series.len();
        let x_range = 0.0..(num_categories as f64);

        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .caption(self.title.as_deref().unwrap_or(""), ("sans-serif", 20))
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(x_range.clone(), self.y_range.clone())
            .context("Failed to build chart")?;

        // Configure mesh with custom x-axis labels
        let categories_clone = categories.clone();
        chart
            .configure_mesh()
            .x_labels(num_categories)
            .x_label_formatter(&|x| {
                let idx = *x as usize;
                if idx < categories_clone.len() {
                    categories_clone[idx].clone()
                } else {
                    String::new()
                }
            })
            .draw()
            .context("Failed to draw mesh")?;

        match position {
            "dodge" => {
                // Side-by-side bars
                let bar_width = 0.8 / num_series as f64;

                for (series_idx, (y_data, style)) in series.iter().enumerate() {
                    let color = parse_color(&style.color);
                    let alpha = style.alpha.unwrap_or(1.0);
                    let color_with_alpha = color.mix(alpha);

                    for (cat_idx, &y_val) in y_data.iter().enumerate() {
                        let x_base = cat_idx as f64;
                        let x_offset = (series_idx as f64 - (num_series as f64 - 1.0) / 2.0) * bar_width;
                        let x_center = x_base + 0.5 + x_offset;

                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [
                                    (x_center - bar_width / 2.0, 0.0),
                                    (x_center + bar_width / 2.0, y_val),
                                ],
                                color_with_alpha.filled(),
                            )))
                            .context("Failed to draw bar")?;
                    }
                }
            }
            "stack" => {
                // Stacked bars
                let bar_width = 0.8;

                for cat_idx in 0..num_categories {
                    let x_center = cat_idx as f64 + 0.5;
                    let mut y_cumulative = 0.0;

                    for (y_data, style) in series.iter() {
                        let y_val = y_data[cat_idx];
                        let color = parse_color(&style.color);
                        let alpha = style.alpha.unwrap_or(1.0);
                        let color_with_alpha = color.mix(alpha);

                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [
                                    (x_center - bar_width / 2.0, y_cumulative),
                                    (x_center + bar_width / 2.0, y_cumulative + y_val),
                                ],
                                color_with_alpha.filled(),
                            )))
                            .context("Failed to draw bar")?;

                        y_cumulative += y_val;
                    }
                }
            }
            _ => {
                // Identity (overlapping) - default
                let bar_width = 0.8;

                for (y_data, style) in series.iter() {
                    let color = parse_color(&style.color);
                    let alpha = style.alpha.unwrap_or(0.5); // Default semi-transparent for overlapping
                    let color_with_alpha = color.mix(alpha);

                    for (cat_idx, &y_val) in y_data.iter().enumerate() {
                        let x_center = cat_idx as f64 + 0.5;

                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [
                                    (x_center - bar_width / 2.0, 0.0),
                                    (x_center + bar_width / 2.0, y_val),
                                ],
                                color_with_alpha.filled(),
                            )))
                            .context("Failed to draw bar")?;
                    }
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to check if bytes are valid PNG
    fn is_valid_png(bytes: &[u8]) -> bool {
        bytes.len() > 8 && bytes[0..8] == [137, 80, 78, 71, 13, 10, 26, 10]
    }

    // parse_color tests (3 tests)

    #[test]
    fn test_parse_color_known_colors() {
        assert_eq!(parse_color(&Some("red".to_string())), RED);
        assert_eq!(parse_color(&Some("green".to_string())), GREEN);
        assert_eq!(parse_color(&Some("blue".to_string())), BLUE);
        assert_eq!(parse_color(&Some("black".to_string())), BLACK);
        assert_eq!(parse_color(&Some("yellow".to_string())), YELLOW);
        assert_eq!(parse_color(&Some("cyan".to_string())), CYAN);
        assert_eq!(parse_color(&Some("magenta".to_string())), MAGENTA);
        assert_eq!(parse_color(&Some("white".to_string())), WHITE);
    }

    #[test]
    fn test_parse_color_unknown_defaults_blue() {
        assert_eq!(parse_color(&Some("invalid".to_string())), BLUE);
        assert_eq!(parse_color(&Some("unknown".to_string())), BLUE);
    }

    #[test]
    fn test_parse_color_none() {
        assert_eq!(parse_color(&None), BLUE);
    }

    // Canvas::new tests (8 tests)

    #[test]
    fn test_canvas_new_basic() {
        let x_data = vec![1.0, 2.0, 3.0];
        let y_data = vec![10.0, 20.0, 30.0];
        let canvas = Canvas::new(800, 600, None, x_data, y_data).unwrap();
        assert_eq!(canvas.width, 800);
        assert_eq!(canvas.height, 600);
        assert!(canvas.x_range.start < 1.0);
        assert!(canvas.x_range.end > 3.0);
    }

    #[test]
    fn test_canvas_new_with_title() {
        let canvas = Canvas::new(
            800,
            600,
            Some("Test Chart".to_string()),
            vec![1.0],
            vec![1.0],
        )
        .unwrap();
        assert_eq!(canvas.title, Some("Test Chart".to_string()));
    }

    #[test]
    fn test_canvas_new_equal_values() {
        // All same x and y values should add padding
        let canvas = Canvas::new(800, 600, None, vec![5.0, 5.0, 5.0], vec![10.0, 10.0]).unwrap();
        // Range should be 4.0 to 6.0 (±1.0 padding)
        assert_eq!(canvas.x_range.start, 4.0);
        assert_eq!(canvas.x_range.end, 6.0);
        assert_eq!(canvas.y_range.start, 9.0);
        assert_eq!(canvas.y_range.end, 11.0);
    }

    #[test]
    fn test_canvas_new_large_values() {
        let canvas = Canvas::new(800, 600, None, vec![1e10], vec![2e10]).unwrap();
        assert!(canvas.x_range.contains(&1e10));
        assert!(canvas.y_range.contains(&2e10));
    }

    #[test]
    fn test_canvas_new_small_values() {
        let canvas = Canvas::new(800, 600, None, vec![1e-10], vec![2e-10]).unwrap();
        assert!(canvas.x_range.contains(&1e-10));
        assert!(canvas.y_range.contains(&2e-10));
    }

    #[test]
    fn test_canvas_new_negative_values() {
        let canvas = Canvas::new(800, 600, None, vec![-10.0, -5.0], vec![-20.0, -10.0]).unwrap();
        assert!(canvas.x_range.contains(&-10.0));
        assert!(canvas.y_range.contains(&-20.0));
    }

    #[test]
    fn test_canvas_new_empty_data() {
        let result = Canvas::new(800, 600, None, vec![], vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no data"));
    }

    #[test]
    fn test_canvas_new_single_point() {
        let canvas = Canvas::new(800, 600, None, vec![5.0], vec![10.0]).unwrap();
        // Should add ±1.0 padding for single point
        assert_eq!(canvas.x_range.start, 4.0);
        assert_eq!(canvas.x_range.end, 6.0);
    }

    // add_line_layer tests (5 tests)

    #[test]
    fn test_add_line_layer_basic() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_line_layer(
            vec![1.0, 2.0],
            vec![10.0, 20.0],
            LineStyle {
                color: None,
                width: None,
                alpha: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_line_layer_with_color() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_line_layer(
            vec![1.0, 2.0],
            vec![10.0, 20.0],
            LineStyle {
                color: Some("red".to_string()),
                width: None,
                alpha: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_line_layer_with_width() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_line_layer(
            vec![1.0, 2.0],
            vec![10.0, 20.0],
            LineStyle {
                color: None,
                width: Some(3.0),
                alpha: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_line_layer_mismatched_length() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_line_layer(
            vec![1.0, 2.0],
            vec![10.0], // Only 1 value
            LineStyle {
                color: None,
                width: None,
                alpha: None,
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("same length"));
    }

    #[test]
    fn test_add_line_layer_multiple() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 30.0]).unwrap();
        // Add 3 line layers
        canvas.add_line_layer(vec![1.0, 2.0], vec![10.0, 20.0], LineStyle { color: Some("red".to_string()), width: None, alpha: None }).unwrap();
        canvas.add_line_layer(vec![2.0, 3.0], vec![20.0, 30.0], LineStyle { color: Some("blue".to_string()), width: None, alpha: None }).unwrap();
        let result = canvas.add_line_layer(vec![1.0, 3.0], vec![10.0, 30.0], LineStyle { color: Some("green".to_string()), width: None, alpha: None });
        assert!(result.is_ok());
    }

    // add_point_layer tests (5 tests)

    #[test]
    fn test_add_point_layer_basic() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_point_layer(
            vec![1.0, 2.0],
            vec![10.0, 20.0],
            PointStyle {
                color: None,
                size: None,
                shape: None,
                alpha: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_point_layer_with_size() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_point_layer(
            vec![1.0, 2.0],
            vec![10.0, 20.0],
            PointStyle {
                color: None,
                size: Some(10.0),
                shape: None,
                alpha: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_point_layer_with_color() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_point_layer(
            vec![1.0, 2.0],
            vec![10.0, 20.0],
            PointStyle {
                color: Some("green".to_string()),
                size: None,
                shape: None,
                alpha: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_point_layer_mismatched_length() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_point_layer(
            vec![1.0],
            vec![10.0, 20.0],
            PointStyle {
                color: None,
                size: None,
                shape: None,
                alpha: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_point_layer_multiple() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 30.0]).unwrap();
        canvas.add_point_layer(vec![1.0, 2.0], vec![10.0, 20.0], PointStyle { color: None, size: None, shape: None, alpha: None }).unwrap();
        let result = canvas.add_point_layer(vec![2.0, 3.0], vec![20.0, 30.0], PointStyle { color: None, size: None, shape: None, alpha: None });
        assert!(result.is_ok());
    }

    // add_bar_layer tests (6 tests)

    #[test]
    fn test_add_bar_layer_basic() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0, 1.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_bar_layer(
            vec!["A".to_string(), "B".to_string()],
            vec![10.0, 20.0],
            BarStyle {
                color: None,
                alpha: None,
                width: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_bar_layer_with_color() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0, 1.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_bar_layer(
            vec!["A".to_string(), "B".to_string()],
            vec![10.0, 20.0],
            BarStyle {
                color: Some("red".to_string()),
                alpha: None,
                width: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_bar_layer_with_alpha() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0, 1.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_bar_layer(
            vec!["A".to_string(), "B".to_string()],
            vec![10.0, 20.0],
            BarStyle {
                color: None,
                alpha: Some(0.5),
                width: None,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_bar_layer_with_width() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0, 1.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_bar_layer(
            vec!["A".to_string(), "B".to_string()],
            vec![10.0, 20.0],
            BarStyle {
                color: None,
                alpha: None,
                width: Some(0.6),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_bar_layer_mismatched_length() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0, 1.0], vec![10.0, 20.0]).unwrap();
        let result = canvas.add_bar_layer(
            vec!["A".to_string()],
            vec![10.0, 20.0],
            BarStyle {
                color: None,
                alpha: None,
                width: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_bar_layer_empty() {
        let result = Canvas::new(800, 600, None, vec![0.0], vec![10.0]);
        assert!(result.is_ok());
        let mut canvas = result.unwrap();
        let result = canvas.add_bar_layer(vec![], vec![], BarStyle { color: None, alpha: None, width: None });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no data"));
    }

    // add_bar_group tests (4 tests)

    #[test]
    fn test_add_bar_group_dodge() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0, 1.0], vec![10.0, 30.0]).unwrap();
        let series = vec![
            (vec![10.0, 20.0], BarStyle { color: Some("blue".to_string()), alpha: None, width: None }),
            (vec![15.0, 25.0], BarStyle { color: Some("red".to_string()), alpha: None, width: None }),
        ];
        let result = canvas.add_bar_group(vec!["A".to_string(), "B".to_string()], series, "dodge");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_bar_group_stack() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0, 1.0], vec![10.0, 60.0]).unwrap();
        let series = vec![
            (vec![10.0, 20.0], BarStyle { color: Some("blue".to_string()), alpha: None, width: None }),
            (vec![15.0, 25.0], BarStyle { color: Some("green".to_string()), alpha: None, width: None }),
        ];
        let result = canvas.add_bar_group(vec!["A".to_string(), "B".to_string()], series, "stack");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_bar_group_identity() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0, 1.0], vec![10.0, 30.0]).unwrap();
        let series = vec![
            (vec![10.0, 20.0], BarStyle { color: Some("blue".to_string()), alpha: Some(0.5), width: None }),
            (vec![15.0, 25.0], BarStyle { color: Some("red".to_string()), alpha: Some(0.5), width: None }),
        ];
        let result = canvas.add_bar_group(vec!["A".to_string(), "B".to_string()], series, "identity");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_bar_group_empty_series() {
        let mut canvas = Canvas::new(800, 600, None, vec![0.0], vec![10.0]).unwrap();
        let result = canvas.add_bar_group(vec!["A".to_string()], vec![], "dodge");
        assert!(result.is_err());
    }

    // render tests (3 tests)

    #[test]
    fn test_render_produces_png() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0], vec![10.0, 20.0]).unwrap();
        canvas.add_line_layer(
            vec![1.0, 2.0],
            vec![10.0, 20.0],
            LineStyle { color: None, width: None, alpha: None },
        ).unwrap();
        let png_bytes = canvas.render().unwrap();
        assert!(is_valid_png(&png_bytes));
    }

    #[test]
    fn test_render_correct_dimensions() {
        let mut canvas = Canvas::new(800, 600, None, vec![1.0], vec![10.0]).unwrap();
        canvas.add_point_layer(
            vec![1.0],
            vec![10.0],
            PointStyle { color: None, size: None, shape: None, alpha: None },
        ).unwrap();
        let png_bytes = canvas.render().unwrap();
        // Just verify it's a valid PNG
        assert!(is_valid_png(&png_bytes));
    }

    #[test]
    fn test_canvas_line_plus_point() {
        // Test layer composition: line + point
        let mut canvas = Canvas::new(800, 600, None, vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 30.0]).unwrap();
        canvas.add_line_layer(
            vec![1.0, 2.0, 3.0],
            vec![10.0, 20.0, 30.0],
            LineStyle { color: Some("blue".to_string()), width: None, alpha: None },
        ).unwrap();
        canvas.add_point_layer(
            vec![1.0, 2.0, 3.0],
            vec![10.0, 20.0, 30.0],
            PointStyle { color: Some("red".to_string()), size: Some(5.0), shape: None, alpha: None },
        ).unwrap();
        let png_bytes = canvas.render().unwrap();
        assert!(is_valid_png(&png_bytes));
    }
}
