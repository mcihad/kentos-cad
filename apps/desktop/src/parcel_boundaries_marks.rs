//! Sınır grafiği on the drawing (docs/adr/2220): Çizimde göster closes the window and leaves the graph's marks over
//! the drawing until İşareti kaldır (`data.unmark`) or another drawing. The graph is worked out again whenever the
//! drawing changes, so a fixed corner's ring goes at once. Each problem is a ring in the danger colour with its kind's
//! words. Each corner is a dot with its number (K…), each boundary's number (S…) stands at its middle on its curve.
//! On the screen a number that would crowd another is left out (a problem's words never), so the view reads at any
//! zoom. The row chosen in the window's table, when one was, is brought forward in the accent: a boundary along its
//! curve, a corner or a problem with a double ring, its name beside it (docs/adr/3020). Nothing is written.
//!
//! docs/adr/3820: an arc drawn with chords shows its strip, the room between the arc and the chain of chords, filled
//! in the danger colour and outlined; problems whose rings would meet on the screen share one ring and one phrase
//! (“Komşuda eksik köşe (2) · Ayrık köşe”), so their words no longer lie over each other.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Pixels, Point, Vector};
use kentos_commands::{ParcelsBoundaries, ParcelsBounded};
use kentos_interaction::Vec2;
use kentos_native_application::parcels_boundaries;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::typography;

use crate::app::App;
use crate::viewport::MarkColors;

/// The most numbers written on the screen at once.
const MOST_NUMBERS: usize = 600;
/// A number's room on the screen, logical pixels: no two numbers within a cell of each other.
const ROOM: (f32, f32) = (34.0, 16.0);
/// A problem's ring's radius on the screen, logical pixels; problems nearer than two of them share one (docs/adr/3820).
const RING: f32 = 9.0;

/// What Çizimde göster keeps: the drawing's session, the parcels asked (none: the parcel layer's), and the marks of
/// the drawing as it was last seen (its generation and revision).
#[derive(Debug, Default)]
pub struct Shown {
    pub session: u64,
    pub uids: Option<Vec<String>>,
    /// The window's distance for arcs drawn with chords (docs/adr/3420; none: 1 µm).
    pub tolerance: Option<f64>,
    /// The row chosen when Çizimde göster was pressed (docs/adr/3020).
    pub focus: Option<Focus>,
    made: RefCell<Option<(u64, u64, Option<Arc<GraphMarks>>)>>,
}

impl Shown {
    pub fn new(session: u64, uids: Option<Vec<String>>) -> Self {
        Self { session, uids, tolerance: None, focus: None, made: RefCell::new(None) }
    }
}

/// The marks: each problem's place and words, each corner's place and number, each boundary's middle and number.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphMarks {
    pub problems: Vec<(Vec2, String)>,
    pub corners: Vec<(Vec2, String)>,
    pub boundaries: Vec<(Vec2, String)>,
    /// Each arc drawn with chords' strip (docs/adr/3820): the arc from its first corner to its last, then the chain
    /// back, a closed outline.
    pub strips: Vec<Vec<Vec2>>,
    /// The chosen row's places (a boundary's curve, a corner's or a problem's place) and its name (docs/adr/3020).
    pub focus: Option<(Vec<Vec2>, String)>,
}

/// The row Çizimde göster goes to (docs/adr/3020): a boundary or a corner by its id, a problem by its kind and place,
/// so it is found again when the graph is worked out anew (gone when the drawing no longer has it).
#[derive(Clone, Debug, PartialEq)]
pub enum Focus {
    Boundary(String),
    Corner(String),
    Problem(String, [f64; 2]),
}

impl Focus {
    /// The row `i` of the table showing `show`.
    pub fn of(g: &ParcelsBounded, show: crate::parcel_boundaries_window::Show, i: usize) -> Option<Focus> {
        use crate::parcel_boundaries_window::Show;
        match show {
            Show::Boundaries => g.boundaries.get(i).map(|b| Focus::Boundary(b.id.clone())),
            Show::Corners => g.corners.get(i).map(|c| Focus::Corner(c.id.clone())),
            Show::Problems => g.problems.get(i).map(|p| Focus::Problem(p.kind.clone(), p.at)),
        }
    }

