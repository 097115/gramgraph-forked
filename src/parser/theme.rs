use nom::{
    bytes::complete::tag,
    character::complete::char,
    multi::separated_list0,
    branch::alt,
    combinator::map,
    sequence::preceded,
    IResult,
};
use crate::parser::ast::{Theme, LegendPosition};
use crate::parser::lexer::{identifier, string_literal, ws};

pub fn parse_theme_minimal(input: &str) -> IResult<&str, Theme> {
    let (input, _) = ws(tag("theme_minimal"))(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, _) = ws(char(')'))(input)?;
    
    Ok((input, Theme {
        background_color: Some("white".to_string()),
        grid_visible: true,
        font_family: Some("sans-serif".to_string()),
        legend_position: LegendPosition::Right,
    }))
}

pub fn parse_theme(input: &str) -> IResult<&str, Theme> {
    let (input, _) = ws(tag("theme"))(input)?;
    let (input, _) = ws(char('('))(input)?;

    let (input, args) = separated_list0(
        ws(char(',')),
        alt((
            map(preceded(ws(tag("legend_position:")), ws(string_literal)), |v| ("legend_position", v)),
        ))
    )(input)?;

    let (input, _) = ws(char(')'))(input)?;

    let mut theme = Theme::default();
    for (key, val) in args {
        match key {
            "legend_position" => {
                theme.legend_position = match val.as_str() {
                    "right" => LegendPosition::Right,
                    "left" => LegendPosition::Left,
                    "top" => LegendPosition::Top,
                    "bottom" => LegendPosition::Bottom,
                    "none" => LegendPosition::None,
                    _ => LegendPosition::Right,
                };
            }
            _ => {}
        }
    }

    Ok((input, theme))
}

pub fn parse_theme_command(input: &str) -> IResult<&str, Theme> {
    alt((parse_theme_minimal, parse_theme))(input)
}
