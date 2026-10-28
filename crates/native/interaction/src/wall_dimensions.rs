//! A wall's live opening and overall dimensions: choose a source, review, place once. On a wall of more than one
//! edge (a closed room, a polyline wall) the straight edge nearest the pointer is measured, or every straight edge
//! at once (docs/adr/0700). A façade of walls or a space's outline along a direction, its openings, walls, grids and
//! overall rows, on one side or all four (docs/adr/0729).
use kentos_commands::{
    CommandResult, DimensionSetFace, DimensionSetOpenings, DimensionSetReference, DimensionSetRow,
    DimensionSetsCreateInput, Entity, STANDARD_STYLE, TextFace,
};
use kentos_domain::dimension_sets::WallEdges;
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{ExecutionContext, dimension_sets_create, geometry::shape};

use crate::edge::Outline;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Cursor, Flow, OptionChoice, Pointer, Preview, Stroke, Tag, TextGhost, Tone, Tool,
};
use crate::tool_settings::{Setting, choices};
use crate::{Format, Level, Vec2, dimension_layout, styles};

pub const ID: &str = "wallDimensions";
pub const LABEL: &str = "Duvar ölçüleri";

/// What the tool measures (docs/adr/0729 §7): one wall's edge, a façade of walls, a space's outline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Wall,
    Facade,
    Space,
}

impl Mode {
    fn of(n: u8) -> Self {
        match n {
            1 => Mode::Facade,
            2 => Mode::Space,
            _ => Mode::Wall,
        }
    }
    fn number(self) -> u8 {
        match self {
            Mode::Wall => 0,
            Mode::Facade => 1,
            Mode::Space => 2,
        }
    }
    fn key(self) -> &'static str {
        match self {
            Mode::Wall => "wall",
            Mode::Facade => "facade",
            Mode::Space => "space",
        }
    }
}

pub struct WallDimensions {
    mode: Mode,
    /// A façade's walls, in the user's order.
    group: Vec<Uuid>,
    space: Option<Uuid>,
    /// A façade's or a space's direction square to its first source's longest straight edge.
    across: bool,
    walls_row: bool,
    grids_row: bool,
    /// A façade's walls measured at their cores.
    core: bool,
    /// The last plan's rows, said, and how many dimensions do not fit or walls were left out.
    notes: Vec<String>,
    tight: usize,
    skipped: usize,
    wall: Option<Uuid>,
    /// The chosen wall's edges; none before a wall is chosen.
    edges: Option<WallEdges>,
    /// The straight edge measured: the one nearest the pointer on a wall of more than one edge; none on a two-point
    /// wall, whose only edge a recipe keeps without a number.
    edge: Option<u32>,
    /// Kenarlar → Bütün kenarlar: a chain on every straight edge, in one step.
    around: bool,
    source: String,
    at: Option<Vec2>,
    /// An entered distance stays authoritative while the pointer visits controls or menus.
    fixed_distance: Option<f64>,
    left: bool,
    face_override: Option<bool>,
    layer: String,
    style: Option<String>,
    openings: bool,
    overall: bool,
    centers: bool,
    gap_mm: f64,
    revision: Option<String>,
    seen: Option<u64>,
    drawn: Preview,
    error: Option<String>,
    count: usize,
    asking: Option<String>,
    layer_choices: Vec<OptionChoice>,
    style_choices: Vec<OptionChoice>,
    distance: String,
    done: bool,
}

impl Default for WallDimensions {
    fn default() -> Self {
        Self {
            mode: Mode::Wall,
            group: Vec::new(),
            space: None,
            across: false,
            walls_row: false,
            grids_row: false,
            core: false,
            notes: Vec::new(),
            tight: 0,
            skipped: 0,
            wall: None,
            edges: None,
            edge: None,
            around: false,
            source: "Duvar seçin".into(),
            at: None,
            fixed_distance: None,
            left: true,
            face_override: None,
            layer: String::new(),
            style: None,
            openings: true,
            overall: true,
            centers: false,
            gap_mm: 7.5,
            revision: None,
            seen: None,
            drawn: Preview::default(),
            error: None,
            count: 0,
            asking: None,
            layer_choices: Vec::new(),
            style_choices: Vec::new(),
            distance: String::new(),
            done: false,
        }
    }
}

/// A wall path's edge `edge`: its start, its left normal and its length; none for a wall without that edge.
fn edge_basis(doc: &Document, uid: Uuid, edge: u32) -> Option<(Vec2, Vec2, f64)> {
    let Entity::Wall(w) = doc.get(doc.slot_of(uid)?)? else {
        return None;
    };
    let (n, k) = (w.wall.pts.len(), edge as usize);
    if k >= n || (!w.wall.closed && k + 1 >= n) {
        return None;
    }
    let a = Vec2::new(w.wall.pts[k].x, w.wall.pts[k].y);
    let b = Vec2::new(w.wall.pts[(k + 1) % n].x, w.wall.pts[(k + 1) % n].y);
    let length = (b.x - a.x).hypot(b.y - a.y);
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some((
        a,
        Vec2::new(-(b.y - a.y) / length, (b.x - a.x) / length),
        length,
    ))
}

/// The wall's straight edge nearest `p` along its path, the lower number on a tie.
fn nearest_edge(doc: &Document, uid: Uuid, edges: &WallEdges, p: Vec2) -> Option<u32> {
    edges
        .straight
        .iter()
        .filter_map(|&k| {
            let (a, n, length) = edge_basis(doc, uid, k)?;
            let b = Vec2::new(a.x + n.y * length, a.y - n.x * length);
            Some((k, kentos_geometry_core::geometry::dist_to_segment(p, a, b)))
        })
        .min_by(|x, y| x.1.total_cmp(&y.1))
        .map(|(k, _)| k)
}

/// A wall with more than one edge needs one named (docs/adr/0700).
fn many(edges: &WallEdges) -> bool {
    edges.closed || edges.count > 1
}

pub(crate) fn wrapped(text: &str) -> Vec<String> {
    let mut rows = vec![String::new()];
    for word in text.split_whitespace() {
        if rows
            .last()
            .is_some_and(|s| s.chars().count() + word.chars().count() > 48)
        {
            rows.push(String::new());
        }
        if let Some(row) = rows.last_mut() {
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(word);
        }
    }
    rows
}

/// A path's longest straight edge's direction; none without one.
fn longest(pts: &[kentos_commands::Vec2], bulges: Option<&[f64]>, closed: bool) -> Option<Vec2> {
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    let mut best: Option<(f64, Vec2)> = None;
    for k in 0..edges {
        if bulges.and_then(|b| b.get(k)).is_some_and(|b| *b != 0.0) {
            continue;
        }
        let (a, b) = (pts[k], pts[(k + 1) % n]);
        let l = (b.x - a.x).hypot(b.y - a.y);
        if l > 0.0 && best.is_none_or(|(m, _)| l > m) {
            best = Some((l, Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)));
        }
    }
    best.map(|(_, d)| d)
}

