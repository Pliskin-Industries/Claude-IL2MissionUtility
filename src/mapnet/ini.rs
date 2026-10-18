//! Editor road-system polylines (`highways.ini`, `railroads.ini`).
//!
//! A geometry line is `#id M material` then image coordinates `x,y`.
//! Tokens that are not coordinates are tags, not positions:
//! `B` and `T` mark a bridge gap but the centerline stays continuous,
//! `S` and `N` mark a vertex, and `L` plus a following length in metres
//! is a rail segment length. Anything else is ignored. Points are kept.
//!
//! Image origin is the top-left of the 9984 square. `svg_to_world` turns
//! that into game X/Z. `Highways_LaneOffset` is a render offset and is
//! not applied here.

use nom::branch::alt;
use nom::bytes::complete::{take_while, take_while1};
use nom::character::complete::{char, digit1, line_ending, space0, space1};
use nom::combinator::{map, map_res, opt, recognize, value};
use nom::multi::many0;
use nom::sequence::{preceded, separated_pair, terminated};
use nom::{IResult, Parser};

/// Image-space polylines, in file order. A line with fewer than two
/// points is not a segment, so it is left out.
pub(super) fn parse_polylines(text: &str) -> Vec<Vec<(f64, f64)>> {
    match polyline_file(text) {
        Ok((_, lines)) => lines,
        Err(_) => Vec::new(),
    }
}

fn polyline_file(input: &str) -> IResult<&str, Vec<Vec<(f64, f64)>>> {
    let (input, items) = many0(line_record).parse(input)?;
    let lines = items
        .into_iter()
        .flatten()
        .filter(|pts| pts.len() >= 2)
        .collect();
    Ok((input, lines))
}

/// One physical line: a polyline, a skipped row, or a blank line.
fn line_record(input: &str) -> IResult<&str, Option<Vec<(f64, f64)>>> {
    alt((
        map(terminated(geometry_line, opt(line_ending)), Some),
        map(
            terminated(
                take_while1(|c: char| c != '\n' && c != '\r'),
                opt(line_ending),
            ),
            |_| None,
        ),
        value(None, line_ending),
    ))
    .parse(input)
}

fn geometry_line(input: &str) -> IResult<&str, Vec<(f64, f64)>> {
    let (input, _) = (char('#'), digit1, space1, char('M'), space1, digit1).parse(input)?;
    let (input, tokens) = many0(preceded(space1, token)).parse(input)?;
    let (input, _) = space0.parse(input)?;
    // Leftover non-coordinates stay on this line; they are not points.
    let (input, _) = take_while(|c: char| c != '\n' && c != '\r').parse(input)?;
    let pts = tokens.into_iter().flatten().collect();
    Ok((input, pts))
}

/// A coordinate, or something to skip. Never fails on a non-empty word,
/// so an unknown tag cannot drop the points around it.
fn token(input: &str) -> IResult<&str, Option<(f64, f64)>> {
    alt((
        map(coord, Some),
        map(take_while1(|c: char| !c.is_whitespace()), |_| None),
    ))
    .parse(input)
}

fn coord(input: &str) -> IResult<&str, (f64, f64)> {
    separated_pair(ini_float, char(','), ini_float).parse(input)
}

fn ini_float(input: &str) -> IResult<&str, f64> {
    map_res(
        recognize((opt(char('-')), digit1, opt((char('.'), digit1)))),
        |s: &str| s.parse::<f64>(),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::parse_polylines;

    #[test]
    fn keeps_centerline_through_bridge_tags() {
        let text = "\
#2780 M 6 1.0,2.0 B S 3.5,4.5 T 5.0,6.0 N 7.0,8.0 L 450.0000 9.0,10.0 QQ 11.0,12.0\r\n\
#9 M 2 8.0,8.0\r\n\
// not geometry\r\n\
\r\n\
#2 M 1 0.0,0.0 1.0,1.0\r\n";
        let lines = parse_polylines(text);
        assert_eq!(
            lines.len(),
            2,
            "a one-point row and a comment are not polylines"
        );
        assert_eq!(
            lines[0],
            vec![
                (1.0, 2.0),
                (3.5, 4.5),
                (5.0, 6.0),
                (7.0, 8.0),
                (9.0, 10.0),
                (11.0, 12.0),
            ]
        );
        assert_eq!(lines[1], vec![(0.0, 0.0), (1.0, 1.0)]);
    }
}
