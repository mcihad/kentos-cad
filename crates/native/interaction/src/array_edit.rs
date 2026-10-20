//! Diziyi düzenle (AutoCAD's ARRAYEDIT, docs/adr/0301): click an item of an associative array (docs/adr/0300); its
//! items are drawn dashed and the prompt offers the array's own properties:
//!
//! - a grid: Satır (S), Sütun (T), Sütun aralığı (A), Satır aralığı (R);
//! - a line: Adet (D); a polar array: Adet (D), Açı (Ç, degrees); a path array: Adet (D), Aralık (A);
//! - Taban (B): the array's first place follows the mouse until a click (the plan is drawn);
//! - Değiştir (G) and a block's name: the clicked item inserts that block instead; Sil (L): the clicked item is
//!   removed; Sıfırla (F): every item as the layout makes it.
//!
//! A number typed after an option is written at once through `cad.arrays.edit`, one undo step “Diziyi düzenle”;
//! the array stays picked for the next change. Enter or Esc drops the option asked for, then the array; with
//! neither, Enter leaves.

use kentos_commands::{
    ArrayEdit as Edit, ArrayId, ArrayLayout, ArraysEdit, AssocArray, CommandResult, Entity,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::{ExecutionContext, arrays_edit};

use crate::Vec2;
use crate::edge::{self, Hover};
use crate::format::Format;
use crate::log::Level;
use crate::modify::ghosts;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tool};

/// The tool's id: its command is `tool.arrayEdit`.
pub const ID: &str = "arrayEdit";
pub const LABEL: &str = "Diziyi düzenle";

/// The preview draws the items' outlines only up to this many.
const MAX_SHOWN: usize = 400;

/// What a typed value or a click is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Rows,
    Cols,
    ColumnGap,
    RowGap,
    Count,
    Spacing,
    Fill,
    Block,
    Base,
}

/// An insert that is an item of an associative array, off locked layers.
fn item(e: &Entity, doc: &Document) -> bool {
    matches!(e, Entity::Insert(_))
        && doc
            .uid(Slot(e.base().id))
            .is_some_and(|u| doc.array_of_item(u).is_some())
        && edge::unlocked(e, doc)
}

/// The item under the pointer: the most specific of the click's candidates that is an array's item the selection
/// filter takes (docs/adr/0278); an insert is hit by its block's pieces, not by an edge of its own.
fn item_at(p: &Pointer, cx: &Context<'_>) -> Option<Slot> {
    let kinds = cx.draft.select_kinds;
    cx.spatial
        .hits(p.raw, cx.pick_tolerance())
        .into_iter()
        .find(|s| {
            cx.doc
                .get(*s)
                .is_some_and(|e| crate::selectable::entity_allowed(kinds, e) && item(e, cx.doc))
        })
}

/// The layout's kind in the prompt's note.
fn kind_text(layout: &ArrayLayout) -> String {
    match *layout {
        ArrayLayout::Grid { rows, cols, .. } => format!("ızgara {rows} × {cols}"),
        ArrayLayout::Line { count, .. } => format!("doğru, {count} öğe"),
        ArrayLayout::Polar { count, .. } => format!("kutupsal, {count} öğe"),
        ArrayLayout::Path { count, .. } => format!("yol boyunca, {count} öğe"),
    }
}

/// A count typed: a whole number of at least 1.
fn count_of(n: f64) -> Option<u32> {
    (n.fract() == 0.0 && (1.0..=10_000.0).contains(&n)).then_some(n as u32)
}

/// The diziyi düzenle tool.
#[derive(Clone, Debug, Default)]
pub struct ArrayEdit {
    /// The array picked and the number of the item clicked (none once it is removed).
    picked: Option<(ArrayId, Option<usize>)>,
    ask: Option<Ask>,
    mouse: Option<Vec2>,
    hover: Option<Hover>,
    drawn: Preview,
    /// What the prompt shows: the format and the picked array as the document has it.
    seen: Option<(Format, Option<AssocArray>)>,
}

