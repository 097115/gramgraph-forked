// Geometry (geom) parser for Grammar of Graphics DSL

use super::ast::{Layer, LineLayer, PointLayer};
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

/// Parse any geometry layer
pub fn parse_geom(input: &str) -> IResult<&str, Layer> {
    alt((parse_line, parse_point))(input)
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
}
