//! Pure architectural chain construction from authoritative source points.
//! Source numbers are caller-owned stable identities, never render pieces.

use crate::jsmath::{PI, atan2, js_hypot};
use crate::ops::subref::Place;
use crate::vec2::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Openings,
    Overall,
    /// A façade's or a space's walls: thicknesses, the walls meeting it, a space's holes (docs/adr/0729).
    Walls,
    /// The grids across a façade or a space, between its outermost points (docs/adr/0729).
    Grids,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Openings {
    Ends,
    Centers,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    UnsupportedWall,
    OpeningUnplaced,
}

#[derive(Clone, Debug)]
pub struct Opening {
    pub source: usize,
    /// start-left, start-right, end-left, end-right, in source direction.
    pub jambs: [Vec2; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub source: usize,
    pub place: Place,
    pub at: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub row: Row,
    pub a: Point,
    pub b: Point,
    pub at: Vec2,
    pub angle: f64,
    pub offset: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub start: Vec2,
    pub direction: Vec2,
    pub normal: Vec2,
    pub length: f64,
}

impl Frame {
    pub fn new(line: [Vec2; 2]) -> Result<Self, Fault> {
        if !line.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
            return Err(Fault::UnsupportedWall);
        }
        let dx = line[1].x - line[0].x;
        let dy = line[1].y - line[0].y;
        let length = js_hypot(dx, dy);
        if !length.is_finite() || length < 1e-9 {
            return Err(Fault::UnsupportedWall);
        }
        let direction = Vec2::new(dx / length, dy / length);
        Ok(Self {
            start: line[0],
            normal: Vec2::new(-direction.y, direction.x),
            direction,
            length,
        })
    }

    pub fn offset(self, at: Vec2) -> f64 {
        (at.x - self.start.x) * self.normal.x + (at.y - self.start.y) * self.normal.y
    }

    pub fn at_offset(self, at: Vec2, offset: f64) -> Vec2 {
        let delta = offset - self.offset(at);
        Vec2::new(at.x + self.normal.x * delta, at.y + self.normal.y * delta)
    }

    fn along(self, p: Vec2) -> f64 {
        (p.x - self.start.x) * self.direction.x + (p.y - self.start.y) * self.direction.y
    }
}

/// The straight edge `edge` of a wall face line (`pts` one per path vertex, `bulges` one per vertex; docs/adr/0700):
/// its two ends and their vertex numbers; none when the line has no such edge or the edge is an arc.
pub fn edge_face(
    pts: &[Vec2],
    bulges: &[f64],
    closed: bool,
    edge: usize,
) -> Option<([Vec2; 2], [usize; 2])> {
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    if edge >= edges || bulges.get(edge).is_some_and(|b| *b != 0.0) {
        return None;
    }
    let next = (edge + 1) % n;
    Some(([pts[edge], pts[next]], [edge, next]))
}

/// The straight edges a wall path has, in order; arcs left out.
pub fn straight_edges(count: usize, bulges: &[f64], closed: bool) -> Vec<usize> {
    let edges = if closed { count } else { count.saturating_sub(1) };
    (0..edges)
        .filter(|&k| bulges.get(k).is_none_or(|b| *b == 0.0))
        .collect()
}

/// Whether an opening's jambs belong to the face edge `line`: their middle lies along the edge, within `reach` (the
/// wall's width) of the face. An opening on another edge of the same wall is left to that edge's chain.
pub fn on_edge(line: [Vec2; 2], jambs: &[Vec2; 4], reach: f64) -> bool {
    let Ok(frame) = Frame::new(line) else {
        return false;
    };
    let mid = Vec2::new(
        (jambs[0].x + jambs[1].x + jambs[2].x + jambs[3].x) * 0.25,
        (jambs[0].y + jambs[1].y + jambs[2].y + jambs[3].y) * 0.25,
    );
    let along = frame.along(mid);
    along >= -1e-9 && along <= frame.length + 1e-9 && frame.offset(mid).abs() <= reach + 1e-9
}

