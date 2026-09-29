//! Parçala (docs/adr/0140): line work cut into separate objects, three ways.
//! The ribbon starts a method by sending its letter right after the tool, and
//! the same letters work typed on the command line: `E`, `U`, and `K` back.
//!
//! - **Kesişimlerden** (the default): objects selected before or after (the
//!   desktop's selection-first base, [`crate::modify`]) come apart where they
//!   cross one another; the pieces are drawn in turn, a ring at each cut.
//!   Enter, the button or a quick right click write.
//! - **Eşit parçalara** (`E`): click a line, an open polyline, an arc or a
//!   circle, then type the count (2 to 10 000); the pieces are shown at the
//!   count kept from last time, a typed count cuts at once, Enter cuts with
//!   the kept one.
//! - **Uzunluktan** (`U`): click near the end to measure from, then type the
//!   piece length in metres; the last piece is what remains. A circle is
//!   measured from its east point, counter-clockwise.
//!
//! The count and the length stay for as long as the app lives
//! ([`crate::tool::Memory`]). The first piece keeps the object's place and
//! persistent id, every piece its layer, colour, weight, attributes and
//! label; the rest are new objects from it. One undo step, “Parçala”,
//! through `cad.entities.edit`; the objects on a locked layer are refused.
//! The pieces are the shared core's (`split_at_crossings`, `split_equal`,
//! `split_by_length`).

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::path::{nearest_s, path_of, point_at_s};
use kentos_geometry_core::ops::split::{split_at_crossings, split_by_length, split_equal};
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::modify::{MAX_GHOSTS, Modify, Stages};
use crate::object::ObjectAction;
use crate::points::plain_number;
use crate::prompt::{Prompt, upper_tr};
use crate::select::SelectBox;
use crate::tool::{
    Context, Cursor, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Tag, Tone, Tool,
};

/// Parçala's id: its command is `tool.split`.
pub const ID: &str = "split";
pub const LABEL: &str = "Parçala";

/// Most pieces one object may come apart into.
const MOST_PIECES: f64 = 10_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Crossings,
    Equal,
    Length,
}

/// The kinds the two click methods take, off locked layers: what has a length
/// to cut. Areas are exploded first; a hatch or a text has none.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Arc(_) | Entity::Circle(_)
    ) && edge::unlocked(e, doc)
}

/// The object picked by a click method, and for Uzunluktan the end measured from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Target {
    slot: Slot,
    from_end: bool,
}

/// Parçala.
pub struct Split {
    mode: Mode,
    crossings: Modify<Crossings>,
    target: Option<Target>,
    /// The pointer's world point, for the tag.
    hover: Option<Vec2>,
    /// The pieces of the picked object, drawn; and what the tag says.
    drawn: Preview,
    lines: Vec<String>,
    /// What the session remembered and the project's units, as of the last event.
    seen: Option<(Memory, Format)>,
}

