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
use crate::parser::lexer::{string_literal, ws};

pub fn parse_labs(input: &str) -> IResult<&str, Labels> {
    let (input, _) = ws(tag("labs"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    let (input, args) = separated_list0(
        ws(char(',')),
        alt((
            map(preceded(ws(tag("title:")), ws(string_literal)), |v| ("title", v)),
            map(preceded(ws(tag("subtitle:")), ws(string_literal)), |v| ("subtitle", v)),
            map(preceded(ws(tag("x:")), ws(string_literal)), |v| ("x", v)),
            map(preceded(ws(tag("y:")), ws(string_literal)), |v| ("y", v)),
            map(preceded(ws(tag("caption:")), ws(string_literal)), |v| ("caption", v)),
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
