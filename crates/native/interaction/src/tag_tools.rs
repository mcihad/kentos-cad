//! Etiket, Hepsini etiketle and Çizelge (docs/adr/0344 §5, plan/cad.md `CAD-ARCH-20`–`22`). The tools say what was
//! clicked and chosen; `cad.tags.create` and `cad.schedules.create` compute and write, one undo step each, and their plan
//! is the preview.
//!
//! - Etiket: a click on an opening, a space, a column, a grid, a wall, a stair, a ramp or a block, then where its tag
//!   goes (Enter: beside it); Şablon (Ş), Biçim (B), Kol (K).
//! - Hepsini etiketle: Açıklık (A), Mahal (M), Kolon (K), Aks (S), Duvar (D) chosen, Enter tags every shown one of them
//!   without a tag.
//! - Çizelge: Tür (T), Grup (G), Toplam (O), Sırala (S); the schedule hangs from the cursor by its top left corner, a
//!   click places it.

use kentos_commands::{
    CommandResult, Entity, NewTag, ScheduleOf, ScheduleSort, SchedulesCreate, TagAll, TagShape,
    TaggedKind, TagsCreate,
};
use kentos_domain::Slot;
use kentos_geometry_core::geom::table::layout_of;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{drawing_font, shape};
use kentos_native_application::{ExecutionContext, schedules_create, tags_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Area, CellGhost, Context, Cursor, Flow, Pointer, Preview, Stroke, Tag, TextGhost, Tone, Tool,
};

pub const TAG_ID: &str = "tag";
pub const TAG_ALL_ID: &str = "tagAll";
pub const SCHEDULE_ID: &str = "scheduleOpenings";
pub const SCHEDULE_SPACES_ID: &str = "scheduleSpaces";

/// The object a tag can be put on under the cursor.
fn taggable_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
    cx.spatial
        .hits(at, cx.pick_tolerance())
        .into_iter()
        .find(|s| {
            cx.doc
                .get(*s)
                .is_some_and(|e| TaggedKind::of_kind(e.kind()).is_some())
        })
}

/// A tag drawn as the preview: its frame and leader as strokes, its words as ghost texts.
fn tag_drawn(e: &Entity, scale: f64, out: &mut Preview) {
    let core = shape(e);
    for piece in kentos_geometry_core::geom::arch::pieces(&core, scale) {
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
            other => out
                .strokes
                .extend(Outline::of(other, None, 1.5, Tone::Accent).strokes),
        }
    }
}

fn plan_words<P>(result: CommandResult<P>) -> Result<P, String> {
    match result {
        CommandResult::Completed { output, .. } => Ok(output),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        CommandResult::Queued { .. } | CommandResult::Cancelled => Err(String::new()),
    }
}

// ── Etiket ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// What the Etiket tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Content,
}

/// Etiket.
#[derive(Clone, Debug, Default)]
pub struct TagTool {
    target: Option<Slot>,
    content: Option<String>,
    shape: Option<TagShape>,
    leader: bool,
    ask: Option<Ask>,
    hover: Option<Vec2>,
    drawn: Preview,
    problem: Option<String>,
}

impl TagTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn made(&self, at: Option<Vec2>, cx: &Context<'_>) -> Option<TagsCreate> {
        let of = cx.doc.uid(self.target?)?.to_string();
        Some(TagsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            tags: vec![NewTag {
                of,
                at: at.map(points::wire),
                content: self.content.clone(),
                shape: self.shape,
                size: None,
                leader: self.leader.then_some(true),
            }],
            all: None,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.problem = None;
        let Some(input) = self.made(self.hover, cx) else {
            return;
        };
        match plan_words(tags_create::plan_in(cx.doc, &input)) {
            Ok(plan) => {
                for e in &plan.tags {
                    tag_drawn(e, cx.doc.settings().plot_scale, &mut self.drawn);
                }
            }
            Err(why) => self.problem = Some(why),
        }
    }

    fn write(&mut self, at: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(input) = self.made(at, cx) else {
            return;
        };
        let result = tags_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            cx.say(
                Level::Success,
                "Etiket yazıldı; nesnesi değişince etiketi de değişir.",
            );
        }
        self.target = None;
        self.drawn = Preview::default();
    }
}

