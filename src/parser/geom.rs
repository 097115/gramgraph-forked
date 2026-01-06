// Geometry (geom) parser for Grammar of Graphics DSL

use super::ast::{BarLayer, BarPosition, Layer, LineLayer, PointLayer};
use super::lexer::{identifier, number_literal, string_literal, ws};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::map,
    multi::separated_list0,
    sequence::preceded,
    IResult,
};

/// Parse a line geometry
/// Format: line() or line(color: "red", width: 2, ...)
pub fn parse_line(input: &str) -> IResult<&str, Layer> {
    let (input, _) = ws(tag("line"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    // Parse optional named arguments
    let (input, args) = separated_list0(
        ws(char(',')),
        alt((
            map(
                preceded(ws(tag("x:")), ws(identifier)),
                |x| ("x", x, 0.0),
            ),
            map(
                preceded(ws(tag("y:")), ws(identifier)),
                |y| ("y", y, 0.0),
            ),
            map(
                preceded(ws(tag("color:")), ws(string_literal)),
                |c| ("color", c, 0.0),
            ),
            map(
                preceded(ws(tag("width:")), ws(number_literal)),
                |w| ("width", String::new(), w),
            ),
            map(
                preceded(ws(tag("alpha:")), ws(number_literal)),
                |a| ("alpha", String::new(), a),
            ),
        )),
    )(input)?;

    let (input, _) = ws(char(')'))(input)?;

    let mut layer = LineLayer::default();

    for (key, str_val, num_val) in args {
        match key {
            "x" => layer.x = Some(str_val),
            "y" => layer.y = Some(str_val),
            "color" => layer.color = Some(str_val),
            "width" => layer.width = Some(num_val),
            "alpha" => layer.alpha = Some(num_val),
            _ => {}
        }
    }

    Ok((input, Layer::Line(layer)))
}

/// Parse a point geometry
/// Format: point() or point(size: 5, color: "blue", ...)
pub fn parse_point(input: &str) -> IResult<&str, Layer> {
    let (input, _) = ws(tag("point"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    // Parse optional named arguments
    let (input, args) = separated_list0(
        ws(char(',')),
        alt((
            map(
                preceded(ws(tag("x:")), ws(identifier)),
                |x| ("x", x, 0.0),
            ),
            map(
                preceded(ws(tag("y:")), ws(identifier)),
                |y| ("y", y, 0.0),
            ),
            map(
                preceded(ws(tag("color:")), ws(string_literal)),
                |c| ("color", c, 0.0),
            ),
            map(
                preceded(ws(tag("size:")), ws(number_literal)),
                |s| ("size", String::new(), s),
            ),
            map(
                preceded(ws(tag("shape:")), ws(string_literal)),
                |sh| ("shape", sh, 0.0),
            ),
            map(
                preceded(ws(tag("alpha:")), ws(number_literal)),
                |a| ("alpha", String::new(), a),
            ),
        )),
    )(input)?;

    let (input, _) = ws(char(')'))(input)?;

    let mut layer = PointLayer::default();

    for (key, str_val, num_val) in args {
        match key {
            "x" => layer.x = Some(str_val),
            "y" => layer.y = Some(str_val),
            "color" => layer.color = Some(str_val),
            "size" => layer.size = Some(num_val),
            "shape" => layer.shape = Some(str_val),
            "alpha" => layer.alpha = Some(num_val),
            _ => {}
        }
    }

    Ok((input, Layer::Point(layer)))
}

/// Parse a bar geometry
/// Format: bar() or bar(color: "red", position: "dodge", ...)
pub fn parse_bar(input: &str) -> IResult<&str, Layer> {
    let (input, _) = ws(tag("bar"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    // Parse optional named arguments
    // We need to handle position specially as it's a string that maps to an enum
    let (input, args) = separated_list0(
        ws(char(',')),
        alt((
            map(
                preceded(ws(tag("x:")), ws(identifier)),
                |x| ("x", x, 0.0),
            ),
            map(
                preceded(ws(tag("y:")), ws(identifier)),
                |y| ("y", y, 0.0),
            ),
            map(
                preceded(ws(tag("color:")), ws(string_literal)),
                |c| ("color", c, 0.0),
            ),
            map(
                preceded(ws(tag("width:")), ws(number_literal)),
                |w| ("width", String::new(), w),
            ),
            map(
                preceded(ws(tag("alpha:")), ws(number_literal)),
                |a| ("alpha", String::new(), a),
            ),
            map(
                preceded(ws(tag("position:")), ws(string_literal)),
                |p| ("position", p, 0.0),
            ),
        )),
    )(input)?;

    let (input, _) = ws(char(')'))(input)?;

    let mut layer = BarLayer::default();

    for (key, str_val, num_val) in args {
        match key {
            "x" => layer.x = Some(str_val),
            "y" => layer.y = Some(str_val),
            "color" => layer.color = Some(str_val),
            "width" => layer.width = Some(num_val),
            "alpha" => layer.alpha = Some(num_val),
            "position" => {
                layer.position = match str_val.as_str() {
                    "dodge" => BarPosition::Dodge,
                    "stack" => BarPosition::Stack,
                    "identity" => BarPosition::Identity,
                    _ => BarPosition::Identity, // default for unknown values
                };
            }
            _ => {}
        }
    }

    Ok((input, Layer::Bar(layer)))
}

/// Parse any geometry layer
pub fn parse_geom(input: &str) -> IResult<&str, Layer> {
    alt((parse_line, parse_point, parse_bar))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_empty() {
        let result = parse_line("line()");
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Line(l) => {
                assert_eq!(l.color, None);
                assert_eq!(l.width, None);
            }
            _ => panic!("Expected Line layer"),
        }
    }

    #[test]
    fn test_parse_line_with_color() {
        let result = parse_line(r#"line(color: "red")"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Line(l) => {
                assert_eq!(l.color, Some("red".to_string()));
            }
            _ => panic!("Expected Line layer"),
        }
    }

    #[test]
    fn test_parse_point_with_size() {
        let result = parse_point("point(size: 5)");
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Point(p) => {
                assert_eq!(p.size, Some(5.0));
            }
            _ => panic!("Expected Point layer"),
        }
    }

    #[test]
    fn test_parse_bar_empty() {
        let result = parse_bar("bar()");
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Bar(b) => {
                assert_eq!(b.color, None);
                assert_eq!(b.alpha, None);
                assert_eq!(b.position, BarPosition::Identity);
            }
            _ => panic!("Expected Bar layer"),
        }
    }

    #[test]
    fn test_parse_bar_with_position() {
        let result = parse_bar(r#"bar(position: "dodge")"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Bar(b) => {
                assert_eq!(b.position, BarPosition::Dodge);
            }
            _ => panic!("Expected Bar layer"),
        }
    }

    #[test]
    fn test_parse_bar_with_stack_position() {
        let result = parse_bar(r#"bar(position: "stack")"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Bar(b) => {
                assert_eq!(b.position, BarPosition::Stack);
            }
            _ => panic!("Expected Bar layer"),
        }
    }

    #[test]
    fn test_parse_bar_with_color() {
        let result = parse_bar(r#"bar(color: "red")"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Bar(b) => {
                assert_eq!(b.color, Some("red".to_string()));
            }
            _ => panic!("Expected Bar layer"),
        }
    }

    #[test]
    fn test_parse_bar_full() {
        let result = parse_bar(r#"bar(position: "stack", color: "blue", alpha: 0.7, width: 0.6)"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Bar(b) => {
                assert_eq!(b.position, BarPosition::Stack);
                assert_eq!(b.color, Some("blue".to_string()));
                assert_eq!(b.alpha, Some(0.7));
                assert_eq!(b.width, Some(0.6));
            }
            _ => panic!("Expected Bar layer"),
        }
    }

    #[test]
    fn test_parse_line_all_params() {
        // Test line with all parameters: x, y, color, width, alpha
        let result = parse_line(r#"line(x: col1, y: col2, color: "red", width: 2, alpha: 0.5)"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Line(l) => {
                assert_eq!(l.x, Some("col1".to_string()));
                assert_eq!(l.y, Some("col2".to_string()));
                assert_eq!(l.color, Some("red".to_string()));
                assert_eq!(l.width, Some(2.0));
                assert_eq!(l.alpha, Some(0.5));
            }
            _ => panic!("Expected Line layer"),
        }
    }

    #[test]
    fn test_parse_point_all_params() {
        // Test point with all parameters: x, y, color, size, alpha
        let result = parse_point(r#"point(x: col1, y: col2, color: "blue", size: 10, alpha: 0.8)"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Point(p) => {
                assert_eq!(p.x, Some("col1".to_string()));
                assert_eq!(p.y, Some("col2".to_string()));
                assert_eq!(p.color, Some("blue".to_string()));
                assert_eq!(p.size, Some(10.0));
                assert_eq!(p.alpha, Some(0.8));
            }
            _ => panic!("Expected Point layer"),
        }
    }

    #[test]
    fn test_parse_geom_whitespace_variations() {
        // Extra spaces around parentheses and commas should be handled
        let result = parse_line(r#"  line ( color: "red" , width: 2 )  "#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Line(l) => {
                assert_eq!(l.color, Some("red".to_string()));
                assert_eq!(l.width, Some(2.0));
            }
            _ => panic!("Expected Line layer"),
        }
    }

    #[test]
    fn test_parse_bar_invalid_position() {
        // Unknown position value defaults to identity
        let result = parse_bar(r#"bar(position: "unknown")"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Bar(b) => {
                assert_eq!(b.position, BarPosition::Identity); // Should default
            }
            _ => panic!("Expected Bar layer"),
        }
    }

    #[test]
    fn test_parse_geom_multiple_params() {
        // Test that multiple parameters work correctly
        let result = parse_line(r#"line(color: "red", width: 2)"#);
        assert!(result.is_ok());
        let (_, layer) = result.unwrap();
        match layer {
            Layer::Line(l) => {
                assert_eq!(l.color, Some("red".to_string()));
                assert_eq!(l.width, Some(2.0));
            }
            _ => panic!("Expected Line layer"),
        }
    }
}