/// The space whose outline holds `p` on a shown layer, the smallest first; none outside every one.
fn space_at(p: Vec2, cx: &Context<'_>) -> Option<Uuid> {
    let inside = |ring: &[kentos_commands::Vec2]| {
        let pts: Vec<Vec2> = ring.iter().map(|q| Vec2::new(q.x, q.y)).collect();
        kentos_geometry_core::geometry::point_in_polygon(p, &pts)
    };
    cx.doc
        .entities_with_uids()
        .filter_map(|(uid, e)| {
            let Entity::Space(s) = e else { return None };
            if !cx.doc.layers().is_visible(&s.base.layer_id) {
                return None;
            }
            let part = s
                .space
                .parts
                .iter()
                .find(|part| inside(&part.outer.pts) && !part.holes.iter().any(|h| inside(&h.pts)))?;
            let ring = &part.outer.pts;
            let area = (0..ring.len())
                .map(|i| {
                    let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                    a.x * b.y - b.x * a.y
                })
                .sum::<f64>()
                .abs();
            Some((uid, area))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(uid, _)| uid)
}

/// A façade's or a space's frame (docs/adr/0729 §2).
#[derive(Clone, Copy, Debug)]
struct SpanFrame {
    origin: Vec2,
    d: Vec2,
    n: Vec2,
    /// How far its bodies reach across, the least and the most.
    lo: f64,
    hi: f64,
    /// Their middle along.
    middle: f64,
}

impl SpanFrame {
    fn across(&self, p: Vec2) -> f64 {
        (p.x - self.origin.x) * self.n.x + (p.y - self.origin.y) * self.n.y
    }
    fn at(&self, along: f64, across: f64) -> Vec2 {
        Vec2::new(
            self.origin.x + self.d.x * along + self.n.x * across,
            self.origin.y + self.d.y * along + self.n.y * across,
        )
    }
}

impl WallDimensions {
    /// Whether the tool has what it measures.
    fn has_source(&self) -> bool {
        match self.mode {
            Mode::Wall => self.wall.is_some(),
            Mode::Facade => !self.group.is_empty(),
            Mode::Space => self.space.is_some(),
        }
    }

    /// A façade's or a space's direction: its first source's longest straight edge's, or square to it.
    fn direction(&self, doc: &Document) -> Option<Vec2> {
        let first = match self.mode {
            Mode::Facade => self.group.first().copied(),
            Mode::Space => self.space,
            Mode::Wall => None,
        }?;
        let d = match doc.get(doc.slot_of(first)?)? {
            Entity::Wall(w) => longest(&w.wall.pts, w.wall.bulges.as_deref(), w.wall.closed),
            Entity::Space(s) => s
                .space
                .parts
                .first()
                .and_then(|part| longest(&part.outer.pts, part.outer.bulges.as_deref(), true)),
            _ => None,
        }?;
        Some(if self.across { Vec2::new(-d.y, d.x) } else { d })
    }

    /// A façade's or a space's frame: an origin, the direction and its left normal, how far its bodies reach across
    /// (the least and the most) and along (its middle); none without a source.
    fn span_frame(&self, doc: &Document) -> Option<SpanFrame> {
        let d = self.direction(doc)?;
        let n = Vec2::new(-d.y, d.x);
        let ids: Vec<Uuid> = match self.mode {
            Mode::Facade => self.group.clone(),
            Mode::Space => self.space.into_iter().collect(),
            Mode::Wall => Vec::new(),
        };
        let mut pts: Vec<Vec2> = Vec::new();
        for uid in ids {
            match doc.slot_of(uid).and_then(|slot| doc.get(slot)) {
                Some(e @ Entity::Wall(_)) => {
                    if let kentos_geometry_core::entity::Shape::Wall { pts: path, bulges, wall } = shape(e)
                        && let Ok(lines) =
                            kentos_geometry_core::geom::wall::lines(&path, bulges.as_deref(), &wall)
                    {
                        for l in [&lines[0], &lines[lines.len() - 1]] {
                            pts.extend(l.pts.iter().map(|q| Vec2::new(q.x, q.y)));
                        }
                    }
                }
                Some(Entity::Space(s)) => {
                    for part in &s.space.parts {
                        pts.extend(part.outer.pts.iter().map(|q| Vec2::new(q.x, q.y)));
                    }
                }
                _ => {}
            }
        }
        let origin = *pts.first()?;
        let o = |q: &Vec2| (q.x - origin.x) * n.x + (q.y - origin.y) * n.y;
        let t = |q: &Vec2| (q.x - origin.x) * d.x + (q.y - origin.y) * d.y;
        let lo = pts.iter().map(o).fold(f64::INFINITY, f64::min);
        let hi = pts.iter().map(o).fold(f64::NEG_INFINITY, f64::max);
        let t0 = pts.iter().map(t).fold(f64::INFINITY, f64::min);
        let t1 = pts.iter().map(t).fold(f64::NEG_INFINITY, f64::max);
        Some(SpanFrame {
            origin,
            d,
            n,
            lo,
            hi,
            middle: (t0 + t1) * 0.5,
        })
    }

    fn place_distance(&mut self, distance: f64, doc: &Document) -> bool {
        if self.mode != Mode::Wall {
            let (Some(at), Some(f)) = (self.at, self.span_frame(doc)) else {
                return false;
            };
            let o = f.across(at);
            let target = if o >= (f.lo + f.hi) * 0.5 { f.hi + distance } else { f.lo - distance };
            self.at = Some(Vec2::new(at.x + f.n.x * (target - o), at.y + f.n.y * (target - o)));
            return true;
        }
        let (Some(uid), Some(at)) = (self.wall, self.at) else {
            return false;
        };
        let Some((a, n, _)) = self.face_basis(doc, uid) else {
            return false;
        };
        let old = (at.x - a.x) * n.x + (at.y - a.y) * n.y;
        let sign = if self.left { 1. } else { -1. };
        self.at = Some(Vec2::new(
            at.x + n.x * (sign * distance - old),
            at.y + n.y * (sign * distance - old),
        ));
        true
    }

    fn face(&self) -> DimensionSetFace {
        if self.left {
            DimensionSetFace::Left
        } else {
            DimensionSetFace::Right
        }
    }

    /// The measured face edge: its start, its left normal and its length.
    fn face_basis(&self, doc: &Document, uid: Uuid) -> Option<(Vec2, Vec2, f64)> {
        Self::face_edge(doc, uid, self.face(), self.edge)
    }

    fn face_edge(
        doc: &Document,
        uid: Uuid,
        face: DimensionSetFace,
        edge: Option<u32>,
    ) -> Option<(Vec2, Vec2, f64)> {
        let points = doc.dimension_set_wall_face(uid, face, edge).ok()?;
        let a = Vec2::new(points[0].x, points[0].y);
        let b = Vec2::new(points[1].x, points[1].y);
        let length = (b.x - a.x).hypot(b.y - a.y);
        (length.is_finite() && length > 0.0).then(|| {
            (
                a,
                Vec2::new(-(b.y - a.y) / length, (b.x - a.x) / length),
                length,
            )
        })
    }

    /// The closed wall's outer face is the left one: on a clockwise path; none for an open wall.
    fn outer_left(&self) -> Option<bool> {
        self.edges
            .as_ref()
            .filter(|e| e.closed)
            .map(|e| !e.counter_clockwise)
    }
    /// The source and the place let go: Başka duvar, Geri al.
    fn forget(&mut self) {
        self.wall = None;
        self.edges = None;
        self.edge = None;
        self.group.clear();
        self.space = None;
        self.source = match self.mode {
            Mode::Space => "Mahal seçin".into(),
            _ => "Duvar seçin".into(),
        };
        self.at = None;
        self.fixed_distance = None;
        self.distance.clear();
    }

    fn request(&self, expected: bool, doc: &Document) -> Option<DimensionSetsCreateInput> {
        let at = self.at?;
        let mut rows = Vec::new();
        if self.openings {
            rows.push(DimensionSetRow::Openings);
        }
        if self.mode != Mode::Wall && self.walls_row {
            rows.push(DimensionSetRow::Walls);
        }
        if self.mode != Mode::Wall && self.grids_row {
            rows.push(DimensionSetRow::Grids);
        }
        if self.overall {
            rows.push(DimensionSetRow::Overall);
        }
        let (wall, walls, space, direction) = match self.mode {
            Mode::Wall => (Some(self.wall?.to_string()), Vec::new(), None, None),
            Mode::Facade => {
                if self.group.is_empty() {
                    return None;
                }
                let d = self.direction(doc)?;
                (
                    None,
                    self.group.iter().map(|u| u.to_string()).collect(),
                    None,
                    Some(kentos_commands::Vec2 { x: d.x, y: d.y }),
                )
            }
            Mode::Space => {
                let d = self.direction(doc)?;
                (
                    None,
                    Vec::new(),
                    Some(self.space?.to_string()),
                    Some(kentos_commands::Vec2 { x: d.x, y: d.y }),
                )
            }
        };
        Some(DimensionSetsCreateInput {
            wall,
            walls,
            space,
            direction,
            reference: if self.mode == Mode::Facade && self.core {
                DimensionSetReference::Core
            } else {
                DimensionSetReference::Faces
            },
            layer_id: self.layer.clone(),
            at: kentos_commands::Vec2 { x: at.x, y: at.y },
            edge: if self.mode == Mode::Wall { self.edge } else { None },
            around: self.around,
            face: if self.mode == Mode::Wall { self.face() } else { DimensionSetFace::Left },
            rows,
            openings: if self.centers {
                DimensionSetOpenings::Centers
            } else {
                DimensionSetOpenings::Ends
            },
            gap_mm: self.gap_mm,
            dimension_style: self.style.clone(),
            expected_revision: expected.then(|| self.revision.clone()).flatten(),
        })
    }

    fn menus(&mut self, cx: &Context<'_>) {
        self.layer_choices = cx
            .doc
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| {
                !cx.doc.layers().is_locked(&l.id)
                    && l.service.is_none()
                    && l.geometry
                        .is_none_or(|geometry| geometry.admits("dimension"))
            })
            .map(|l| OptionChoice {
                typed: l.id.clone(),
                label: cx.doc.layers().path(&l.id),
                checked: l.id == self.layer,
                icon: None,
                preview: None,
                command: None,
            })
            .collect();
        self.style_choices = vec![OptionChoice {
            typed: STANDARD_STYLE.into(),
            label: STANDARD_STYLE.into(),
            checked: self.style.is_none(),
            icon: None,
            preview: None,
            command: None,
        }];
        self.style_choices
            .extend(
                cx.doc
                    .settings()
                    .dimension_styles
                    .iter()
                    .map(|s| OptionChoice {
                        typed: s.name.clone(),
                        label: s.name.clone(),
                        checked: self.style.as_deref() == Some(s.name.as_str()),
                        icon: None,
                        preview: None,
                        command: None,
                    }),
            );
    }

    fn see(&mut self, cx: &Context<'_>) {
        if let Some(distance) = self.fixed_distance {
            self.place_distance(distance, cx.doc);
        }
        self.seen = Some(cx.doc.revision());
        self.menus(cx);
        self.drawn = Preview::default();
        self.error = None;
        self.revision = None;
        self.count = 0;
        self.notes.clear();
        self.tight = 0;
        self.skipped = 0;
        // A façade's walls or the space, dashed.
        let outlined: Vec<Uuid> = match self.mode {
            Mode::Wall => Vec::new(),
            Mode::Facade => self.group.clone(),
            Mode::Space => self.space.into_iter().collect(),
        };
        for uid in outlined {
            if let Some(entity) = cx.doc.slot_of(uid).and_then(|s| cx.doc.get(s)) {
                self.drawn
                    .strokes
                    .extend(Outline::of(&shape(entity), Some([6., 4.]), 2.0, Tone::Snap).strokes);
            }
        }
        if self.mode != Mode::Wall
            && let (Some(at), Some(f)) = (self.at, self.span_frame(cx.doc))
        {
            let o = f.across(at);
            self.distance = cx
                .format()
                .length(if o >= (f.lo + f.hi) * 0.5 { o - f.hi } else { f.lo - o }.abs());
        }
        if let Some(uid) = self.wall.filter(|_| self.mode == Mode::Wall) {
            if let Some(entity) = cx.doc.slot_of(uid).and_then(|s| cx.doc.get(s)) {
                self.drawn
                    .strokes
                    .extend(Outline::of(&shape(entity), Some([6., 4.]), 2.0, Tone::Snap).strokes);
            }
            // The measured face edges, solid over the dashed wall: the one edge, or every straight edge.
            let measured: Vec<Option<u32>> = match &self.edges {
                Some(edges) if self.around && many(edges) => {
                    edges.straight.iter().map(|&k| Some(k)).collect()
                }
                _ => vec![self.edge],
            };
            for edge in measured {
                if let Some((a, n, length)) = Self::face_edge(cx.doc, uid, self.face(), edge) {
                    let mut stroke = Stroke::solid(
                        vec![a, Vec2::new(a.x + n.y * length, a.y - n.x * length)],
                        false,
                    );
                    stroke.width = 3.0;
                    stroke.tone = Tone::Snap;
                    self.drawn.strokes.push(stroke);
                }
            }
            if let (Some((a, n, _)), Some(at)) = (self.face_basis(cx.doc, uid), self.at) {
                self.distance = cx
                    .format()
                    .length(((at.x - a.x) * n.x + (at.y - a.y) * n.y).abs());
            }
        }
        let Some(input) = self.request(false, cx.doc) else {
            return;
        };
        // The command's read-only plan is the only source of geometry: no project clone per pointer event.
        match dimension_sets_create::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                self.revision = Some(output.revision);
                self.count = output.entities.len();
                self.tight = output.tight.len();
                self.skipped = output.skipped.len();
                for (i, entity) in output.entities.iter().enumerate() {
                    // A value that does not fit is drawn in the danger colour, never hidden (docs/adr/0729 §6).
                    let tight = output.tight.contains(&(i as u32));
                    self.dimension_ghost(entity, tight, cx);
                }
                let name = |r: DimensionSetRow| match r {
                    DimensionSetRow::Openings => "Açıklıklar",
                    DimensionSetRow::Walls => "Duvarlar",
                    DimensionSetRow::Grids => "Akslar",
                    DimensionSetRow::Overall => "Toplam",
                };
                for note in output.rows.iter().take(4) {
                    self.notes.push(format!(
                        "{}: {} ölçü · {} kaynak",
                        name(note.row),
                        note.dimensions,
                        note.sources
                    ));
                }
                let edges = self
                    .edges
                    .as_ref()
                    .filter(|e| self.around && many(e))
                    .map_or(1, |e| e.straight.len());
                let sides = if self.mode != Mode::Wall && self.around { 4 } else { 1 };
                let mut lines = vec![if edges > 1 {
                    format!("{} ölçü · {edges} kenar · tek adım", self.count)
                } else if sides > 1 {
                    format!("{} ölçü · dört yan · tek adım", self.count)
                } else {
                    format!("{} ölçü · tek adım", self.count)
                }];
                if sides == 1 {
                    lines.extend(self.notes.iter().cloned());
                }
                if self.tight > 0 {
                    lines.push(format!("{} ölçünün yazısı sığmıyor", self.tight));
                }
                if self.skipped > 0 {
                    lines.push(format!("{} duvar eğik ya da yaylı: ölçülmedi", self.skipped));
                }
                lines.push(
                    if self.fixed_distance.is_some() {
                        "Uzaklık sabit · tıklayın veya Enter ile yerleştirin"
                    } else {
                        "Yerini seçin veya Enter ile yerleştirin"
                    }
                    .into(),
                );
                self.drawn.tag = self.at.map(|at| Tag { at, lines });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.error = Some(error.message),
            _ => self.error = Some("Ölçü önizlemesi hazır değil; işlemi yeniden başlatın.".into()),
        }
        if let Some(why) = &self.error {
            self.drawn.tag = self.at.map(|at| Tag {
                at,
                lines: wrapped(why),
            });
            self.drawn.tag_tone = Tone::Danger;
        }
    }

    fn dimension_ghost(&mut self, entity: &Entity, tight: bool, cx: &Context<'_>) {
        let mut preview = dimension_preview(entity, &cx.format(), cx.doc.settings().drawing_font);
        if tight {
            for stroke in &mut preview.strokes {
                stroke.tone = Tone::Danger;
            }
        }
        self.drawn.strokes.extend(preview.strokes);
        self.drawn.texts.extend(preview.texts);
    }

    /// A façade of these walls (docs/adr/0729 §7): the rows start south of them, a few heights off.
    fn choose_group(&mut self, walls: Vec<Uuid>, cx: &mut Context<'_>) {
        if walls.is_empty() {
            return;
        }
        self.group = walls;
        let layers: Vec<String> = self
            .group
            .iter()
            .filter_map(|u| cx.doc.slot_of(*u).and_then(|s| cx.doc.get(s)))
            .map(|e| e.base().layer_id.clone())
            .collect();
        self.source = match self.group.len() {
            1 => format!(
                "Cephe · {} · köşe duvarlarıyla",
                cx.doc
                    .layers()
                    .get(&layers.first().cloned().unwrap_or_default())
                    .map_or(String::new(), |l| l.name.clone())
            ),
            n => format!("Cephe · {n} duvar"),
        };
        self.place_first(cx);
        self.see(cx);
    }

    fn choose_space(&mut self, uid: Uuid, cx: &mut Context<'_>) {
        let Some(Entity::Space(s)) = cx.doc.slot_of(uid).and_then(|slot| cx.doc.get(slot)) else {
            return;
        };
        self.source = match (&s.space.number, &s.space.name) {
            (Some(n), Some(name)) => format!("Mahal · {n} {name}"),
            (Some(n), None) => format!("Mahal · {n}"),
            (None, Some(name)) => format!("Mahal · {name}"),
            (None, None) => "Mahal".into(),
        };
        self.space = Some(uid);
        self.place_first(cx);
        self.see(cx);
    }

    /// A façade's or a space's first row's place: on the right of its direction (south of a façade drawn west to
    /// east), three dimension heights off its bodies, in their middle along.
    fn place_first(&mut self, cx: &Context<'_>) {
        if let Some(f) = self.span_frame(cx.doc) {
            let distance = 3.0 * styles::standard_dimension_height(cx.doc.settings());
            self.at = Some(f.at(f.middle, f.lo - distance));
        }
    }

    fn choose(&mut self, slot: Slot, near: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(Entity::Wall(w)) = cx.doc.get(slot) else {
            return;
        };
        let Some(uid) = cx.doc.uid(slot) else { return };
        if self.mode == Mode::Facade {
            self.choose_group(vec![uid], cx);
            return;
        }
        let Some(edges) = cx
            .doc
            .dimension_set_wall_edges(uid)
            .filter(|e| !e.straight.is_empty())
        else {
            cx.say(
                Level::Info,
                "Bu duvarın düz kenarı yok; ölçü dizisi yalnız düz kenarı ölçer. Düz kenarlı bir duvar seçin.",
            );
            return;
        };
        let layer = cx
            .doc
            .layers()
            .get(&w.base.layer_id)
            .map_or(w.base.layer_id.as_str(), |l| l.name.as_str());
        self.source = if many(&edges) {
            format!("Duvar · {layer} · {} kenar", edges.count)
        } else {
            format!("Duvar · {layer}")
        };
        self.edge = many(&edges).then(|| {
            near.and_then(|p| nearest_edge(cx.doc, uid, &edges, p))
                .unwrap_or(edges.straight[0])
        });
        // A closed wall starts on its outer face.
        if edges.closed && self.face_override.is_none() {
            self.left = !edges.counter_clockwise;
        }
        self.wall = Some(uid);
        self.edges = Some(edges);
        if let Some((a, n, length)) = self.face_basis(cx.doc, uid) {
            let distance = 3.0 * styles::standard_dimension_height(cx.doc.settings());
            let side = if self.left { 1.0 } else { -1.0 };
            self.at = Some(Vec2::new(
                a.x + n.y * length * 0.5 + n.x * distance * side,
                a.y - n.x * length * 0.5 + n.y * distance * side,
            ));
        }
        self.see(cx);
    }

    fn source_at(p: &Pointer, cx: &Context<'_>) -> Option<Slot> {
        let allowed = |slot: Slot| {
            cx.doc.get(slot).is_some_and(|entity| {
                matches!(entity, Entity::Wall(_))
                    && crate::selectable::entity_allowed(cx.draft.select_kinds, entity)
                    && crate::selectable::on_selected_layer(cx.doc, entity)
            })
        };
        let edge = cx
            .spatial
            .hits(p.raw, cx.pick_tolerance())
            .into_iter()
            .find(|slot| allowed(*slot));
        if edge.is_some() {
            return edge;
        }
        // Walls without a hatch have no generic pickable interior. This tool
        // still accepts their real body, including unhatched finish bands.
        // The spatial bounds query preserves visibility and avoids a broad pick radius.
        cx.spatial
            .store()
            .near(p.raw, 0.)
            .into_iter()
            .filter_map(|item| {
                let slot = Slot(item.id as u32);
                if !allowed(slot) {
                    return None;
                }
                let entity = cx.doc.get(slot)?;
                let kentos_geometry_core::entity::Shape::Wall { pts, bulges, wall } = shape(entity)
                else {
                    return None;
                };
                let (ring, holes) =
                    kentos_geometry_core::geom::wall::body(&pts, bulges.as_deref(), &wall).ok()?;
                if !kentos_geometry_core::geometry::point_in_polygon(p.raw, &ring)
                    || holes
                        .iter()
                        .flatten()
                        .any(|hole| kentos_geometry_core::geometry::point_in_polygon(p.raw, hole))
                {
                    return None;
                }
                // The nearest physical centre line: the faces' middle along each edge of the path.
                let faces =
                    kentos_geometry_core::geom::wall::lines(&pts, bulges.as_deref(), &wall).ok()?;
                let (left, right) = (faces.first()?, faces.last()?);
                let n = left.pts.len().min(right.pts.len());
                let middle = |i: usize| {
                    Vec2::new(
                        (left.pts[i].x + right.pts[i].x) * 0.5,
                        (left.pts[i].y + right.pts[i].y) * 0.5,
                    )
                };
                let edges = if wall.closed { n } else { n.saturating_sub(1) };
                (0..edges)
                    .map(|i| {
                        kentos_geometry_core::geometry::dist_to_segment(
                            p.raw,
                            middle(i),
                            middle((i + 1) % n),
                        )
                    })
                    .min_by(f64::total_cmp)
                    .map(|d| (slot, d))
            })
            .min_by(|a, b| {
                a.1.total_cmp(&b.1)
                    .then_with(|| cx.doc.place(a.0).cmp(&cx.doc.place(b.0)))
            })
            .map(|(slot, _)| slot)
    }

    fn place_at(&mut self, at: Vec2, cx: &Context<'_>) {
        self.at = Some(at);
        if let Some(uid) = self.wall.filter(|_| self.mode == Mode::Wall) {
            // The edge under the pointer, on a wall of more than one.
            if let Some(edges) = self.edges.as_ref().filter(|e| many(e))
                && let Some(k) = nearest_edge(cx.doc, uid, edges, at)
            {
                self.edge = Some(k);
            }
            if let Some(left) = self.outer_left().filter(|_| self.around) {
                // Every edge of a closed wall is measured on its outer face.
                self.left = left;
            } else if let Some((mut a, n, _)) = edge_basis(cx.doc, uid, self.edge.unwrap_or(0)) {
                if let (Ok(left), Ok(right)) = (
                    cx.doc
                        .dimension_set_wall_face(uid, DimensionSetFace::Left, self.edge),
                    cx.doc
                        .dimension_set_wall_face(uid, DimensionSetFace::Right, self.edge),
                ) {
                    a = Vec2::new(
                        (left[0].x + right[0].x) * 0.5,
                        (left[0].y + right[0].y) * 0.5,
                    );
                }
                self.left = self
                    .face_override
                    .unwrap_or((at.x - a.x) * n.x + (at.y - a.y) * n.y >= 0.0);
            }
        }
        self.see(cx);
    }

    fn write(&mut self, cx: &mut Context<'_>) -> bool {
        if self.count == 0 || self.error.is_some() {
            cx.say(
                Level::Warn,
                self.error.clone().unwrap_or_else(|| {
                    "Önce bir duvar seçin ve ölçü sıralarını kontrol edin.".into()
                }),
            );
            return false;
        }
        let Some(input) = self.request(true, cx.doc) else {
            return false;
        };
        let result = dimension_sets_create::execute(&mut ExecutionContext::new(cx.doc), input);
        match crate::points::written(result, cx) {
            Some(output) => {
                let ids: Vec<Slot> = output
                    .uids
                    .iter()
                    .filter_map(|u| u.parse().ok())
                    .filter_map(|u| cx.doc.slot_of(u))
                    .collect();
                if !ids.is_empty() {
                    cx.selection.set(ids);
                }
                cx.say(
                    Level::Success,
                    match (self.mode, output.sets.len()) {
                        (Mode::Wall, n) if n > 1 => format!(
                            "{} ölçü {n} kenara yerleştirildi; duvar ve açıklıkları değişince güncellenir.",
                            self.count
                        ),
                        (Mode::Wall, _) => format!(
                            "{} ölçü yerleştirildi; duvarın açıklıkları değişince güncellenir.",
                            self.count
                        ),
                        (Mode::Facade, n) => format!(
                            "{} ölçü {} yerleştirildi; duvarlar, açıklıklar, bölmeler ve akslar değişince güncellenir.",
                            self.count,
                            if n > 1 { "cephenin dört yanına" } else { "cepheye" }
                        ),
                        (Mode::Space, n) => format!(
                            "{} ölçü {} yerleştirildi; mahal ve açıklıkları değişince güncellenir.",
                            self.count,
                            if n > 1 { "mahalin dört yanına" } else { "mahale" }
                        ),
                    },
                );
                self.done = true;
                true
            }
            None => {
                self.see(cx);
                false
            }
        }
    }
}