impl Split {
    pub fn new() -> Self {
        Self {
            mode: Mode::Crossings,
            crossings: Modify::with(Crossings::default()),
            target: None,
            hover: None,
            drawn: Preview::default(),
            lines: Vec::new(),
            seen: None,
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    fn format(&self) -> Format {
        self.seen.map(|(_, f)| f).unwrap_or_default()
    }

    /// A method chosen by its letter: the tool starts over in it.
    fn switch(&mut self, mode: Mode, cx: &mut Context<'_>) {
        if mode == self.mode {
            return;
        }
        self.target = None;
        self.drawn = Preview::default();
        self.lines.clear();
        cx.selection.set_hover(None);
        self.mode = mode;
        if mode == Mode::Crossings {
            self.crossings = Modify::with(Crossings::default());
            let _ = self.crossings.activate(cx);
        }
        self.see(cx);
    }

    /// The pieces the picked object comes apart into, or why it does not.
    fn pieces(&self, t: Target, cx: &Context<'_>) -> Result<Vec<Shape>, String> {
        let Some(e) = cx.doc.get(t.slot) else {
            return Err(String::new());
        };
        let core = edge::core(e);
        let m = *cx.memory;
        let f = cx.format();
        let pieces = match self.mode {
            Mode::Equal => split_equal(&core, f64::from(m.split_parts)),
            _ => split_by_length(&core, m.split_length, t.from_end),
        };
        pieces
            .map(|p| p.into_iter().map(|e| e.shape).collect())
            .ok_or_else(|| match (self.mode, path_of(&core.shape)) {
                (Mode::Equal, _) => format!(
                    "Bu nesne {} eşit parçaya bölünemez; parça sayısı 2 ile 10 000 arasında olmalı.",
                    m.split_parts
                ),
                (_, Some(path)) if path.length / m.split_length > MOST_PIECES => format!(
                    "{} uzunlukla en çok 10 000 parça olur; nesne {} boyunda. Uzunluğu artırın.",
                    f.length(m.split_length),
                    f.length(path.length)
                ),
                (_, Some(path)) if m.split_length >= path.length => format!(
                    "Uzunluk ({}) nesnenin boyundan ({}) kısa olmalı; nesne bölünmedi.",
                    f.length(m.split_length),
                    f.length(path.length)
                ),
                _ => "Bu nesne verilen uzunlukta bölünemez.".to_owned(),
            })
    }

    /// The picked object's pieces drawn in turn, from the geometry now; for
    /// Uzunluktan, where the measuring starts is ringed.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.lines.clear();
        self.see(cx);
        let Some(t) = self.target else { return };
        let Some(e) = cx.doc.get(t.slot) else { return };
        match self.pieces(t, cx) {
            Ok(pieces) => {
                self.drawn = pieces_preview(&pieces);
                self.lines = vec![format!("{} parça", pieces.len())];
            }
            Err(_) => {
                let outline = Outline::of(&shape_of(e), Some([5.0, 3.0]), 1.5, Tone::Danger);
                self.drawn.strokes = outline.strokes;
                self.drawn.marks = outline.marks;
                self.lines = vec!["Bölünemez".to_owned()];
            }
        }
        if self.mode == Mode::Length
            && let Some(path) = path_of(&shape_of(e)).filter(|p| !p.closed)
        {
            self.drawn.markers.push(Marker {
                at: point_at_s(&path, if t.from_end { path.length } else { 0.0 }),
                shape: MarkerShape::Ring(7.0),
                tone: Tone::Snap,
            });
            self.lines.push(format!(
                "{} uzunlukla",
                cx.format().length(cx.memory.split_length)
            ));
        }
    }

    /// Writes the picked object's pieces (one undo step) and picks again.
    fn cut(&mut self, cx: &mut Context<'_>) {
        let Some(t) = self.target else { return };
        let pieces = match self.pieces(t, cx) {
            Ok(pieces) => pieces,
            Err(error) => {
                if !error.is_empty() {
                    cx.say(Level::Warn, error);
                }
                return;
            }
        };
        let n = pieces.len();
        if write_pieces(&[(t.slot, pieces)], cx).is_none() {
            return;
        }
        let f = cx.format();
        let text = match self.mode {
            Mode::Equal => format!("Nesne {n} eşit parçaya bölündü."),
            _ => format!(
                "Nesne {n} parçaya bölündü: {} uzunluğunda parçalar, kalan son parçada.",
                f.length(cx.memory.split_length)
            ),
        };
        cx.say(Level::Success, text);
        self.target = None;
        self.drawn = Preview::default();
        self.lines.clear();
        self.see(cx);
    }

    /// A click on an object in Eşit parçalara or Uzunluktan.
    fn pick(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick(p, cx, editable) else {
            cx.say(
                Level::Warn,
                "Bölünecek bir çizgi, açık çoklu çizgi, yay ya da daireye tıklayın; alanları önce Patlat ile çizgilere ayırın.",
            );
            return;
        };
        // The end nearer the click is where the measuring starts.
        let from_end = cx
            .doc
            .get(slot)
            .and_then(|e| {
                let path = path_of(&shape_of(e))?;
                Some(!path.closed && nearest_s(&path, p.raw) > path.length / 2.0)
            })
            .unwrap_or(false);
        self.target = Some(Target { slot, from_end });
        cx.selection.set_hover(None);
        self.redraw(cx);
    }

    /// A number in a click method: the count or the length, cutting at once
    /// when an object is picked.
    fn typed_value(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(n) = plain_number(text) else {
            return false;
        };
        if self.mode == Mode::Equal {
            if n.fract() != 0.0 || !(2.0..=MOST_PIECES).contains(&n) {
                cx.say(
                    Level::Warn,
                    "Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.",
                );
                return true;
            }
            cx.memory.split_parts = n as u32;
        } else {
            if n <= 0.0 {
                cx.say(Level::Warn, "Uzunluk sıfırdan büyük olmalı.");
                return true;
            }
            cx.memory.split_length = n;
        }
        self.see(cx);
        if self.target.is_some() {
            self.cut(cx);
        }
        true
    }

    /// The methods other than the running one, as options.
    fn options(&self, prompt: Prompt) -> Prompt {
        let prompt = prompt.then();
        match self.mode {
            Mode::Crossings => prompt
                .option("Eşit parçalara", "E")
                .option("Uzunluktan", "U"),
            Mode::Equal => prompt
                .option("Kesişimlerden", "K")
                .option("Uzunluktan", "U"),
            Mode::Length => prompt
                .option("Kesişimlerden", "K")
                .option("Eşit parçalara", "E"),
        }
    }
}

impl Default for Split {
    fn default() -> Self {
        Self::new()
    }
}

/// An object's geometry as the core takes it.
fn shape_of(e: &Entity) -> Shape {
    kentos_native_application::geometry::shape(e)
}

/// The pieces drawn in turn, a ring at each cut.
fn pieces_preview(pieces: &[Shape]) -> Preview {
    let mut out = Outline::default();
    let mut markers = Vec::new();
    for (i, piece) in pieces.iter().enumerate() {
        let tone = if i % 2 == 0 { Tone::Accent } else { Tone::Snap };
        out = out.and(Outline::of(piece, None, 3.0, tone));
        if i > 0
            && let Some(at) = start_of(piece)
        {
            markers.push(Marker {
                at,
                shape: MarkerShape::Ring(4.0),
                tone: Tone::Accent,
            });
        }
    }
    Preview {
        strokes: out.strokes,
        marks: out.marks,
        markers,
        ..Preview::default()
    }
}

/// Where a piece begins.
fn start_of(s: &Shape) -> Option<Vec2> {
    match s {
        Shape::Line { a, .. } => Some(*a),
        Shape::Polyline { pts, .. } => pts.first().copied(),
        Shape::Arc { c, r, a0, .. } => Some(Vec2::new(c.x + r * a0.cos(), c.y + r * a0.sin())),
        _ => None,
    }
}

/// Writes objects come apart into pieces as one edit, “Parçala”: the first
/// piece takes the object's place and id, the others are new objects from
/// it, every one with its data. The objects and pieces written, or `None`
/// (the command's refusal said). The pieces become the selection when
/// `select` is asked by the caller through the returned slots.
fn write_pieces(parts: &[(Slot, Vec<Shape>)], cx: &mut Context<'_>) -> Option<(usize, Vec<Slot>)> {
    let mut changes = Vec::new();
    let mut objects = 0;
    for (slot, pieces) in parts {
        let uid = edge::uid(cx.doc, *slot);
        let mut geometries = pieces.iter().filter_map(edge::geometry);
        let Some(first) = geometries.next() else {
            continue;
        };
        objects += 1;
        changes.push(EntityEdit::Replace {
            uid: uid.clone(),
            geometry: first,
            keep_data: Some(true),
        });
        for geometry in geometries {
            changes.push(EntityEdit::add(uid.clone(), geometry, Some(true)));
        }
    }
    if changes.is_empty() {
        return None;
    }
    let out = edge::write(EditOperation::Split, changes, cx)?;
    let slots: Vec<Slot> = out
        .changed
        .iter()
        .chain(&out.created)
        .filter_map(|uid| kentos_domain::Uuid::parse_str(uid).ok())
        .filter_map(|uid| cx.doc.slot_of(uid))
        .collect();
    let doc = &*cx.doc;
    cx.selection.retain(|s| doc.get(s).is_some());
    cx.selection.set_hover(None);
    Some((objects, slots))
}

impl Tool for Split {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn cursor(&self) -> Cursor {
        match self.mode {
            Mode::Crossings => self.crossings.cursor(),
            _ => Cursor::Pick,
        }
    }