    /// Its places in `g`: a boundary's curve (an arc as points along it), a corner's or a problem's place; none when
    /// `g` no longer has it.
    pub fn points(&self, g: &ParcelsBounded) -> Vec<Vec2> {
        let at = |p: [f64; 2]| Vec2::new(p[0], p[1]);
        let corner = |id: &str| g.corners.iter().find(|c| c.id == id).map(|c| at(c.at));
        match self {
            Focus::Boundary(id) => {
                let Some(b) = g.boundaries.iter().find(|b| b.id == *id) else { return Vec::new() };
                let (Some(a), Some(e)) = (corner(&b.from), corner(&b.to)) else { return Vec::new() };
                if b.bulge == 0.0 {
                    return vec![a, e];
                }
                // A chord's sagitta under a two-thousandth of the boundary: smooth at any view that shows it whole.
                let tol = ((e.x - a.x).hypot(e.y - a.y) / 2000.0).max(1e-6);
                kentos_geometry_core::tessellate::bulge_path(&[a, e], Some(&[b.bulge]), false, tol)
            }
            Focus::Corner(id) => corner(id).into_iter().collect(),
            Focus::Problem(kind, p) => {
                if g.problems.iter().any(|q| q.kind == *kind && q.at == *p) { vec![at(*p)] } else { Vec::new() }
            }
        }
    }

    /// Its name: the boundary's or the corner's id, the problem's words.
    pub fn name(&self) -> String {
        match self {
            Focus::Boundary(id) | Focus::Corner(id) => id.clone(),
            Focus::Problem(kind, _) => crate::parcel_boundaries_window::problem_word(kind).to_owned(),
        }
    }

    /// The view on it: its places' box, at least a tenth of the graph's (`all`) wide each way.
    pub fn bounds(&self, g: &ParcelsBounded, all: Option<[f64; 4]>) -> Option<[f64; 4]> {
        let pts = self.points(g);
        let first = pts.first()?;
        let b = pts.iter().fold([first.x, first.y, first.x, first.y], |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]);
        let least = all.map_or(10.0, |a| ((a[2] - a[0]).max(a[3] - a[1]) * 0.1).max(2.0));
        let half = ((b[2] - b[0]).max(b[3] - b[1]) / 2.0).max(least / 2.0);
        let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
        Some([cx - half, cy - half, cx + half, cy + half])
    }
}

impl GraphMarks {
    pub fn of(g: &ParcelsBounded) -> Self {
        let at = |p: [f64; 2]| Vec2::new(p[0], p[1]);
        let places: BTreeMap<&str, Vec2> = g.corners.iter().map(|c| (c.id.as_str(), at(c.at))).collect();
        let boundaries = g
            .boundaries
            .iter()
            .filter_map(|b| {
                let (a, e) = (places.get(b.from.as_str())?, places.get(b.to.as_str())?);
                // Its middle on its curve: a positive bulge bows right of the way (docs/adr/2000).
                let (dx, dy) = (e.x - a.x, e.y - a.y);
                let middle = Vec2::new((a.x + e.x) / 2.0 + dy * b.bulge / 2.0, (a.y + e.y) / 2.0 - dx * b.bulge / 2.0);
                Some((middle, b.id.clone()))
            })
            .collect();
        // The strip is as narrow as its widest place (`gap`): its arc drawn by points two hundred times finer.
        let strips = g
            .problems
            .iter()
            .filter(|p| p.kind == "approxArc")
            .filter_map(|p| strip(g, &places, &p.boundaries, p.gap.map(|w| (w / 200.0).max(1e-6))))
            .collect();
        Self {
            focus: None,
            strips,
            problems: g
                .problems
                .iter()
                .map(|p| (at(p.at), crate::parcel_boundaries_window::problem_short(&p.kind).to_owned()))
                .collect(),
            corners: g.corners.iter().map(|c| (at(c.at), c.id.clone())).collect(),
            boundaries,
        }
    }
}

