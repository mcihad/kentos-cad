//! Senaryo ölçüleri's access on the drawing (docs/adr/2260): Erişimi çizimde göster closes the window and leaves the
//! scenario side's access over the drawing until İşareti kaldır (`data.unmark`) or another drawing. The `from` areas
//! reached are filled in the success colour, those not reached in the danger colour; the `to` areas are outlined,
//! dashed, in the accent. The marks are worked out again with the same input whenever the drawing changes, so housing
//! a moved park now reaches turns at once. Altyapıyı çizimde göster (docs/adr/3060) shows the infrastructure measure the
//! same way: the measured areas near the layer filled in the success colour, those beyond it in the danger colour, the
//! layer's objects drawn over in the accent; nearness only, not a line's capacity. Nothing is written. Each measured area
//! is joined to its nearest object by a dashed line from its centroid, the distance at its middle (docs/adr/3460).
//! Docs/adr/5060: the access along a network draws each area's way along it (its two legs and the way between); Eğimi
//! çizimde göster fills the scenario's measured areas' parts by their slope class, the gentlest in the success colour
//! through the warning colour to the steepest in the danger colour, with a key of the classes and their areas (the
//! window's numbers) in the lower left corner.

use std::cell::RefCell;
use std::sync::Arc;

use iced::widget::canvas::{self, Path, Stroke};
use kentos_commands::{ImpactNear, ImpactReach, ScenariosImpact, ScenariosImpacted};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
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
    /// The slope map, not the access (docs/adr/5060).
    pub slope: bool,
    made: RefCell<Option<(u64, u64, Option<Arc<ReachMarks>>)>>,
}

impl Shown {
    pub fn new(session: u64, input: ScenariosImpact) -> Self {
        Self { session, input, near: false, slope: false, made: RefCell::new(None) }
    }

    /// Altyapıyı çizimde göster's (docs/adr/3060).
    pub fn near(session: u64, input: ScenariosImpact) -> Self {
        Self { session, input, near: true, slope: false, made: RefCell::new(None) }
    }

    /// Eğimi çizimde göster's (docs/adr/5060): the comparison asked with its slope map.
    pub fn slope(session: u64, mut input: ScenariosImpact) -> Self {
        if let Some(s) = input.slope.as_mut() {
            s.map = true;
        }
        Self { session, input, near: false, slope: true, made: RefCell::new(None) }
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
    /// Each measured area's centroid joined to its nearest object's nearest place, and the distance as the drawing
    /// writes lengths (docs/adr/3460).
    pub links: Vec<([f64; 2], [f64; 2], String)>,
    /// Along a network (docs/adr/5060): each area's way to its nearest target, as points.
    pub routes: Vec<Vec<[f64; 2]>>,
    /// The slope map (docs/adr/5060): each part and its class; the classes' key, each its words and its area.
    pub pieces: Vec<(Area, usize)>,
    pub key: Vec<(String, String)>,
}

/// An area of a ring and holes as the command writes them.
fn area_of(ring: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) -> Area {
    let r = |pts: &[[f64; 2]]| Ring { pts: pts.iter().map(|p| kentos_geometry_core::Vec2::new(p[0], p[1])).collect(), bulges: None };
    Area { outer: r(ring), holes: holes.iter().map(|h| r(h)).collect() }
}

/// Class `k` of `n`'s colour: the success colour for the gentlest through the warning colour to the danger colour for
/// the steepest.
pub fn class_colour(k: usize, n: usize, tokens: &Tokens) -> iced::Color {
    let t = if n > 1 { k as f32 / (n - 1) as f32 } else { 0.0 };
    let mix = |a: iced::Color, b: iced::Color, t: f32| {
        iced::Color::from_rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1.0)
    };
    if t <= 0.5 { mix(tokens.success, tokens.warning, t * 2.0) } else { mix(tokens.warning, tokens.danger, t * 2.0 - 1.0) }
}