impl Tool for TagTool {
    fn id(&self) -> &'static str {
        TAG_ID
    }

    fn label(&self) -> &'static str {
        "Etiket"
    }

    fn cursor(&self) -> Cursor {
        if self.target.is_none() {
            Cursor::Pick
        } else {
            Cursor::Cross
        }
    }

    fn snaps(&self) -> bool {
        self.target.is_some()
    }

    fn point_count(&self) -> usize {
        usize::from(self.target.is_some())
    }

    fn prompt(&self) -> Prompt {
        if self.ask == Some(Ask::Content) {
            return Prompt::new(
                "Etiket",
                "şablonu yazın ({number}, {type}, {name}, {area} … ; boş: türün şablonu)",
            )
            .option("Geri", "G");
        }
        let shape = self.shape.map_or("Türe göre", TagShape::label);
        let step = if self.target.is_none() {
            "etiketlenecek nesneye tıklayın (açıklık, mahal, kolon, aks, duvar, merdiven, rampa, blok)"
        } else {
            "etiketin yerine tıklayın, Enter ile nesnenin yanına"
        };
        Prompt::new("Etiket", step)
            .option_with(
                "Şablon",
                "Ş",
                self.content.clone().unwrap_or_else(|| "Türün".into()),
            )
            .option_with("Biçim", "B", shape.to_owned())
            .option_with(
                "Kol",
                "K",
                if self.leader { "Var" } else { "Yok" }.to_owned(),
            )
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        if self.target.is_none() {
            cx.selection.set_hover(taggable_at(p.raw, cx));
        } else {
            self.hover = Some(p.world);
            self.see(cx);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.target.is_none() {
            self.target = taggable_at(p.raw, cx);
            if self.target.is_none() {
                cx.say(Level::Info, "Burada etiketlenecek nesne yok.");
            }
            self.see(cx);
            return;
        }
        self.write(Some(p.world), cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if self.ask == Some(Ask::Content) {
            if key != "G" {
                let t = text.trim_end_matches(['\r', '\n']);
                self.content = (!js_trim(t).is_empty()).then(|| t.replace("\\n", "\n"));
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "Ş" | "S" => self.ask = Some(Ask::Content),
            "B" => {
                self.shape = match self.shape {
                    None => Some(TagShape::Circle),
                    Some(TagShape::None) => None,
                    Some(s) => TagShape::ALL
                        .iter()
                        .skip_while(|x| **x != s)
                        .nth(1)
                        .copied(),
                };
                self.see(cx);
            }
            "K" => {
                self.leader = !self.leader;
                self.see(cx);
            }
            _ => {
                if self.target.is_some()
                    && let Some(p) = cx.typed_point(text, None, self.hover)
                {
                    self.write(Some(p), cx);
                    return true;
                }
                return false;
            }
        }
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.is_some() {
            self.ask = None;
            return true;
        }
        if self.target.take().is_some() {
            self.drawn = Preview::default();
            self.see(cx);
            return true;
        }
        false
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.target.is_none() {
            return Flow::Exit;
        }
        self.write(None, cx);
        Flow::Stay
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        if let (Some(why), Some(at)) = (&self.problem, self.hover)
            && !why.is_empty()
        {
            out.tag = Some(Tag {
                at,
                lines: vec![why.clone()],
            });
            out.tag_tone = Tone::Danger;
        }
        out
    }
}

// ── Hepsini etiketle ───────────────────────────────────────────────────────────────────────────────────────────

/// Hepsini etiketle.
#[derive(Clone, Debug)]
pub struct TagAllTool {
    kinds: Vec<TaggedKind>,
}

impl Default for TagAllTool {
    fn default() -> Self {
        Self {
            kinds: vec![TaggedKind::Opening, TaggedKind::Space],
        }
    }
}

impl TagAllTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn flip(&mut self, k: TaggedKind) {
        if let Some(i) = self.kinds.iter().position(|x| *x == k) {
            self.kinds.remove(i);
        } else {
            self.kinds.push(k);
            self.kinds.sort();
        }
    }
}

const ALL_KEYS: [(&str, TaggedKind); 5] = [
    ("A", TaggedKind::Opening),
    ("M", TaggedKind::Space),
    ("K", TaggedKind::Column),
    ("S", TaggedKind::Grid),
    ("D", TaggedKind::Wall),
];

impl Tool for TagAllTool {
    fn id(&self) -> &'static str {
        TAG_ALL_ID
    }

    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn pointer_down(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }

    fn label(&self) -> &'static str {
        "Hepsini etiketle"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        let mut p = Prompt::new(
            "Hepsini etiketle",
            "türleri seçin, Enter ile etiketsiz ve görünenleri etiketleyin",
        );
        for (key, k) in ALL_KEYS {
            let on = self.kinds.contains(&k);
            p = p.option_with(
                k.label(),
                key,
                if on { "Açık" } else { "Kapalı" }.to_owned(),
            );
        }
        p.option("Etiketle", "Enter")
    }

    fn input(&mut self, text: &str, _cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        match ALL_KEYS.iter().find(|(k, _)| *k == key) {
            Some((_, k)) => {
                self.flip(*k);
                true
            }
            None => false,
        }
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.kinds.is_empty() {
            cx.say(Level::Info, "Etiketlenecek tür seçilmedi.");
            return Flow::Stay;
        }
        let input = TagsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            tags: Vec::new(),
            all: Some(TagAll {
                kinds: self.kinds.clone(),
                layers: None,
                content: None,
                shape: None,
                size: None,
                phases: None,
            }),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        };
        let result = tags_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            cx.say(
                Level::Success,
                format!("{} etiket yazıldı.", out.uids.len()),
            );
        }
        Flow::Exit
    }
}

