//! Aks, Kolon, Merdiven, Rampa and Aksları numarala (docs/adr/0343 §5, plan/cad.md `CAD-ARCH-31`–`36`). The tools say
//! what was clicked and chosen; `cad.grids.create`, `cad.columns.create`, `cad.stairs.create`, `cad.ramps.create` and
//! `cad.arch.edit` compute and write, one undo step each, and their plan is the preview.
//!
//! - Aks: two clicks a grid, its label the next one (A, B … or Etiket (E) typed); Seri (S) types the spacings
//!   (“6 6 4.5”): the first grid's two points, then a click on the side the others go.
//! - Kolon: a click its centre; near where two grids cross it stands there, tied to them. Biçim (B), En (E), Boy (Y),
//!   Döndür (D), Etiket (K); Aks kesişimleri (A) puts one at every crossing of the grids shown.
//! - Merdiven and Rampa: the walking line's points (two to four), Enter writes. Genişlik (G), Rıht sayısı (S), Rıht
//!   (R), Basamak (D), Sahanlık (H); a ramp's Yükseklik (Y) and Eğim (E). Their checks are said, never refused.
//! - Aksları numarala: Başlangıç (B), then the grids clicked in order; Enter writes.

use kentos_commands::{
    ArchEdit, ArchOperation, ColumnCrossings, ColumnShape, ColumnsCreate, CommandResult, Entity,
    GridSeries, GridsCreate, NewColumn, NewGrid, NewRamp, NewStair, RampsCreate, SeriesSide,
    StairsCreate,
};
use kentos_domain::Slot;
use kentos_domain::arch_follow::grid_edge;
use kentos_geometry_core::geom::arch;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;
use kentos_native_application::{
    ExecutionContext, arch_edit, columns_create, grids_create, ramps_create, stairs_create,
};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, Tone, Tool};

pub const GRID_ID: &str = "grid";
pub const COLUMN_ID: &str = "column";
pub const STAIR_ID: &str = "stair";
pub const RAMP_ID: &str = "ramp";
pub const GRID_NUMBER_ID: &str = "gridNumber";

/// How near a crossing a column's click is taken to it, pixels' worth of the pick aperture.
const CROSSING_REACH: f64 = 2.0;

/// The objects of a plan drawn as the preview's strokes.
fn drawn(entities: &[Entity], out: &mut Preview) {
    for e in entities {
        out.strokes
            .extend(Outline::of(&shape(e), None, 2.0, Tone::Accent).strokes);
    }
}

/// The plan's objects, or why there are none; its warnings' words.
fn planned<P>(
    result: CommandResult<P>,
    objects: impl FnOnce(P) -> Vec<Entity>,
) -> (Vec<Entity>, Option<String>, Vec<String>) {
    match result {
        CommandResult::Completed { output, warnings } => (
            objects(output),
            None,
            warnings.into_iter().map(|w| w.message).collect(),
        ),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => (Vec::new(), Some(error.message), Vec::new()),
        CommandResult::Queued { .. } | CommandResult::Cancelled => (Vec::new(), None, Vec::new()),
    }
}

/// The grids shown, with their paths.
fn grids_shown(
    cx: &Context<'_>,
) -> Vec<(Slot, String, kentos_geometry_core::geom::intersect::Edge)> {
    cx.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Grid(g) if cx.doc.layers().is_visible(&e.base().layer_id) => {
                let slot = Slot(e.base().id);
                Some((slot, cx.doc.uid(slot)?.to_string(), grid_edge(&g.grid)?))
            }
            _ => None,
        })
        .collect()
}

/// The grid under the pointer.
fn grid_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
    cx.spatial
        .hits(at, cx.pick_tolerance())
        .into_iter()
        .find(|s| matches!(cx.doc.get(*s), Some(Entity::Grid(_))))
}

/// What a tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Label,
    Spacings,
    Width,
    Depth,
    Turn,
    Risers,
    Riser,
    Tread,
    Landing,
    Rise,
    Slope,
}

fn ask_words(ask: Ask) -> &'static str {
    match ask {
        Ask::Label => "etiketi yazın",
        Ask::Spacings => "aralıkları boşlukla yazın (6 6 4.5 gibi; boş: seri yok)",
        Ask::Width => "genişliği yazın",
        Ask::Depth => "boyu yazın",
        Ask::Turn => "dönüşü derece olarak yazın",
        Ask::Risers => "rıht sayısını yazın",
        Ask::Riser => "rıht yüksekliğini yazın",
        Ask::Tread => "basamak derinliğini yazın",
        Ask::Landing => "sahanlığın boyunu yazın",
        Ask::Rise => "yükseklik farkını yazın",
        Ask::Slope => "eğimi yüzde olarak yazın",
    }
}