/// The actual dimension layout and value, shared by source-edit previews.
pub(crate) fn dimension_preview(
    entity: &Entity,
    format: &Format,
    font: Option<kentos_commands::DrawingFont>,
) -> Preview {
    let mut preview = Preview::default();
    let Entity::Dimension(d) = entity else {
        return preview;
    };
    let Some(layout) = dimension_layout(entity) else {
        return preview;
    };
    preview.strokes.extend(
        layout
            .lines
            .iter()
            .map(|&[a, b]| Stroke::solid(vec![a, b], false)),
    );
    preview.strokes.extend(
        layout
            .fills
            .into_iter()
            .flatten()
            .map(|p| Stroke::solid(p, true)),
    );
    preview.texts.push(TextGhost {
        p: layout.text_at,
        text: format.dimension_value(
            layout.prefix,
            layout.unit,
            layout.value,
            &d.look,
            d.text.as_deref(),
        ),
        height: d.height,
        rotation: layout.rotation,
        align: Some(kentos_geometry_core::text::TextAlign::MiddleCenter),
        mask: d.mask,
        face: TextFace {
            font: d.look.font.or(font),
            ..Default::default()
        },
        width_factor: 1.0,
    });
    preview
}

/// A source command's complete preview, including added and disappearing chain members.
pub(crate) fn append_followers(
    preview: &mut Preview,
    followers: &kentos_commands::DerivedEntitiesPlan,
    cx: &Context<'_>,
) {
    for entity in followers
        .updated
        .iter()
        .chain(&followers.added)
        .take(crate::modify::MAX_GHOSTS)
    {
        if matches!(entity, Entity::Dimension(_)) {
            let mut own = dimension_preview(entity, &cx.format(), cx.doc.settings().drawing_font);
            for stroke in &mut own.strokes {
                stroke.tone = Tone::Snap;
            }
            preview.strokes.extend(own.strokes);
            preview.texts.extend(own.texts);
        } else {
            preview
                .strokes
                .extend(Outline::of(&shape(entity), Some([2., 3.]), 1.0, Tone::Snap).strokes);
        }
    }
    for entity in followers
        .removed
        .iter()
        .filter_map(|uid| {
            uid.parse()
                .ok()
                .and_then(|uid| cx.doc.slot_of(uid))
                .and_then(|slot| cx.doc.get(slot))
        })
        .take(crate::modify::MAX_GHOSTS)
    {
        preview
            .strokes
            .extend(Outline::of(&shape(entity), Some([2., 3.]), 1.0, Tone::Danger).strokes);
    }
}