/// A boundary's curve from `a` to `e`, an arc as points along it: a chord's sagitta under a two-thousandth of it, or
/// under `finer` when that is less.
fn curve(a: Vec2, e: Vec2, bulge: f64, finer: Option<f64>) -> Vec<Vec2> {
    if bulge == 0.0 {
        return vec![a, e];
    }
    let tol = ((e.x - a.x).hypot(e.y - a.y) / 2000.0).max(1e-6);
    let tol = finer.map_or(tol, |f| f.min(tol));
    kentos_geometry_core::tessellate::bulge_path(&[a, e], Some(&[bulge]), false, tol)
}

/// An arc drawn with chords' strip (docs/adr/3420's `boundaries`: the arc, then the chain's boundaries): the arc from
/// its `from` corner to its `to`, then the chain walked back from `to` to `from`; none when the chain does not join
/// them.
fn strip(g: &ParcelsBounded, places: &BTreeMap<&str, Vec2>, ids: &[String], finer: Option<f64>) -> Option<Vec<Vec2>> {
    let find = |id: &String| g.boundaries.iter().find(|b| b.id == *id);
    let (arc, chain) = ids.split_first()?;
    let arc = find(arc)?;
    let mut out = curve(*places.get(arc.from.as_str())?, *places.get(arc.to.as_str())?, arc.bulge, finer);
    // The chain from the arc's end back to its start, each boundary taken in the way it is walked.
    let mut left: Vec<_> = chain.iter().filter_map(find).collect();
    let mut at = arc.to.as_str();
    while at != arc.from.as_str() {
        let k = left.iter().position(|b| b.from == at || b.to == at)?;
        let b = left.swap_remove(k);
        let (next, bulge) = if b.from == at { (b.to.as_str(), b.bulge) } else { (b.from.as_str(), -b.bulge) };
        let piece = curve(*places.get(at)?, *places.get(next)?, bulge, finer);
        out.extend(piece.into_iter().skip(1));
        at = next;
    }
    Some(out)
}

/// The problems as the screen shows them (docs/adr/3820): those whose rings would meet (nearer than two radii) share
/// the first one's place, their words in one phrase — each kind once in the order met, with its count when more than
/// one.
pub fn gathered(problems: &[(Point, &str)]) -> Vec<(Point, String)> {
    let mut groups: Vec<(Point, Vec<(&str, usize)>)> = Vec::new();
    for (c, w) in problems {
        let near = groups.iter_mut().find(|(g, _)| (g.x - c.x).hypot(g.y - c.y) < 2.0 * RING);
        match near {
            Some((_, words)) => match words.iter_mut().find(|(k, _)| k == w) {
                Some((_, n)) => *n += 1,
                None => words.push((w, 1)),
            },
            None => groups.push((*c, vec![(w, 1)])),
        }
    }
    groups
        .into_iter()
        .map(|(c, words)| {
            let phrase = words
                .iter()
                .map(|(w, n)| if *n > 1 { format!("{w} ({n})") } else { (*w).to_owned() })
                .collect::<Vec<_>>()
                .join(" · ");
            (c, phrase)
        })
        .collect()
}

impl App {
    /// The graph's marks for the drawing as it is: none when nothing is shown, for another drawing, or when the
    /// parcels asked no longer read (one was deleted). Kept until the drawing changes.
    pub(crate) fn parcel_graph_marks(&self) -> Option<Arc<GraphMarks>> {
        let shown = self.parcel_graph.as_ref()?;
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
        let input = ParcelsBoundaries { uids: shown.uids.clone(), arc_tolerance: shown.tolerance };
        let made = parcels_boundaries::preview(&doc.model, &input).ok().map(|g| {
            let mut marks = GraphMarks::of(&g);
            // The chosen row, found again in the graph as it is now (docs/adr/3020).
            marks.focus = shown.focus.as_ref().map(|f| (f.points(&g), f.name())).filter(|(pts, _)| !pts.is_empty());
            Arc::new(marks)
        });
        *shown.made.borrow_mut() = Some((generation, revision, made.clone()));
        made
    }
}

