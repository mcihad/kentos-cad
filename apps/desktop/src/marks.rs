//! Marks over the drawing that follow the pointer (docs/adr/0029), drawn on
//! Iced's canvas like the draft (preview.rs), as the web draws them on its
//! overlay canvas (`viewport/overlay.ts` `drawSnap`, `tools/preview.ts`
//! `drawSelectionBox`):
//!
//! - the object snap marker: its kind's glyph in the snap colour
//!   (`--canvas-snap`), 1.5 px, and its Turkish name above-right of it with
//!   a halo of the area's colour, so it never meets the measurement below-right;
//! - the selection box: left to right a window (blue, solid), right to left
//!   a crossing (the snap colour, dashed 5/4), each filled at 10 %;
//! - the selection's grips (`drawGrips`, docs/adr/0068), at most 150 objects':
//!   accent squares edged in the area's colour, thinned to one per 9 px;
//!   a path's mid grips as small hollow diamonds while their segment is 28 px
//!   long; the grip being moved larger, in the drawing's ink.
//!
//! - the crosshair at the pointer (`drawCrosshair`): arms of the chosen
//!   length (`appearance.crosshair`), shorter with a pick box while an
//!   object is wanted; none for Kaydır or while the middle button pans.
//!
//! They change with every pointer move and hold one glyph or one box, so
//! they are not scene parts; the selection's own highlight is (viewport.rs).

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::{Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use kentos_domain::Slot;
use kentos_interaction::object_tracking::TrackHit;
use kentos_interaction::{GripSet, SelectBox, SnapHit, SnapKind, Vec2};
use kentos_render_wgpu::Camera;
use kentos_ui::theme::{Tokens, typography};

use crate::input::CameraView;
use crate::viewport::MarkColors;

/// A snap kind as the web names it beside the marker (`SNAP_LABEL`).
pub fn snap_label(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Endpoint => "Uç nokta",
        SnapKind::Midpoint => "Orta nokta",
        SnapKind::Center => "Merkez",
        SnapKind::Node => "Nokta",
        SnapKind::Quadrant => "Çeyrek",
        SnapKind::Intersection => "Kesişim",
        SnapKind::Perpendicular => "Dik",
        SnapKind::Tangent => "Teğet",
        SnapKind::Nearest => "En yakın",
        SnapKind::Centroid => "Ağırlık merkezi",
        SnapKind::Extension => "Uzantı",
        SnapKind::Parallel => "Paralel",
        SnapKind::Grid => "Karelaj",
        SnapKind::Insertion => "Ekleme noktası",
    }
}

/// A snap kind as the traces write it (the web's `SnapKind` names).
pub fn snap_name(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Endpoint => "endpoint",
        SnapKind::Midpoint => "midpoint",
        SnapKind::Center => "center",
        SnapKind::Node => "node",
        SnapKind::Quadrant => "quadrant",
        SnapKind::Intersection => "intersection",
        SnapKind::Perpendicular => "perpendicular",
        SnapKind::Tangent => "tangent",
        SnapKind::Nearest => "nearest",
        SnapKind::Centroid => "centroid",
        SnapKind::Extension => "extension",
        SnapKind::Parallel => "parallel",
        SnapKind::Grid => "grid",
        SnapKind::Insertion => "insertion",
    }
}

/// The marks' canvas program: the snap marker, the selection box and the
/// selection's grips, when there are.
pub struct Marks {
    pub camera: Camera,
    pub snap: Option<SnapHit>,
    /// What the snap marker says after its kind (docs/adr/0271): the object's kind and, while Tab steps
    /// through the candidates, which one of how many (“çizgi · 2/5”).
    pub snap_note: Option<String>,
    pub select: Option<SelectBox>,
    pub grips: Vec<GripSet>,
    /// The selected dynamic blocks' parameter grips (docs/adr/0319 §1), in the info blue.
    pub params: Vec<kentos_geometry_core::block_dynamic::ParamGrip>,
    /// The grip being moved.
    /// The hot grips: the one being moved and those made hot with Shift (docs/adr/0280).
    pub hot: Vec<(Slot, usize)>,
    /// The vertices Köşe tablosu's selected rows name (docs/adr/0172 §3).
    pub marked: Vec<Vec2>,
    /// The place the data search's Git marked and its coordinates' words
    /// (docs/adr/0178 §6).
    pub found: Option<(Vec2, String)>,
    /// The Topoloji tab's finding (docs/adr/0202 §5).
    pub problem: Option<crate::topology::ProblemMark>,
    /// The points of associative dimensions whose reference broke (docs/adr/0249 §3).
    pub broken: Vec<Vec2>,
    /// What the selected associative dimensions are bound to (docs/adr/0320 §4).
    pub bonds: Vec<kentos_interaction::dimension_bonds::Bond>,
    /// The constraints' marks (constraint_view.rs, docs/adr/0338 §1).
    pub constraints: Option<std::sync::Arc<crate::constraint_view::ConstraintMarks>>,
    /// The pointer over the drawing, for the mark under it.
    pub constraint_pointer: Option<Point>,
    /// Nesne izleme's points and the alignment the cursor is locked to (tracking.rs).
    pub tracking: Option<TrackingMarks>,
    /// The crosshair at the pointer, when the pointer is over the drawing.
    pub crosshair: Option<Crosshair>,
    /// The digitizing locks' guides (docs/adr/0166 §6).
    pub locks: Option<LockMarks>,
    pub colors: MarkColors,
}

/// The digitizing locks over the drawing (docs/adr/0166 §6; the web's
/// `drawLocks`), in the snap colour as the other guides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockMarks {
    /// The point the locks are measured from.
    pub reference: Vec2,
    /// The locked direction's unit vector, and whether it runs both ways.
    pub direction: Option<(Vec2, bool)>,
    /// The locked length, metres.
    pub length: Option<f64>,
    /// The edge a Paralel or Dik lock was picked on (docs/adr/0166 §3).
    pub edge: Option<kentos_geometry_core::geom::intersect::Edge>,
    /// Referans noktası's or Yapım kipi's point, marked “R” (§5).
    pub named: Option<Vec2>,
    /// Whether locks hold (else only the reference is marked).
    pub locks: bool,
    /// The locked coordinates' lines, a point on each and its direction (docs/adr/0267 §1).
    pub lines: [Option<(Vec2, Vec2)>; 2],
}

/// How long the crosshair's arms are (`appearance.crosshair`, the web's `CROSSHAIR_ARM`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrosshairSize {
    Small,
    #[default]
    Medium,
    /// Across the whole drawing.
    Full,
}

impl CrosshairSize {
    /// The setting's word: `small`, `medium`, `full`; anything else the default.
    pub fn parse(word: &str) -> Self {
        match word {
            "small" => Self::Small,
            "full" => Self::Full,
            _ => Self::Medium,
        }
    }