impl ReachMarks {
    /// The scenario side's slope map of `answer` (docs/adr/5060): its parts by class, and the key of the classes with
    /// the scenario's area of each, as the window gives them (one class, “Ölçülen alan”, without breaks).
    pub fn of_slope(answer: &ScenariosImpacted, f: &kentos_interaction::Format) -> Self {
        let pieces: Vec<(Area, usize)> = answer
            .slope_map
            .iter()
            .flat_map(|m| &m.alternative)
            .map(|p| (area_of(&p.ring, &p.holes), p.class as usize))
            .collect();
        let mut key: Vec<(String, String)> = answer
            .measures
            .iter()
            .filter(|m| m.id.starts_with("slopeClass"))
            .map(|m| (m.label.clone(), f.area(m.alternative)))
            .collect();
        if key.is_empty() {
            let whole: f64 = answer.slope_map.iter().flat_map(|m| &m.alternative).map(|p| p.size).sum();
            key.push(("Ölçülen alan (sınıf sınırı yok)".to_owned(), f.area(whole)));
        }
        Self { pieces, key, ..Self::default() }
    }

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
            routes: r.alternative_routes.iter().map(|w| w.path.clone()).collect(),
            ..Self::default()
        }
    }

    /// The scenario side's nearness of `n` (docs/adr/3060): its measured areas near and beyond, and its layer's
    /// objects as lines.
    pub fn of_near(doc: &Document, n: &ImpactNear, f: &kentos_interaction::Format) -> Self {
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
        let links = n.alternative_links.iter().map(|l| (l.from, l.to, f.length(l.distance))).collect();
        Self { reached: areas(&n.alternative_near), unreached: areas(&n.alternative_far), lines, links, ..Self::default() }
    }

    /// The box of every area's outer ring (their vertices), when there is one.
    pub fn bounds(&self) -> Option<[f64; 4]> {
        let routes: Vec<kentos_geometry_core::Vec2> =
            self.routes.iter().flatten().map(|p| kentos_geometry_core::Vec2::new(p[0], p[1])).collect();
        self.reached
            .iter()
            .chain(&self.unreached)
            .chain(&self.targets)
            .chain(self.pieces.iter().map(|(a, _)| a))
            .flat_map(|a| a.outer.pts.iter())
            .chain(self.lines.iter().flatten())
            .chain(routes.iter())
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
        let shown = self.gis.impact_reach.as_ref()?;
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
            if shown.slope {
                answer.slope_map.is_some().then(|| Arc::new(ReachMarks::of_slope(&answer, &self.format())))
            } else if shown.near {
                answer.near.map(|n| Arc::new(ReachMarks::of_near(&doc.model, &n, &self.format())))
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
    // The slope map (docs/adr/5060): each class's parts filled in its colour, outlined thinly on a halo.
    let classes = m.key.len().max(1);
    for k in 0..classes {
        let parts: Vec<Area> = m.pieces.iter().filter(|(_, c)| *c == k).map(|(a, _)| a.clone()).collect();
        if parts.is_empty() {
            continue;
        }
        let colour = class_colour(k, classes, tokens);
        let shape = path_of(&parts, camera);
        frame.fill(
            &shape,
            canvas::Fill { style: canvas::Style::Solid(colour.scale_alpha(0.55)), rule: canvas::fill::Rule::EvenOdd },
        );
        frame.stroke(&shape, stroke(2.5, colors.halo));
        frame.stroke(&shape, stroke(1.0, colour));
    }
    if !m.pieces.is_empty() {
        slope_key(frame, m, colors, tokens);
    }
    // Each area's way along the network (docs/adr/5060): solid in the accent on a halo, a dot at each end.
    let at = |p: &[f64; 2]| {
        let s = camera.world_to_screen(kentos_geometry_core::Vec2::new(p[0], p[1]));
        iced::Point::new(s[0] as f32, s[1] as f32)
    };
    for way in m.routes.iter().filter(|w| w.len() >= 2) {
        let line = Path::new(|b| {
            for (k, p) in way.iter().enumerate() {
                if k == 0 {
                    b.move_to(at(p));
                } else {
                    b.line_to(at(p));
                }
            }
        });
        frame.stroke(&line, stroke(5.0, colors.halo));
        frame.stroke(&line, stroke(2.0, tokens.accent));
        for p in [way[0], way[way.len() - 1]] {
            frame.fill(&Path::circle(at(&p), 3.5), tokens.accent);
        }
    }
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
    // Each area to its nearest object (docs/adr/3460): dashed on a halo, a ring at the object's place, the distance at
    // the middle.
    let at = |p: [f64; 2]| {
        let s = camera.world_to_screen(kentos_geometry_core::Vec2::new(p[0], p[1]));
        iced::Point::new(s[0] as f32, s[1] as f32)
    };
    for (from, to, words) in &m.links {
        let (from, to) = (*from, *to);
        let (a, b) = (at(from), at(to));
        let link = Path::line(a, b);
        frame.stroke(&link, stroke(4.0, colors.halo));
        frame.stroke(&link, Stroke { line_dash: canvas::LineDash { segments: &[6.0, 4.0], offset: 0 }, ..stroke(1.5, tokens.accent) });
        frame.stroke(&Path::circle(b, 4.0), stroke(1.5, tokens.accent));
        let text = canvas::Text {
            content: words.clone(),
            position: iced::Point::new((a.x + b.x) / 2.0 + 6.0, (a.y + b.y) / 2.0),
            color: colors.halo,
            size: iced::Pixels(kentos_ui::theme::typography::scaled(11.0)),
            font: kentos_ui::theme::typography::ui_strong(),
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        };
        for (ox, oy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            frame.fill_text(canvas::Text { position: text.position + iced::Vector::new(ox, oy), ..text.clone() });
        }
        frame.fill_text(canvas::Text { color: tokens.accent, ..text });
    }
}

/// The slope map's key (docs/adr/5060) in the lower left corner: a swatch, the class's words and the scenario's area of
/// it, row by row, on the window's popover colour.
fn slope_key(frame: &mut canvas::Frame, m: &ReachMarks, colors: &MarkColors, tokens: &Tokens) {
    let size = kentos_ui::theme::typography::scaled(12.0);
    let row = size * 1.7;
    let pad = 10.0;
    let width = kentos_ui::theme::typography::scaled(340.0);
    let height = pad * 2.0 + row * (m.key.len() as f32 + 1.0);
    let top = frame.height() - height - 16.0;
    let left = 16.0;
    let panel = Path::rectangle(iced::Point::new(left, top), iced::Size::new(width, height));
    frame.fill(&panel, tokens.popover.scale_alpha(0.94));
    frame.stroke(&panel, Stroke::default().with_color(tokens.border).with_width(1.0));
    let text = |content: String, x: f32, y: f32, color: iced::Color, strong: bool| canvas::Text {
        content,
        position: iced::Point::new(x, y),
        color,
        size: iced::Pixels(size),
        font: if strong { kentos_ui::theme::typography::ui_strong() } else { kentos_ui::theme::typography::ui() },
        align_y: iced::alignment::Vertical::Center,
        ..canvas::Text::default()
    };
    frame.fill_text(text("Eğim sınıfları (senaryo)".to_owned(), left + pad, top + pad + row / 2.0, tokens.text, true));
    let n = m.key.len().max(1);
    for (k, (words, area)) in m.key.iter().enumerate() {
        let y = top + pad + row * (k as f32 + 1.5);
        let swatch = Path::rectangle(iced::Point::new(left + pad, y - size / 2.0), iced::Size::new(size * 1.4, size));
        frame.fill(&swatch, class_colour(k, n, tokens).scale_alpha(0.75));
        frame.stroke(&swatch, Stroke::default().with_color(colors.halo).with_width(1.0));
        frame.fill_text(text(words.clone(), left + pad + size * 2.0, y, tokens.text, false));
        frame.fill_text(canvas::Text {
            align_x: iced::alignment::Horizontal::Right.into(),
            ..text(area.clone(), left + width - pad, y, tokens.muted, false)
        });
    }
}