/// The cells a number's room takes, to keep the next ones off it.
#[derive(Default)]
struct Crowd(HashSet<(i32, i32)>);

impl Crowd {
    fn cell(p: Point) -> (i32, i32) {
        ((p.x / ROOM.0).floor() as i32, (p.y / ROOM.1).floor() as i32)
    }

    /// Takes `p`'s cell when neither it nor a cell next to it is taken.
    fn take(&mut self, p: Point) -> bool {
        let (x, y) = Self::cell(p);
        if (-1..=1).any(|dx| (-1..=1).any(|dy| self.0.contains(&(x + dx, y + dy)))) {
            return false;
        }
        self.0.insert((x, y));
        true
    }

    /// The cells of words `width` wide starting at `p`, as tall as a line of them.
    fn cells(p: Point, width: f32) -> Vec<(i32, i32)> {
        let (x0, y0) = Self::cell(p);
        let (x1, y1) = Self::cell(Point::new(p.x + width, p.y + typography::scaled(11.0) * 1.2));
        (x0..=x1).flat_map(|x| (y0..=y1).map(move |y| (x, y))).collect()
    }

    /// Whether words `width` wide at `p` meet nothing taken.
    fn free(&self, p: Point, width: f32) -> bool {
        self.met(p, width, &[]) == 0
    }

    /// How many taken cells words `width` wide at `p` meet, but those in `own` (their own ring's).
    fn met(&self, p: Point, width: f32, own: &[(i32, i32)]) -> usize {
        Self::cells(p, width).iter().filter(|c| self.0.contains(c) && !own.contains(c)).count()
    }

    /// Takes the cells of words `width` wide at `p` whatever is there (a problem's words).
    fn force(&mut self, p: Point, width: f32) {
        self.0.extend(Self::cells(p, width));
    }
}

/// About how wide words are on the screen, logical pixels.
fn width_of(content: &str) -> f32 {
    content.chars().count() as f32 * typography::scaled(11.0) * 0.6
}

/// Words at `at` in `color` on a halo of the area's colour.
fn words(frame: &mut canvas::Frame, at: Point, content: &str, color: iced::Color, halo: iced::Color, strong: bool) {
    let text = Text {
        content: content.to_owned(),
        position: at,
        color: halo,
        size: Pixels(typography::scaled(11.0)),
        font: if strong { typography::ui_strong() } else { typography::ui() },
        align_y: iced::alignment::Vertical::Top,
        ..Text::default()
    };
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        frame.fill_text(Text { position: text.position + Vector::new(dx, dy), ..text.clone() });
    }
    frame.fill_text(Text { color, ..text });
}