impl ArrayEdit {
    pub fn new() -> Self {
        Self::default()
    }

    fn array<'d>(&self, doc: &'d Document) -> Option<&'d AssocArray> {
        let (id, _) = self.picked?;
        doc.array(id)
    }

    fn see(&mut self, cx: &Context<'_>) {
        let array = self.array(cx.doc).cloned();
        if self.picked.is_some() && array.is_none() {
            // Undone or removed elsewhere: back to picking.
            self.picked = None;
            self.ask = None;
        }
        self.seen = Some((cx.format(), array));
    }

    fn input_for(&self, edit: Edit) -> Option<ArraysEdit> {
        let (id, _) = self.picked?;
        Some(ArraysEdit {
            array: id.to_text(),
            edit,
            expected_revision: None,
        })
    }

    /// Writes `edit` through the command; the item clicked is let go when it is removed.
    fn apply(&mut self, edit: Edit, cx: &mut Context<'_>) {
        let Some(input) = self.input_for(edit) else {
            return;
        };
        let removing = matches!(input.edit, Edit::Remove { .. });
        let result = arrays_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            if removing && let Some((_, item)) = self.picked.as_mut() {
                *item = None;
            }
            let mut words = Vec::new();
            if !out.created.is_empty() {
                words.push(format!("{} öğe eklendi", out.created.len()));
            }
            if !out.changed.is_empty() {
                words.push(format!("{} öğe yerleşti", out.changed.len()));
            }
            if !out.removed.is_empty() {
                words.push(format!("{} öğe silindi", out.removed.len()));
            }
            let said = if words.is_empty() {
                "Dizi değişmedi.".to_owned()
            } else {
                format!("Dizi düzenlendi: {}.", words.join(", "))
            };
            cx.say(Level::Success, said);
        }
        self.ask = None;
        self.see(cx);
        self.redraw(cx);
    }

    /// The layout with `ask`'s field set from the typed number `n` (`length` the same in metres); none when the
    /// layout has no such field or the number does not fit it (said).
    fn layout_with(
        &self,
        ask: Ask,
        n: f64,
        length: f64,
        cx: &mut Context<'_>,
    ) -> Option<ArrayLayout> {
        let mut layout = self.array(cx.doc)?.layout.clone();
        let mut whole = || {
            let c = count_of(n);
            if c.is_none() {
                cx.say(Level::Warn, "1 ile 10 000 arasında bir tam sayı yazın.");
            }
            c
        };
        match (&mut layout, ask) {
            (ArrayLayout::Grid { rows, .. }, Ask::Rows) => *rows = whole()?,
            (ArrayLayout::Grid { cols, .. }, Ask::Cols) => *cols = whole()?,
            (ArrayLayout::Grid { dx, .. }, Ask::ColumnGap) => *dx = length,
            (ArrayLayout::Grid { dy, .. }, Ask::RowGap) => *dy = length,
            (
                ArrayLayout::Line { count, .. }
                | ArrayLayout::Polar { count, .. }
                | ArrayLayout::Path { count, .. },
                Ask::Count,
            ) => *count = whole()?,
            (ArrayLayout::Path { spacing, .. }, Ask::Spacing) => *spacing = Some(length),
            (ArrayLayout::Polar { fill, .. }, Ask::Fill) => *fill = n,
            _ => return None,
        }
        Some(layout)
    }

    /// The preview: the picked array's items dashed; with Taban, the plan of the first place at the mouse.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let Some(array) = self.array(cx.doc) else {
            return;
        };
        let mut inserts: Vec<Entity> = Vec::new();
        if self.ask == Some(Ask::Base)
            && let Some(mouse) = self.mouse
            && let Some(input) = self.input_for(Edit::Placement {
                p: Some(wire(mouse)),
                scale: None,
                rotation: None,
                mirror: None,
            })
            && let CommandResult::Completed { output, .. } = arrays_edit::plan_in(cx.doc, &input)
        {
            inserts.extend(output.changed);
        } else {
            for item in &array.items {
                if let Some(slot) = item.uid.and_then(|u| cx.doc.slot_of(Uuid::from_bytes(u.0)))
                    && let Some(e) = cx.doc.get(slot)
                {
                    inserts.push(e.clone());
                }
            }
        }
        for e in inserts.iter().take(MAX_SHOWN) {
            if let Entity::Insert(i) = e {
                let paths = cx.spatial.store().insert_outlines(
                    &i.block.to_text(),
                    Vec2::new(i.p.x, i.p.y),
                    i.scale,
                    i.rotation,
                    i.mirror,
                );
                let (strokes, marks) = ghosts(&paths);
                self.drawn.strokes.extend(strokes);
                self.drawn.marks.extend(marks);
            }
        }
    }
}