    fn prompt(&self) -> Prompt {
        let m = self.memory();
        let f = self.format();
        let prompt = match self.mode {
            Mode::Crossings => return self.options(self.crossings.prompt()),
            Mode::Equal => match self.target {
                None => Prompt::new(LABEL, "bölünecek nesneye tıklayın")
                    .note(format!("parça sayısı {}", m.split_parts)),
                Some(_) => Prompt::new(
                    LABEL,
                    format!("parça sayısını yazın (Enter: {} parça)", m.split_parts),
                ),
            },
            Mode::Length => match self.target {
                None => Prompt::new(
                    LABEL,
                    "bölünecek nesneye, ölçümün başlayacağı uca yakın tıklayın",
                )
                .note(format!("uzunluk {}", f.length(m.split_length))),
                Some(_) => Prompt::new(
                    LABEL,
                    format!(
                        "parça uzunluğunu yazın (Enter: {})",
                        f.length(m.split_length)
                    ),
                ),
            },
        };
        self.options(prompt)
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Kesişimlerden is the default and waits for its letter to be changed:
    /// it never leaves at once, so the ribbon's method reaches the tool.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        self.crossings.activate(cx)
    }

    fn snaps(&self) -> bool {
        self.mode == Mode::Crossings && self.crossings.snaps()
    }