// ── Çizelge ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Çizelge: a schedule placed from the cursor.
#[derive(Clone, Debug)]
pub struct ScheduleTool {
    /// What it was started for (its id).
    start: ScheduleOf,
    of: ScheduleOf,
    group: bool,
    total: bool,
    sorted: bool,
    /// The plan's table at the origin, made again when an option changes.
    table: Option<kentos_commands::TableEntity>,
    problem: Option<String>,
    hover: Option<Vec2>,
    ghost: Preview,
    /// Placed: the tool leaves, as Tablo ekle's placement does.
    done: bool,
}

impl ScheduleTool {
    pub fn new(of: ScheduleOf) -> Self {
        Self {
            start: of,
            of,
            group: false,
            total: of != ScheduleOf::Openings,
            sorted: true,
            table: None,
            problem: None,
            hover: None,
            ghost: Preview::default(),
            done: false,
        }
    }

    fn input_at(&self, at: Vec2, cx: &Context<'_>) -> SchedulesCreate {
        SchedulesCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            at: points::wire(at),
            of: self.of,
            columns: self.group.then(|| grouped_columns(self.of)),
            filter: None,
            sort: if self.sorted {
                vec![ScheduleSort {
                    column: 0,
                    descending: false,
                }]
            } else {
                Vec::new()
            },
            group: self.group.then_some(true),
            total: self.total.then_some(true),
            height: None,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    fn plan(&mut self, cx: &Context<'_>) {
        self.table = None;
        self.problem = None;
        match plan_words(schedules_create::plan_in(
            cx.doc,
            &self.input_at(Vec2::new(0.0, 0.0), cx),
        )) {
            Ok(p) => {
                if let Entity::Table(t) = p.table {
                    self.table = Some(t);
                }
            }
            Err(why) => self.problem = Some(why),
        }
    }

    fn place_ghost(&mut self, cx: &Context<'_>) {
        self.ghost = Preview::default();
        let (Some(h), Some(t)) = (self.hover, self.table.as_ref()) else {
            return;
        };
        let mut t = t.clone();
        t.p = points::wire(h);
        let entity = Entity::Table(t.clone());
        let Some(layout) = layout_of(
            &shape(&entity),
            drawing_font(cx.doc.settings().drawing_font),
        ) else {
            return;
        };
        self.ghost.areas = layout
            .frame
            .iter()
            .map(|strip| Area {
                rings: vec![strip.to_vec()],
                fill: 0.35,
                width: 1.0,
                dash: None,
                fill_tone: Tone::Accent,
            })
            .collect();
        self.ghost.strokes = layout
            .lines
            .iter()
            .map(|[a, b]| Stroke::dashed(vec![*a, *b], false, [4.0, 3.0]))
            .collect();
        self.ghost.cells = layout
            .cells
            .iter()
            .filter_map(|c| {
                let words = t.cells.get(c.row)?.get(c.col)?;
                (!words.is_empty()).then(|| CellGhost {
                    at: c.at,
                    text: words.clone(),
                    height: t.height,
                    bold: c.bold,
                    face: t.face.clone(),
                })
            })
            .collect();
    }

    fn write(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let input = self.input_at(at, cx);
        let result = schedules_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            self.done = true;
            self.ghost = Preview::default();
            cx.selection.set([Slot(out.id)]);
            cx.say(
                Level::Success,
                format!(
                    "{}: {} satır. Kaynakları değişince satırları da değişir.",
                    self.of.label(),
                    out.rows
                ),
            );
        }
    }
}

