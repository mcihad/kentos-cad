//! Yapıştır: the web's `PasteTool` (`apps/web/src/tools/editTools.ts`),
//! step for step (docs/adr/0056). The clipboard's objects follow the cursor
//! by their base point (the lower left corner of their box when they were
//! put aside) until a click or a typed point places them: a coordinate,
//! `@dY,dX` from the base point, a distance toward the cursor. They are
//! pasted in one undo step “Yapıştır” ([`crate::clipboard::paste`]),
//! selected, and the tool leaves. Enter and Esc leave without pasting.
//!
//! The tool is not in the catalog: `edit.paste` runs it with a copy of the
//! clipboard ([`crate::Session::run`]), and repeating the last command does
//! not bring it back. The ghosts are the web's: at most 400 objects' outlines,
//! dashed, from a geometry store of the copies' own.

use kentos_contracts::Entity;
use kentos_geometry_core::geom::affine::translation;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::tools::point_text::point_from_text;

use crate::Vec2;
use crate::clipboard;
use crate::format::Format;
use crate::modify::{MAX_GHOSTS, ghosts};
use crate::prompt::Prompt;
use crate::spatial::record;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tool};

/// Yapıştır's id (the web's `PasteTool.id`); no command starts it by this name.
pub const ID: &str = "paste";
pub const LABEL: &str = clipboard::PASTE_LABEL;

/// Yapıştır with the objects it places.
pub struct Paste {
    items: Vec<Entity>,
    base: Vec2,
    /// The cursor (snapped when a snap applies).
    hover: Option<Vec2>,
    /// The copies in a geometry store of their own, numbered 1…n: their
    /// ghosts come from it. Made at the first move (the web's `copies()`).
    store: Option<Store>,
    ghosts: Vec<Stroke>,
    marks: Vec<Vec2>,
    /// Placed (or refused): the tool leaves.
    done: bool,
}

impl Paste {
    /// The tool with copies of the clipboard's objects and their base point.
    pub fn new(items: Vec<Entity>, base: Vec2) -> Self {
        Self {
            items,
            base,
            hover: None,
            store: None,
            ghosts: Vec::new(),
            marks: Vec::new(),
            done: false,
        }
    }

    /// Places the objects with their base point at `at` and leaves.
    fn place(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let slots = clipboard::paste(&self.items, at.x - self.base.x, at.y - self.base.y, cx);
        if !slots.is_empty() {
            cx.selection.set(slots);
        }
        self.ghosts.clear();
        self.marks.clear();
        self.done = true;
    }

    /// The ghosts with the base point at the cursor.
    fn refresh(&mut self) {
        let Some(hover) = self.hover else {
            return;
        };
        let items = &self.items;
        let store = self.store.get_or_insert_with(|| {
            let mut store = Store::new();
            store.put_many(items.iter().enumerate().map(|(i, e)| {
                let (_, layer, label, shape) = record(e);
                ((i + 1) as f64, layer, label, shape)
            }));
            store
        });
        let shown = items.len().min(MAX_GHOSTS);
        let ids: Vec<f64> = (1..=shown).map(|i| i as f64).collect();
        let m = translation(hover.x - self.base.x, hover.y - self.base.y);
        let paths = store.transform_outlines(&ids, &[m], shown);
        (self.ghosts, self.marks) = ghosts(&paths);
    }
}

impl Tool for Paste {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            LABEL,
            "yerleştirme noktasını belirtin ya da koordinat yazın",
        )
    }

    /// The web's tool counts no points: Ctrl+Z undoes the drawing.
    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.refresh();
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.place(p.world, cx);
    }

    /// A typed point: `@dY,dX` and a distance are measured from the base point.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(at) = point_from_text(text, Some(self.base), self.hover, |_| None) else {
            return false;
        };
        self.place(at, cx);
        true
    }

    /// Enter leaves without pasting (the web's `confirm`).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            strokes: self.ghosts.clone(),
            marks: self.marks.clone(),
            ..Preview::default()
        }
    }

    fn finished(&self) -> bool {
        self.done
    }
}
