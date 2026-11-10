//! Doğrama listesi (docs/adr/8038 §6; M38): the joinery list's block placed at a click, its top left corner at the
//! cursor. The block is written as the drawing's types give it (`kentos_domain::joinery_list`) and the document keeps
//! it so; the block and its insert are one undo step “Doğrama listesi”, on the schedules' layer (docs/adr/8008).

use kentos_commands::{BlocksInsert, CommandResult};
use kentos_domain::joinery_list::{JOINERY_LIST_ID, extent};
use kentos_native_application::{ExecutionContext, blocks_insert};

use crate::Vec2;
use crate::log::Level;
use crate::points::{Taken, wire};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tool};

/// The tool's id and its command's (`tool.joineryList`).
pub const ID: &str = "joineryList";
pub const LABEL: &str = "Doğrama listesi";

/// Doğrama listesi's placement.
#[derive(Default)]
pub struct JoineryListTool {
    d: Taken,
    /// The list's box about its base point, metres, as of the last look.
    bounds: Option<[f64; 4]>,
    ghost: Preview,
    done: bool,
}

impl JoineryListTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        let rows = cx.doc.joinery_rows();
        let loose = cx
            .doc
            .entities()
            .filter(|e| {
                matches!(e, kentos_commands::Entity::Opening(o)
                    if matches!(o.opening.form, kentos_commands::OpeningForm::Door | kentos_commands::OpeningForm::Window)
                        && !o.opening.follows_type())
            })
            .count();
        let written =
            crate::arch_layer::writing(&crate::arch_layer::Kind::Schedule, cx, |layer, cx| {
                let group = cx.doc.begin_group(LABEL);
                let block = cx.doc.joinery_list_block();
                let made = if cx.doc.block(JOINERY_LIST_ID).is_some() {
                    cx.doc.update_block(block).map(|_| ())
                } else {
                    cx.doc.add_block(block)
                };
                if let Err(why) = made {
                    cx.doc.cancel_group(group);
                    cx.say(Level::Warn, why.to_string());
                    return false;
                }
                let input = BlocksInsert {
                    parameters: None,
                    block: Some(JOINERY_LIST_ID),
                    name: None,
                    at: wire(p),
                    scale: Some(1.0),
                    y_scale: None,
                    rotation: Some(0.0),
                    mirror: None,
                    layer_id: layer.to_owned(),
                    color: None,
                    attributes: None,
                    attrs: None,
                    label: None,
                    symbol: None,
                    arch: Default::default(),
                    expected_revision: None,
                };
                match blocks_insert::execute(&mut ExecutionContext::new(cx.doc), input) {
                    CommandResult::Completed { .. } => {
                        cx.doc.end_group(group);
                        true
                    }
                    CommandResult::Failed { error } | CommandResult::NeedsInput { error } => {
                        cx.doc.cancel_group(group);
                        cx.say(Level::Warn, error.message);
                        false
                    }
                    _ => {
                        cx.doc.cancel_group(group);
                        false
                    }
                }
            });
        if written {
            let codes: Vec<String> = rows
                .iter()
                .map(|r| r.kind.code.clone().unwrap_or_else(|| r.kind.name.clone()))
                .collect();
            let openings: usize = rows.iter().map(|r| r.count).sum();
            let mut said = format!(
                "{LABEL}: {} tip ({}), {openings} açıklık; tipler değişince liste kendini yeniden yazar.",
                rows.len(),
                codes.join(", ")
            );
            if loose > 0 {
                said.push_str(&format!(
                    " Tipe bağlı olmayan {loose} açıklık listede yok: Doğrama kodları onlara tip ve kod verir."
                ));
            }
            cx.say(Level::Success, said);
        }
        self.ghost = Preview::default();
        self.done = true;
    }

    /// The list's outline at the cursor.
    fn place_ghost(&mut self, cx: &Context<'_>) {
        self.ghost = Preview::default();
        let Some(h) = self.d.hover else {
            return;
        };
        let b = *self.bounds.get_or_insert_with(|| {
            extent(&cx.doc.joinery_list_block()).unwrap_or([0.0, -2.0, 6.0, 0.0])
        });
        let ring = vec![
            Vec2::new(h.x + b[0], h.y + b[1]),
            Vec2::new(h.x + b[2], h.y + b[1]),
            Vec2::new(h.x + b[2], h.y + b[3]),
            Vec2::new(h.x + b[0], h.y + b[3]),
        ];
        self.ghost.strokes = vec![Stroke::dashed(ring, true, [4.0, 3.0])];
    }
}

impl Tool for JoineryListTool {
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
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "listenin sol üst köşesine tıklayın ya da Y,X yazın")
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
