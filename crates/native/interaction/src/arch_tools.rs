//! Aks, Kolon, Merdiven, Rampa and Aksları numarala (docs/adr/0343 §5, plan/cad.md `CAD-ARCH-31`–`36`). The tools say
//! what was clicked and chosen; `cad.grids.create`, `cad.columns.create`, `cad.stairs.create`, `cad.ramps.create` and
//! `cad.arch.edit` compute and write, one undo step each, and their plan is the preview.
//!
//! - Aks: two clicks a grid, its label the next one (A, B … or Etiket (E) typed); Seri (S) types the spacings
//!   (“6 6 4.5”): the first grid's two points, then a click on the side the others go.
//! - Kolon: a click its centre; near where two grids cross it stands there, tied to them. Biçim (B), En (E), Boy (Y),
//!   Döndür (D), Etiket (K); Aks kesişimleri (A) puts one at every crossing of the grids shown.
//! - Merdiven and Rampa: the walking line's points (two to four), Enter writes. A stair from its Yükseklik (Y; Etkin
//!   kat's height until one is typed, docs/adr/0713): En fazla
//!   rıht (R), Rıht sayısı (S, else from the height), Basamak (D), Genişlik (G), Sahanlık (H), Kesme (K), Numaralar
//!   (N); its last flight ends at its last riser (docs/adr/0704). A ramp's Yükseklik (Y) and Eğim (E). Their checks
//!   are said, never refused; the same values are in the tool's settings pane. Biçim (B) Döner, and Döner merdiven
//!   (docs/adr/0709): its centre, its walking line's start, then the way it turns as the pointer goes round; it ends at
//!   its last riser, or where clicked, or at a turn typed in degrees.
//! - Aksları numarala: Başlangıç (B), then the grids clicked in order; Enter writes.

use kentos_commands::{
    ArchEdit, ArchOperation, ColumnCrossings, ColumnShape, ColumnsCreate, CommandResult, Entity,
    GridSeries, GridsCreate, NewColumn, NewGrid, NewRamp, NewStair, RampsCreate, Railings,
    SeriesSide, StairsCreate,
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
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, TextGhost, Tone, Tool};
use crate::tool_settings::Setting;

pub const GRID_ID: &str = "grid";
pub const COLUMN_ID: &str = "column";
pub const STAIR_ID: &str = "stair";
/// Döner merdiven (docs/adr/0709): the stair tool started curved.
pub const SPIRAL_ID: &str = "stairCurved";
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
    Tread,
    Landing,
    /// En uzun kol (docs/adr/0723 §1).
    RampRun,
    /// A ramp's start: its level's, over it, or its own (docs/adr/0723 §2).
    RampBase,
    Rise,
    Slope,
    Height,
    MaxRiser,
    Cut,
}

fn ask_words(ask: Ask) -> &'static str {
    match ask {
        Ask::Label => "etiketi yazın",
        Ask::Spacings => "aralıkları boşlukla yazın (6 6 4.5 gibi; boş: seri yok)",
        Ask::Width => "genişliği yazın",
        Ask::Depth => "boyu yazın",
        Ask::Turn => "dönüşü derece olarak yazın",
        Ask::Risers => "rıht sayısını yazın (0: yükseklikten hesaplanır)",
        Ask::Height => "merdivenin yüksekliğini yazın (iki kat arası)",
        Ask::MaxRiser => "en yüksek rıhtı yazın",
        Ask::Cut => "planın kesme yüksekliğini yazın (0: kesme yok)",
        Ask::Tread => "basamak derinliğini yazın",
        Ask::Landing => "sahanlığın boyunu yazın",
        Ask::RampRun => "en uzun eğimli kolu yazın (boş: sahanlık yalnız dönüşlerde)",
        Ask::RampBase => "başlangıç kotunu yazın (boş ya da kat: katın kotu; kat+0.15: kattan fark; sayı: kendi kotu)",
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
            system: None,
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
            // Etkin kat (docs/adr/0713).
            arch: cx.level_arch(),
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
                    arch: cx.level_arch(),
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

/// Merdiven or Rampa: a walking line of two to four points and its values. A stair is drawn from its height
/// (docs/adr/0704): its risers as many as keep each under the highest riser unless given, its last flight ending at
/// its last riser, its cut line at the plan's cut height; the preview is the command's plan, risers and all.
#[derive(Clone, Debug)]
pub struct FlightTool {
    ramp: bool,
    /// Döner merdiven (docs/adr/0709): its points its centre and its walking line's start; the pointer's way round the
    /// centre turns it. `spiral`: started as Döner merdiven (its own tool).
    curved: bool,
    spiral: bool,
    /// The turn the pointer has gone round the centre from the start (counter-clockwise positive) and its last angle.
    turned: f64,
    seen_at: Option<f64>,
    /// The planned curved stair: its walking line's radius and turn.
    arc_design: Option<(f64, f64)>,
    width: f64,
    /// A stair's whole rise, its highest riser, its risers (0: from the height), its tread, its cut height.
    height: f64,
    /// The rise was typed (Y): Etkin kat's height is no longer followed (docs/adr/0713's 27 October addendum).
    height_typed: bool,
    /// Etkin kat's floor-to-floor height, as last seen: the stair's rise until one is typed.
    level_height: Option<f64>,
    max_riser: f64,
    risers: u32,
    tread: f64,
    cut: Option<f64>,
    numbering: bool,
    /// The last flight ends at the last riser, a ramp's where its slope says (else where it was clicked).
    fit: bool,
    /// The sides with a railing (docs/adr/0705 §2).
    railings: Option<Railings>,
    landing: Option<f64>,
    /// En uzun kol: a landing wherever a sloped run would be longer (docs/adr/0723 §1).
    max_run: Option<f64>,
    /// A ramp's start (docs/adr/0723 §2): its own elevation typed, else Etkin kat's elevation plus `base_offset`.
    ramp_base: Option<f64>,
    base_offset: f64,
    /// Etkin kat's floor elevation, as last seen.
    level_elevation: Option<f64>,
    rise: f64,
    slope: Option<f64>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
    notes: Vec<String>,
    /// The planned stair: its risers, a riser's height, its tread and its walking line's length.
    design: Option<(u32, f64, f64, f64)>,
    /// The planned ramp: its slope (given or its rise over its sloped length), its sloped length, its levels.
    ramp_design: Option<(f64, f64, f64, f64)>,
}

/// A railing choice's words.
fn railing_words(r: Railings) -> &'static str {
    r.words()
}