impl Tool for ArrayEdit {
    fn cursor(&self) -> Cursor {
        if self.ask == Some(Ask::Base) {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let (f, array) = self.seen.clone().unwrap_or_default();
        let Some(array) = array.filter(|_| self.picked.is_some()) else {
            return Prompt::new(LABEL, "düzenlenecek ilişkili dizinin bir öğesine tıklayın");
        };
        let item = self.picked.and_then(|(_, k)| k);
        match self.ask {
            Some(Ask::Rows) => return Prompt::new(LABEL, "satır sayısını yazın"),
            Some(Ask::Cols) => return Prompt::new(LABEL, "sütun sayısını yazın"),
            Some(Ask::ColumnGap) => return Prompt::new(LABEL, "sütunlar arasındaki aralığı yazın"),
            Some(Ask::RowGap) => return Prompt::new(LABEL, "satırlar arasındaki aralığı yazın"),
            Some(Ask::Count) => return Prompt::new(LABEL, "öğe sayısını yazın"),
            Some(Ask::Spacing) => return Prompt::new(LABEL, "öğeler arasındaki aralığı yazın"),
            Some(Ask::Fill) => {
                return Prompt::new(LABEL, "doldurma açısını derece olarak yazın (360 tam tur)");
            }
            Some(Ask::Block) => {
                return Prompt::new(LABEL, "öğenin yerleştireceği bloğun adını yazın");
            }
            Some(Ask::Base) => return Prompt::new(LABEL, "dizinin ilk yerine tıklayın"),
            None => {}
        }
        let mut prompt = Prompt::new(LABEL, "bir özellik seçin ya da Enter'la başka diziye geçin")
            .note(match item {
                Some(k) => format!("{}; öğe {k}", kind_text(&array.layout)),
                None => kind_text(&array.layout),
            })
            .then();
        prompt = match array.layout {
            ArrayLayout::Grid { rows, cols, dx, dy } => prompt
                .option_with("Satır", "S", rows.to_string())
                .option_with("Sütun", "T", cols.to_string())
                .option_with("Sütun aralığı", "A", f.length(dx))
                .option_with("Satır aralığı", "R", f.length(dy)),
            ArrayLayout::Line { count, .. } => prompt.option_with("Adet", "D", count.to_string()),
            ArrayLayout::Polar { count, fill, .. } => prompt
                .option_with("Adet", "D", count.to_string())
                .option_with("Açı", "Ç", format!("{fill}°")),
            ArrayLayout::Path { count, spacing, .. } => prompt
                .option_with("Adet", "D", count.to_string())
                .option_with(
                    "Aralık",
                    "A",
                    spacing.map_or_else(|| "eşit".to_owned(), |s| f.length(s)),
                ),
        };
        prompt = prompt.option("Taban", "B");
        if item.is_some() {
            prompt = prompt.option("Değiştir", "G").option("Sil", "L");
        }
        prompt.option("Sıfırla", "F")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        self.ask == Some(Ask::Base)
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.mouse = Some(p.world);
        if self.picked.is_none() {
            let hit = item_at(p, cx);
            cx.selection.set_hover(hit);
            self.hover = hit.map(|slot| Hover { slot, at: p.raw });
        }
        self.redraw(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.ask == Some(Ask::Base) {
            let to = wire(p.world);
            return self.apply(
                Edit::Placement {
                    p: Some(to),
                    scale: None,
                    rotation: None,
                    mirror: None,
                },
                cx,
            );
        }
        let Some(slot) = item_at(p, cx) else {
            if self.picked.is_none() {
                cx.say(
                    Level::Warn,
                    "İlişkili bir dizinin kilitsiz katmandaki öğesine tıklayın.",
                );
            }
            return;
        };
        let Some((id, k)) = cx
            .doc
            .uid(slot)
            .and_then(|u| cx.doc.array_of_item(u))
            .map(|(a, k)| (a.id, k))
        else {
            return;
        };
        self.picked = Some((id, Some(k)));
        self.ask = None;
        cx.selection.set_hover(None);
        self.see(cx);
        self.redraw(cx);
    }

    /// An option's letter asks for its value (or does it at once: Sil, Sıfırla); then a number or a block's name.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if self.picked.is_none() {
            return false;
        }
        let item = self.picked.and_then(|(_, k)| k);
        if self.ask == Some(Ask::Block) {
            let name = js_trim(text).to_owned();
            if name.is_empty() {
                return false;
            }
            if let Some(k) = item {
                self.apply(
                    Edit::Replace {
                        items: vec![k as u32],
                        block: name,
                    },
                    cx,
                );
            }
            return true;
        }
        if let Some(ask) = self.ask {
            let Some((n, length)) = points::plain_value(text, cx)
                .or_else(|| parse_number(text).map(|n| (n, cx.format().to_metres(n))))
            else {
                return false;
            };
            if let Some(layout) = self.layout_with(ask, n, length, cx) {
                self.apply(Edit::Layout { layout }, cx);
            }
            return true;
        }
        let kind = self.array(cx.doc).map(|a| a.layout.clone());
        let ask = match (upper_tr(js_trim(text)).as_str(), kind) {
            ("S", Some(ArrayLayout::Grid { .. })) => Some(Ask::Rows),
            ("T", Some(ArrayLayout::Grid { .. })) => Some(Ask::Cols),
            ("A", Some(ArrayLayout::Grid { .. })) => Some(Ask::ColumnGap),
            ("R", Some(ArrayLayout::Grid { .. })) => Some(Ask::RowGap),
            (
                "D",
                Some(
                    ArrayLayout::Line { .. } | ArrayLayout::Polar { .. } | ArrayLayout::Path { .. },
                ),
            ) => Some(Ask::Count),
            ("A", Some(ArrayLayout::Path { .. })) => Some(Ask::Spacing),
            ("Ç", Some(ArrayLayout::Polar { .. })) => Some(Ask::Fill),
            ("B", _) => Some(Ask::Base),
            ("G", _) if item.is_some() => Some(Ask::Block),
            ("L", _) => {
                if let Some(k) = item {
                    self.apply(
                        Edit::Remove {
                            items: vec![k as u32],
                        },
                        cx,
                    );
                }
                return true;
            }
            ("F", _) => {
                self.apply(Edit::Reset { items: None }, cx);
                return true;
            }
            _ => return false,
        };
        self.ask = ask;
        self.see(cx);
        self.redraw(cx);
        true
    }

    /// Drops the option asked for, then the array; with neither, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        if self.ask.take().is_some() {
            self.redraw(cx);
            return Flow::Stay;
        }
        if self.picked.take().is_some() {
            self.drawn = Preview::default();
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if self.ask.take().is_some() {
            self.redraw(cx);
            return true;
        }
        if self.picked.take().is_some() {
            self.drawn = Preview::default();
            return true;
        }
        false
    }

    /// A Block's name after Değiştir takes spaces.
    fn takes_words(&self) -> bool {
        self.ask == Some(Ask::Block)
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
