//! Tablo ekle's placement: the web's `TablePlaceTool`
//! (`apps/web/src/tools/tablePlaceTool.ts`) on its `PointInputTool` base,
//! step for step (docs/adr/0184 §3). The table Tablo ekle made hangs from the
//! cursor by its top left corner, its lines dashed, its frame's band lightly
//! filled and its words faint; a click or a typed point writes it on the
//! active layer through `cad.entities.create` (step “Tablo”), selected, and
//! the tool leaves. Enter and Esc leave without writing.
//!
//! The tool is not in the catalog: Tablo ekle's window runs it with the
//! table ([`crate::Session::run`]); repeating the last command does not bring
//! it back.

use kentos_contracts::{CreateOperation, EntityBase, EntityGeometry};
use kentos_geometry_core::geom::table::layout_of;
use kentos_native_application::geometry::{drawing_font, entity_of, shape};

use crate::Vec2;
use crate::log::Level;
use crate::points::{self, Taken, wire};
use crate::prompt::Prompt;
use crate::tool::{Area, CellGhost, Context, Flow, Pointer, Preview, Stroke, Tone, Tool};

/// The tool's id (the web's `TablePlaceTool.id`); no command starts it by this name.
pub const ID: &str = "tablePlace";
pub const LABEL: &str = "Tablo ekle";

/// Tablo ekle's placement with the table it writes (its corner at the origin).
pub struct TablePlace {
    d: Taken,
    table: EntityGeometry,
    /// What it is called: Tablo ekle, or the command that made the table
    /// (Koordinat çizelgesi, docs/adr/0185 §1).
    label: &'static str,
    /// The table at the cursor, as of the last pointer move.
    ghost: Preview,
    /// Written (or refused): the tool leaves.
    done: bool,
}

/// `table` with its top left corner at `p`.
fn placed(table: &EntityGeometry, p: Vec2) -> EntityGeometry {
    let mut out = table.clone();
    if let EntityGeometry::Table { p: at, .. } = &mut out {
        *at = wire(p);
    }
    out
}

impl TablePlace {
    /// The tool with the table it places.
    pub fn new(table: EntityGeometry) -> Self {
        Self::named(table, LABEL)
    }

    /// The tool with the table it places, called `label`.
    pub fn named(table: EntityGeometry, label: &'static str) -> Self {
        Self {
            d: Taken::default(),
            table,
            label,
            ghost: Preview::default(),
            done: false,
        }
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        let Some(out) = points::write_objects(
            vec![placed(&self.table, p)],
            Some(CreateOperation::Table),
            cx,
        ) else {
            return;
        };
        let slots: Vec<kentos_domain::Slot> =
            out.ids.iter().map(|&id| kentos_domain::Slot(id)).collect();
        cx.selection.set(slots);
        cx.say(
            Level::Success,
            "Tablo eklendi: düzenlemek için çift tıklayın.",
        );
        self.ghost = Preview::default();
        self.done = true;
    }

    /// The table as it would be placed at the cursor.
    fn place_ghost(&mut self, cx: &Context<'_>) {
        self.ghost = Preview::default();
        let Some(h) = self.d.hover else {
            return;
        };
        let geometry = placed(&self.table, h);
        let base = EntityBase {
            id: 0,
            layer_id: String::new(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        };
        let entity = entity_of(&geometry, base);
        let Some(layout) = layout_of(
            &shape(&entity),
            drawing_font(cx.doc.settings().drawing_font),
        ) else {
            return;
        };
        let kentos_contracts::Entity::Table(t) = &entity else {
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
}

impl Tool for TablePlace {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        self.label
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            self.label,
            "tablonun sol üst köşesine tıklayın ya da Y,X yazın",
        )
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
        self.place_ghost(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match cx.typed_point(text, self.d.last(), self.d.hover) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    /// It never holds a point: a confirm leaves (the web's `PointInputTool.confirm`).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    fn preview(&self, _format: &crate::format::Format) -> Preview {
        Preview {
            tracking: self.d.tracking,
            ..self.ghost.clone()
        }
    }

    fn finished(&self) -> bool {
        self.done
    }
}
