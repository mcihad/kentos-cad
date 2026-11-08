//! Senaryo ölçüleri's access on the drawing (docs/adr/2260): Erişimi çizimde göster closes the window and leaves the
//! scenario side's access over the drawing until İşareti kaldır (`data.unmark`) or another drawing. The `from` areas
//! reached are filled in the success colour, those not reached in the danger colour; the `to` areas are outlined,
//! dashed, in the accent. The marks are worked out again with the same input whenever the drawing changes, so housing
//! a moved park now reaches turns at once. Altyapıyı çizimde göster (docs/adr/3060) shows the infrastructure measure the
//! same way: the measured areas near the layer filled in the success colour, those beyond it in the danger colour, the
//! layer's objects drawn over in the accent; nearness only, not a line's capacity. Nothing is written.

use std::cell::RefCell;
use std::sync::Arc;

use iced::widget::canvas::{self, Path, Stroke};
use kentos_commands::{ImpactNear, ImpactReach, ScenariosImpact};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::ring_edges;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_native_application::geometry::shape;
use kentos_native_application::scenarios_impact;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::Tokens;

use crate::app::App;
use crate::marks::edge_points;
use crate::viewport::MarkColors;

/// What Erişimi çizimde göster keeps: the drawing's session, the comparison's input, and the marks of the drawing as
/// it was last seen (its generation and revision).
#[derive(Debug, Default)]
pub struct Shown {
    pub session: u64,
    pub input: ScenariosImpact,
    /// The infrastructure measure's nearness, not the access (docs/adr/3060).
    pub near: bool,
    made: RefCell<Option<(u64, u64, Option<Arc<ReachMarks>>)>>,
}

impl Shown {
    pub fn new(session: u64, input: ScenariosImpact) -> Self {
        Self { session, input, near: false, made: RefCell::new(None) }
    }

    /// Altyapıyı çizimde göster's (docs/adr/3060).
    pub fn near(session: u64, input: ScenariosImpact) -> Self {
        Self { session, input, near: true, made: RefCell::new(None) }
    }
}

/// The scenario side's areas: the `from` areas reached and not, and the `to` areas.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReachMarks {
    pub reached: Vec<Area>,
    pub unreached: Vec<Area>,
    pub targets: Vec<Area>,
    /// The infrastructure layer's objects as lines, arcs as points along them (docs/adr/3060).
    pub lines: Vec<Vec<kentos_geometry_core::Vec2>>,
}

impl ReachMarks {
    /// The scenario side's areas of `r`, by their objects in `doc`.
    pub fn of(doc: &Document, r: &ImpactReach) -> Self {
        let areas = |uids: &[String]| -> Vec<Area> {
            uids.iter()
                .filter_map(|u| Uuid::parse_str(u).ok())
                .filter_map(|u| doc.slot_of(u))
                .filter_map(|s| doc.get(s))
                .flat_map(|e| areas_of_entity(&shape(e)))
                .collect()
        };
        Self {
            reached: areas(&r.alternative_reached),
            unreached: areas(&r.alternative_unreached),
            targets: areas(&r.alternative_targets),
            lines: Vec::new(),
        }
    }

    /// The scenario side's nearness of `n` (docs/adr/3060): its measured areas near and beyond, and its layer's
    /// objects as lines.
    pub fn of_near(doc: &Document, n: &ImpactNear) -> Self {
        let entities = |uids: &[String]| -> Vec<&kentos_commands::Entity> {
            uids.iter()
                .filter_map(|u| Uuid::parse_str(u).ok())
                .filter_map(|u| doc.slot_of(u))
                .filter_map(|s| doc.get(s))
                .collect()
        };
        let areas = |uids: &[String]| -> Vec<Area> { entities(uids).into_iter().flat_map(|e| areas_of_entity(&shape(e))).collect() };
        let lines = entities(&n.alternative_lines)
            .into_iter()
            .filter_map(|e| match shape(e) {
                kentos_geometry_core::entity::Shape::Line { a, b } => Some(vec![a, b]),
                kentos_geometry_core::entity::Shape::Polyline { pts, bulges, .. } => {
                    let size: f64 = pts.windows(2).map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y)).sum();
                    Some(kentos_geometry_core::tessellate::bulge_path(&pts, bulges.as_deref(), false, (size / 2000.0).max(1e-6)))
                }
                _ => None,
            })
            .collect();
        Self { reached: areas(&n.alternative_near), unreached: areas(&n.alternative_far), targets: Vec::new(), lines }
    }

    /// The box of every area's outer ring (their vertices), when there is one.
    pub fn bounds(&self) -> Option<[f64; 4]> {
        self.reached
            .iter()
            .chain(&self.unreached)
            .chain(&self.targets)
            .flat_map(|a| a.outer.pts.iter())
            .chain(self.lines.iter().flatten())
            .fold(
            None,
            |b: Option<[f64; 4]>, p| {
                Some(b.map_or([p.x, p.y, p.x, p.y], |b| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]))
            },
        )
    }
}