    fn snap_from(&self) -> Option<Vec2> {
        match self.mode {
            Mode::Crossings => self.crossings.snap_from(),
            _ => None,
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.mode == Mode::Crossings {
            return self.crossings.pointer_move(p, cx);
        }
        self.see(cx);
        self.hover = Some(p.raw);
        if self.target.is_none() {
            edge::hover(p, cx, editable);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.mode == Mode::Crossings {
            return self.crossings.pointer_down(p, cx);
        }
        self.pick(p, cx);
    }

    fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.mode == Mode::Crossings {
            self.crossings.pointer_up(p, cx);
        }
    }

    /// The method letters, then what the running method reads: the count or the length.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match upper_tr(js_trim(text)).as_str() {
            "E" => {
                self.switch(Mode::Equal, cx);
                return true;
            }
            "U" => {
                self.switch(Mode::Length, cx);
                return true;
            }
            "K" => {
                self.switch(Mode::Crossings, cx);
                return true;
            }
            _ => {}
        }
        match self.mode {
            Mode::Crossings => self.crossings.input(text, cx),
            _ => self.typed_value(text, cx),
        }
    }

    /// Kesişimlerden: the selection goes on, then writes. The click methods
    /// cut the picked object with the kept value; with none picked, leave.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.mode == Mode::Crossings {
            return self.crossings.confirm(cx);
        }
        self.see(cx);
        if self.target.is_some() {
            self.cut(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.mode == Mode::Crossings {
            return self.crossings.cancel(cx);
        }
        if self.target.is_none() {
            return false;
        }
        self.target = None;
        self.drawn = Preview::default();
        self.lines.clear();
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.mode == Mode::Crossings && self.crossings.finished()
    }

    fn select_box(&self) -> Option<SelectBox> {
        match self.mode {
            Mode::Crossings => self.crossings.select_box(),
            _ => None,
        }
    }

    fn accepts_points(&self) -> bool {
        self.mode == Mode::Crossings && self.crossings.accepts_points()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.mode == Mode::Crossings && self.crossings.accept_point(p, cx)
    }

    fn preview(&self, format: &Format) -> Preview {
        if self.mode == Mode::Crossings {
            return self.crossings.preview(format);
        }
        let mut preview = self.drawn.clone();
        preview.tag = self.hover.filter(|_| !self.lines.is_empty()).map(|at| Tag {
            at,
            lines: self.lines.clone(),
        });
        preview
    }
}