    /// An arm's length in pixels: 16, 40, or through the drawing; 55 % of it,
    /// rounded, while an object is picked, unless across the drawing (the web's).
    pub fn arm(self, pick: bool) -> f32 {
        let arm = match self {
            Self::Small => 16.0,
            Self::Medium => 40.0,
            Self::Full => return 1e5,
        };
        if pick { (arm * 0.55_f32).round() } else { arm }
    }
}

/// The crosshair's look: its arms, and whether an object is picked (a
/// shorter cross with a pick box, the web's `pick` cursor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crosshair {
    pub size: CrosshairSize,
    pub pick: bool,
}

/// What object tracking shows (the web's `drawObjectTracking`).
pub struct TrackingMarks {
    /// The acquired points (tracking points and ends).
    pub acquired: Vec<Vec2>,
    /// The acquired edges (Paralel): where they were rested on and their directions.
    pub edges: Vec<(Vec2, Vec2)>,
    /// The lock, when no snap wins over it.
    pub track: Option<TrackHit>,
    /// “İzleme 12.500 m < 0°” or “İzleme: kesişim”.
    pub label: Option<String>,
    /// The extensions or the parallel the snap lies on, dashed (docs/adr/0163 §2).
    pub paths: Vec<Guide>,
    /// What the snap marker says instead of its kind's name (“Uzantı 12.063 m”).
    pub snap_label: Option<String>,
}

/// A dashed guide of the snap additions: an extension from its end to the
/// snap (a line, or points around an arc), or the parallel's whole line.
#[derive(Clone, Debug, PartialEq)]
pub enum Guide {
    Path(Vec<Vec2>),
    Line { through: Vec2, dir: Vec2 },
}

impl Marks {
    pub fn is_empty(&self) -> bool {
        self.snap.is_none()
            && self.select.is_none()
            && self.grips.is_empty()
            && self.params.is_empty()
            && self.marked.is_empty()
            && self.found.is_none()
            && self.problem.is_none()
            && self.broken.is_empty()
            && self.bonds.is_empty()
            && self.constraints.is_none()
            && self.tracking.is_none()
            && self.crosshair.is_none()
            && self.locks.is_none()
    }
}

impl<Message> canvas::Program<Message> for Marks {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let accent = Tokens::of(theme).accent;
        grips(
            &mut frame,
            &self.grips,
            &self.hot,
            &self.camera,
            &self.colors,
            accent,
        );
        bonds(&mut frame, &self.bonds, &self.camera, &self.colors, accent);
        if let Some(c) = &self.constraints {
            crate::constraint_view::draw(
                &mut frame,
                c,
                &self.camera,
                &self.colors,
                &Tokens::of(theme),
                self.constraint_pointer,
            );
        }
        param_grips(&mut frame, &self.params, &self.camera, &self.colors);
        marked(&mut frame, &self.marked, &self.camera, &self.colors, accent);
        if let Some((p, words)) = &self.found {
            found(&mut frame, *p, words, &self.camera, &self.colors, accent);
        }
        broken(
            &mut frame,
            &self.broken,
            &self.camera,
            &self.colors,
            Tokens::of(theme).danger,
        );
        if let Some(m) = &self.problem {
            problem(
                &mut frame,
                m,
                &self.camera,
                &self.colors,
                Tokens::of(theme).danger,
            );
        }
        if let Some(b) = self.select {
            select_box(&mut frame, b, &self.camera, &self.colors);
        }
        if let Some(l) = &self.locks {
            locks(&mut frame, l, &self.camera, &self.colors);
        }
        if let Some(t) = &self.tracking {
            tracking(&mut frame, t, &self.camera, &self.colors);
        }
        if let Some(hit) = self.snap {
            let [x, y] = self.camera.world_to_screen(hit.point);
            // On the pixel's middle, as the web strokes it.
            let at = Point::new(x.round() as f32 + 0.5, y.round() as f32 + 0.5);
            let label = self
                .tracking
                .as_ref()
                .and_then(|t| t.snap_label.as_deref())
                .unwrap_or(snap_label(hit.kind));
            let label = match &self.snap_note {
                Some(note) => format!("{label} · {note}"),
                None => label.to_owned(),
            };
            snap_marker(&mut frame, hit.kind, at, &label, &self.colors);
        }
        // Last, over the rest, as the web draws it.
        if let (Some(c), Some(at)) = (self.crosshair, cursor.position_in(bounds)) {
            crosshair(&mut frame, c, at, bounds.size(), &self.colors);
        }
        vec![frame.into_geometry()]
    }
}

/// The web's `drawGrips`: every grip but the one being moved, which is drawn
/// last, larger and in the drawing's ink.
fn grips(
    frame: &mut canvas::Frame,
    sets: &[GripSet],
    hot: &[(Slot, usize)],
    camera: &Camera,
    colors: &MarkColors,
    accent: iced::Color,
) {
    let edge =
        |width: f32, color: iced::Color| Stroke::default().with_color(color).with_width(width);
    let view = CameraView(camera);
    for set in sets {
        let mut last: Option<[f64; 2]> = None;
        for (i, &p) in set.points.iter().enumerate() {
            if hot.contains(&(set.slot, i)) {
                continue;
            }
            let s = camera.world_to_screen(p);
            let (x, y) = (s[0].round() as f32, s[1].round() as f32);
            if set.segments.get(i).is_some_and(Option::is_some) {
                // Mid grips (add a vertex, bend an arc): small hollow diamonds, hidden on short segments.
                if !set.shown(i, &view) {
                    continue;
                }
                let diamond = Path::new(|b| {
                    b.move_to(Point::new(x, y - 4.0));
                    b.line_to(Point::new(x + 4.0, y));
                    b.line_to(Point::new(x, y + 4.0));
                    b.line_to(Point::new(x - 4.0, y));
                    b.close();
                });
                frame.fill(&diamond, colors.halo);
                frame.stroke(&diamond, edge(1.0, accent));
                continue;
            }
            if last.is_some_and(|l| (s[0] - l[0]).abs() < 9.0 && (s[1] - l[1]).abs() < 9.0) {
                continue;
            }
            last = Some(s);
            frame.fill_rectangle(Point::new(x - 3.0, y - 3.0), Size::new(6.0, 6.0), accent);
            frame.stroke(
                &Path::rectangle(Point::new(x - 3.5, y - 3.5), Size::new(7.0, 7.0)),
                edge(1.0, colors.halo),
            );
        }
    }
    let moving = hot.iter().filter_map(|&(slot, index)| {
        sets.iter()
            .find(|set| set.slot == slot)
            .and_then(|set| set.points.get(index))
    });
    for &p in moving {
        let s = camera.world_to_screen(p);
        let (x, y) = (s[0].round() as f32, s[1].round() as f32);
        frame.fill_rectangle(Point::new(x - 4.0, y - 4.0), Size::new(8.0, 8.0), colors.fg);
        frame.stroke(
            &Path::rectangle(Point::new(x - 4.5, y - 4.5), Size::new(9.0, 9.0)),
            edge(1.5, accent),
        );
    }
}

