//! Snapping on the SVG editor's canvas (the web's `svgSnap.ts`): an index of
//! the drawing's snap points (the SVG core's `SnapIndex`), built once per
//! drag, the grid when no point is near, and the marker drawn where the
//! pointer snapped: the main CAD's shapes (square node, triangle middle,
//! circle centre, cross crossing, right angle, tangent circle) with the
//! kind's name beside it.

use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Color, Pixels, Point};
use kentos_svg_core::nodes::NodeRef;
use kentos_svg_core::shape::Pt;
use kentos_svg_core::snap::{Guide, Kind, SnapHit, SnapIndex, SnapSource};
use kentos_ui::theme::typography;

use super::state::SvgEditor;

/// The pointer's reach, in screen pixels.
const RADIUS_PX: f64 = 8.0;

/// What a snap leaves out and where it comes from (`SnapOptions`).
#[derive(Clone, Debug, Default)]
pub struct SnapOpts {
    /// Shapes moving with the pointer.
    pub exclude: Vec<String>,
    /// Nodes moving with the pointer: the path and its nodes.
    pub nodes: Option<(String, Vec<NodeRef>)>,
    /// Where the drag started (perpendicular, tangent).
    pub from: Option<Pt>,
    /// No grid fallback (a handle, a measure point).
    pub no_grid: bool,
    /// A guide being dragged (never snaps to itself).
    pub guide: Option<String>,
}

#[derive(Default)]
pub struct Snapper {
    index: Option<SnapIndex>,
    key: String,
    /// The last snap, drawn until the pointer moves off it.
    pub hit: Option<SnapHit>,
}

impl Snapper {
    /// Forget the index (the drawing, the guides or the options changed).
    pub fn reset(&mut self) {
        self.index = None;
        self.key.clear();
        self.hit = None;
    }
}

fn key_of(o: &SnapOpts) -> String {
    let nodes = o.nodes.as_ref().map_or_else(String::new, |(shape, refs)| {
        let refs: Vec<String> = refs
            .iter()
            .map(|r| format!("{}:{}", r.sub, r.index))
            .collect();
        format!("{shape}:{}", refs.join(","))
    });
    format!(
        "{}|{}|{}",
        o.exclude.join(","),
        nodes,
        o.guide.as_deref().unwrap_or("")
    )
}

impl SvgEditor {
    /// A point snapped as the options say (`snap`).
    pub fn snap(&mut self, p: Pt, o: &SnapOpts) -> Pt {
        self.snapper.hit = None;
        let opt = &self.options;
        if opt.snap_objects && !opt.snap_kinds.is_empty() {
            let key = key_of(o);
            if self.snapper.index.is_none() || key != self.snapper.key {
                let guides = self
                    .doc
                    .guides
                    .iter()
                    .filter(|g| o.guide.as_deref() != Some(g.id.as_str()))
                    .map(|g| Guide {
                        x: g.x,
                        y: g.y,
                        angle: g.angle,
                    })
                    .collect();
                let skip = o.nodes.as_ref().map_or_else(Vec::new, |(shape, refs)| {
                    refs.iter()
                        .map(|r| (shape.clone(), r.sub, r.index))
                        .collect()
                });
                let src = SnapSource {
                    shapes: self.doc.shapes.clone(),
                    guides,
                    width: self.doc.width,
                    height: self.doc.height,
                    kinds: opt.snap_kinds.clone(),
                    exclude: o.exclude.clone(),
                    skip,
                };
                self.snapper.index = SnapIndex::new(&src).ok();
                self.snapper.key = key;
            }
            if let Some(ix) = &self.snapper.index
                && let Some(hit) = ix.query(p, RADIUS_PX / self.camera.zoom, o.from)
            {
                let q = hit.p;
                self.snapper.hit = Some(hit);
                return q;
            }
        }
        let opt = &self.options;
        if o.no_grid || !opt.snap_grid || opt.grid.is_nan() || opt.grid <= 0.0 {
            return p;
        }
        let g = opt.grid;
        [
            kentos_native_style::classify::js_round(p[0] / g) * g,
            kentos_native_style::classify::js_round(p[1] / g) * g,
        ]
    }
}