// ── Aks ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Aks.
#[derive(Clone, Debug, Default)]
pub struct GridTool {
    label: Option<String>,
    spacings: Option<Vec<f64>>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
    shown: Option<String>,
}

impl GridTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn made(&self, cx: &Context<'_>) -> Option<GridsCreate> {
        let at = self.hover?;
        let mut input = GridsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            grids: Vec::new(),
            series: None,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        };
        match (&self.spacings, self.pts.as_slice()) {
            (None, [a]) => input.grids.push(NewGrid {
                pts: points::wire_all(&[*a, at]),
                label: self.label.clone(),
                ..NewGrid::default()
            }),
            (Some(spacings), [a, b]) => {
                let side = if (b.x - a.x) * (at.y - a.y) - (b.y - a.y) * (at.x - a.x) >= 0.0 {
                    SeriesSide::Left
                } else {
                    SeriesSide::Right
                };
                input.series = Some(GridSeries {
                    from: points::wire(*a),
                    to: points::wire(*b),
                    spacings: spacings.clone(),
                    side: Some(side),
                    start: self.label.clone(),
                    bubbles: None,
                    bubble_size: None,
                    arch: None,
                });
            }
            (Some(_), [a]) => input.grids.push(NewGrid {
                pts: points::wire_all(&[*a, at]),
                label: self.label.clone(),
                ..NewGrid::default()
            }),
            _ => return None,
        }
        Some(input)
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        self.shown = None;
        let Some(input) = self.made(cx) else { return };
        let (made, why, _) = planned(grids_create::plan_in(cx.doc, &input), |p| p.grids);
        self.shown = made.iter().find_map(|e| match e {
            Entity::Grid(g) => Some(g.grid.label.clone()),
            _ => None,
        });
        drawn(&made, &mut self.drawn);
        self.why = why;
    }

    fn write(&mut self, input: GridsCreate, cx: &mut Context<'_>) {
        let n = input.grids.len() + input.series.as_ref().map_or(0, |s| s.spacings.len() + 1);
        let result = grids_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            cx.say(Level::Success, format!("{n} aks çizildi."));
            self.label = None;
            self.pts.clear();
        }
    }

    fn ready(&self) -> bool {
        match &self.spacings {
            None => self.pts.len() == 2,
            Some(_) => self.pts.len() == 3,
        }
    }
}