/// A dynamic block's parameter grips (docs/adr/0319 §1), as AutoCAD's are told apart by their shapes: a linear's a
/// pair of arrowheads along its line, a polar's a diamond, an angle's a disc with a ring, a point's a disc, a flip's
/// two arrowheads facing each other; in the window box's blue, on a halo.
fn param_grips(
    frame: &mut canvas::Frame,
    grips: &[kentos_geometry_core::block_dynamic::ParamGrip],
    camera: &Camera,
    colors: &MarkColors,
) {
    use kentos_geometry_core::block_dynamic::GripKind;
    let fill = colors.window;
    let edge = Stroke::default().with_color(colors.halo).with_width(1.0);
    // A triangle with its tip `r` px from (x, y) along (ux, uy).
    let head = |x: f32, y: f32, ux: f32, uy: f32, r: f32| {
        let (px, py) = (-uy, ux);
        Path::new(|b| {
            b.move_to(Point::new(x + ux * r, y + uy * r));
            b.line_to(Point::new(x + px * 4.5, y + py * 4.5));
            b.line_to(Point::new(x - px * 4.5, y - py * 4.5));
            b.close();
        })
    };
    for g in grips {
        let s = camera.world_to_screen(g.at);
        let (x, y) = (s[0].round() as f32 + 0.5, s[1].round() as f32 + 0.5);
        match g.kind {
            GripKind::Linear => {
                // The screen's y runs down: the drawing's direction with its y turned.
                let (ux, uy) = g.along.map_or((1.0, 0.0), |u| (u.x as f32, -(u.y as f32)));
                for (sx, sy) in [(ux, uy), (-ux, -uy)] {
                    let p = head(x + sx * 3.0, y + sy * 3.0, sx, sy, 6.0);
                    frame.fill(&p, fill);
                    frame.stroke(&p, edge);
                }
            }
            GripKind::Polar => {
                let p = Path::new(|b| {
                    b.move_to(Point::new(x, y - 6.0));
                    b.line_to(Point::new(x + 6.0, y));
                    b.line_to(Point::new(x, y + 6.0));
                    b.line_to(Point::new(x - 6.0, y));
                    b.close();
                });
                frame.fill(&p, fill);
                frame.stroke(&p, edge);
            }
            GripKind::Angle => {
                let ring = Path::circle(Point::new(x, y), 6.0);
                frame.stroke(
                    &ring,
                    Stroke::default().with_color(colors.halo).with_width(3.5),
                );
                frame.stroke(&ring, Stroke::default().with_color(fill).with_width(2.0));
                frame.fill(&Path::circle(Point::new(x, y), 2.5), fill);
            }
            GripKind::Point => {
                let p = Path::circle(Point::new(x, y), 5.0);
                frame.fill(&p, fill);
                frame.stroke(&p, edge);
            }
            GripKind::Flip => {
                for sx in [1.0f32, -1.0] {
                    let p = head(x + sx * 7.0, y, -sx, 0.0, 6.0);
                    frame.fill(&p, fill);
                    frame.stroke(&p, edge);
                }
            }
        }
    }
}

/// The web's `drawMarkedVertices` (docs/adr/0172 §3): an accent ring round
/// each vertex Köşe tablosu's selected rows name, on a halo so it shows on
/// any drawing; at most 2 000.
fn marked(
    frame: &mut canvas::Frame,
    pts: &[Vec2],
    camera: &Camera,
    colors: &MarkColors,
    accent: iced::Color,
) {
    if pts.is_empty() {
        return;
    }
    let rings = Path::new(|b| {
        for &p in pts.iter().take(2000) {
            let s = camera.world_to_screen(p);
            let c = Point::new(s[0].round() as f32 + 0.5, s[1].round() as f32 + 0.5);
            b.circle(c, 8.0);
        }
    });
    let stroke =
        |width: f32, color: iced::Color| Stroke::default().with_color(color).with_width(width);
    frame.stroke(&rings, stroke(4.0, colors.halo));
    frame.stroke(&rings, stroke(2.0, accent));
}

/// The web's `drawSearchMark` (docs/adr/0178 §6): the place the data search
/// marked, an accent ring with four ticks and a dot in it on a halo so it
/// shows on any drawing, its coordinates written below on the right.
fn found(
    frame: &mut canvas::Frame,
    p: Vec2,
    words: &str,
    camera: &Camera,
    colors: &MarkColors,
    accent: iced::Color,
) {
    let s = camera.world_to_screen(p);
    let at = Point::new(s[0].round() as f32 + 0.5, s[1].round() as f32 + 0.5);
    let shape = Path::new(|b| {
        b.circle(at, 8.0);
        for (dx, dy) in [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)] {
            b.move_to(Point::new(at.x + dx * 8.0, at.y + dy * 8.0));
            b.line_to(Point::new(at.x + dx * 15.0, at.y + dy * 15.0));
        }
    });
    let stroke =
        |width: f32, color: iced::Color| Stroke::default().with_color(color).with_width(width);
    frame.stroke(&shape, stroke(4.0, colors.halo));
    frame.stroke(&shape, stroke(2.0, accent));
    frame.fill(&Path::circle(at, 2.0), accent);
    let label = Text {
        content: words.to_owned(),
        position: Point::new(at.x + 12.0, at.y + 12.0),
        color: colors.halo,
        size: Pixels(typography::scaled(11.0)),
        font: typography::ui_strong(),
        align_y: iced::alignment::Vertical::Top,
        ..Text::default()
    };
    for (dx, dy) in [
        (-1.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
    ] {
        frame.fill_text(Text {
            position: label.position + Vector::new(dx, dy),
            ..label.clone()
        });
    }
    frame.fill_text(Text {
        color: accent,
        ..label
    });
}

/// An edge as points along it on the screen (arcs every 7.5° at most).
fn edge_points(e: &kentos_geometry_core::geom::intersect::Edge, camera: &Camera) -> Vec<Point> {
    use kentos_geometry_core::geom::intersect::Edge;
    let screen = |x: f64, y: f64| {
        let s = camera.world_to_screen(Vec2::new(x, y));
        Point::new(s[0] as f32, s[1] as f32)
    };
    match *e {
        Edge::Seg { a, b } => vec![screen(a.x, a.y), screen(b.x, b.y)],
        Edge::Arc { c, r, a0, sweep } => {
            let n = ((sweep.abs() / (std::f64::consts::PI / 24.0)).ceil() as usize).max(2);
            (0..=n)
                .map(|i| {
                    let a = a0 + sweep * i as f64 / n as f64;
                    screen(c.x + a.cos() * r, c.y + a.sin() * r)
                })
                .collect()
        }
    }
}

