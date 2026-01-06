// Command parser for PlotPipe DSL

use super::ast::Command;
use super::lexer::{identifier, number_literal, string_literal, ws};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::{map, opt},
    multi::separated_list0,
    sequence::preceded,
    IResult,
};

/// Parse a chart command
/// Format: chart(x: col, y: col) or chart(x: col, y: col, title: "...")
pub fn parse_chart(input: &str) -> IResult<&str, Command> {
    let (input, _) = ws(tag("chart"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    // Parse x: column
    let (input, _) = ws(tag("x:"))(input)?;
    let (input, x_col) = ws(identifier)(input)?;
    let (input, _) = ws(char(','))(input)?;

    // Parse y: column
    let (input, _) = ws(tag("y:"))(input)?;
    let (input, y_col) = ws(identifier)(input)?;

    // Parse optional title: "..."
    let (input, title) = opt(preceded(
        ws(char(',')),
        preceded(ws(tag("title:")), ws(string_literal)),
    ))(input)?;

    let (input, _) = ws(char(')'))(input)?;

    Ok((
        input,
        Command::Chart {
            x: x_col,
            y: y_col,
            title,
        },
    ))
}

/// Parse a layer_line command
/// Format: layer_line() or layer_line(color: "red") or layer_line(color: "red", stroke: 2)
pub fn parse_layer_line(input: &str) -> IResult<&str, Command> {
    let (input, _) = ws(tag("layer_line"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    // Parse optional named arguments
    let (input, args) = separated_list0(
        ws(char(',')),
        alt((
            map(
                preceded(ws(tag("color:")), ws(string_literal)),
                |c| ("color", c, 0),
            ),
            map(
                preceded(ws(tag("stroke:")), ws(number_literal)),
                |n| ("stroke", String::new(), n as u32),
            ),
        )),
    )(input)?;

    let (input, _) = ws(char(')'))(input)?;

    let mut color = None;
    let mut stroke = None;

    for (key, str_val, num_val) in args {
        match key {
            "color" => color = Some(str_val),
            "stroke" => stroke = Some(num_val),
            _ => {}
        }
    }

    Ok((input, Command::LayerLine { color, stroke }))
}

/// Parse a layer_point command
/// Format: layer_point() or layer_point(size: 5) or layer_point(shape: "circle", size: 5)
pub fn parse_layer_point(input: &str) -> IResult<&str, Command> {
    let (input, _) = ws(tag("layer_point"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    // Parse optional named arguments
    let (input, args) = separated_list0(
        ws(char(',')),
        alt((
            map(
                preceded(ws(tag("shape:")), ws(string_literal)),
                |s| ("shape", s, 0),
            ),
            map(
                preceded(ws(tag("size:")), ws(number_literal)),
                |n| ("size", String::new(), n as u32),
            ),
            map(
                preceded(ws(tag("color:")), ws(string_literal)),
                |c| ("color", c, 0),
            ),
        )),
    )(input)?;

    let (input, _) = ws(char(')'))(input)?;

    let mut shape = None;
    let mut size = None;
    let mut color = None;

    for (key, str_val, num_val) in args {
        match key {
            "shape" => shape = Some(str_val),
            "size" => size = Some(num_val),
            "color" => color = Some(str_val),
            _ => {}
        }
    }

    Ok((
        input,
        Command::LayerPoint { shape, size, color },
    ))
}

/// Parse any command
pub fn parse_command(input: &str) -> IResult<&str, Command> {
    alt((parse_chart, parse_layer_line, parse_layer_point))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chart() {
        let result = parse_chart("chart(x: time, y: temp)");
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        match cmd {
            Command::Chart { x, y, title } => {
                assert_eq!(x, "time");
                assert_eq!(y, "temp");
                assert_eq!(title, None);
            }
            _ => panic!("Expected Chart command"),
        }
    }

    #[test]
    fn test_parse_chart_with_title() {
        let result = parse_chart(r#"chart(x: time, y: temp, title: "Test")"#);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        match cmd {
            Command::Chart { x, y, title } => {
                assert_eq!(x, "time");
                assert_eq!(y, "temp");
                assert_eq!(title, Some("Test".to_string()));
            }
            _ => panic!("Expected Chart command"),
        }
    }

    #[test]
    fn test_parse_layer_line() {
        let result = parse_layer_line(r#"layer_line(color: "red")"#);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        match cmd {
            Command::LayerLine { color, stroke } => {
                assert_eq!(color, Some("red".to_string()));
                assert_eq!(stroke, None);
            }
            _ => panic!("Expected LayerLine command"),
        }
    }

    #[test]
    fn test_parse_layer_point() {
        let result = parse_layer_point("layer_point(size: 5)");
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        match cmd {
            Command::LayerPoint { shape, size, color } => {
                assert_eq!(shape, None);
                assert_eq!(size, Some(5));
                assert_eq!(color, None);
            }
            _ => panic!("Expected LayerPoint command"),
        }
    }
}