/// Korkuluk: none, left, right or both, as one walks up (docs/adr/0705 §2).
fn railings_setting(on: Option<Railings>) -> Setting {
    Setting::choice(
        "O",
        "Korkuluk",
        crate::tool_settings::choices(
            &[("none", "Yok"), ("left", "Sol"), ("right", "Sağ"), ("both", "İki yan")],
            on.map_or("none", Railings::as_str),
        ),
    )
}

/// The plan of a stair or ramp drawn as the preview: its pieces as strokes (dashed past a stair's cut), its numbers
/// as ghost texts.
pub(crate) fn flight_drawn(entities: &[Entity], scale: f64, out: &mut Preview) {
    for e in entities {
        for piece in arch::pieces(&shape(e), scale) {
            match &piece.shape {
                kentos_geometry_core::entity::Shape::Text {
                    p, text, height, ..
                } => out.texts.push(TextGhost {
                    p: *p,
                    text: text.clone(),
                    height: *height,
                    rotation: 0.0,
                    align: Some(kentos_geometry_core::text::TextAlign::MiddleCenter),
                    mask: false,
                    face: Default::default(),
                    width_factor: 1.0,
                }),
                other => {
                    let dash = (piece.stroke.line_type.as_deref() == Some("dashed")).then_some([5.0, 4.0]);
                    out.strokes.extend(Outline::of(other, dash, 1.5, Tone::Accent).strokes);
                }
            }
        }
    }
}

impl FlightTool {
    pub fn stair() -> Self {
        Self {
            ramp: false,
            curved: false,
            spiral: false,
            turned: 0.0,
            seen_at: None,
            arc_design: None,
            width: 1.2,
            height: 3.0,
            height_typed: false,
            level_height: None,
            max_riser: kentos_commands::DEFAULT_MAX_RISER,
            risers: 0,
            tread: 0.28,
            cut: Some(1.1),
            numbering: false,
            fit: true,
            railings: None,
            landing: None,
            max_run: None,
            ramp_base: None,
            base_offset: 0.0,
            level_elevation: None,
            rise: 0.5,
            slope: None,
            pts: Vec::new(),
            hover: None,
            ask: None,
            drawn: Preview::default(),
            why: None,
            notes: Vec::new(),
            design: None,
            ramp_design: None,
        }
    }

    /// Rampa: 1.50 m wide, 0.50 m up at 6 % (KentOS's default, not a regulation's), its last flight ending where its
    /// slope says.
    pub fn ramp() -> Self {
        Self {
            ramp: true,
            width: 1.5,
            slope: Some(6.0),
            ..Self::stair()
        }
    }

    /// Döner merdiven (docs/adr/0709): 1.00 m wide, its walking line's radius where its start is clicked.
    pub fn curved_stair() -> Self {
        Self {
            curved: true,
            spiral: true,
            width: 1.0,
            ..Self::stair()
        }
    }

    /// A stair's rise: typed, else Etkin kat's height (docs/adr/0713's 27 October addendum), else its default.
    fn stair_height(&self) -> f64 {
        self.followed_height().unwrap_or(self.height)
    }

    /// Etkin kat's height while the stair follows it.
    fn followed_height(&self) -> Option<f64> {
        if self.height_typed { None } else { self.level_height }
    }

