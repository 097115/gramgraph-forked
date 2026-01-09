use nom::{
    bytes::complete::tag,
    character::complete::char,
    branch::alt,
    combinator::{map, opt},
    sequence::{preceded, delimited},
    IResult,
};
use crate::parser::ast::{AxisScale, ScaleType};
use crate::parser::lexer::{number_literal, ws};

pub fn parse_scale_x_log10(input: &str) -> IResult<&str, AxisScale> {
    let (input, _) = ws(tag("scale_x_log10"))(input)?;
    let (input, _) = delimited(tag("("), ws(tag("")), tag(")"))(input)?;
    Ok((input, AxisScale { scale_type: ScaleType::Log10, limits: None }))
}

pub fn parse_scale_y_log10(input: &str) -> IResult<&str, AxisScale> {
    let (input, _) = ws(tag("scale_y_log10"))(input)?;
    let (input, _) = delimited(tag("("), ws(tag("")), tag(")"))(input)?;
    Ok((input, AxisScale { scale_type: ScaleType::Log10, limits: None }))
}

pub fn parse_scale_x_reverse(input: &str) -> IResult<&str, AxisScale> {
    let (input, _) = ws(tag("scale_x_reverse"))(input)?;
    let (input, _) = delimited(tag("("), ws(tag("")), tag(")"))(input)?;
    Ok((input, AxisScale { scale_type: ScaleType::Reverse, limits: None }))
}

pub fn parse_scale_y_reverse(input: &str) -> IResult<&str, AxisScale> {
    let (input, _) = ws(tag("scale_y_reverse"))(input)?;
    let (input, _) = delimited(tag("("), ws(tag("")), tag(")"))(input)?;
    Ok((input, AxisScale { scale_type: ScaleType::Reverse, limits: None }))
}

pub fn parse_xlim(input: &str) -> IResult<&str, AxisScale> {
    let (input, _) = ws(tag("xlim"))(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, min) = ws(number_literal)(input)?;
    let (input, _) = ws(char(','))(input)?;
    let (input, max) = ws(number_literal)(input)?;
    let (input, _) = ws(char(')'))(input)?;
    Ok((input, AxisScale { scale_type: ScaleType::Linear, limits: Some((min, max)) }))
}

pub fn parse_ylim(input: &str) -> IResult<&str, AxisScale> {
    let (input, _) = ws(tag("ylim"))(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, min) = ws(number_literal)(input)?;
    let (input, _) = ws(char(','))(input)?;
    let (input, max) = ws(number_literal)(input)?;
    let (input, _) = ws(char(')'))(input)?;
    Ok((input, AxisScale { scale_type: ScaleType::Linear, limits: Some((min, max)) }))
}

pub fn parse_scale_command(input: &str) -> IResult<&str, (bool, AxisScale)> {
    alt((
        map(parse_scale_x_log10, |s| (true, s)),
        map(parse_scale_y_log10, |s| (false, s)),
        map(parse_scale_x_reverse, |s| (true, s)),
        map(parse_scale_y_reverse, |s| (false, s)),
        map(parse_xlim, |s| (true, s)),
        map(parse_ylim, |s| (false, s)),
    ))(input)
}