impl Tool for GridTool {
    fn id(&self) -> &'static str {
        GRID_ID
    }

    fn label(&self) -> &'static str {
        "Aks"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.pts.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        if let Some(ask) = self.ask {
            return Prompt::new("Aks", ask_words(ask)).option("Geri", "G");
        }
        let step = match (self.spacings.is_some(), self.pts.len()) {
            (_, 0) => "aksın başına tıklayın",
            (_, 1) => "aksın sonuna tıklayın",
            _ => "diğer aksların gideceği yana tıklayın",
        };
        Prompt::new("Aks", step)
            .option_with(
                "Etiket",
                "E",
                self.label.clone().unwrap_or_else(|| "Sıradaki".into()),
            )
            .option_with(
                "Seri",
                "S",
                self.spacings.as_ref().map_or_else(
                    || "Yok".into(),
                    |s| {
                        s.iter()
                            .map(|d| d.to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    },
                ),
            )
            .option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.hover = Some(p);
        self.pts.push(p);
        if self.ready() {
            self.pts.pop();
            if let Some(input) = self.made(cx) {
                self.write(input, cx);
            }
        }
        self.see(cx);
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if let Some(ask) = self.ask {
            if key == "G" {
                self.ask = None;
                return true;
            }
            match ask {
                Ask::Label => {
                    let t = js_trim(text);
                    self.label = (!t.is_empty()).then(|| t.to_owned());
                }
                _ if js_trim(text).is_empty() => self.spacings = None,
                _ => {
                    let list: Option<Vec<f64>> = text
                        .split_whitespace()
                        .map(|w| cx.typed_length(w))
                        .collect();
                    match list {
                        Some(l) if !l.is_empty() => self.spacings = Some(l),
                        _ => {
                            cx.say(
                                Level::Warn,
                                "Aralıklar okunamadı: sayıları boşlukla yazın (6 6 4.5 gibi).",
                            );
                            return true;
                        }
                    }
                }
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "E" => self.ask = Some(Ask::Label),
            "S" => self.ask = Some(Ask::Spacings),
            _ => {
                if let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover) {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        }
        self.see(cx);
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            return true;
        }
        if !self.pts.is_empty() {
            self.pts.clear();
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        out.path = self.pts.clone();
        if let Some(h) = self.hover
            && !self.pts.is_empty()
        {
            out.path.push(h);
        }
        out.tag = self.hover.map(|at| Tag {
            at,
            lines: match (&self.why, &self.shown) {
                (Some(why), _) => vec!["Aks".into(), why.clone()],
                (None, Some(l)) => vec![format!("Aks {l}")],
                _ => vec!["Aks".into()],
            },
        });
        out.tag_tone = if self.why.is_some() {
            Tone::Danger
        } else {
            Tone::Accent
        };
        out
    }
}

// ── Kolon ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// Kolon.
#[derive(Clone, Debug)]
pub struct ColumnTool {
    round: bool,
    width: f64,
    depth: Option<f64>,
    turn: f64,
    tag: Option<String>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
    place: Option<String>,
}

impl Default for ColumnTool {
    fn default() -> Self {
        Self {
            round: false,
            width: 0.3,
            depth: Some(0.5),
            turn: 0.0,
            tag: None,
            hover: None,
            ask: None,
            drawn: Preview::default(),
            why: None,
            place: None,
        }
    }
}

impl ColumnTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The crossing of two grids shown nearest `at` within reach: the grids' ids.
    fn crossing_at(at: Vec2, cx: &Context<'_>) -> Option<[String; 2]> {
        let reach = cx.pick_tolerance() * CROSSING_REACH;
        let grids = grids_shown(cx);
        let mut best: Option<(f64, [String; 2])> = None;
        for (i, (_, ua, ea)) in grids.iter().enumerate() {
            for (_, ub, eb) in &grids[i + 1..] {
                for p in arch::grid_crossings(ea, eb) {
                    let d = (p.x - at.x).hypot(p.y - at.y);
                    if d <= reach && best.as_ref().is_none_or(|(b, _)| d < *b) {
                        best = Some((d, [ua.clone(), ub.clone()]));
                    }
                }
            }
        }
        best.map(|(_, g)| g)
    }

    fn column(&self, at: Vec2, cx: &Context<'_>) -> NewColumn {
        NewColumn {
            at: Some(points::wire(at)),
            grids: Self::crossing_at(at, cx).map(|g| g.to_vec()),
            shape: self.round.then_some(ColumnShape::Round),
            width: self.width,
            depth: if self.round { None } else { self.depth },
            rotation: (self.turn != 0.0).then_some(self.turn),
            tag: None,
            ..NewColumn::default()
        }
    }

    fn made(
        &self,
        columns: Vec<NewColumn>,
        crossings: Option<ColumnCrossings>,
        cx: &Context<'_>,
    ) -> ColumnsCreate {
        ColumnsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            columns,
            crossings,
            tag_start: self.tag.clone(),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        self.place = None;
        let Some(at) = self.hover else { return };
        let input = self.made(vec![self.column(at, cx)], None, cx);
        let (made, why, _) = planned(columns_create::plan_in(cx.doc, &input), |p| p.columns);
        self.place = made.iter().find_map(|e| match e {
            Entity::Column(c) => c.column.place.clone(),
            _ => None,
        });
        drawn(&made, &mut self.drawn);
        self.why = why;
    }

    fn write(&mut self, input: ColumnsCreate, cx: &mut Context<'_>) {
        let result = columns_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            let n = out.ids.len();
            cx.say(Level::Success, format!("{n} kolon konuldu."));
            if let Some(t) = &self.tag {
                let mut next = t.clone();
                for _ in 0..n {
                    next = kentos_commands::next_label(&next);
                }
                self.tag = Some(next);
            }
        }
    }
}