impl Tool for WallDimensions {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn point_count(&self) -> usize {
        usize::from(self.has_source())
    }
    fn cursor(&self) -> Cursor {
        if self.has_source() {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }
    fn snaps(&self) -> bool {
        false
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.layer = cx.doc.layers().active().into();
        self.style = styles::dimension_style(cx.memory, cx.doc.settings()).map(|s| s.name.clone());
        self.openings = cx.memory.wall_dimensions_rows & 1 != 0;
        self.overall = cx.memory.wall_dimensions_rows & 2 != 0;
        self.centers = cx.memory.wall_dimensions_centers;
        self.gap_mm = cx.memory.wall_dimensions_gap_mm;
        self.around = cx.memory.wall_dimensions_around;
        self.mode = Mode::of(cx.memory.wall_dimensions_mode);
        self.walls_row = cx.memory.wall_dimensions_rows & 4 != 0;
        self.grids_row = cx.memory.wall_dimensions_rows & 8 != 0;
        self.across = cx.memory.wall_dimensions_across;
        self.core = cx.memory.wall_dimensions_core;
        self.menus(cx);
        if !self
            .layer_choices
            .iter()
            .any(|choice| choice.typed == self.layer)
            && let Some(choice) = self.layer_choices.first()
        {
            self.layer = choice.typed.clone();
        }
        // The selection says the source: walls a façade (several always), a space its outline (docs/adr/0729 §7).
        let selected: Vec<(Slot, &Entity)> = cx
            .selection
            .ids()
            .iter()
            .filter_map(|s| cx.doc.get(*s).map(|e| (*s, e)))
            .collect();
        let walls: Vec<Uuid> = selected
            .iter()
            .filter(|(_, e)| matches!(e, Entity::Wall(_)))
            .filter_map(|(s, _)| cx.doc.uid(*s))
            .collect();
        let spaces: Vec<Uuid> = selected
            .iter()
            .filter(|(_, e)| matches!(e, Entity::Space(_)))
            .filter_map(|(s, _)| cx.doc.uid(*s))
            .collect();
        if walls.len() > 1 {
            self.mode = Mode::Facade;
            self.choose_group(walls, cx);
        } else if let ([uid], []) = (walls.as_slice(), spaces.as_slice()) {
            if self.mode == Mode::Space {
                self.mode = Mode::Wall;
            }
            if let Some(slot) = cx.doc.slot_of(*uid) {
                self.choose(slot, None, cx);
            }
        } else if let ([], [uid]) = (walls.as_slice(), spaces.as_slice()) {
            self.mode = Mode::Space;
            self.choose_space(*uid, cx);
        } else {
            self.menus(cx);
        }
        cx.memory.wall_dimensions_mode = self.mode.number();
        Flow::Stay
    }
    fn prompt(&self) -> Prompt {
        if self.asking.is_some() {
            return Prompt::new(LABEL, "değeri yazın").option("Geri", "G");
        }
        Prompt::new(
            LABEL,
            if self.has_source() && self.fixed_distance.is_some() {
                "girilen uzaklık sabit; tıklayın veya Enter ile yerleştirin"
            } else if self.mode == Mode::Wall && self.edges.as_ref().is_some_and(|e| many(e) && !self.around) {
                "ölçülecek kenarı ve sıraların yerini imleçle seçin veya uzaklık yazın"
            } else if self.has_source() {
                "ölçü sıralarının yerini seçin veya uzaklık yazın"
            } else if self.mode == Mode::Space {
                "ölçülendirilecek mahalin içine tıklayın"
            } else if self.mode == Mode::Facade {
                "cephenin duvarına tıklayın (birden çok duvar için önce duvarları seçin)"
            } else {
                "ölçülendirilecek duvara tıklayın"
            },
        )
        .option(if self.mode == Mode::Space { "Başka mahal" } else { "Başka duvar" }, "K")
        .option("Yerleştir", "Enter")
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if !self.has_source() {
            let hover = match self.mode {
                Mode::Space => space_at(p.raw, cx).and_then(|u| cx.doc.slot_of(u)),
                _ => Self::source_at(p, cx),
            };
            cx.selection.set_hover(hover);
            return;
        }
        if self.fixed_distance.is_some() {
            if self.seen != Some(cx.doc.revision()) {
                self.see(cx);
            }
            return;
        }
        if self.at != Some(p.raw) || self.seen != Some(cx.doc.revision()) {
            self.place_at(p.raw, cx);
        }
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if !self.has_source() {
            if self.mode == Mode::Space {
                match space_at(p.raw, cx) {
                    Some(uid) => self.choose_space(uid, cx),
                    None => cx.say(Level::Info, "Ölçülendirilecek mahalin içine tıklayın."),
                }
            } else if let Some(slot) = Self::source_at(p, cx) {
                self.choose(slot, Some(p.raw), cx);
            } else {
                cx.say(Level::Info, "Ölçülendirilecek duvara tıklayın.");
            }
        } else {
            if self.fixed_distance.is_none() {
                self.place_at(p.raw, cx);
            }
            self.write(cx);
        }
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(key) = self.asking.take() {
            return self.configure(&key, text, cx);
        }
        let key = upper_tr(text.trim());
        if key == "K" {
            self.forget();
            self.see(cx);
            return true;
        }
        if self
            .settings(&cx.format())
            .iter()
            .any(|s| s.key == key && s.editable)
        {
            self.asking = Some(key);
            return true;
        }
        if self.has_source() && cx.typed_length(text).is_some() {
            if self.configure("D", text, cx) {
                self.write(cx);
            }
            return true;
        }
        false
    }
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.take().is_some() {
            return Flow::Stay;
        }
        if self.write(cx) {
            Flow::Exit
        } else {
            Flow::Stay
        }
    }
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.asking.take().is_some()
    }
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            return true;
        }
        if !self.has_source() {
            return false;
        }
        self.forget();
        self.see(cx);
        true
    }
    fn finished(&self) -> bool {
        self.done
    }
    fn preview(&self, _: &Format) -> Preview {
        self.drawn.clone()
    }
    fn settings(&self, _: &Format) -> Vec<Setting> {
        let face = match self.face_override {
            None => "auto",
            Some(true) => "left",
            Some(false) => "right",
        };
        // A closed wall's faces by where they are: the outer one on the right of a counter-clockwise path.
        let faces = match &self.edges {
            Some(e) if e.closed && e.counter_clockwise => {
                [("auto", "İmleç tarafı"), ("right", "Dış yüz"), ("left", "İç yüz")]
            }
            Some(e) if e.closed => [("auto", "İmleç tarafı"), ("left", "Dış yüz"), ("right", "İç yüz")],
            _ => [
                ("auto", "İmleç tarafı"),
                ("left", "Duvarın sol yüzü"),
                ("right", "Duvarın sağ yüzü"),
            ],
        };
        let all_outer = self.around && self.outer_left().is_some();
        let spanned = self.mode != Mode::Wall;
        let mut rows = vec![
            Setting::choice(
                "M",
                "Kaynak türü",
                choices(
                    &[("wall", "Duvar"), ("facade", "Cephe"), ("space", "Mahal")],
                    self.mode.key(),
                ),
            ),
            Setting::text("C", "Kaynak", self.source.clone()).read_only(),
            Setting::choice("L", "Ölçü katmanı", self.layer_choices.clone()),
            Setting::choice("S", "Ölçü stili", self.style_choices.clone()),
        ];
        if spanned {
            rows.push(Setting::choice(
                "O",
                "Doğrultu",
                choices(
                    &[("along", "Kaynak boyunca"), ("across", "Kaynağa dik")],
                    if self.across { "across" } else { "along" },
                ),
            ));
            if self.mode == Mode::Facade {
                rows.push(Setting::choice(
                    "Y",
                    "Ölçülen çizgi",
                    choices(
                        &[("faces", "Dış yüzler"), ("core", "Çekirdek")],
                        if self.core { "core" } else { "faces" },
                    ),
                ));
            }
            rows.push(Setting::choice(
                "N",
                "Yanlar",
                choices(
                    &[("one", "İmleç yanı"), ("all", "Dört yan")],
                    if self.around { "all" } else { "one" },
                ),
            ));
        } else {
            rows.push(if all_outer {
                Setting::text("F", "Ölçülen yüz", "Dış yüz (bütün kenarlar)").read_only()
            } else {
                Setting::choice("F", "Ölçülen yüz", choices(&faces, face))
            });
        }
        rows.push(Setting::toggle("A", "Açıklık sırası", self.openings));
        rows.push(Setting::choice(
            "B",
            "Açıklıkları ölç",
            choices(
                &[
                    ("ends", "Açıklık uçları"),
                    ("centers", "Açıklık merkezleri"),
                ],
                if self.centers { "centers" } else { "ends" },
            ),
        ));
        if spanned {
            rows.push(Setting::toggle("W", "Duvarlar sırası", self.walls_row));
            rows.push(Setting::toggle("G", "Akslar sırası", self.grids_row));
        }
        rows.push(Setting::toggle("T", "Toplam boy", self.overall));
        rows.push(Setting::choice(
            "P",
            "Yerleşim",
            choices(
                &[("cursor", "İmleçle"), ("fixed", "Sabit uzaklık")],
                if self.fixed_distance.is_some() {
                    "fixed"
                } else {
                    "cursor"
                },
            ),
        ));
        rows.push(Setting::text(
            "D",
            if spanned { "Gövdeye uzaklık" } else { "Duvara uzaklık" },
            self.distance.clone(),
        ));
        rows.push(Setting::text("R", "Sıra aralığı (kağıtta)", format!("{} mm", self.gap_mm)));
        if !self.notes.is_empty() && !self.around {
            rows.push(Setting::text("Q", "Sıralar", self.notes.join(" · ")).read_only());
        }
        rows.push(Setting::hint(
            "H",
            "Duvar ölçüleri",
            match self.mode {
                _ if self.fixed_distance.is_some() => {
                    "Girilen uzaklık korunur. Tıklama veya Enter ile yerleştirin. Yeniden fareyle yer seçmek için Yerleşim → İmleçle seçin."
                }
                Mode::Wall => {
                    "Duvarın imlece en yakın düz kenarındaki açıklıkları ve toplam boyu birlikte ölçülendirir; Kenarlar → Bütün kenarlar kapalı duvarın her kenarına dış yüzden zincir koyar. İmleçle yer seçin veya Duvara uzaklık girin; girilen uzaklık sabitlenir. Ölçü stili ve mevcut çizim ölçeği kullanılır."
                }
                Mode::Facade => {
                    "Seçili duvarları (ya da tıklanan duvarı köşe duvarlarıyla) bir cephe olarak ölçer: Açıklıklar sırası cepheden görünen köşeleri ve açıklıkları, Duvarlar sırası kalınlıkları ve saplanan bölmeleri, Akslar sırası dik aksları, Toplam en dıştaki iki noktayı. Sıralar imlecin yanında; Dört yan dört yanına birden koyar. Sonradan kapı, bölme ya da aks eklenince ölçüler güncellenir."
                }
                Mode::Space => {
                    "Mahalin sınırını doğrultu boyunca ölçer: Açıklıklar sırası sınırın köşelerini ve imleç yanındaki kenarın kapı ve pencerelerini, Duvarlar sırası içindeki kolon ve boşlukları da, Toplam mahalin net boyunu. Mahal değişince ölçüler güncellenir."
                }
            },
        ));
        // Kenarlar: shown while it can matter, before a wall or on one with more than one straight edge.
        if self.mode == Mode::Wall
            && self
                .edges
                .as_ref()
                .is_none_or(|e| e.straight.len() > 1)
        {
            let at = rows.iter().position(|r| r.key == "A").unwrap_or(rows.len());
            rows.insert(
                at,
                Setting::choice(
                    "N",
                    "Kenarlar",
                    choices(
                        &[("one", "Bu kenar"), ("all", "Bütün kenarlar")],
                        if self.around { "all" } else { "one" },
                    ),
                ),
            );
        }
        if self.tight > 0 {
            rows.push(Setting::notice(
                "U",
                "Sığmayan yazı",
                format!(
                    "{} ölçünün değeri uzatma çizgilerinin arasına sığmıyor; gizlenmedi, kırmızıyla gösterildi. Sıra aralığını büyütün ya da ölçü stilinde yazıyı dışarı alın.",
                    self.tight
                ),
            ));
        }
        if let Some(why) = &self.error {
            rows.push(Setting::notice("E", "Yerleştirilemiyor", why));
        }
        rows
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let value = value.trim();
        let valid = match key {
            "L" if self.layer_choices.iter().any(|c| c.typed == value) => {
                self.layer = value.into();
                true
            }
            "S" if self.style_choices.iter().any(|c| c.typed == value) => {
                self.style = (value != STANDARD_STYLE).then(|| value.into());
                styles::take_dimension(value, cx)
            }
            "F" => match value {
                "auto" => {
                    self.face_override = None;
                    true
                }
                "left" => {
                    self.face_override = Some(true);
                    self.left = true;
                    true
                }
                "right" => {
                    self.face_override = Some(false);
                    self.left = false;
                    true
                }
                _ => false,
            },
            "A" | "T" | "W" | "G" if matches!(value, "yes" | "no") => {
                let on = value == "yes";
                match key {
                    "A" => self.openings = on,
                    "T" => self.overall = on,
                    "W" => self.walls_row = on,
                    _ => self.grids_row = on,
                }
                true
            }
            "M" if matches!(value, "wall" | "facade" | "space") => {
                let mode = match value {
                    "facade" => Mode::Facade,
                    "space" => Mode::Space,
                    _ => Mode::Wall,
                };
                if mode != self.mode {
                    // The wall chosen goes on as the façade's first; else the source is chosen again.
                    let wall = self.wall.filter(|_| mode == Mode::Facade);
                    self.mode = mode;
                    self.forget();
                    if let Some(uid) = wall {
                        self.choose_group(vec![uid], cx);
                    }
                }
                true
            }
            "O" if matches!(value, "along" | "across") => {
                self.across = value == "across";
                if self.fixed_distance.is_none() {
                    self.place_first(cx);
                }
                true
            }
            "Y" if matches!(value, "faces" | "core") => {
                self.core = value == "core";
                true
            }
            "B" if matches!(value, "ends" | "centers") => {
                self.centers = value == "centers";
                true
            }
            "N" if matches!(value, "one" | "all") => {
                self.around = value == "all";
                true
            }
            "R" => match paper_mm(value) {
                Some(mm) if mm > 0.0 && mm <= 1000.0 => {
                    self.gap_mm = mm;
                    true
                }
                _ => false,
            },
            "P" => match value {
                "cursor" => {
                    self.fixed_distance = None;
                    true
                }
                "fixed" => {
                    if self.mode != Mode::Wall
                        && let (Some(at), Some(f)) = (self.at, self.span_frame(cx.doc))
                    {
                        let o = f.across(at);
                        self.fixed_distance =
                            Some(if o >= (f.lo + f.hi) * 0.5 { o - f.hi } else { f.lo - o }.abs());
                        true
                    } else if let (Some(uid), Some(at)) = (self.wall, self.at)
                        && let Some((a, n, _)) = self.face_basis(cx.doc, uid)
                    {
                        self.fixed_distance = Some(((at.x - a.x) * n.x + (at.y - a.y) * n.y).abs());
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            "D" => match (self.has_source(), self.at, cx.typed_length(value)) {
                (true, Some(_), Some(d))
                    if d.is_finite() && d > 0.0 && self.place_distance(d, cx.doc) =>
                {
                    self.fixed_distance = Some(d);
                    true
                }
                _ => false,
            },
            _ => false,
        };
        if valid {
            cx.memory.wall_dimensions_rows = u8::from(self.openings)
                | (u8::from(self.overall) * 2)
                | (u8::from(self.walls_row) * 4)
                | (u8::from(self.grids_row) * 8);
            cx.memory.wall_dimensions_centers = self.centers;
            cx.memory.wall_dimensions_gap_mm = self.gap_mm;
            cx.memory.wall_dimensions_around = self.around;
            cx.memory.wall_dimensions_mode = self.mode.number();
            cx.memory.wall_dimensions_across = self.across;
            cx.memory.wall_dimensions_core = self.core;
            if self.mode == Mode::Wall
                && (key == "N" || key == "F" && self.face_override.is_none())
                && self.fixed_distance.is_none()
                && let Some(at) = self.at
            {
                self.place_at(at, cx);
            } else {
                self.see(cx);
            }
        } else {
            cx.say(
                Level::Warn,
                if key == "R" {
                    "Kağıt üzerinde 0'dan büyük, en çok 1000 mm bir aralık girin; örneğin 7.5 mm."
                } else if key == "P" && !self.has_source() {
                    "Önce ölçülendirilecek duvarı ya da mahali seçin."
                } else {
                    "Geçerli bir değer seçin; uzaklığı 40 cm veya 0.4 m gibi yazabilirsiniz."
                },
            );
        }
        valid
    }
}

fn paper_mm(text: &str) -> Option<f64> {
    let text = text.trim();
    let number = text.strip_suffix("mm").unwrap_or(text).trim();
    kentos_geometry_core::tools::point_text::parse_number(number).filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paper_spacing_is_paper_millimetres_and_rejects_other_units() {
        assert_eq!(paper_mm("7.5 mm"), Some(7.5));
        assert_eq!(paper_mm("12"), Some(12.0));
        assert_eq!(paper_mm("0.5 m"), None);
        assert_eq!(paper_mm("10 cm"), None);
    }
}