    /// Sees Etkin kat's height (a positive, finite floor to floor) and its elevation.
    fn see_level(&mut self, cx: &Context<'_>) {
        self.level_elevation = cx.level.and_then(|id| {
            cx.doc.settings().arch.as_ref()?.levels.iter().find(|l| l.id == id).map(|l| l.elevation)
        });
        self.level_height = cx
            .level
            .and_then(|id| {
                cx.doc
                    .settings()
                    .arch
                    .as_ref()?
                    .levels
                    .iter()
                    .find(|l| l.id == id)?
                    .height
            })
            .filter(|h| h.is_finite() && *h > 0.0 && *h <= 100.0);
    }

    /// The stair's rise in the settings: Etkin kat's, said so.
    fn height_words(&self, f: &Format) -> String {
        match self.followed_height() {
            Some(h) => format!("{} · katın", f.length(h)),
            None => f.length(self.height),
        }
    }

    /// A ramp's start in the settings and the prompt: its own, else its level's plus its offset, said so.
    fn base_words(&self, f: &Format) -> String {
        match (self.ramp_base, self.level_elevation) {
            (Some(b), _) => f.length(b),
            (None, Some(e)) if self.base_offset == 0.0 => format!("{} · katın", f.length(e)),
            (None, Some(e)) => format!(
                "{} · katın {}{}",
                f.length(e + self.base_offset),
                if self.base_offset < 0.0 { "−" } else { "+" },
                f.length(self.base_offset.abs())
            ),
            (None, None) => f.length(self.base_offset),
        }
    }

    /// A ramp's start on the prompt: its own, else Kat and its offset.
    fn base_option(&self) -> String {
        match (self.ramp_base, self.level_elevation) {
            (Some(b), _) => b.to_string(),
            (None, Some(_)) if self.base_offset == 0.0 => "Kat".into(),
            (None, Some(_)) if self.base_offset < 0.0 => format!("Kat−{}", -self.base_offset),
            (None, Some(_)) => format!("Kat+{}", self.base_offset),
            (None, None) => self.base_offset.to_string(),
        }
    }

    /// A ramp's start as the command takes it (docs/adr/0723 §2): its own base, else bound to Etkin kat.
    fn ramp_start(&self, cx: &Context<'_>) -> (Option<f64>, Option<f64>) {
        match (self.ramp_base, cx.level) {
            (Some(b), _) => (Some(b), None),
            (None, Some(_)) => (None, Some(self.base_offset)),
            (None, None) => ((self.base_offset != 0.0).then_some(self.base_offset), None),
        }
    }

