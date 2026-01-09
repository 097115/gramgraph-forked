// Pipeline parser for Grammar of Graphics DSL

use super::aesthetics::parse_aesthetics;
use super::ast::PlotSpec;
use super::coord::parse_coord_flip;
use super::facet::parse_facet_wrap;
use super::geom::parse_geom;
use super::labels::parse_labs;
use super::scale::parse_scale_command;
use super::theme::parse_theme_command;
use super::lexer::ws;
use nom::{
    bytes::complete::tag,
    combinator::{eof, opt},
    multi::many0,
    IResult,
};

/// Parse a complete plot specification
/// Format: [aes(...) |] geom() | geom() | ...
pub fn parse_plot_spec(input: &str) -> IResult<&str, PlotSpec> {
    // Optional: consume leading "df"
    let (input, _) = opt(ws(tag("df")))(input)?;

    // If input starts with "|", consume it
    let (input, _) = opt(ws(tag("|")))(input)?;

    // Try to parse aesthetics (optional but recommended)
    let (input, aesthetics) = opt(parse_aesthetics)(input)?;

    // If we parsed aesthetics, consume the pipe separator
    let (input, _) = if aesthetics.is_some() {
        let (input, _) = ws(tag("|"))(input)?;
        (input, ())
    } else {
        (input, ())
    };

    // Parse first geometry (required)
    let (input, first_geom) = parse_geom(input)?;

    // Parse additional geometries
    let (input, mut remaining_geoms) = many0(|input| {
        let (input, _) = ws(tag("|"))(input)?;
        parse_geom(input)
    })(input)?;

    // Parse optional facet_wrap at the end
    let (input, facet) = opt(|input| {
        let (input, _) = ws(tag("|"))(input)?;
        parse_facet_wrap(input)
    })(input)?;

    // Parse optional coord_flip at the end
    let (input, coord) = opt(|input| {
        let (input, _) = ws(tag("|"))(input)?;
        parse_coord_flip(input)
    })(input)?;

    // Parse optional labs
    let (input, labels) = opt(|input| {
        let (input, _) = ws(tag("|"))(input)?;
        parse_labs(input)
    })(input)?;

    // Parse optional theme
    let (input, theme) = opt(|input| {
        let (input, _) = ws(tag("|"))(input)?;
        parse_theme_command(input)
    })(input)?;

    // Parse optional scales
    let (input, scales) = many0(|input| {
        let (input, _) = ws(tag("|"))(input)?;
        parse_scale_command(input)
    })(input)?;

    // Consume trailing whitespace and ensure end of input
    let (input, _) = ws(eof)(input)?;

    // Build layers vec
    let mut layers = vec![first_geom];
    layers.append(&mut remaining_geoms);

    let mut x_scale = None;
    let mut y_scale = None;
    for (is_x, s) in scales {
        if is_x { x_scale = Some(s); }
        else { y_scale = Some(s); }
    }

    Ok((
        input,
        PlotSpec {
            aesthetics,
            layers,
            labels,
            facet,
            coord,
            theme,
            x_scale,
            y_scale,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_aes_and_line() {
        let result = parse_plot_spec("aes(x: time, y: temp) | line()");
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.aesthetics.is_some());
        assert_eq!(spec.layers.len(), 1);
    }

    #[test]
    fn test_parse_multiple_layers() {
        let result = parse_plot_spec(r#"aes(x: one, y: two) | line(color: "red") | point(size: 5)"#);
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.aesthetics.is_some());
        assert_eq!(spec.layers.len(), 2);
    }

    #[test]
    fn test_parse_no_aesthetics() {
        // Allow geoms without explicit aes for backward compat / convenience
        let result = parse_plot_spec("line()");
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.aesthetics.is_none());
        assert_eq!(spec.layers.len(), 1);
    }

    #[test]
    fn test_parse_with_df_prefix() {
        let result = parse_plot_spec("df | aes(x: a, y: b) | line()");
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.aesthetics.is_some());
        assert_eq!(spec.layers.len(), 1);
    }

    #[test]
    fn test_parse_plot_spec_trailing_pipe() {
        // Trailing pipe should fail (nothing after last pipe)
        assert!(parse_plot_spec("aes(x: a, y: b) | line() |").is_err());
    }

    #[test]
    fn test_parse_plot_spec_missing_geom() {
        // Aesthetics without any geometry should fail (needs at least one geom)
        assert!(parse_plot_spec("aes(x: a, y: b)").is_err());
    }

    #[test]
    fn test_parse_plot_spec_empty_input() {
        // Empty input should fail
        assert!(parse_plot_spec("").is_err());
    }

    #[test]
    fn test_parse_plot_spec_three_layers() {
        // Three layers: line + point + bar
        let result = parse_plot_spec(r#"aes(x: a, y: b) | line() | point() | bar()"#);
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert_eq!(spec.layers.len(), 3);
    }

    #[test]
    fn test_parse_plot_spec_df_without_aes() {
        // df prefix without aesthetics should succeed
        let result = parse_plot_spec("df | line()");
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.aesthetics.is_none());
        assert_eq!(spec.layers.len(), 1);
    }

    #[test]
    fn test_parse_plot_spec_with_facet_wrap() {
        let result = parse_plot_spec("aes(x: time, y: sales) | line() | facet_wrap(by: region)");
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.facet.is_some());
        let facet = spec.facet.unwrap();
        assert_eq!(facet.by, "region");
    }

    #[test]
    fn test_parse_plot_spec_with_facet_wrap_full() {
        let result = parse_plot_spec(r#"aes(x: time, y: sales) | line() | facet_wrap(by: region, ncol: Some(2), scales: "free_x")"#);
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.facet.is_some());
        let facet = spec.facet.unwrap();
        assert_eq!(facet.by, "region");
        assert_eq!(facet.ncol, Some(2));
    }

    #[test]
    fn test_parse_plot_spec_without_facet() {
        let result = parse_plot_spec("aes(x: time, y: sales) | line()");
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.facet.is_none());
    }

    #[test]
    fn test_parse_plot_spec_with_labs_and_theme() {
        let result = parse_plot_spec(r#"aes(x: x, y: y) | line() | labs(title: "My Plot", x: "Time") | theme(legend_position: "none")"#);
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert_eq!(spec.labels.as_ref().unwrap().title, Some("My Plot".to_string()));
        assert_eq!(spec.labels.as_ref().unwrap().x, Some("Time".to_string()));
        assert_eq!(spec.theme.as_ref().unwrap().legend_position, crate::parser::ast::LegendPosition::None);
    }

    #[test]
    fn test_parse_histogram_pipeline() {
        let input = r#"aes(x: value) | histogram(bins: 5) | labs(title: "Distribution", x: "Value", y: "Count") | theme_minimal()"#;
        let result = parse_plot_spec(input);
        match &result {
            Ok(_) => println!("Parsed successfully"),
            Err(e) => println!("Parse error: {:?}", e),
        }
        assert!(result.is_ok());
        let (_, spec) = result.unwrap();
        assert!(spec.aesthetics.is_some());
        assert_eq!(spec.layers.len(), 1);
        if let crate::parser::ast::Layer::Bar(b) = &spec.layers[0] {
             match b.stat {
                 crate::parser::ast::Stat::Bin { bins } => assert_eq!(bins, 5),
                 _ => panic!("Expected Bin stat"),
             }
        } else {
            panic!("Expected Bar layer (histogram)");
        }
    }
}