impl App {
    /// The access marks for the drawing as it is: none when nothing is shown, for another drawing, or when the
    /// comparison no longer reads (its scenario was removed). Kept until the drawing changes.
    pub(crate) fn impact_reach_marks(&self) -> Option<Arc<ReachMarks>> {
        let shown = self.impact_reach.as_ref()?;
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
        let made = scenarios_impact::preview(&doc.model, &shown.input).ok().and_then(|answer| {
            if shown.near {
                answer.near.map(|n| Arc::new(ReachMarks::of_near(&doc.model, &n)))
            } else {
                answer.reach.map(|reach| Arc::new(ReachMarks::of(&doc.model, &reach)))
            }
        });
        *shown.made.borrow_mut() = Some((generation, revision, made.clone()));
        made
    }
}

/// The areas as one path on the screen: every ring, arcs as points along them.
fn path_of(areas: &[Area], camera: &Camera) -> Path {
    Path::new(|b| {
        for a in areas {
            for ring in std::iter::once(&a.outer).chain(&a.holes) {
                let mut first = true;
                for e in ring_edges(ring) {
                    for (k, q) in edge_points(&e, camera).into_iter().enumerate() {
                        if first {
                            b.move_to(q);
                            first = false;
                        } else if k > 0 {
                            b.line_to(q);
                        }
                    }
                }
                b.close();
            }
        }
    })
}

/// The marks on the screen: the reached and the unreached areas filled and outlined on a halo, then the targets
/// outlined dashed.
pub fn draw(frame: &mut canvas::Frame, m: &ReachMarks, camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
    let stroke = |width: f32, color: iced::Color| Stroke::default().with_color(color).with_width(width);
    for (areas, color) in [(&m.reached, tokens.success), (&m.unreached, tokens.danger)] {
        if areas.is_empty() {
            continue;
        }
        let shape = path_of(areas, camera);
        frame.fill(
            &shape,
            canvas::Fill { style: canvas::Style::Solid(color.scale_alpha(0.28)), rule: canvas::fill::Rule::EvenOdd },
        );
        frame.stroke(&shape, stroke(3.5, colors.halo));
        frame.stroke(&shape, stroke(1.5, color));
    }
    if !m.targets.is_empty() {
        let shape = path_of(&m.targets, camera);
        frame.stroke(&shape, stroke(4.5, colors.halo));
        frame.stroke(
            &shape,
            Stroke { line_dash: canvas::LineDash { segments: &[8.0, 5.0], offset: 0 }, ..stroke(2.0, tokens.accent) },
        );
    }
    // The infrastructure's lines over all, in the accent on a halo (docs/adr/3060).
    if !m.lines.is_empty() {
        let lines = Path::new(|b| {
            for line in &m.lines {
                for (k, p) in line.iter().enumerate() {
                    let s = camera.world_to_screen(*p);
                    let q = iced::Point::new(s[0] as f32, s[1] as f32);
                    if k == 0 {
                        b.move_to(q);
                    } else {
                        b.line_to(q);
                    }
                }
            }
        });
        frame.stroke(&lines, stroke(6.0, colors.halo));
        frame.stroke(&lines, stroke(3.0, tokens.accent));
    }
}
