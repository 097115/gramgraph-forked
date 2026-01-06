// Aesthetics parser for Grammar of Graphics DSL

use super::ast::Aesthetics;
use super::lexer::{identifier, ws};
use nom::{
    bytes::complete::tag,
    character::complete::char,
    sequence::preceded,
    IResult,
};

/// Parse aesthetics specification
/// Format: aes(x: col, y: col)
pub fn parse_aesthetics(input: &str) -> IResult<&str, Aesthetics> {
    let (input, _) = ws(tag("aes"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    // Parse x: column
    let (input, _) = ws(tag("x:"))(input)?;
    let (input, x_col) = ws(identifier)(input)?;
    let (input, _) = ws(char(','))(input)?;

    // Parse y: column
    let (input, _) = ws(tag("y:"))(input)?;
    let (input, y_col) = ws(identifier)(input)?;

    let (input, _) = ws(char(')'))(input)?;

    Ok((input, Aesthetics { x: x_col, y: y_col }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_aesthetics() {
        let result = parse_aesthetics("aes(x: time, y: temp)");
        assert!(result.is_ok());
        let (_, aes) = result.unwrap();
        assert_eq!(aes.x, "time");
        assert_eq!(aes.y, "temp");
    }

    #[test]
    fn test_parse_aesthetics_with_whitespace() {
        let result = parse_aesthetics("  aes( x: time , y: temp )  ");
        assert!(result.is_ok());
        let (_, aes) = result.unwrap();
        assert_eq!(aes.x, "time");
        assert_eq!(aes.y, "temp");
    }
}
