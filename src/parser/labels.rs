use nom::{
    bytes::complete::tag,
    character::complete::char,
    multi::separated_list0,
    branch::alt,
    combinator::map,
    sequence::preceded,
    IResult,
};
use crate::parser::ast::Labels;
use crate::parser::lexer::{string_literal, variable_reference, ws};

pub fn parse_labs(input: &str) -> IResult<&str, Labels> {
    let (input, _) = ws(tag("labs"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    let (input, args) = separated_list0(
        ws(char(',')),
        alt((
            // title: can be "literal" or $var
            map(preceded(ws(tag("title:")), ws(string_literal)), |v| ("title", v)),
            map(preceded(ws(tag("title:")), ws(variable_reference)), |v| ("title", format!("${}", v))),
            // subtitle: can be "literal" or $var
            map(preceded(ws(tag("subtitle:")), ws(string_literal)), |v| ("subtitle", v)),
            map(preceded(ws(tag("subtitle:")), ws(variable_reference)), |v| ("subtitle", format!("${}", v))),
            // x: can be "literal" or $var
            map(preceded(ws(tag("x:")), ws(string_literal)), |v| ("x", v)),
            map(preceded(ws(tag("x:")), ws(variable_reference)), |v| ("x", format!("${}", v))),
            // y: can be "literal" or $var
            map(preceded(ws(tag("y:")), ws(string_literal)), |v| ("y", v)),
            map(preceded(ws(tag("y:")), ws(variable_reference)), |v| ("y", format!("${}", v))),
            // caption: can be "literal" or $var
            map(preceded(ws(tag("caption:")), ws(string_literal)), |v| ("caption", v)),
            map(preceded(ws(tag("caption:")), ws(variable_reference)), |v| ("caption", format!("${}", v))),
        ))
    )(input)?;

    let (input, _) = ws(char(')'))(input)?;

    let mut labels = Labels::default();
    for (key, val) in args {
        match key {
            "title" => labels.title = Some(val),
            "subtitle" => labels.subtitle = Some(val),
            "x" => labels.x = Some(val),
            "y" => labels.y = Some(val),
            "caption" => labels.caption = Some(val),
            _ => {}
        }
    }

    Ok((input, labels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_labs_with_variables() {
        let result = parse_labs("labs(title: $chart_title, x: $x_label)");
        assert!(result.is_ok());
        let (_, labels) = result.unwrap();
        assert_eq!(labels.title, Some("$chart_title".to_string()));
        assert_eq!(labels.x, Some("$x_label".to_string()));
    }

    #[test]
    fn test_parse_labs_mixed() {
        let result = parse_labs(r#"labs(title: "My Chart", y: $y_label)"#);
        assert!(result.is_ok());
        let (_, labels) = result.unwrap();
        assert_eq!(labels.title, Some("My Chart".to_string()));
        assert_eq!(labels.y, Some("$y_label".to_string()));
    }
}