const KINDS_OF: [ScheduleOf; 4] = ScheduleOf::ALL;

/// A grouped schedule's columns: the fields the objects share, then how many and how much.
fn grouped_columns(of: ScheduleOf) -> Vec<kentos_commands::ScheduleColumn> {
    use kentos_commands::ScheduleField::*;
    let fields: &[kentos_commands::ScheduleField] = match of {
        ScheduleOf::Openings => &[Type, Form, Width, Height, Count],
        ScheduleOf::Spaces => &[Usage, Count, Area],
        ScheduleOf::Columns => &[Type, Width, Count],
        ScheduleOf::Walls => &[Type, Thickness, Count, Length, Area],
    };
    fields
        .iter()
        .map(|&field| kentos_commands::ScheduleColumn {
            field,
            title: None,
            attr: None,
        })
        .collect()
}

impl Tool for ScheduleTool {
    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn id(&self) -> &'static str {
        if self.start == ScheduleOf::Spaces {
            SCHEDULE_SPACES_ID
        } else {
            SCHEDULE_ID
        }
    }

    fn label(&self) -> &'static str {
        self.of.label()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.plan(cx);
        Flow::Stay
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.write(p, cx);
        true
    }

    fn prompt(&self) -> Prompt {
        let on = |b: bool| if b { "Açık" } else { "Kapalı" }.to_owned();
        Prompt::new(self.of.label(), "çizelgenin sol üst köşesine tıklayın")
            .option_with("Tür", "T", self.of.label().to_owned())
            .option_with("Grup", "G", on(self.group))
            .option_with("Toplam", "O", on(self.total))
            .option_with("Sırala", "S", on(self.sorted))
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.place_ghost(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.write(p.world, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match upper_tr(js_trim(text)).as_str() {
            "T" => {
                let i = KINDS_OF.iter().position(|k| *k == self.of).unwrap_or(0);
                self.of = KINDS_OF[(i + 1) % KINDS_OF.len()];
            }
            "G" => {
                self.group = !self.group;
                // Alike rows counted: a total says how many in all.
                self.total = self.total || self.group;
            }
            "O" => self.total = !self.total,
            "S" => self.sorted = !self.sorted,
            _ => match cx.typed_point(text, None, self.hover) {
                Some(p) => {
                    self.write(p, cx);
                    return true;
                }
                None => return false,
            },
        }
        self.plan(cx);
        self.place_ghost(cx);
        true
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.ghost.clone();
        if let (Some(why), Some(at)) = (&self.problem, self.hover)
            && !why.is_empty()
        {
            out.tag = Some(Tag {
                at,
                lines: vec![why.clone()],
            });
            out.tag_tone = Tone::Danger;
        }
        out
    }
}
