// Pipeline parser for Grammar of Graphics DSL

use super::aesthetics::parse_aesthetics;
use super::ast::{Layer, PlotSpec};
use super::geom::parse_geom;
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

    // Consume trailing whitespace and ensure end of input
    let (input, _) = ws(eof)(input)?;

    // Build layers vec
    let mut layers = vec![first_geom];
    layers.append(&mut remaining_geoms);

    Ok((
        input,
        PlotSpec {
            aesthetics,
            layers,
            labels: None, // Future: parse labs() command
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
}
