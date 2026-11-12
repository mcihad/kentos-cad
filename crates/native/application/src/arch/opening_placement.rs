//! Opening placement measurements over the native wall path, shared by preview
//! and semantic creation. The shell never measures a curved host as a chord.
use crate::{checks::Stop, codes, openings::fail};
use kentos_commands::{OpeningFields, OpeningSide, Vec2};
use kentos_domain::{
    Document,
    opening_follow::{self, WallLine},
};
use kentos_geometry_core::{
    Vec2 as Point,
    ops::{
        edges::edge_length,
        path::{nearest_s, point_at_s, tangent_at_s},
    },
};

pub(crate) fn span(
    line: &WallLine,
    ends: [Vec2; 2],
    side_point: Option<Vec2>,
    path: &str,
) -> Result<(f64, f64, Option<OpeningSide>), Stop> {
    let [a, b] = ends.map(|p| nearest_s(&line.path, Point::new(p.x, p.y)));
    // Finite-path projection clamps outside an open endpoint. Do not quietly
    // shorten the opening when a jamb was requested past that endpoint.
    for (q, s) in ends.into_iter().zip([a, b]) {
        let base = point_at_s(&line.path, s);
        let t = tangent_at_s(&line.path, s);
        let along = (q.x - base.x) * t.x + (q.y - base.y) * t.y;
        if (s <= 1e-9 && along < -1e-9) || (s >= line.path.length - 1e-9 && along > 1e-9) {
            return Err(fail(codes::OPENING_OFF_WALL,"Pervaz noktası duvarın ucundan taşıyor; nokta duvar boyunca uçların arasında olmalı.".into(),&format!("{path}.span")));
        }
    }
    let at = (a + b) / 2.0;
    let q = side_point.unwrap_or(ends[1]);
    let s = nearest_s(&line.path, Point::new(q.x, q.y));
    let base = point_at_s(&line.path, s);
    let t = tangent_at_s(&line.path, s);
    let cross = t.x * (q.y - base.y) - t.y * (q.x - base.x);
    let side = if cross < 0.0 {
        OpeningSide::Right
    } else {
        OpeningSide::Left
    };
    Ok((at, (b - a).abs(), Some(side)))
}

/// Clear distances from the jambs to the ends of their host edge, along the
/// analytic wall path. Computed from the planned opening, never from the cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measurement {
    pub start_clear: f64,
    pub end_clear: f64,
    pub width: f64,
    /// Its wall's thickness (docs/adr/0719: what a niche's depth is told against).
    pub thickness: f64,
}
pub fn measure(doc: &Document, opening: &OpeningFields) -> Option<Measurement> {
    let line = doc.wall_line_of(&opening.host)?;
    let (lo, hi) = opening_follow::span(opening);
    let i = line
        .path
        .cum
        .iter()
        .take(line.path.edges.len())
        .enumerate()
        .find(|(i, start)| {
            lo >= **start - 1e-9 && hi <= **start + edge_length(&line.path.edges[*i]) + 1e-9
        })?
        .0;
    Some(Measurement {
        start_clear: lo - line.path.cum[i],
        end_clear: line.path.cum[i] + edge_length(&line.path.edges[i]) - hi,
        width: opening.width,
        thickness: line.spec.width,
    })
}
