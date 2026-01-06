// Pipeline parser for PlotPipe DSL

use super::ast::Pipeline;
use super::command::parse_command;
use super::lexer::ws;
use nom::{
    bytes::complete::tag,
    combinator::{eof, opt},
    multi::separated_list1,
    IResult,
};

/// Parse a complete pipeline
/// Format: [df |] command | command | ...
pub fn parse_pipeline(input: &str) -> IResult<&str, Pipeline> {
    // Optional: consume leading "df"
    let (input, _) = opt(ws(tag("df")))(input)?;

    // If input starts with "|", consume it
    let (input, _) = opt(ws(tag("|")))(input)?;

    // Parse commands separated by pipes
    let (input, commands) = separated_list1(ws(tag("|")), parse_command)(input)?;

    // Consume trailing whitespace and ensure end of input
    let (input, _) = ws(eof)(input)?;

    Ok((input, Pipeline { commands }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_command() {
        let result = parse_pipeline("chart(x: one, y: two)");
        assert!(result.is_ok());
        let (_, pipeline) = result.unwrap();
        assert_eq!(pipeline.commands.len(), 1);
    }

    #[test]
    fn test_parse_two_commands() {
        let result = parse_pipeline(r#"chart(x: one, y: two) | layer_line(color: "red")"#);
        assert!(result.is_ok());
        let (_, pipeline) = result.unwrap();
        assert_eq!(pipeline.commands.len(), 2);
    }

    #[test]
    fn test_parse_with_df() {
        let result = parse_pipeline(r#"df | chart(x: one, y: two)"#);
        assert!(result.is_ok());
        let (_, pipeline) = result.unwrap();
        assert_eq!(pipeline.commands.len(), 1);
    }

    #[test]
    fn test_parse_with_leading_pipe() {
        let result = parse_pipeline(r#"| chart(x: one, y: two)"#);
        assert!(result.is_ok());
        let (_, pipeline) = result.unwrap();
        assert_eq!(pipeline.commands.len(), 1);
    }

    #[test]
    fn test_parse_complex_pipeline() {
        let result = parse_pipeline(
            r#"chart(x: one, y: two, title: "Test") | layer_line(color: "red") | layer_point(size: 5)"#
        );
        assert!(result.is_ok());
        let (_, pipeline) = result.unwrap();
        assert_eq!(pipeline.commands.len(), 3);
    }

    #[test]
    fn test_parse_with_whitespace() {
        let result = parse_pipeline("  chart(x: one, y: two)  |  layer_line(color: \"red\")  ");
        assert!(result.is_ok());
        let (_, pipeline) = result.unwrap();
        assert_eq!(pipeline.commands.len(), 2);
    }
}