/// The web's `drawProblemMark` (docs/adr/0202 §5): a topology finding's
/// regions filled and outlined, its edges bold, in the danger colour on a
/// halo; its place marked as Koordinata git marks one, its problem's name
/// beside it.
fn problem(
    frame: &mut canvas::Frame,
    m: &crate::topology::ProblemMark,
    camera: &Camera,
    colors: &MarkColors,
    danger: iced::Color,
) {
    use kentos_geometry_core::geom::region::ring_edges;
    let stroke =
        |width: f32, color: iced::Color| Stroke::default().with_color(color).with_width(width);
    if !m.regions.is_empty() {
        let shape = Path::new(|b| {
            for a in &m.regions {
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
        });
        frame.fill(
            &shape,
            canvas::Fill {
                style: canvas::Style::Solid(danger.scale_alpha(0.3)),
                rule: canvas::fill::Rule::EvenOdd,
            },
        );
        frame.stroke(&shape, stroke(3.5, colors.halo));
        frame.stroke(&shape, stroke(1.5, danger));
    }
    if !m.edges.is_empty() {
        let shape = Path::new(|b| {
            for e in &m.edges {
                for (k, q) in edge_points(e, camera).into_iter().enumerate() {
                    if k == 0 {
                        b.move_to(q);
                    } else {
                        b.line_to(q);
                    }
                }
            }
        });
        frame.stroke(&shape, stroke(6.0, colors.halo));
        frame.stroke(&shape, stroke(3.0, danger));
    }
    found(frame, m.at, &m.label, camera, colors, danger);
}

/// What a selected associative dimension is bound to (docs/adr/0320 §4): the object's edges dashed in the accent
/// colour on a halo half a pixel wider each side, and the place a filled diamond on a halo; only what is in the
/// view. One picture with exact coverage (label_picture.rs): the canvas's four samples left a rim round its
/// wider halo, and its arcs' long chords fell beside the object.
/// The parts a constraint names, as a dimension's bonds are shown (constraint_view.rs).
pub(crate) fn draw_bonds(
    frame: &mut canvas::Frame,
    parts: &[kentos_interaction::dimension_bonds::Bond],
    camera: &Camera,
    colors: &MarkColors,
    accent: iced::Color,
) {
    bonds(frame, parts, camera, colors, accent);
}

fn bonds(
    frame: &mut canvas::Frame,
    bonds: &[kentos_interaction::dimension_bonds::Bond],
    camera: &Camera,
    colors: &MarkColors,
    accent: iced::Color,
) {
    if bonds.is_empty() {
        return;
    }
    let size = frame.size();
    let screen = |p: Vec2| {
        let [x, y] = camera.world_to_screen(p);
        Point::new(x as f32, y as f32)
    };
    let room = (-20.0, -20.0, size.width + 20.0, size.height + 20.0);
    let mut runs: Vec<Vec<Point>> = Vec::new();
    let mut diamonds: Vec<Point> = Vec::new();
    for b in bonds {
        for run in &b.edges {
            let pts: Vec<Point> = run.iter().map(|q| screen(*q)).collect();
            runs.extend(clipped(&pts, room));
        }
        let at = screen(b.at);
        if at.x >= -10.0 && at.y >= -10.0 && at.x <= size.width + 10.0 && at.y <= size.height + 10.0
        {
            diamonds.push(Point::new(at.x.round() + 0.5, at.y.round() + 0.5));
        }
    }
    let lines: Vec<Path> = runs
        .iter()
        .map(|run| {
            Path::new(|p| {
                p.move_to(run[0]);
                for q in &run[1..] {
                    p.line_to(*q);
                }
            })
        })
        .collect();
    let shapes: Vec<Path> = diamonds
        .iter()
        .map(|&Point { x, y }| {
            Path::new(|p| {
                p.move_to(Point::new(x, y - 6.0));
                p.line_to(Point::new(x + 6.0, y));
                p.line_to(Point::new(x, y + 6.0));
                p.line_to(Point::new(x - 6.0, y));
                p.close();
            })
        })
        .collect();
    let dash = [6.0f32, 4.0];
    let bits = |c: iced::Color| [c.r.to_bits(), c.g.to_bits(), c.b.to_bits(), c.a.to_bits()];
    let mut numbers: Vec<u32> = bits(accent).into_iter().chain(bits(colors.halo)).collect();
    for run in &runs {
        numbers.push(run.len() as u32);
        numbers.extend(run.iter().flat_map(|q| [q.x.to_bits(), q.y.to_bits()]));
    }
    numbers.extend(diamonds.iter().flat_map(|q| [q.x.to_bits(), q.y.to_bits()]));
    let key = crate::label_picture::Key {
        what: "bonds".to_owned(),
        numbers,
    };
    use crate::label_picture::{Local, Part};
    let plain = Local { width_factor: 1.0 };
    let pictured = crate::label_picture::draw_on_screen(frame, key, |add| {
        add(Part::Stroke(&lines, plain, colors.halo, 3.0));
        add(Part::Dash(&lines, plain, accent, 2.0, &dash));
        add(Part::Stroke(&shapes, plain, colors.halo, 3.0));
        add(Part::Fill(&shapes, plain, accent));
    });
    if pictured {
        return;
    }
    // Too large for a picture: the canvas's paths.
    for line in &lines {
        frame.stroke(
            line,
            Stroke::default().with_color(colors.halo).with_width(3.0),
        );
        frame.stroke(
            line,
            Stroke {
                line_dash: canvas::LineDash {
                    segments: &dash,
                    offset: 0,
                },
                ..Stroke::default().with_color(accent).with_width(2.0)
            },
        );
    }
    for diamond in &shapes {
        frame.stroke(
            diamond,
            Stroke::default().with_color(colors.halo).with_width(3.0),
        );
        frame.fill(diamond, accent);
    }
}

/// A run of screen points cut to the box `(left, top, right, bottom)`: the pieces inside, each of two or more
/// points (Liang–Barsky on every segment).
fn clipped(run: &[Point], (l, t, r, b): (f32, f32, f32, f32)) -> Vec<Vec<Point>> {
    let mut out: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    for w in run.windows(2) {
        let (p, q) = (w[0], w[1]);
        let (dx, dy) = (q.x - p.x, q.y - p.y);
        let (mut t0, mut t1) = (0.0f32, 1.0f32);
        let mut inside = true;
        for (pp, qq) in [(-dx, p.x - l), (dx, r - p.x), (-dy, p.y - t), (dy, b - p.y)] {
            if pp == 0.0 {
                if qq < 0.0 {
                    inside = false;
                }
            } else {
                let u = qq / pp;
                if pp < 0.0 {
                    t0 = t0.max(u);
                } else {
                    t1 = t1.min(u);
                }
            }
        }
        if !inside || t0 > t1 {
            if current.len() >= 2 {
                out.push(std::mem::take(&mut current));
            }
            current.clear();
            continue;
        }
        let a = Point::new(p.x + t0 * dx, p.y + t0 * dy);
        let c = Point::new(p.x + t1 * dx, p.y + t1 * dy);
        if current.last() != Some(&a) {
            if current.len() >= 2 {
                out.push(std::mem::take(&mut current));
            }
            current.clear();
            current.push(a);
        }
        current.push(c);
        if t1 < 1.0 {
            out.push(std::mem::take(&mut current));
        }
    }
    if current.len() >= 2 {
        out.push(current);
    }
    out
}

/// A broken reference of an associative dimension (docs/adr/0249 §3): a small warning triangle, its tip over the
/// point, in the danger colour on a halo, “!” inside; only those in the view.
fn broken(
    frame: &mut canvas::Frame,
    points: &[Vec2],
    camera: &Camera,
    colors: &MarkColors,
    danger: iced::Color,
) {
    let size = frame.size();
    for p in points {
        let [x, y] = camera.world_to_screen(*p);
        let (x, y) = (x.round() as f32 + 0.5, y.round() as f32 + 0.5);
        if x < -20.0 || y < -20.0 || x > size.width + 20.0 || y > size.height + 20.0 {
            continue;
        }
        // Above the point, so the dimension's own end stays seen.
        let (top, base) = (y - 20.0, y - 4.0);
        let tri = Path::new(|b| {
            b.move_to(Point::new(x, top));
            b.line_to(Point::new(x + 9.0, base));
            b.line_to(Point::new(x - 9.0, base));
            b.close();
        });
        frame.stroke(
            &tri,
            Stroke::default().with_color(colors.halo).with_width(3.5),
        );
        frame.fill(&tri, danger);
        frame.fill_text(Text {
            content: "!".to_owned(),
            position: Point::new(x, base - 1.0),
            color: colors.halo,
            size: Pixels(typography::scaled(10.0)),
            font: typography::ui_strong(),
            align_x: iced::advanced::text::Alignment::Center,
            align_y: iced::alignment::Vertical::Bottom,
            ..Text::default()
        });
        let tick = Path::circle(Point::new(x, y), 2.5);
        frame.fill(&tick, danger);
    }
}

/// The web's `drawCrosshair`: the drawing's ink at 85 %, 1 px on the
/// pixel's middle; a pick box 10 px across while an object is picked.
fn crosshair(frame: &mut canvas::Frame, c: Crosshair, at: Point, area: Size, colors: &MarkColors) {
    let (x, y) = (at.x.round() + 0.5, at.y.round() + 0.5);
    // Across the drawing at most: a full arm reaches every edge.
    let arm = c.size.arm(c.pick).min(area.width + area.height);
    let gap = if c.pick { 5.0 } else { 0.0 };
    let cross = Path::new(|b| {
        b.move_to(Point::new(x - arm, y));
        b.line_to(Point::new(x - gap, y));
        b.move_to(Point::new(x + gap, y));
        b.line_to(Point::new(x + arm, y));
        b.move_to(Point::new(x, y - arm));
        b.line_to(Point::new(x, y - gap));
        b.move_to(Point::new(x, y + gap));
        b.line_to(Point::new(x, y + arm));
        if c.pick {
            b.rectangle(
                Point::new(x - gap, y - gap),
                Size::new(gap * 2.0, gap * 2.0),
            );
        }
    });
    frame.stroke(
        &cross,
        Stroke::default()
            .with_color(colors.fg.scale_alpha(0.85))
            .with_width(1.0),
    );
}

/// A world direction (x east, y north) as the screen shows it, `k` pixels long per unit: right and down, the
/// view's twist taken in (docs/adr/0285 §2).
pub(crate) fn screen_dir(camera: &Camera, x: f64, y: f64, k: f64) -> (f32, f32) {
    let (c, s) = camera.twist_cos_sin();
    ((k * (x * c + y * s)) as f32, (-k * (y * c - x * s)) as f32)
}

/// The web's `drawSelectionBox`. The corners are where the camera now shows their world points (docs/adr/0277): a
/// pan or a zoom while the button is down keeps the box on the drawing.
fn select_box(frame: &mut canvas::Frame, b: SelectBox, camera: &Camera, colors: &MarkColors) {
    let b = match b.worlds {
        Some((from, to)) => SelectBox {
            from: camera.world_to_screen(from),
            to: camera.world_to_screen(to),
            ..b
        },
        None => b,
    };
    let crossing = b.crossing();
    let color = if crossing { colors.snap } else { colors.window };
    let (x0, y0) = (b.from[0].min(b.to[0]) as f32, b.from[1].min(b.to[1]) as f32);
    let size = Size::new(
        (b.to[0] - b.from[0]).abs() as f32,
        (b.to[1] - b.from[1]).abs() as f32,
    );
    frame.fill_rectangle(Point::new(x0, y0), size, color.scale_alpha(0.1));
    let outline = Path::rectangle(Point::new(x0 + 0.5, y0 + 0.5), size);
    let segments: &[f32] = if crossing { &[5.0, 4.0] } else { &[] };
    frame.stroke(
        &outline,
        Stroke {
            line_dash: LineDash {
                segments,
                offset: 0,
            },
            ..Stroke::default().with_color(color).with_width(1.0)
        },
    );
}

/// The web's `drawLocks`: the locked direction a dashed line from the
/// reference (a ray when it runs one way), the locked length a dashed circle
/// round it; dashed 3/4 at 85 %, as the tracking guides.
fn locks(frame: &mut canvas::Frame, l: &LockMarks, camera: &Camera, colors: &MarkColors) {
    let dashed = Stroke {
        line_dash: LineDash {
            segments: &[3.0, 4.0],
            offset: 0,
        },
        ..Stroke::default()
            .with_color(iced::Color {
                a: colors.snap.a * 0.85,
                ..colors.snap
            })
            .with_width(1.0)
    };
    let [ox, oy] = camera.world_to_screen(l.reference);
    let o = Point::new(ox as f32, oy as f32);
    // The reference point: a cross and “R” (the web's `drawLocks`).
    if let Some(r) = l.named {
        let [x, y] = camera.world_to_screen(r);
        let c = Point::new(x as f32, y as f32);
        let solid = Stroke::default().with_color(colors.snap).with_width(1.5);
        frame.stroke(
            &Path::line(c - Vector::new(6.0, 6.0), c + Vector::new(6.0, 6.0)),
            solid,
        );
        frame.stroke(
            &Path::line(c - Vector::new(6.0, -6.0), c + Vector::new(6.0, -6.0)),
            solid,
        );
        halo_label(frame, "R", c, colors);
    }
    if !l.locks {
        return;
    }
    // The picked edge, solid (the web's `drawLocks`).
    if let Some(edge) = l.edge {
        let at = |p: Vec2| {
            let [x, y] = camera.world_to_screen(p);
            Point::new(x as f32, y as f32)
        };
        let path = Path::new(|b| match edge {
            kentos_geometry_core::geom::intersect::Edge::Seg { a, b: end } => {
                b.move_to(at(a));
                b.line_to(at(end));
            }
            kentos_geometry_core::geom::intersect::Edge::Arc { c, r, a0, sweep } => {
                let n = ((sweep.abs() / (std::f64::consts::PI / 48.0)).ceil() as usize).max(8);
                for i in 0..=n {
                    let t = a0 + sweep * i as f64 / n as f64;
                    let q = at(Vec2::new(c.x + r * t.cos(), c.y + r * t.sin()));
                    if i == 0 {
                        b.move_to(q);
                    } else {
                        b.line_to(q);
                    }
                }
            }
        });
        frame.stroke(
            &path,
            Stroke::default().with_color(colors.snap).with_width(2.0),
        );
    }
    if let Some((u, both)) = l.direction {
        let (dx, dy) = screen_dir(camera, u.x, u.y, 1e4);
        let back = if both { 1.0 } else { 0.0 };
        frame.stroke(
            &Path::line(
                o - Vector::new(dx * back, dy * back),
                o + Vector::new(dx, dy),
            ),
            dashed,
        );
    }
    for &(through, u) in l.lines.iter().flatten() {
        let [x, y] = camera.world_to_screen(through);
        let p = Point::new(x as f32, y as f32);
        let (dx, dy) = screen_dir(camera, u.x, u.y, 1e4);
        frame.stroke(
            &Path::line(p - Vector::new(dx, dy), p + Vector::new(dx, dy)),
            dashed,
        );
    }
    if let Some(length) = l.length {
        let r = (length * camera.scale) as f32;
        // A circle that is a dot, or wider than any screen, says nothing.
        if r > 1.0 && r < 1e5 {
            frame.stroke(&Path::circle(o, r), dashed);
        }
    }
}

/// The web's `drawSnap`: the kind's glyph and its name above-right.
/// The web's `drawObjectTracking`: a 10 px cross on every acquired point
/// (1.5 px, the snap colour); the lock's lines dashed 3/4 from their points
/// across the area, at 85 %; and its label above-right of the cursor's
/// point with the area's colour as a halo.
fn tracking(frame: &mut canvas::Frame, t: &TrackingMarks, camera: &Camera, colors: &MarkColors) {
    let cross = Stroke::default().with_color(colors.snap).with_width(1.5);
    let dashed = Stroke {
        line_dash: LineDash {
            segments: &[3.0, 4.0],
            offset: 0,
        },
        ..Stroke::default()
            .with_color(iced::Color {
                a: colors.snap.a * 0.85,
                ..colors.snap
            })
            .with_width(1.0)
    };
    let screen = |p: Vec2| {
        let [x, y] = camera.world_to_screen(p);
        Point::new(x as f32, y as f32)
    };
    // The snap additions' guides under the marks (docs/adr/0163 §2).
    for guide in &t.paths {
        match guide {
            Guide::Path(points) => {
                let path = Path::new(|b| {
                    for (i, &p) in points.iter().enumerate() {
                        if i == 0 {
                            b.move_to(screen(p));
                        } else {
                            b.line_to(screen(p));
                        }
                    }
                });
                frame.stroke(&path, dashed);
            }
            Guide::Line { through, dir } => {
                let o = screen(*through);
                let (dx, dy) = screen_dir(camera, dir.x, dir.y, 1e4);
                frame.stroke(
                    &Path::line(o - Vector::new(dx, dy), o + Vector::new(dx, dy)),
                    dashed,
                );
            }
        }
    }
    // An acquired edge: two short strokes along it where it was rested on.
    for &(at, dir) in &t.edges {
        let c = screen(at);
        let (ux, uy) = screen_dir(camera, dir.x, dir.y, 1.0);
        let (nx, ny) = (-uy * 2.5, ux * 2.5);
        frame.stroke(
            &Path::new(|b| {
                for side in [-1.0, 1.0] {
                    let o = c + Vector::new(nx * side, ny * side);
                    b.move_to(o - Vector::new(ux * 5.0, uy * 5.0));
                    b.line_to(o + Vector::new(ux * 5.0, uy * 5.0));
                }
            }),
            cross,
        );
    }
    for &p in &t.acquired {
        let [x, y] = camera.world_to_screen(p);
        let (x, y) = (x.round() as f32 + 0.5, y.round() as f32 + 0.5);
        frame.stroke(
            &Path::new(|b| {
                b.move_to(Point::new(x - 5.0, y));
                b.line_to(Point::new(x + 5.0, y));
                b.move_to(Point::new(x, y - 5.0));
                b.line_to(Point::new(x, y + 5.0));
            }),
            cross,
        );
    }
    let Some(track) = &t.track else {
        return;
    };
    for line in &track.lines {
        let [ox, oy] = camera.world_to_screen(line.origin);
        let r = line.angle.to_radians();
        let (dx, dy) = screen_dir(camera, r.cos(), r.sin(), 1e4);
        frame.stroke(
            &Path::line(
                Point::new(ox as f32, oy as f32),
                Point::new(ox as f32 + dx, oy as f32 + dy),
            ),
            dashed,
        );
    }
    if let Some(text) = &t.label {
        let [x, y] = camera.world_to_screen(track.point);
        let at = Point::new(x.round() as f32, y.round() as f32);
        halo_label(frame, text, at, colors);
    }
}

/// A label above-right of `at` (bottom at y − 7), the snap colour on a halo of the area's.
fn halo_label(frame: &mut canvas::Frame, content: &str, at: Point, colors: &MarkColors) {
    let label = Text {
        content: content.to_owned(),
        position: at + Vector::new(9.0, -7.0),
        color: colors.halo,
        size: Pixels(typography::scaled(10.5)),
        font: typography::ui_strong(),
        align_y: iced::alignment::Vertical::Bottom,
        ..Text::default()
    };
    for (dx, dy) in [
        (-1.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
    ] {
        frame.fill_text(Text {
            position: label.position + Vector::new(dx, dy),
            ..label.clone()
        });
    }
    frame.fill_text(Text {
        color: colors.snap,
        ..label
    });
}

fn snap_marker(
    frame: &mut canvas::Frame,
    kind: SnapKind,
    at: Point,
    text: &str,
    colors: &MarkColors,
) {
    let (x, y) = (at.x, at.y);
    let p = |dx: f32, dy: f32| Point::new(x + dx, y + dy);
    let glyph = Path::new(|b| match kind {
        SnapKind::Midpoint => {
            b.move_to(p(0.0, -6.0));
            b.line_to(p(6.0, 5.0));
            b.line_to(p(-6.0, 5.0));
            b.close();
        }
        SnapKind::Center | SnapKind::Node => b.circle(at, 5.5),
        SnapKind::Quadrant => {
            b.move_to(p(0.0, -6.0));
            b.line_to(p(6.0, 0.0));
            b.line_to(p(0.0, 6.0));
            b.line_to(p(-6.0, 0.0));
            b.close();
        }
        SnapKind::Intersection => {
            b.move_to(p(-5.0, -5.0));
            b.line_to(p(5.0, 5.0));
            b.move_to(p(5.0, -5.0));
            b.line_to(p(-5.0, 5.0));
        }
        SnapKind::Perpendicular => {
            b.move_to(p(-6.0, -6.0));
            b.line_to(p(-6.0, 5.0));
            b.line_to(p(6.0, 5.0));
            b.move_to(p(-6.0, 0.0));
            b.line_to(p(0.0, 0.0));
            b.line_to(p(0.0, 5.0));
        }
        SnapKind::Tangent => {
            b.circle(p(0.0, 1.0), 4.5);
            b.move_to(p(-6.5, -4.5));
            b.line_to(p(6.5, -4.5));
        }
        SnapKind::Nearest => {
            b.move_to(p(-5.0, -5.0));
            b.line_to(p(5.0, -5.0));
            b.line_to(p(-5.0, 5.0));
            b.line_to(p(5.0, 5.0));
            b.close();
        }
        // docs/adr/0163 §1: a diamond with a dot; a plus on a dashed line
        // (the line below); two slanted strokes; a small grid.
        SnapKind::Centroid => {
            b.move_to(p(0.0, -6.0));
            b.line_to(p(6.0, 0.0));
            b.line_to(p(0.0, 6.0));
            b.line_to(p(-6.0, 0.0));
            b.close();
            b.circle(at, 1.6);
        }
        SnapKind::Extension => {
            b.move_to(p(0.0, -5.0));
            b.line_to(p(0.0, 5.0));
            b.move_to(p(-5.0, 0.0));
            b.line_to(p(5.0, 0.0));
        }
        SnapKind::Parallel => {
            b.move_to(p(-7.0, 6.0));
            b.line_to(p(-1.0, -6.0));
            b.move_to(p(-1.0, 6.0));
            b.line_to(p(5.0, -6.0));
        }
        SnapKind::Grid => {
            for d in [-3.0, 3.0] {
                b.move_to(p(d, -5.0));
                b.line_to(p(d, 5.0));
                b.move_to(p(-5.0, d));
                b.line_to(p(5.0, d));
            }
        }
        // docs/adr/0269 §1: two squares, one over the other's corner (AutoCAD's insertion mark).
        SnapKind::Insertion => {
            b.rectangle(p(-6.0, -6.0), Size::new(7.0, 7.0));
            b.rectangle(p(-1.0, -1.0), Size::new(7.0, 7.0));
        }
        SnapKind::Endpoint => b.rectangle(p(-5.0, -5.0), Size::new(10.0, 10.0)),
    });
    let stroke = Stroke::default().with_color(colors.snap).with_width(1.5);
    frame.stroke(&glyph, stroke);
    if kind == SnapKind::Extension {
        let line = Path::line(p(-9.0, 0.0), p(9.0, 0.0));
        frame.stroke(
            &line,
            Stroke {
                line_dash: LineDash {
                    segments: &[3.0, 2.5],
                    offset: 0,
                },
                ..stroke
            },
        );
    }
    // Above-right; bottom of the text at y − 7, with the area's colour as a halo.
    let label = Text {
        content: text.to_owned(),
        position: p(9.0, -7.0),
        color: colors.halo,
        size: Pixels(typography::scaled(10.5)),
        font: typography::ui_strong(),
        align_y: iced::alignment::Vertical::Bottom,
        ..Text::default()
    };
    for (dx, dy) in [
        (-1.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
    ] {
        frame.fill_text(Text {
            position: label.position + Vector::new(dx, dy),
            ..label.clone()
        });
    }
    frame.fill_text(Text {
        color: colors.snap,
        ..label
    });
}

impl crate::app::App {
    /// The crosshair the marks draw at the pointer (the web's
    /// `drawCrosshair`): the running tool's kind, the chosen arms; none for
    /// Kaydır or while the middle button pans.
    pub(crate) fn crosshair_mark(&self) -> Option<Crosshair> {
        use kentos_interaction::Cursor;
        match self.session.cursor() {
            _ if self.panning => None,
            Cursor::Grab => None,
            cursor => Some(Crosshair {
                size: self.crosshair,
                pick: cursor == Cursor::Pick,
            }),
        }
    }
}

impl crate::app::App {
    /// What the selected associative dimensions are bound to (docs/adr/0320 §4); none while a tool runs or the
    /// selection is past the grips' limit.
    pub(crate) fn bond_marks(&self) -> Vec<kentos_interaction::dimension_bonds::Bond> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        if self.selection.len() > kentos_interaction::select::GRIP_LIMIT {
            return Vec::new();
        }
        kentos_interaction::dimension_bonds::bonds(&doc.model, self.selection.ids())
    }

    /// The points of the associative dimensions whose reference broke (docs/adr/0249 §3): from the document's index
    /// of such dimensions, not a walk over the drawing; and where walls meet and do not join (docs/adr/0340 §6), from
    /// the store's joins; the openings in no wall and the spaces that lost their walls.
    pub(crate) fn broken_marks(&self) -> Vec<Vec2> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let model = &doc.model;
        let mut out: Vec<Vec2> = self
            .spatial
            .store()
            .wall_issues()
            .iter()
            .map(|i| Vec2::new(i.at.x, i.at.y))
            .collect();
        // The openings in no wall (docs/adr/0341 §1).
        out.extend(
            self.spatial
                .store()
                .loose_openings()
                .into_iter()
                .map(|p| Vec2::new(p.x, p.y)),
        );
        // The spaces whose walls no longer close round them (docs/adr/0342 §3), above their label: its lines stand
        // about the pole, four of its heights tall.
        let above = 4.0 * kentos_geometry_core::geom::space::LABEL_MM * model.settings().plot_scale / 1000.0;
        for slot in model.lost_spaces() {
            match model.get(slot) {
                Some(kentos_contracts::Entity::Space(s)) => {
                    if let Some(p) = kentos_native_application::spaces::label_point(&s.space.parts) {
                        out.push(Vec2::new(p.x, p.y + above));
                    }
                }
                // A column whose grid is gone (docs/adr/0343 §3), at its centre.
                Some(kentos_contracts::Entity::Column(c)) => out.push(Vec2::new(c.column.at.x, c.column.at.y)),
                // A tag whose object is gone (docs/adr/0344 §1), above its middle.
                Some(kentos_contracts::Entity::Tag(t)) => out.push(Vec2::new(t.tag.at.x, t.tag.at.y + above)),
                _ => {}
            }
        }
        for slot in model.broken_dimensions() {
            let Some(kentos_contracts::Entity::Dimension(d)) = model.get(slot) else {
                continue;
            };
            let Some(refs) = &d.refs else { continue };
            for (k, r) in refs.each() {
                let p = match k {
                    "a" => Some(d.a),
                    "b" => Some(d.b),
                    _ => d.c,
                };
                if let (false, Some(p)) = (r.live(), p) {
                    out.push(Vec2::new(p.x, p.y));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod bond_tests {
    use super::*;

    #[test]
    fn a_run_is_cut_to_the_box() {
        let p = |x: f32, y: f32| Point::new(x, y);
        let room = (0.0, 0.0, 100.0, 100.0);
        // Inside: as it is.
        assert_eq!(
            clipped(&[p(10.0, 10.0), p(20.0, 10.0), p(20.0, 30.0)], room),
            vec![vec![p(10.0, 10.0), p(20.0, 10.0), p(20.0, 30.0)]]
        );
        // Across the box: the inside piece only.
        assert_eq!(
            clipped(&[p(-50.0, 50.0), p(150.0, 50.0)], room),
            vec![vec![p(0.0, 50.0), p(100.0, 50.0)]]
        );
        // Out and back in: two pieces.
        assert_eq!(
            clipped(
                &[p(50.0, 50.0), p(50.0, 200.0), p(60.0, 200.0), p(60.0, 50.0)],
                room
            ),
            vec![
                vec![p(50.0, 50.0), p(50.0, 100.0)],
                vec![p(60.0, 100.0), p(60.0, 50.0)]
            ]
        );
        // Wholly outside: nothing.
        assert!(clipped(&[p(-10.0, -10.0), p(-20.0, 300.0)], room).is_empty());
    }
}

#[cfg(test)]
mod crosshair_tests {
    use super::*;
    use crate::app::Message;
    use crate::files_testing::app_with_drawing;

    #[test]
    fn the_arms_are_the_webs_and_shorter_for_a_pick() {
        use CrosshairSize::*;
        assert_eq!((Small.arm(false), Small.arm(true)), (16.0, 9.0));
        assert_eq!((Medium.arm(false), Medium.arm(true)), (40.0, 22.0));
        assert_eq!((Full.arm(false), Full.arm(true)), (1e5, 1e5));
        assert_eq!(CrosshairSize::parse("full"), Full);
        assert_eq!(CrosshairSize::parse("?"), Medium);
    }

    #[test]
    fn the_crosshair_follows_the_tool_the_setting_and_the_pan() {
        let mut app = app_with_drawing();
        let pick = |size| Some(Crosshair { size, pick: true });
        // No command: the select tool picks objects.
        assert_eq!(app.crosshair_mark(), pick(CrosshairSize::Medium));
        // A drawing tool wants a point: the full cross.
        let _ = app.update(Message::Run("tool.line"));
        assert_eq!(
            app.crosshair_mark(),
            Some(Crosshair {
                size: CrosshairSize::Medium,
                pick: false
            })
        );
        // Sil picks, Kaydır shows its hand instead.
        let _ = app.update(Message::Run("tool.erase"));
        assert_eq!(app.crosshair_mark(), pick(CrosshairSize::Medium));
        let _ = app.update(Message::Run("tool.pan"));
        assert_eq!(app.crosshair_mark(), None);
        let _ = app.update(Message::Run("tool.cancel"));
        // The setting's arms, kept through the settings window's Kaydet.
        let _ = app
            .settings
            .choose(&[("appearance.crosshair", serde_json::json!("full"))]);
        app.apply_settings();
        assert_eq!(app.crosshair_mark(), pick(CrosshairSize::Full));
        // None while the middle button pans; back with the next plain move.
        let _ = app.update(Message::Viewport(crate::viewport::Event::Panned {
            by: iced::Vector::new(4.0, 0.0),
            at: iced::Point::new(100.0, 100.0),
        }));
        assert_eq!(app.crosshair_mark(), None);
        let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(
            iced::Point::new(110.0, 100.0),
        )));
        assert_eq!(app.crosshair_mark(), pick(CrosshairSize::Full));
    }

    /// Pictures for the owner: the pick crosshair, the drawing tool's cross,
    /// the full crosshair, with grid north and the scale bar
    /// (`.run/shots/imlec-*`):
    ///
    /// ```text
    /// cargo test -p kentos-desktop marks::crosshair_tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::{Point, Size, mouse};
        use kentos_ui::snapshot::Snapshot;

        let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let shot = |app: &mut crate::app::App, name: &str, size: Size, at: Point| {
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut crate::app::App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, crate::app::App::view, &mut update);
            snapshot.step(
                app,
                crate::app::App::view,
                &mut update,
                &[iced::Event::Mouse(mouse::Event::CursorMoved {
                    position: at,
                })],
            );
            snapshot.settle(app, crate::app::App::view, &mut update);
            let file = out.join(format!("{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let wide = Size::new(1440.0, 900.0);
            let narrow = Size::new(1100.0, 650.0);
            shot(
                &mut app,
                &format!("imlec-secim-1440{suffix}"),
                wide,
                Point::new(520.0, 470.0),
            );
            let _ = app.update(Message::Run("tool.line"));
            shot(
                &mut app,
                &format!("imlec-cizgi-1100{suffix}"),
                narrow,
                Point::new(380.0, 360.0),
            );
            let _ = app.update(Message::Run("tool.cancel"));
            let _ = app
                .settings
                .choose(&[("appearance.crosshair", serde_json::json!("full"))]);
            app.apply_settings();
            shot(
                &mut app,
                &format!("imlec-tam-1440{suffix}"),
                wide,
                Point::new(520.0, 470.0),
            );
        }
    }
}
