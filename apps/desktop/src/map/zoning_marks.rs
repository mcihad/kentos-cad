//! İmar planı denetimi's road overlaps on the drawing (plan/map-cad.md MAP-09, docs/adr/3040): Bulguları göster leaves
//! the parts of decisions and parcels that enter a road's written width (roadWidth, roadParcel) hatched over the
//! drawing in the danger colour until İşareti kaldır (`data.unmark`) or another drawing. The regions are the core's
//! rule's own (`zoning_check::with_regions`), worked out again with the same input whenever the drawing changes, so a
//! parcel moved off the road loses its hatch at once. Nothing is written.

use std::cell::RefCell;
use std::sync::Arc;

use iced::Point;
use iced::widget::canvas::{self, Path, Stroke};
use kentos_commands::ZoningCheck;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::ring_edges;
use kentos_native_application::zoning_check;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::Tokens;

use crate::app::App;
use crate::marks::edge_points;
use crate::viewport::MarkColors;

/// The kinds whose regions are hatched: a road's corridor entered by a decision or a parcel.
pub const HATCHED: [&str; 2] = ["roadWidth", "roadParcel"];

/// What Bulguları göster keeps: the drawing's session, the check's input, and the marks of the drawing as it was last
/// seen (its generation and revision).
#[derive(Debug, Default)]
pub struct Shown {
    pub session: u64,
    pub input: ZoningCheck,
    made: RefCell<Option<(u64, u64, Option<Arc<ZoningMarks>>)>>,
}

impl Shown {
    pub fn new(session: u64, input: ZoningCheck) -> Self {
        Self { session, input, made: RefCell::new(None) }
    }
}

/// The road overlaps' regions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZoningMarks {
    pub regions: Vec<Area>,
}

impl App {
    /// The road overlaps for the drawing as it is: none when nothing is shown, for another drawing, when the check no
    /// longer reads or finds none. Kept until the drawing changes.
    pub(crate) fn zoning_marks(&self) -> Option<Arc<ZoningMarks>> {
        let shown = self.map.zoning_shown_marks.as_ref()?;
        let doc = self.document.as_ref()?;
        if doc.session != shown.session {
            return None;
        }
        let (generation, revision) = (doc.model.generation(), doc.model.revision());
        if let Some((g, r, made)) = shown.made.borrow().as_ref()
            && *g == generation
            && *r == revision
        {
            return made.clone();
        }
        let made = zoning_check::with_regions(&doc.model, &shown.input).ok().and_then(|(checked, regions)| {
            let regions: Vec<Area> = checked
                .findings
                .iter()
                .zip(regions)
                .filter(|(f, _)| HATCHED.contains(&f.kind.as_str()))
                .flat_map(|(_, r)| r)
                .collect();
            (!regions.is_empty()).then(|| Arc::new(ZoningMarks { regions }))
        });
        *shown.made.borrow_mut() = Some((generation, revision, made.clone()));
        made
    }
}

/// The regions' rings on the screen, arcs as points along them.
fn rings_on_screen(areas: &[Area], camera: &Camera) -> Vec<Vec<Point>> {
    let mut out = Vec::new();
    for a in areas {
        for ring in std::iter::once(&a.outer).chain(&a.holes) {
            let mut pts: Vec<Point> = Vec::new();
            for e in ring_edges(ring) {
                for (k, q) in edge_points(&e, camera).into_iter().enumerate() {
                    if k > 0 || pts.is_empty() {
                        pts.push(q);
                    }
                }
            }
            if pts.len() >= 3 {
                out.push(pts);
            }
        }
    }
    out
}

/// The pieces of the line x − y = c inside the rings (even-odd), as screen segments.
fn hatch_line(rings: &[Vec<Point>], c: f32) -> Vec<(Point, Point)> {
    // Along the line, parameter t = x; it meets an edge where x − y = c.
    let mut ts: Vec<f32> = Vec::new();
    for ring in rings {
        for (k, a) in ring.iter().enumerate() {
            let b = ring[(k + 1) % ring.len()];
            let (fa, fb) = (a.x - a.y - c, b.x - b.y - c);
            if (fa > 0.0) != (fb > 0.0) {
                let s = fa / (fa - fb);
                ts.push(a.x + (b.x - a.x) * s);
            }
        }
    }
    ts.sort_by(f32::total_cmp);
    ts.chunks_exact(2).map(|p| (Point::new(p[0], p[0] - c), Point::new(p[1], p[1] - c))).collect()
}

/// The overlaps on the screen: each region a light fill, hatched at 45° every 7 px, outlined on a halo, in the danger
/// colour. Each region alone: two that cover the same place (a decision's part and a parcel's in one road) are both
/// drawn, never cancelled by an even-odd rule across them.
pub fn draw(frame: &mut canvas::Frame, m: &ZoningMarks, camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
    for region in &m.regions {
        draw_region(frame, std::slice::from_ref(region), camera, colors, tokens);
    }
}

/// One region (its outer ring and holes, even-odd among themselves).
fn draw_region(frame: &mut canvas::Frame, region: &[Area], camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
    let rings = rings_on_screen(region, camera);
    if rings.is_empty() {
        return;
    }
    let shape = Path::new(|b| {
        for ring in &rings {
            b.move_to(ring[0]);
            for q in &ring[1..] {
                b.line_to(*q);
            }
            b.close();
        }
    });
    let danger = tokens.danger;
    frame.fill(
        &shape,
        canvas::Fill { style: canvas::Style::Solid(danger.scale_alpha(0.14)), rule: canvas::fill::Rule::EvenOdd },
    );
    // The hatch across the rings' box, each line cut to the inside.
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for q in rings.iter().flatten() {
        lo = lo.min(q.x - q.y);
        hi = hi.max(q.x - q.y);
    }
    let hatch = Path::new(|b| {
        let mut c = (lo / 7.0).floor() * 7.0;
        let mut lines = 0;
        while c <= hi && lines < 4000 {
            for (p, q) in hatch_line(&rings, c) {
                b.move_to(p);
                b.line_to(q);
            }
            c += 7.0;
            lines += 1;
        }
    });
    let stroke = |width: f32, color: iced::Color| Stroke::default().with_color(color).with_width(width);
    frame.stroke(&hatch, stroke(1.0, danger.scale_alpha(0.8)));
    frame.stroke(&shape, stroke(3.5, colors.halo));
    frame.stroke(&shape, stroke(1.5, danger));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hatch_line_is_cut_to_the_inside_even_odd() {
        // A 20 × 20 square with a 10 × 10 hole, the line x − y = 0 through both.
        let square = vec![Point::new(0.0, 0.0), Point::new(20.0, 0.0), Point::new(20.0, 20.0), Point::new(0.0, 20.0)];
        let hole = vec![Point::new(5.0, 5.0), Point::new(15.0, 5.0), Point::new(15.0, 15.0), Point::new(5.0, 15.0)];
        let pieces = hatch_line(&[square, hole], 0.5);
        assert_eq!(pieces.len(), 2, "{pieces:?}");
        assert!((pieces[0].0.x - 0.5).abs() < 0.01 && (pieces[0].1.x - 5.5).abs() < 0.01, "{pieces:?}");
        assert!((pieces[1].0.x - 15.0).abs() < 0.01 && (pieces[1].1.x - 20.0).abs() < 0.01, "{pieces:?}");
    }
}