/// All dimensions share each row's world line even when a centre is not on its face. `ends` are the face line's
/// vertex numbers at the edge's two ends (`[0, 1]` for a two-point wall).
#[allow(clippy::too_many_arguments)]
pub fn plan(
    line: [Vec2; 2],
    part: usize,
    ends: [usize; 2],
    face: Face,
    openings: &[Opening],
    mode: Openings,
    rows: &[Row],
    at: Vec2,
    gap: f64,
    row_places: &[(Row, Vec2)],
) -> Result<Vec<Span>, Fault> {
    let frame = Frame::new(line)?;
    if !at.x.is_finite() || !at.y.is_finite() || !gap.is_finite() || gap <= 0.0 {
        return Err(Fault::UnsupportedWall);
    }
    let start = Point {
        source: 0,
        place: Place::Vertex {
            part,
            ring: 0,
            index: ends[0],
        },
        at: line[0],
    };
    let end = Point {
        source: 0,
        place: Place::Vertex {
            part,
            ring: 0,
            index: ends[1],
        },
        at: line[1],
    };
    let mut nodes = vec![start, end];
    if rows.contains(&Row::Openings) {
        for opening in openings {
            if !opening
                .jambs
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite())
            {
                return Err(Fault::OpeningUnplaced);
            }
            if mode == Openings::Ends {
                for index in if face == Face::Left { [0, 2] } else { [1, 3] } {
                    nodes.push(Point {
                        source: opening.source,
                        place: Place::Vertex {
                            part: 0,
                            ring: 0,
                            index,
                        },
                        at: opening.jambs[index],
                    });
                }
            } else {
                let mid =
                    |a: Vec2, b: Vec2| Vec2::new(a.x + (b.x - a.x) * 0.5, a.y + (b.y - a.y) * 0.5);
                nodes.push(Point {
                    source: opening.source,
                    place: Place::Center,
                    at: mid(
                        mid(opening.jambs[0], opening.jambs[1]),
                        mid(opening.jambs[2], opening.jambs[3]),
                    ),
                });
            }
        }
    }
    if nodes.iter().any(|p| {
        let t = frame.along(p.at);
        t < -1e-9 || t > frame.length + 1e-9
    }) {
        return Err(Fault::OpeningUnplaced);
    }
    nodes.sort_by(|a, b| {
        frame
            .along(a.at)
            .total_cmp(&frame.along(b.at))
            .then_with(|| (a.source != 0).cmp(&(b.source != 0)))
            .then_with(|| a.source.cmp(&b.source))
    });
    nodes.dedup_by(|a, b| frame.along(a.at) == frame.along(b.at));
    // Distinct microscopic gaps are refused rather than silently hidden.
    if nodes
        .windows(2)
        .any(|p| frame.along(p[1].at) - frame.along(p[0].at) < 1e-9)
    {
        return Err(Fault::OpeningUnplaced);
    }
    let mut spans = Vec::new();
    let side = if frame.offset(at) < 0.0 { -1.0 } else { 1.0 };
    let angle = atan2(frame.direction.y, frame.direction.x) * 180.0 / PI;
    for (index, row) in rows.iter().copied().enumerate() {
        let row_at = row_places
            .iter()
            .find(|(r, _)| *r == row)
            .map(|(_, p)| *p)
            .unwrap_or_else(|| frame.at_offset(at, frame.offset(at) + side * gap * index as f64));
        let points: Vec<_> = if row == Row::Overall {
            vec![start, end]
        } else {
            nodes.clone()
        };
        for pair in points.windows(2) {
            let offset = (row_at.x - pair[0].at.x) * frame.normal.x
                + (row_at.y - pair[0].at.y) * frame.normal.y;
            spans.push(Span {
                row,
                a: pair[0],
                b: pair[1],
                at: row_at,
                angle,
                offset,
            });
        }
    }
    Ok(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_intervals_centres_world_placement_and_large_coordinates() {
        for origin in [Vec2::new(0.0, 0.0), Vec2::new(500_000.0, 4_400_000.0)] {
            let p = |x, y| Vec2::new(origin.x + x, origin.y + y);
            let openings = [Opening {
                source: 1,
                jambs: [p(3.5, 0.1), p(3.5, -0.1), p(4.5, 0.1), p(4.5, -0.1)],
            }];
            let rows = [Row::Openings, Row::Overall];
            let spans = plan(
                [p(0.0, 0.1), p(10.0, 0.1)],
                0,
                [0, 1],
                Face::Left,
                &openings,
                Openings::Ends,
                &rows,
                p(0.0, 1.1),
                0.3,
                &[],
            )
            .unwrap();
            assert_eq!(spans.len(), 4);
            for (s, (a, b)) in spans
                .iter()
                .zip([(0.0, 3.5), (3.5, 4.5), (4.5, 10.0), (0.0, 10.0)])
            {
                assert_eq!(s.a.at, p(a, 0.1));
                assert_eq!(s.b.at, p(b, 0.1));
            }
            assert!((spans[0].offset - 1.0).abs() < 1e-9);
            assert!((spans[3].offset - 1.3).abs() < 1e-9);
            let centres = plan(
                [p(0.0, 0.1), p(10.0, 0.1)],
                0,
                [0, 1],
                Face::Left,
                &openings,
                Openings::Centers,
                &rows[..1],
                p(0.0, 1.1),
                0.3,
                &[],
            )
            .unwrap();
            assert_eq!(centres.len(), 2);
            assert_eq!(centres[0].b.at, p(4.0, 0.0));
            assert!((centres[1].offset - 1.1).abs() < 1e-9);
            let manual = plan(
                [p(0.0, 0.1), p(10.0, 0.1)],
                0,
                [0, 1],
                Face::Left,
                &openings,
                Openings::Ends,
                &rows,
                p(0.0, 1.1),
                0.6,
                &[(Row::Overall, p(0.0, 2.1))],
            )
            .unwrap();
            assert!((manual[3].offset - 2.0).abs() < 1e-9);
        }
    }

    #[test]
    fn rotated_and_reversed_rows_measure_along_the_host_and_keep_the_world_line() {
        for direction in [Vec2::new(0.6, 0.8), Vec2::new(-0.6, -0.8)] {
            let normal = Vec2::new(-direction.y, direction.x);
            let origin = Vec2::new(500_000., 4_400_000.);
            let p = |along: f64, across: f64| {
                Vec2::new(
                    origin.x + direction.x * along + normal.x * across,
                    origin.y + direction.y * along + normal.y * across,
                )
            };
            let openings = [Opening {
                source: 1,
                jambs: [p(3.5, 0.1), p(3.5, -0.1), p(4.5, 0.1), p(4.5, -0.1)],
            }];
            let spans = plan(
                [p(0., 0.1), p(10., 0.1)],
                0,
                [0, 1],
                Face::Left,
                &openings,
                Openings::Centers,
                &[Row::Openings, Row::Overall],
                p(0., 1.1),
                0.3,
                &[],
            )
            .unwrap();
            for (s, expected) in spans.iter().zip([4., 6., 10.]) {
                let along =
                    (s.b.at.x - s.a.at.x) * direction.x + (s.b.at.y - s.a.at.y) * direction.y;
                assert!((along - expected).abs() < 1e-8);
                let world_line =
                    (s.a.at.x - origin.x) * normal.x + (s.a.at.y - origin.y) * normal.y + s.offset;
                assert!((world_line - if s.row == Row::Overall { 1.4 } else { 1.1 }).abs() < 1e-8);
            }
            assert_eq!(spans[0].b.place, Place::Center);
        }
    }
}