/// The marker's outline, 5 px round the point (the shapes of DESIGN.md §8).
fn mark_path(kind: Kind, x: f32, y: f32) -> Path {
    let r = 5.0;
    Path::new(|b| match kind {
        Kind::Cusp | Kind::BboxCorner => {
            b.rectangle(Point::new(x - r, y - r), iced::Size::new(2.0 * r, 2.0 * r));
        }
        Kind::Smooth => {
            b.move_to(Point::new(x, y - r - 1.0));
            b.line_to(Point::new(x + r + 1.0, y));
            b.line_to(Point::new(x, y + r + 1.0));
            b.line_to(Point::new(x - r - 1.0, y));
            b.close();
        }
        Kind::Mid | Kind::BboxMid => {
            b.move_to(Point::new(x, y - r - 1.0));
            b.line_to(Point::new(x + r + 1.0, y + r));
            b.line_to(Point::new(x - r - 1.0, y + r));
            b.close();
        }
        Kind::Centre | Kind::BboxCentre => {
            b.circle(Point::new(x, y), r);
            b.move_to(Point::new(x - 1.5, y));
            b.line_to(Point::new(x + 1.5, y));
        }
        Kind::Intersection => {
            b.move_to(Point::new(x - r, y - r));
            b.line_to(Point::new(x + r, y + r));
            b.move_to(Point::new(x + r, y - r));
            b.line_to(Point::new(x - r, y + r));
        }
        Kind::Perpendicular => {
            b.move_to(Point::new(x - r, y - r));
            b.line_to(Point::new(x - r, y + r));
            b.line_to(Point::new(x + r, y + r));
            b.move_to(Point::new(x - r, y));
            b.line_to(Point::new(x, y));
            b.line_to(Point::new(x, y + r));
        }
        Kind::Tangent => {
            b.circle(Point::new(x, y), r);
            b.move_to(Point::new(x - r - 2.0, y - r));
            b.line_to(Point::new(x + r + 2.0, y - r));
        }
        Kind::Guide => {
            b.move_to(Point::new(x - r, y - r));
            b.line_to(Point::new(x + r, y + r));
            b.line_to(Point::new(x - r, y + r));
            b.line_to(Point::new(x + r, y - r));
            b.close();
        }
        Kind::Page => {
            b.rectangle(Point::new(x - r, y - r), iced::Size::new(2.0 * r, 2.0 * r));
            b.move_to(Point::new(x - r, y));
            b.line_to(Point::new(x + r, y));
            b.move_to(Point::new(x, y - r));
            b.line_to(Point::new(x, y + r));
        }
    })
}

/// Where a label's chip sits against its point.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hang {
    /// The chip's bottom left at the point.
    Above,
    /// Its top left just under the point: a second line under an `Above` one, at any text size.
    Below,
}

/// A label in screen space (the web's `svge__tag`, a chip on both platforms):
/// `color` on a chip of the panel's colour. A chip reads over shapes and lines,
/// where a halo blurred small bold letters.
pub fn tag(
    frame: &mut Frame,
    content: &str,
    at: Point,
    hang: Hang,
    color: Color,
    halo: Color,
    strong: bool,
) {
    let size = typography::scaled(10.5);
    let width = typography::current().text_width(content, size, strong);
    let height = (size * 1.35).round();
    let top = match hang {
        Hang::Above => at.y - height,
        Hang::Below => at.y + 1.0,
    };
    let chip = iced::widget::canvas::Path::rounded_rectangle(
        Point::new(at.x - 3.0, top),
        iced::Size::new(width + 6.0, height),
        3.0.into(),
    );
    frame.fill(&chip, Color { a: 0.9, ..halo });
    frame.fill_text(Text {
        content: content.to_owned(),
        position: Point::new(at.x, top + height / 2.0),
        color,
        size: Pixels(size),
        font: if strong {
            typography::ui_strong()
        } else {
            typography::ui()
        },
        align_y: iced::alignment::Vertical::Center,
        ..Text::default()
    });
}

/// The marker of the last snap with its name (`Snapper.draw`).
pub fn draw_snap(frame: &mut Frame, ed: &SvgEditor, snap: Color, halo: Color) {
    let Some(hit) = &ed.snapper.hit else {
        return;
    };
    let [x, y] = ed.camera.to_screen(hit.p);
    let (x, y) = (x as f32, y as f32);
    frame.stroke(
        &mark_path(hit.kind, x, y),
        Stroke::default().with_color(snap).with_width(1.6),
    );
    tag(
        frame,
        hit.label,
        Point::new(x + 9.0, y - 9.0),
        Hang::Above,
        snap,
        halo,
        true,
    );
}