    fn name(&self) -> &'static str {
        match (self.ramp, self.curved) {
            (true, _) => "Rampa",
            (false, true) => "Döner merdiven",
            (false, false) => "Merdiven",
        }
    }

    /// Whether the clicks draw a curved stair.
    fn spiralling(&self) -> bool {
        self.curved && !self.ramp
    }

    /// The angle of `p` round the clicked centre.
    fn angle_of(&self, p: Vec2) -> Option<f64> {
        let c = *self.pts.first()?;
        let (dx, dy) = (p.x - c.x, p.y - c.y);
        (dx != 0.0 || dy != 0.0).then(|| dy.atan2(dx))
    }

    /// Follows the pointer round the centre (docs/adr/0709): its turn grows by each move's own, so that going on round
    /// past a half turn keeps its way; less than a whole turn.
    fn follow(&mut self, p: Vec2) {
        if !self.spiralling() || self.pts.len() != 2 {
            return;
        }
        let Some(a) = self.angle_of(p) else { return };
        if let Some(last) = self.seen_at.or_else(|| self.angle_of(self.pts[1])) {
            let tau = std::f64::consts::TAU;
            let d = (a - last + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI;
            self.turned = (self.turned + d).clamp(-(tau - 1e-6), tau - 1e-6);
        }
        self.seen_at = Some(a);
    }

    /// A curved stair's walking line and its bulges: from its start round its centre by the pointer's turn; with Son
    /// kol rıhtlara göre a quarter turn its way (the command ends it at its last riser). None before the start or with
    /// no turn yet.
    fn arc_line(&self) -> Option<(Vec<Vec2>, Vec<f64>)> {
        let [c, a] = self.pts[..] else { return None };
        if self.turned.abs() < 1e-6 {
            return None;
        }
        let sweep = if self.fit {
            std::f64::consts::FRAC_PI_2.copysign(self.turned)
        } else {
            self.turned
        };
        let r = (a.x - c.x).hypot(a.y - c.y);
        let t = (a.y - c.y).atan2(a.x - c.x) + sweep;
        Some((vec![a, Vec2::new(c.x + r * t.cos(), c.y + r * t.sin())], vec![(sweep / 4.0).tan(), 0.0]))
    }

    /// Writes the curved stair the pointer has turned; says so when it has not turned yet.
    fn write_arc(&mut self, cx: &mut Context<'_>) {
        match self.arc_line() {
            Some((pts, bulges)) => {
                self.run(&pts, Some(bulges), cx, true);
                self.pts.clear();
                self.turned = 0.0;
                self.seen_at = None;
            }
            None => cx.say(
                Level::Warn,
                "Döner merdivenin yönü yok: imleci merkezin çevresinde merdivenin döneceği yana götürün ya da dönüşü derece yazın.",
            ),
        }
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
        bulges: Option<Vec<f64>>,
        cx: &mut Context<'_>,
        write: bool,
    ) -> (Vec<Entity>, Option<String>, Vec<String>) {
        let layer_id = cx.doc.layers().active().to_owned();
        let color = cx.draft.color_text();
        let line_weight = cx.draft.line_weight;
        if self.ramp {
            let (base, base_offset) = self.ramp_start(cx);
            let input = RampsCreate {
                layer_id,
                ramps: vec![NewRamp {
                    base,
                    base_offset,
                    pts: points::wire_all(pts),
                    width: self.width,
                    rise: self.rise,
                    slope: self.slope,
                    landing: self.landing,
                    max_run: self.max_run,
                    fit: self.fit.then_some(true),
                    railings: self.railings,
                    // Etkin kat (docs/adr/0713).
                    arch: cx.level_arch(),
                    ..NewRamp::default()
                }],
                color,
                line_weight,
                expected_revision: None,
            };
            if write {
                let result = ramps_create::execute(&mut ExecutionContext::new(cx.doc), input);
                if points::written(result, cx).is_some() {
                    cx.say(
                        Level::Success,
                        if self.fit {
                            "Rampa çizildi; son kol eğime göre bitti."
                        } else {
                            "Rampa çizildi."
                        },
                    );
                }
                return (Vec::new(), None, Vec::new());
            }
            planned(ramps_create::plan_in(cx.doc, &input), |p| p.ramps)
        } else {
            let input = StairsCreate {
                layer_id,
                stairs: vec![NewStair {
                    pts: points::wire_all(pts),
                    bulges,
                    width: self.width,
                    risers: self.risers,
                    tread: self.tread,
                    height: Some(self.stair_height()),
                    max_riser: Some(self.max_riser),
                    cut_height: self.cut,
                    fit: self.fit.then_some(true),
                    landing: self.landing,
                    numbering: self.numbering.then_some(true),
                    railings: self.railings,
                    // Etkin kat (docs/adr/0713).
                    arch: cx.level_arch(),
                    ..NewStair::default()
                }],
                color,
                line_weight,
                expected_revision: None,
            };
            if write {
                let result = stairs_create::execute(&mut ExecutionContext::new(cx.doc), input);
                if points::written(result, cx).is_some() {
                    cx.say(
                        Level::Success,
                        match (self.spiralling(), self.fit) {
                            (true, true) => "Döner merdiven çizildi; yay son rıhtta bitti.",
                            (true, false) => "Döner merdiven çizildi.",
                            (false, true) => "Merdiven çizildi; son kol son rıhtta bitti.",
                            (false, false) => "Merdiven çizildi.",
                        },
                    );
                }
                return (Vec::new(), None, Vec::new());
            }
            planned(stairs_create::plan_in(cx.doc, &input), |p| p.stairs)
        }
    }

    fn see(&mut self, cx: &mut Context<'_>) {
        self.see_level(cx);
        self.drawn = Preview::default();
        self.why = None;
        self.notes.clear();
        self.design = None;
        self.ramp_design = None;
        self.arc_design = None;
        let (pts, bulges) = if self.spiralling() {
            match self.arc_line() {
                Some((pts, bulges)) => (pts, Some(bulges)),
                None => {
                    self.circles();
                    return;
                }
            }
        } else {
            (self.line(true), None)
        };
        if pts.len() < 2 {
            return;
        }
        let (made, why, notes) = self.run(&pts, bulges, cx, false);
        flight_drawn(&made, cx.doc.settings().plot_scale, &mut self.drawn);
        self.arc_design = made.iter().find_map(|e| match e {
            Entity::Stair(s) => {
                let [a, b] = s.stair.pts[..] else { return None };
                kentos_geometry_core::geom::bulge::bulge_arc(
                    kentos_geometry_core::vec2::Vec2::new(a.x, a.y),
                    kentos_geometry_core::vec2::Vec2::new(b.x, b.y),
                    s.stair.bulge(),
                )
                .map(|arc| (arc.r, arc.sweep))
            }
            _ => None,
        });
        self.design = made.iter().find_map(|e| match e {
            Entity::Stair(s) => {
                let walk: Vec<kentos_geometry_core::vec2::Vec2> = s
                    .stair
                    .pts
                    .iter()
                    .map(|p| kentos_geometry_core::vec2::Vec2::new(p.x, p.y))
                    .collect();
                Some((
                    s.stair.risers,
                    s.stair.riser_height,
                    s.stair.tread,
                    arch::Walk::curved(&walk, s.stair.bulge()).length(),
                ))
            }
            _ => None,
        });
        self.ramp_design = made.iter().find_map(|e| match e {
            Entity::Ramp(r) => {
                let shape = shape(e);
                let kentos_geometry_core::entity::Shape::Ramp { pts, ramp } = &shape else {
                    return None;
                };
                let run = arch::ramp_run(&arch::Walk::of(pts), ramp);
                let slope = r.ramp.slope.unwrap_or(if run > 0.0 { r.ramp.rise / run * 100.0 } else { 0.0 });
                let base = r.ramp.base.unwrap_or(0.0);
                Some((slope, run, base, base + r.ramp.rise))
            }
            _ => None,
        });
        self.why = why;
        self.notes = notes;
    }

    /// Before a curved stair turns: its walking line's circle through the pointer (or its start) and its sides' circles
    /// half its width in and out, dashed.
    fn circles(&mut self) {
        let Some(&c) = self.pts.first() else { return };
        let Some(at) = self.pts.get(1).copied().or(self.hover) else { return };
        let r = (at.x - c.x).hypot(at.y - c.y);
        if r <= 0.0 {
            return;
        }
        for (radius, dash) in [(r, None), (r - self.width / 2.0, Some([5.0, 4.0])), (r + self.width / 2.0, Some([5.0, 4.0]))] {
            if radius > 0.0 {
                let circle = kentos_geometry_core::entity::Shape::Circle {
                    c: kentos_geometry_core::vec2::Vec2::new(c.x, c.y),
                    r: radius,
                };
                self.drawn.strokes.extend(Outline::of(&circle, dash, 1.5, Tone::Accent).strokes);
            }
        }
    }

    /// The planned stair in words: its risers and their height, its tread, the step rule, its walking line; a ramp's
    /// slope, sloped length and levels.
    fn design_words(&self, f: &Format) -> Vec<String> {
        if let Some((slope, run, from, to)) = self.ramp_design {
            return vec![
                format!("Eğim %{} · eğimli uzunluk {}", crate::fixed(slope, 2), f.length(run)),
                format!("Kot {} → {}", arch::level_text(from), arch::level_text(to)),
            ];
        }
        let Some((risers, riser, tread, run)) = self.design else {
            return Vec::new();
        };
        let mut out = vec![
            format!("{risers} rıht × {}, basamak {}", f.length(riser), f.length(tread)),
            format!("2R+G {} · yürüme çizgisi {}", f.length(2.0 * riser + tread), f.length(run)),
        ];
        // A curved stair's tread is measured on its walking line; on its inner side it is shorter (docs/adr/0709).
        if let Some((r, sweep)) = self.arc_design {
            // Going up a counter-clockwise stair one turns left.
            out.push(format!(
                "yarıçap {} · {}° {} döner",
                f.length(r),
                crate::fixed(sweep.abs().to_degrees(), 0),
                if sweep < 0.0 { "sağa" } else { "sola" },
            ));
            out.push(format!("iç kenarda basamak {}", f.length(tread * (r - self.width / 2.0) / r)));
        }
        out
    }

    /// Applies one setting's value: the stair's or the ramp's own; false when it does not read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let words = upper_tr(js_trim(value));
        let length = |cx: &mut Context<'_>| cx.typed_length(value).filter(|v| v.is_finite() && *v > 0.0);
        match key {
            "G" => length(cx).map(|v| self.width = v).is_some(),
            "H" if words.is_empty() || words.starts_with("GEN") => {
                self.landing = None;
                true
            }
            "H" => length(cx).map(|v| self.landing = Some(v)).is_some(),
            "L" if self.ramp && (words.is_empty() || words.starts_with("YOK")) => {
                self.max_run = None;
                true
            }
            "L" if self.ramp => length(cx).map(|v| self.max_run = Some(v)).is_some(),
            // Empty or Kat: Etkin kat's elevation; Kat+0.15: over it; a number: its own (docs/adr/0723 §2).
            "Z" if self.ramp && (words.is_empty() || words.starts_with("KAT")) => {
                let rest = js_trim(value).get(3..).unwrap_or("").trim();
                let rest = rest.strip_prefix('+').unwrap_or(rest).trim();
                let offset = if rest.is_empty() { Some(0.0) } else { cx.typed_length(rest).filter(|v| v.is_finite()) };
                offset
                    .map(|o| {
                        self.ramp_base = None;
                        self.base_offset = o;
                    })
                    .is_some()
            }
            "Z" if self.ramp => cx.typed_length(value).filter(|v| v.is_finite()).map(|v| self.ramp_base = Some(v)).is_some(),
            "Y" if self.ramp => cx.typed_length(value).filter(|v| v.is_finite()).map(|v| self.rise = v).is_some(),
            "E" if self.ramp && (words.is_empty() || words.starts_with("HES")) => {
                self.slope = None;
                true
            }
            "E" if self.ramp => parse_number(value)
                .filter(|v| *v > 0.0)
                .map(|v| self.slope = Some(v))
                .is_some(),
            // Empty or Kat: Etkin kat's height again (docs/adr/0713's 27 October addendum).
            "Y" if words.is_empty() || words.contains("KAT") => {
                self.height_typed = false;
                true
            }
            "Y" => length(cx)
                .map(|v| {
                    self.height = v;
                    self.height_typed = true;
                })
                .is_some(),
            "R" => length(cx).map(|v| self.max_riser = v).is_some(),
            "S" if words.is_empty() || words.starts_with("HES") => {
                self.risers = 0;
                true
            }
            "S" => parse_number(value)
                .filter(|v| *v >= 0.0 && *v <= f64::from(kentos_commands::MAX_RISERS) && v.fract() == 0.0)
                .map(|v| self.risers = v as u32)
                .is_some(),
            "D" => length(cx).map(|v| self.tread = v).is_some(),
            "K" if words.is_empty() || words == "YOK" || parse_number(value) == Some(0.0) => {
                self.cut = None;
                true
            }
            "K" => length(cx).map(|v| self.cut = Some(v)).is_some(),
            "O" => match words.as_str() {
                "" | "NONE" | "YOK" => {
                    self.railings = None;
                    true
                }
                "LEFT" | "SOL" => {
                    self.railings = Some(Railings::Left);
                    true
                }
                // The settings pane sends `right`, which Turkish capitals make RİGHT.
                "RIGHT" | "RİGHT" | "SAĞ" => {
                    self.railings = Some(Railings::Right);
                    true
                }
                w if w == "BOTH" || w.starts_with("İKİ") => {
                    self.railings = Some(Railings::Both);
                    true
                }
                _ => false,
            },
            "B" if !self.ramp => {
                let curved = match words.as_str() {
                    w if w == "CURVED" || w.starts_with("DÖNER") => true,
                    w if w == "STRAIGHT" || w == "STRAİGHT" || w.starts_with("DÜZ") || w.starts_with("YOL") => false,
                    _ => return false,
                };
                self.shape(curved);
                true
            }
            "U" => match words.as_str() {
                // The settings pane's `fit` and `clicked` in Turkish capitals are FİT and CLİCKED.
                w if w == "FIT" || w == "FİT" || w.starts_with("RIHT") || w.starts_with("EĞİM") || w.starts_with("YAY") => {
                    self.fit = true;
                    true
                }
                w if w == "CLICKED" || w == "CLİCKED" || w.starts_with("TIKLA") => {
                    self.fit = false;
                    true
                }
                _ => false,
            },
            "N" => match words.as_str() {
                "YES" | "AÇIK" | "EVET" => {
                    self.numbering = true;
                    true
                }
                "NO" | "KAPALI" | "HAYIR" => {
                    self.numbering = false;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

impl FlightTool {
    /// Biçim: a curved or a straight stair; the points clicked so far go with the other shape's (they mean other things).
    fn shape(&mut self, curved: bool) {
        if self.curved != curved {
            self.curved = curved;
            self.pts.clear();
            self.turned = 0.0;
            self.seen_at = None;
        }
    }
}

impl Tool for FlightTool {
    fn id(&self) -> &'static str {
        match (self.ramp, self.spiral) {
            (true, _) => RAMP_ID,
            (false, true) => SPIRAL_ID,
            (false, false) => STAIR_ID,
        }
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

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see_level(cx);
        Flow::Stay
    }

    /// The stair's height follows the new level's at once (docs/adr/0720 §4).
    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.pts.pop().is_some() {
            self.turned = 0.0;
            self.seen_at = None;
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        if let Some(ask) = self.ask {
            return Prompt::new(self.name(), ask_words(ask)).option("Geri", "G");
        }
        if self.spiralling() {
            let n = |v: f64| v.to_string();
            let step = match self.pts.len() {
                0 => "döner merdivenin merkezine tıklayın",
                1 => "yürüme çizgisinin başına tıklayın (merkeze uzaklığı yarıçap; çıkış buradan)",
                _ if self.fit => "imleci merdivenin döneceği yana götürüp tıklayın; yay son rıhtta biter",
                _ => "yayın bitişine tıklayın ya da dönüşü derece yazın (eksi: saat yönünde)",
            };
            return Prompt::new(self.name(), step)
                .option_with("Biçim", "B", "Döner")
                .option_with(
                    "Yükseklik",
                    "Y",
                    self.followed_height().map_or_else(|| n(self.height), |h| format!("{} (katın)", n(h))),
                )
                .option_with("En fazla rıht", "R", n(self.max_riser))
                .option_with(
                    "Rıht sayısı",
                    "S",
                    if self.risers == 0 { "Hesaplanır".into() } else { self.risers.to_string() },
                )
                .option_with("Basamak", "D", n(self.tread))
                .option_with("Genişlik", "G", n(self.width))
                .option_with("Kesme", "K", self.cut.map_or_else(|| "Yok".into(), n))
                .option_with("Numaralar", "N", if self.numbering { "Açık" } else { "Kapalı" })
                .option_with("Korkuluk", "O", self.railings.map_or("Yok", railing_words))
                .option_with("Yay", "U", if self.fit { "Rıhtlara göre" } else { "Tıklanan yerde" });
        }
        let n = |v: f64| v.to_string();
        if self.ramp {
            let step = match self.pts.len() {
                0 => "orta çizginin başına tıklayın",
                1 if self.fit => "çıkış yönüne ya da dönüşe tıklayın; son kol eğime göre biter",
                1 => "sonraki noktaya tıklayın",
                _ => "sonraki noktaya tıklayın ya da Enter ile yazın",
            };
            return Prompt::new(self.name(), step)
                .option_with("Yükseklik", "Y", n(self.rise))
                .option_with(
                    "Eğim",
                    "E",
                    self.slope.map_or_else(|| "Hesaplanır".into(), |s| format!("%{s}")),
                )
                .option_with("Genişlik", "G", n(self.width))
                .option_with("Sahanlık", "H", self.landing.map_or_else(|| "Genişlik kadar".into(), n))
                .option_with("En uzun kol", "L", self.max_run.map_or_else(|| "Yok".into(), n))
                .option_with("Başlangıç", "Z", self.base_option())
                .option_with("Korkuluk", "O", self.railings.map_or("Yok", railing_words))
                .option_with("Son kol", "U", if self.fit { "Eğime göre" } else { "Tıklanan yerde" })
                .option("Bitir", "Enter");
        }
        let step = match self.pts.len() {
            0 => "yürüme çizgisinin başına tıklayın (çıkış buradan)",
            1 if self.fit => "çıkış yönüne ya da dönüşe tıklayın; son kol rıhtlara göre biter",
            1 => "çıkış yönüne ya da dönüşe tıklayın",
            _ => "dönüşe tıklayın ya da Enter ile bitirin",
        };
        Prompt::new(self.name(), step)
            .option_with("Biçim", "B", "Düz, L, U")
            .option_with(
                    "Yükseklik",
                    "Y",
                    self.followed_height().map_or_else(|| n(self.height), |h| format!("{} (katın)", n(h))),
                )
            .option_with("En fazla rıht", "R", n(self.max_riser))
            .option_with(
                "Rıht sayısı",
                "S",
                if self.risers == 0 { "Hesaplanır".into() } else { self.risers.to_string() },
            )
            .option_with("Basamak", "D", n(self.tread))
            .option_with("Genişlik", "G", n(self.width))
            .option_with("Sahanlık", "H", self.landing.map_or_else(|| "Genişlik kadar".into(), n))
            .option_with("Kesme", "K", self.cut.map_or_else(|| "Yok".into(), n))
            .option_with("Numaralar", "N", if self.numbering { "Açık" } else { "Kapalı" })
            .option_with("Korkuluk", "O", self.railings.map_or("Yok", railing_words))
            .option_with("Son kol", "U", if self.fit { "Rıhtlara göre" } else { "Tıklanan yerde" })
            .option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.follow(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.see_level(cx);
        if self.spiralling() {
            // Its centre, its start, then the end of its turn (docs/adr/0709).
            match self.pts.len() {
                0 | 1 => {
                    self.pts.push(p);
                    self.turned = 0.0;
                    self.seen_at = None;
                }
                _ => {
                    self.hover = Some(p);
                    self.follow(p);
                    self.write_arc(cx);
                }
            }
            self.see(cx);
            return true;
        }
        self.pts.push(p);
        if self.pts.len() == kentos_commands::MAX_FLIGHT_POINTS {
            let pts = std::mem::take(&mut self.pts);
            self.run(&pts, None, cx, true);
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
            let field = match ask {
                Ask::Width => "G",
                Ask::Landing => "H",
                Ask::RampRun => "L",
                Ask::RampBase => "Z",
                Ask::Rise | Ask::Height => "Y",
                Ask::Slope => "E",
                Ask::MaxRiser => "R",
                Ask::Risers => "S",
                Ask::Tread => "D",
                Ask::Cut => "K",
                _ => "",
            };
            if !self.apply(field, text, cx) {
                cx.say(
                    Level::Warn,
                    "Değer okunamadı; sıfırdan büyük bir sayı yazın (Rıht sayısı: 0 hesaplanır, Kesme: 0 yok).",
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
            "L" if self.ramp => Some(Ask::RampRun),
            "Z" if self.ramp => Some(Ask::RampBase),
            "Y" if self.ramp => Some(Ask::Rise),
            "E" if self.ramp => Some(Ask::Slope),
            "Y" => Some(Ask::Height),
            "R" if !self.ramp => Some(Ask::MaxRiser),
            "S" if !self.ramp => Some(Ask::Risers),
            "D" if !self.ramp => Some(Ask::Tread),
            "K" if !self.ramp => Some(Ask::Cut),
            "N" if !self.ramp => {
                self.numbering = !self.numbering;
                self.see(cx);
                return true;
            }
            "U" => {
                self.fit = !self.fit;
                self.see(cx);
                return true;
            }
            "B" if !self.ramp => {
                self.shape(!self.curved);
                self.see(cx);
                return true;
            }
            "O" => {
                self.railings = match self.railings {
                    None => Some(Railings::Left),
                    Some(Railings::Left) => Some(Railings::Right),
                    Some(Railings::Right) => Some(Railings::Both),
                    Some(Railings::Both) => None,
                };
                self.see(cx);
                return true;
            }
            _ => {
                // A curved stair's turn typed in the project's angle (docs/adr/0709): its way, and with Yay tıklanan
                // yerde its size.
                if self.spiralling()
                    && self.pts.len() == 2
                    && let Some(n) = parse_number(text)
                {
                    let tau = std::f64::consts::TAU;
                    self.turned = cx.format().turn_from_typed(n).clamp(-(tau - 1e-6), tau - 1e-6);
                    self.write_arc(cx);
                    self.see(cx);
                    return true;
                }
                if let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover) {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        };
        true
    }

    fn settings(&self, f: &Format) -> Vec<Setting> {
        let landing = self.landing.map_or_else(|| "Genişlik kadar".into(), |v| f.length(v));
        if self.ramp {
            let mut rows = vec![
                Setting::text("Y", "Yükseklik farkı", f.length(self.rise)),
                Setting::text(
                    "E",
                    "Eğim (%)",
                    self.slope.map_or_else(|| "Hesaplanır".into(), |s| s.to_string()),
                ),
                Setting::text("G", "Genişlik", f.length(self.width)),
                Setting::text("H", "Sahanlık", landing),
                Setting::text("L", "En uzun kol", self.max_run.map_or_else(|| "Yok".into(), |v| f.length(v))),
                Setting::text("Z", "Başlangıç kotu", self.base_words(f)),
                railings_setting(self.railings),
                Setting::choice(
                    "U",
                    "Son kol",
                    crate::tool_settings::choices(
                        &[("fit", "Eğime göre biter"), ("clicked", "Tıklanan yerde biter")],
                        if self.fit { "fit" } else { "clicked" },
                    ),
                ),
            ];
            let words = self.design_words(f);
            if !words.is_empty() {
                rows.push(Setting::hint("T", "Tasarım", words.join("; ")));
            }
            rows.push(Setting::hint(
                "P",
                "Profil",
                "Eğim KentOS'un öntanımlı değeridir; erişilebilirlik yönetmeliği denetimi değildir.",
            ));
            return rows;
        }
        let mut rows = vec![
            Setting::choice(
                "B",
                "Biçim",
                crate::tool_settings::choices(
                    &[("straight", "Düz, L ya da U"), ("curved", "Döner")],
                    if self.curved { "curved" } else { "straight" },
                ),
            ),
            Setting::text("Y", "Yükseklik", self.height_words(f)),
            Setting::text("R", "En fazla rıht", f.length(self.max_riser)),
            Setting::text(
                "S",
                "Rıht sayısı",
                if self.risers == 0 { "Hesaplanır".into() } else { self.risers.to_string() },
            ),
            Setting::text("D", "Basamak", f.length(self.tread)),
            Setting::text("G", "Genişlik", f.length(self.width)),
            Setting::text("H", "Sahanlık", landing),
            Setting::text("K", "Kesme yüksekliği", self.cut.map_or_else(|| "Yok".into(), |v| f.length(v))),
            Setting::toggle("N", "Basamak numaraları", self.numbering),
            railings_setting(self.railings),
            Setting::choice(
                "U",
                if self.curved { "Yay" } else { "Son kol" },
                crate::tool_settings::choices(
                    &[("fit", "Rıhtlara göre biter"), ("clicked", "Tıklanan yerde biter")],
                    if self.fit { "fit" } else { "clicked" },
                ),
            ),
        ];
        if self.curved {
            // A curved stair has no turns, so no landings (docs/adr/0709).
            rows.retain(|r| r.key != "H");
        }
        let words = self.design_words(f);
        if !words.is_empty() {
            rows.push(Setting::hint("T", "Tasarım", words.join("; ")));
        }
        rows.push(Setting::hint(
            "P",
            "Profil",
            "En fazla rıht, basamak ve kesme KentOS'un öntanımlı değerleridir; yönetmelik denetimi değildir.",
        ));
        rows
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.settings(&Format::default())
            .into_iter()
            .find(|s| s.key == key)
            .map_or_else(Vec::new, |s| s.choices)
    }

    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if self.apply(key, value, cx) {
            self.ask = None;
            self.see(cx);
            return true;
        }
        cx.say(
            Level::Warn,
            "Değer uygulanmadı: uzunluklar sıfırdan büyük; Rıht sayısı 0 (hesaplanır) ile 400 arası tam sayı; Kesme 0 ya da Yok ile kapanır.",
        );
        false
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
            self.turned = 0.0;
            self.seen_at = None;
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see_level(cx);
        if self.spiralling() {
            if self.pts.len() == 2 {
                self.write_arc(cx);
                self.see(cx);
                return Flow::Stay;
            }
            return Flow::Exit;
        }
        if self.pts.len() >= 2 {
            let pts = std::mem::take(&mut self.pts);
            self.run(&pts, None, cx, true);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        out.path = if self.spiralling() {
            // The radius: the centre to the start, or to the pointer before the start.
            match (self.pts.first(), self.pts.get(1).copied().or(self.hover)) {
                (Some(&c), Some(a)) => vec![c, a],
                _ => Vec::new(),
            }
        } else {
            self.line(true)
        };
        out.tag = self.hover.map(|at| {
            let mut lines = vec![self.name().to_owned()];
            if let Some(why) = &self.why {
                lines.push(why.clone());
            } else {
                lines.extend(self.design_words(format));
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
