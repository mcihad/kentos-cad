//! Veri çıkarımı tablosunu yerleştir (docs/adr/1030 §8): the Veri çıkarımı window's table hangs from the cursor by its
//! top left corner, as `cad.extract.table` plans it (its parts and lines dashed, its words faint); a click or a typed
//! point writes it through the same command (one undo step “Veri çıkarımı tablosu”), selected, and the tool leaves.
//! Enter and Esc leave without writing. The tool is not in the catalog: the window runs it with its recipe.

use kentos_commands::{CommandResult, Entity, ExtractTable, TableEntity};
use kentos_domain::Slot;
use kentos_geometry_core::geom::table::{cell_line, layout_of};
use kentos_native_application::geometry::{drawing_font, shape};
use kentos_native_application::{ExecutionContext, extract_table};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Area, CellGhost, Context, Flow, Pointer, Preview, Stroke, Tag, Tone, Tool};

/// The tool's id; no command starts it by this name.
pub const ID: &str = "extractPlace";
pub const LABEL: &str = "Veri çıkarımı tablosu";

/// The placement with the window's input (its `at` the cursor's).
pub struct ExtractPlace {
    input: ExtractTable,
    table: Option<TableEntity>,
    problem: Option<String>,
    hover: Option<Vec2>,
    ghost: Preview,
    done: bool,
}

impl ExtractPlace {
    pub fn new(input: ExtractTable) -> Self {
        Self {
            input,
            table: None,
            problem: None,
            hover: None,
            ghost: Preview::default(),
            done: false,
        }
    }

    fn at(&self, p: Vec2) -> ExtractTable {
        let mut input = self.input.clone();
        input.at = points::wire(p);
        input
    }

    /// The table as the command plans it at the origin (it is moved with the cursor, not planned again).
    fn plan(&mut self, cx: &Context<'_>) {
        self.table = None;
        self.problem = None;
        match extract_table::plan_in(cx.doc, &self.at(Vec2::new(0.0, 0.0))) {
            CommandResult::Completed { output, .. } => {
                if let Entity::Table(t) = output.table {
                    self.table = Some(t);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.problem = Some(error.message);
            }
            _ => {}
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
                let words = cell_line(t.cells.get(c.row)?.get(c.col)?, c.line);
                (!words.is_empty()).then(|| CellGhost {
                    at: c.at,
                    text: words.to_owned(),
                    height: t.height,
                    bold: c.bold,
                    face: t.face.clone(),
                })
            })
            .collect();
    }

    fn write(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let input = self.at(at);
        let result = extract_table::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            self.done = true;
            self.ghost = Preview::default();
            let slot = kentos_domain::Uuid::parse_str(&out.uid)
                .ok()
                .and_then(|u| cx.doc.slot_of(u));
            if let Some(slot) = slot {
                cx.selection.set([Slot(slot.0)]);
            }
            cx.say(
                Level::Success,
                format!(
                    "{LABEL}: {} satır, {} oluşum. Kaynakları değişince Tabloyu güncelle ile yenilenir.",
                    out.rows, out.occurrences
                ),
            );
        }
    }
}

impl Tool for ExtractPlace {
    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
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
        Prompt::new(LABEL, "tablonun sol üst köşesine tıklayın ya da Y,X yazın")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.place_ghost(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.write(p.world, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match cx.typed_point(text, None, self.hover) {
            Some(p) => {
                self.write(p, cx);
                true
            }
            None => false,
        }
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