/// Kesişimlerden's part after the selection.
#[derive(Clone, Debug, Default)]
pub struct Crossings {
    /// The selected objects off locked layers, in the order they were listed.
    targets: Vec<Slot>,
    /// Each object that comes apart and its pieces, from the drawing as of `generation`.
    parts: Vec<(Slot, Vec<Shape>)>,
    generation: Option<u64>,
    drawn: Preview,
}

impl Crossings {
    /// The pieces the selected objects come apart into where they cross, kept
    /// until the drawing changes.
    fn refresh(&mut self, cx: &Context<'_>) {
        let generation = cx.doc.generation();
        if self.generation == Some(generation) {
            return;
        }
        self.generation = Some(generation);
        let listed: Vec<(Slot, kentos_geometry_core::entity::Entity)> = self
            .targets
            .iter()
            .filter_map(|slot| Some((*slot, edge::core(cx.doc.get(*slot)?))))
            .collect();
        let entities: Vec<_> = listed.iter().map(|(_, e)| e.clone()).collect();
        self.parts = split_at_crossings(&entities)
            .into_iter()
            .map(|p| {
                (
                    listed[p.index].0,
                    p.pieces.into_iter().map(|e| e.shape).collect(),
                )
            })
            .collect();
        let mut out = Preview::default();
        for (_, pieces) in self.parts.iter().take(MAX_GHOSTS) {
            let one = pieces_preview(pieces);
            out.strokes.extend(one.strokes);
            out.marks.extend(one.marks);
            out.markers.extend(one.markers);
        }
        self.drawn = out;
    }

    fn pieces_total(&self) -> usize {
        self.parts.iter().map(|(_, p)| p.len()).sum()
    }
}

impl Stages for Crossings {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    /// The selection is confirmed: what it holds off locked layers is looked
    /// at. It never leaves here: nothing to cut is said on the confirm.
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.targets = ObjectAction::targets(cx);
        self.generation = None;
        self.refresh(cx);
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        if !self.targets.is_empty() {
            self.refresh(cx);
        }
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        if self.parts.is_empty() {
            return Prompt::new(LABEL, "seçilen nesneler birbirini kesmiyor")
                .note("kesişen çizgileri seçin")
                .then();
        }
        Prompt::new(LABEL, "Enter ya da sağ tıkla parçalayın")
            .note(format!(
                "{} nesne {} parçaya bölünecek",
                self.parts.len(),
                self.pieces_total()
            ))
            .then()
            .option("Uygula", "Enter")
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, _text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }

    fn typed_points(&self) -> bool {
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.generation = None;
        self.refresh(cx);
        if self.parts.is_empty() {
            cx.say(
                Level::Warn,
                "Seçilen nesneler birbirini kesmiyor; bölünecek bir şey yok. Kesişen çizgileri seçin ya da Eşit parçalara (E), Uzunluktan (U) ile tek nesneyi bölün.",
            );
            return Flow::Exit;
        }
        let parts = std::mem::take(&mut self.parts);
        let Some((objects, slots)) = write_pieces(&parts, cx) else {
            self.generation = None;
            return Flow::Stay;
        };
        let pieces = slots.len();
        cx.selection.set(slots);
        cx.say(
            Level::Success,
            format!("{objects} nesne kesişim yerlerinden {pieces} parçaya bölündü."),
        );
        Flow::Exit
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }

    fn stage_preview(&self, hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        let mut preview = self.drawn.clone();
        preview.tag = hover.map(|at| Tag {
            at,
            lines: if self.parts.is_empty() {
                vec!["Kesişim yok".to_owned()]
            } else {
                vec![
                    format!("{} nesne", self.parts.len()),
                    format!("{} parça", self.pieces_total()),
                ]
            },
        });
        if self.parts.is_empty() {
            preview.tag_tone = Tone::Danger;
        }
        Some(preview)
    }
}