impl Tool for ColumnTool {
    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn id(&self) -> &'static str {
        COLUMN_ID
    }

    fn label(&self) -> &'static str {
        "Kolon"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn prompt(&self) -> Prompt {
        if let Some(ask) = self.ask {
            return Prompt::new("Kolon", ask_words(ask)).option("Geri", "G");
        }
        let fmt = |v: f64| v.to_string();
        let mut p = Prompt::new(
            "Kolon",
            "kolonun merkezine ya da iki aksın kesişimine tıklayın",
        )
        .option_with(
            "Biçim",
            "B",
            if self.round { "Daire" } else { "Dikdörtgen" },
        )
        .option_with("En", "E", fmt(self.width));
        if !self.round {
            p = p
                .option_with("Boy", "Y", self.depth.map_or_else(|| fmt(self.width), fmt))
                .option_with("Döndür", "D", fmt(self.turn.to_degrees()));
        }
        p.option_with(
            "Etiket",
            "K",
            self.tag.clone().unwrap_or_else(|| "Yok".into()),
        )
        .option("Aks kesişimleri", "A")
        .option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        let input = self.made(vec![self.column(p, cx)], None, cx);
        self.write(input, cx);
        self.see(cx);
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if let Some(ask) = self.ask {
            if key == "G" {
                self.ask = None;
                return true;
            }
            match ask {
                Ask::Label => {
                    let t = js_trim(text);
                    self.tag = (!t.is_empty()).then(|| t.to_owned());
                }
                Ask::Turn => match parse_number(text) {
                    Some(d) => self.turn = d.to_radians(),
                    None => return true,
                },
                _ => match cx.typed_length(text).filter(|v| *v > 0.0) {
                    Some(v) if ask == Ask::Width => self.width = v,
                    Some(v) => self.depth = Some(v),
                    None => return true,
                },
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "B" => self.round = !self.round,
            "E" => self.ask = Some(Ask::Width),
            "Y" if !self.round => self.ask = Some(Ask::Depth),
            "D" if !self.round => self.ask = Some(Ask::Turn),
            "K" => self.ask = Some(Ask::Label),
            "A" => {
                let grids: Vec<String> = grids_shown(cx).into_iter().map(|(_, u, _)| u).collect();
                if grids.len() < 2 {
                    cx.say(
                        Level::Warn,
                        "Kesişim için çizimde en az iki görünen aks olmalı.",
                    );
                    return true;
                }
                let crossings = ColumnCrossings {
                    grids,
                    shape: self.round.then_some(ColumnShape::Round),
                    width: self.width,
                    depth: if self.round { None } else { self.depth },
                    rotation: (self.turn != 0.0).then_some(self.turn),
                    kind: None,
                    fill: None,
                    arch: None,
                };
                let input = self.made(Vec::new(), Some(crossings), cx);
                self.write(input, cx);
            }
            _ => {
                if let Some(p) = cx.typed_point(text, None, self.hover) {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        }
        self.see(cx);
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.ask.take().is_some()
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        out.tag = self.hover.map(|at| Tag {
            at,
            lines: match (&self.why, &self.place) {
                (Some(why), _) => vec!["Kolon".into(), why.clone()],
                (None, Some(p)) => vec![format!("Kolon · {p}")],
                _ => vec!["Kolon".into()],
            },
        });
        out.tag_tone = if self.why.is_some() {
            Tone::Danger
        } else {
            Tone::Accent
        };
        out
    }
}

// ── Merdiven ve Rampa ──────────────────────────────────────────────────────────────────────────────────────────

/// Merdiven or Rampa: a walking line of two to four points and its values.
#[derive(Clone, Debug)]
pub struct FlightTool {
    ramp: bool,
    width: f64,
    risers: u32,
    riser: f64,
    tread: f64,
    landing: Option<f64>,
    rise: f64,
    slope: Option<f64>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
    notes: Vec<String>,
}

impl FlightTool {
    pub fn stair() -> Self {
        Self {
            ramp: false,
            width: 1.2,
            risers: 18,
            riser: 0.17,
            tread: 0.28,
            landing: None,
            rise: 0.5,
            slope: None,
            pts: Vec::new(),
            hover: None,
            ask: None,
            drawn: Preview::default(),
            why: None,
            notes: Vec::new(),
        }
    }

    pub fn ramp() -> Self {
        Self {
            ramp: true,
            width: 1.5,
            ..Self::stair()
        }
    }

    fn name(&self) -> &'static str {
        if self.ramp { "Rampa" } else { "Merdiven" }
    }

    fn line(&self, with_hover: bool) -> Vec<Vec2> {
        let mut pts = self.pts.clone();
        if with_hover
            && pts.len() < kentos_commands::MAX_FLIGHT_POINTS
            && let Some(h) = self.hover
        {
            pts.push(h);
        }
        pts
    }

    /// The plan or the write for the walking line `pts`.
    fn run(
        &self,
        pts: &[Vec2],
        cx: &mut Context<'_>,
        write: bool,
    ) -> (Vec<Entity>, Option<String>, Vec<String>) {
        let layer_id = cx.doc.layers().active().to_owned();
        let color = cx.draft.color_text();
        let line_weight = cx.draft.line_weight;
        if self.ramp {
            let input = RampsCreate {
                layer_id,
                ramps: vec![NewRamp {
                    pts: points::wire_all(pts),
                    width: self.width,
                    rise: self.rise,
                    slope: self.slope,
                    landing: self.landing,
                    ..NewRamp::default()
                }],
                color,
                line_weight,
                expected_revision: None,
            };
            if write {
                let result = ramps_create::execute(&mut ExecutionContext::new(cx.doc), input);
                if points::written(result, cx).is_some() {
                    cx.say(Level::Success, "Rampa çizildi.");
                }
                return (Vec::new(), None, Vec::new());
            }
            planned(ramps_create::plan_in(cx.doc, &input), |p| p.ramps)
        } else {
            let input = StairsCreate {
                layer_id,
                stairs: vec![NewStair {
                    pts: points::wire_all(pts),
                    width: self.width,
                    risers: self.risers,
                    riser_height: self.riser,
                    tread: self.tread,
                    landing: self.landing,
                    ..NewStair::default()
                }],
                color,
                line_weight,
                expected_revision: None,
            };
            if write {
                let result = stairs_create::execute(&mut ExecutionContext::new(cx.doc), input);
                if points::written(result, cx).is_some() {
                    cx.say(Level::Success, "Merdiven çizildi.");
                }
                return (Vec::new(), None, Vec::new());
            }
            planned(stairs_create::plan_in(cx.doc, &input), |p| p.stairs)
        }
    }

    fn see(&mut self, cx: &mut Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        self.notes.clear();
        let pts = self.line(true);
        if pts.len() < 2 {
            return;
        }
        let (made, why, notes) = self.run(&pts, cx, false);
        drawn(&made, &mut self.drawn);
        self.why = why;
        self.notes = notes;
    }
}

impl Tool for FlightTool {
    fn id(&self) -> &'static str {
        if self.ramp { RAMP_ID } else { STAIR_ID }
    }

    fn label(&self) -> &'static str {
        self.name()
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.pts.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        if let Some(ask) = self.ask {
            return Prompt::new(self.name(), ask_words(ask)).option("Geri", "G");
        }
        let step = match self.pts.len() {
            0 => "yürüme çizgisinin başına tıklayın",
            1 => "sonraki noktaya tıklayın",
            _ => "sonraki noktaya tıklayın ya da Enter ile yazın",
        };
        let n = |v: f64| v.to_string();
        let mut p = Prompt::new(self.name(), step).option_with("Genişlik", "G", n(self.width));
        if self.ramp {
            p = p.option_with("Yükseklik", "Y", n(self.rise)).option_with(
                "Eğim",
                "E",
                self.slope
                    .map_or_else(|| "Hesaplanır".into(), |s| format!("%{s}")),
            );
        } else {
            p = p
                .option_with("Rıht sayısı", "S", self.risers.to_string())
                .option_with("Rıht", "R", n(self.riser))
                .option_with("Basamak", "D", n(self.tread));
        }
        p.option_with(
            "Sahanlık",
            "H",
            self.landing.map_or_else(|| "Genişlik kadar".into(), n),
        )
        .option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.pts.push(p);
        if self.pts.len() == kentos_commands::MAX_FLIGHT_POINTS {
            let pts = std::mem::take(&mut self.pts);
            self.run(&pts, cx, true);
        }
        self.see(cx);
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if let Some(ask) = self.ask {
            if key == "G" {
                self.ask = None;
                return true;
            }
            let ok = match ask {
                Ask::Risers => parse_number(text)
                    .filter(|v| *v >= 1.0 && v.fract() == 0.0)
                    .map(|v| self.risers = v as u32)
                    .is_some(),
                Ask::Slope => parse_number(text)
                    .filter(|v| *v > 0.0)
                    .map(|v| self.slope = Some(v))
                    .is_some(),
                Ask::Rise => cx.typed_length(text).map(|v| self.rise = v).is_some(),
                _ => match cx.typed_length(text).filter(|v| *v > 0.0) {
                    Some(v) => {
                        match ask {
                            Ask::Width => self.width = v,
                            Ask::Riser => self.riser = v,
                            Ask::Tread => self.tread = v,
                            _ => self.landing = Some(v),
                        }
                        true
                    }
                    None => false,
                },
            };
            if !ok {
                cx.say(
                    Level::Warn,
                    "Değer okunamadı; sıfırdan büyük bir sayı yazın.",
                );
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        self.ask = match key.as_str() {
            "G" => Some(Ask::Width),
            "H" => Some(Ask::Landing),
            "Y" if self.ramp => Some(Ask::Rise),
            "E" if self.ramp => Some(Ask::Slope),
            "S" if !self.ramp => Some(Ask::Risers),
            "R" if !self.ramp => Some(Ask::Riser),
            "D" if !self.ramp => Some(Ask::Tread),
            _ => {
                if let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover) {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        };
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            return true;
        }
        if !self.pts.is_empty() {
            self.pts.clear();
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.pts.len() >= 2 {
            let pts = std::mem::take(&mut self.pts);
            self.run(&pts, cx, true);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        out.path = self.line(true);
        out.tag = self.hover.map(|at| {
            let mut lines = vec![self.name().to_owned()];
            if let Some(why) = &self.why {
                lines.push(why.clone());
            } else {
                lines.extend(self.notes.iter().take(2).cloned());
            }
            Tag { at, lines }
        });
        out.tag_tone = if self.why.is_some() {
            Tone::Danger
        } else {
            Tone::Accent
        };
        out
    }
}

// ── Aksları numarala ───────────────────────────────────────────────────────────────────────────────────────────

/// Aksları numarala.
#[derive(Clone, Debug)]
pub struct GridNumberTool {
    start: String,
    picked: Vec<Slot>,
    ask: bool,
    hover: Option<Vec2>,
    drawn: Preview,
}

impl Default for GridNumberTool {
    fn default() -> Self {
        Self {
            start: "A".into(),
            picked: Vec::new(),
            ask: false,
            hover: None,
            drawn: Preview::default(),
        }
    }
}

impl GridNumberTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn made(&self, cx: &Context<'_>) -> Option<ArchEdit> {
        let uids: Option<Vec<String>> = self
            .picked
            .iter()
            .map(|s| cx.doc.uid(*s).map(|u| u.to_string()))
            .collect();
        Some(ArchEdit {
            uids: uids?,
            operation: ArchOperation::Renumber {
                start: self.start.clone(),
            },
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        if self.picked.is_empty() {
            return;
        }
        let Some(input) = self.made(cx) else { return };
        let (made, _, _) = planned(arch_edit::plan_in(cx.doc, &input), |p| p.elements);
        drawn(&made, &mut self.drawn);
    }
}

impl Tool for GridNumberTool {
    fn point_count(&self) -> usize {
        self.picked.len()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.picked.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn id(&self) -> &'static str {
        GRID_NUMBER_ID
    }

    fn label(&self) -> &'static str {
        "Aksları numarala"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        if self.ask {
            return Prompt::new("Aksları numarala", "ilk etiketi yazın (A ya da 1 gibi)")
                .option("Geri", "G");
        }
        let step = if self.picked.is_empty() {
            "aksları sırayla tıklayın"
        } else {
            "sıradaki aksı tıklayın, Enter ile numaralayın"
        };
        Prompt::new("Aksları numarala", step)
            .option_with("Başlangıç", "B", self.start.clone())
            .option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        cx.selection.set_hover(grid_at(p.raw, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if let Some(slot) = grid_at(p.raw, cx)
            && !self.picked.contains(&slot)
        {
            self.picked.push(slot);
            self.see(cx);
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if self.ask {
            if key != "G" && !js_trim(text).is_empty() {
                self.start = js_trim(text).to_owned();
            }
            self.ask = false;
            self.see(cx);
            return true;
        }
        if key == "B" {
            self.ask = true;
            return true;
        }
        false
    }

    fn takes_words(&self) -> bool {
        self.ask
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask {
            self.ask = false;
            return true;
        }
        if self.picked.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.picked.is_empty() {
            return Flow::Exit;
        }
        if let Some(input) = self.made(cx) {
            let n = input.uids.len();
            let result = arch_edit::execute(&mut ExecutionContext::new(cx.doc), input);
            if points::written(result, cx).is_some() {
                cx.say(Level::Success, format!("{n} aks numaralandı."));
            }
        }
        self.picked.clear();
        self.see(cx);
        Flow::Stay
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        out.tag = self.hover.map(|at| Tag {
            at,
            lines: vec![format!("{} aks seçildi", self.picked.len())],
        });
        out
    }
}