/// The marks on the screen: the problems' rings and words first, then the corners' dots and numbers in the accent,
/// then the boundaries' numbers in the drawing's ink; only what is in the view, numbers thinned.
pub fn draw(
    frame: &mut canvas::Frame,
    m: &GraphMarks,
    camera: &Camera,
    colors: &MarkColors,
    accent: iced::Color,
    danger: iced::Color,
) {
    let size = frame.size();
    let screen = |p: Vec2| {
        let s = camera.world_to_screen(p);
        Point::new(s[0].round() as f32 + 0.5, s[1].round() as f32 + 0.5)
    };
    let inside = |p: Point| p.x >= -24.0 && p.y >= -24.0 && p.x <= size.width + 24.0 && p.y <= size.height + 24.0;
    let stroke = |width: f32, color: iced::Color| Stroke::default().with_color(color).with_width(width);
    let mut crowd = Crowd::default();
    // Problems whose rings would meet share one ring and one phrase (docs/adr/3820).
    let problems: Vec<(Point, &str)> =
        m.problems.iter().map(|(p, w)| (screen(*p), w.as_str())).filter(|(c, _)| inside(*c)).collect();
    let problems = gathered(&problems);
    // An arc drawn with chords' strip under all: filled faintly in the danger colour, outlined (docs/adr/3820).
    for outline in &m.strips {
        let on: Vec<Point> = outline.iter().map(|p| screen(*p)).collect();
        if on.len() < 3 {
            continue;
        }
        let shape = Path::new(|b| {
            b.move_to(on[0]);
            for q in &on[1..] {
                b.line_to(*q);
            }
            b.close();
        });
        frame.fill(&shape, iced::Color { a: 0.22, ..danger });
        frame.stroke(&shape, stroke(3.0, colors.halo));
        frame.stroke(&shape, stroke(1.5, danger));
    }
    let corners: Vec<(Point, &str)> =
        m.corners.iter().map(|(p, n)| (screen(*p), n.as_str())).filter(|(c, _)| inside(*c)).collect();
    // The corners' dots, all of them in the view.
    let dots = Path::new(|b| {
        for (c, _) in &corners {
            b.circle(*c, 2.5);
        }
    });
    frame.stroke(&dots, stroke(2.0, colors.halo));
    frame.fill(&dots, accent);
    // The problems: a ring on a halo, the words above right; never left out.
    let rings = Path::new(|b| {
        for (c, _) in &problems {
            b.circle(*c, RING);
        }
    });
    frame.stroke(&rings, stroke(4.0, colors.halo));
    frame.stroke(&rings, stroke(2.0, danger));
    // The rings first, so no problem's words run over another's ring; then each problem's words where they meet
    // nothing taken: above right, below right, above left, below left.
    let ring = |c: Point| Crowd::cells(c + Vector::new(-10.0, -10.0), 20.0);
    for (c, _) in &problems {
        crowd.0.extend(ring(*c));
    }
    for (c, w) in &problems {
        let width = width_of(w);
        let own = ring(*c);
        // The first spot that meets nothing, else the one that meets least (the first of those).
        let spots = [
            Vector::new(12.0, -20.0),
            Vector::new(12.0, 14.0),
            Vector::new(-12.0 - width, -20.0),
            Vector::new(-12.0 - width, 14.0),
        ]
        .map(|v| *c + v);
        let at = spots
            .iter()
            .copied()
            .min_by_key(|p| crowd.met(*p, width, &own))
            .unwrap_or(spots[0]);
        crowd.force(at, width);
        words(frame, at, w.as_str(), danger, colors.halo, true);
    }
    let mut written = 0;
    for (c, n) in &corners {
        if written >= MOST_NUMBERS {
            break;
        }
        let at = *c + Vector::new(5.0, -16.0);
        if crowd.free(at, width_of(n)) && crowd.take(at) {
            words(frame, at, n, accent, colors.halo, true);
            written += 1;
        }
    }
    for (p, n) in &m.boundaries {
        if written >= MOST_NUMBERS {
            break;
        }
        let c = screen(*p);
        if !inside(c) {
            continue;
        }
        let at = c + Vector::new(-8.0, -7.0);
        if crowd.free(at, width_of(n)) && crowd.take(at) {
            words(frame, at, n, colors.fg, colors.halo, false);
            written += 1;
        }
    }
    // The chosen row over all (docs/adr/3020): a boundary along its curve, a place with a double ring; its name.
    if let Some((pts, name)) = &m.focus {
        let on: Vec<Point> = pts.iter().map(|p| screen(*p)).collect();
        if on.len() >= 2 {
            let line = Path::new(|b| {
                b.move_to(on[0]);
                for q in &on[1..] {
                    b.line_to(*q);
                }
            });
            frame.stroke(&line, stroke(8.0, colors.halo));
            frame.stroke(&line, stroke(4.0, accent));
        } else if let Some(c) = on.first() {
            let rings = Path::new(|b| {
                b.circle(*c, 13.0);
                b.circle(*c, 18.0);
            });
            frame.stroke(&rings, stroke(5.0, colors.halo));
            frame.stroke(&rings, stroke(2.5, accent));
        }
        if let Some(c) = on.get(on.len() / 2) {
            words(frame, *c + Vector::new(20.0, 8.0), name, accent, colors.halo, true);
        }
    }
}
