//! Doğrama listesi (docs/adr/8038 §6; M38): the joinery list's block placed at a click, its top left corner at the
//! cursor. The drawing keeps the block in step with its types (`kentos_domain::joinery_list`). With a placement
//! present, Enter updates that definition in place; an explicit Yeni kopya places another instance.

use kentos_commands::{BlocksInsert, CommandResult, OpeningTypesEdit, OpeningTypesOperation};
use kentos_domain::joinery_list::{JOINERY_LIST_ID, extent};
use kentos_native_application::{ExecutionContext, blocks_insert, opening_types_edit};

use crate::Vec2;
use crate::log::Level;
use crate::points::{Taken, wire};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, Tool};

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
    coverage: kentos_domain::joinery_list::Coverage,
    seen: Option<u64>,
    placed: usize,
    separate: bool,
    assign_codes: bool,
    done: bool,
}

impl JoineryListTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn refresh(&mut self, cx: &Context<'_>) {
        if self.seen != Some(cx.doc.revision()) {
            self.seen = Some(cx.doc.revision());
            self.coverage = cx.doc.joinery_coverage();
            self.placed = cx.doc.block_uses(JOINERY_LIST_ID);
            self.bounds = None;
        }
    }

    fn updates_existing(&self) -> bool {
        self.placed > 0 && !self.separate
    }

    fn accept(&mut self, at: Option<Vec2>, cx: &mut Context<'_>) {
        self.refresh(cx);
        let updating = self.updates_existing();
        let place = at.filter(|_| !updating);
        if !updating && place.is_none() {
            return;
        }
        if self.coverage.total == 0 {
            cx.say(
                Level::Warn,
                "Doğrama listesi: çizimde kapı veya pencere yok.",
            );
            return;
        }
        if self.coverage.listed == 0 && !self.assign_codes {
            cx.say(
                Level::Warn,
                "Önce Kodları oluştur ve listele seçeneğini açın.",
            );
            return;
        }
        let written = crate::arch_layer::writing(
            &crate::arch_layer::Kind::Schedule,
            cx,
            |layer, cx| {
                let group = cx.doc.begin_group(LABEL);
                if self.assign_codes && self.coverage.coded < self.coverage.total {
                    let input = OpeningTypesEdit {
                        operation: OpeningTypesOperation::Codes {},
                        on_refused: Default::default(),
                        expected_revision: None,
                    };
                    let result =
                        opening_types_edit::execute(&mut ExecutionContext::new(cx.doc), input);
                    if crate::points::written(result, cx).is_none() {
                        cx.doc.cancel_group(group);
                        return false;
                    }
                }
                if cx.doc.joinery_coverage().listed == 0 {
                    cx.doc.cancel_group(group);
                    cx.say(Level::Warn, "Listelenecek kapı veya pencere yok. Kilitli katmanları ve proje tiplerini kontrol edin.");
                    return false;
                }
                let block = cx.doc.joinery_list_block();
                let made = if let Some(existing) = cx.doc.block(JOINERY_LIST_ID) {
                    if *existing == block {
                        Ok(())
                    } else {
                        cx.doc.update_block(block).map(|_| ())
                    }
                } else {
                    cx.doc.add_block(block)
                };
                if let Err(why) = made {
                    cx.doc.cancel_group(group);
                    cx.say(Level::Warn, why.to_string());
                    return false;
                }
                let Some(p) = place else {
                    cx.doc.end_group(group);
                    return true;
                };
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
            },
        );
        if written {
            if let Some(p) = place {
                self.d.begin(p, cx);
            }
            let rows = cx.doc.joinery_rows();
            let coverage = cx.doc.joinery_coverage();
            let openings: usize = rows.iter().map(|r| r.count).sum();
            let mut said = if updating {
                format!("{LABEL}: {} yerleşim güncel · {} tip · {openings} açıklık.", self.placed, rows.len())
            } else {
                let codes: Vec<_> = rows.iter().map(|r| r.kind.code.as_deref().unwrap_or(&r.kind.name)).collect();
                format!(
                    "{LABEL}: {} tip ({}), {openings} açıklık; tipler değişince liste kendini yeniden yazar.",
                    rows.len(),
                    codes.join(", ")
                )
            };
            if coverage.coded < coverage.total {
                said.push_str(&format!(
                    " {} açıklık kod bekliyor.",
                    coverage.total - coverage.coded
                ));
            }
            cx.say(Level::Success, said);
            self.ghost = Preview::default();
            self.done = true;
        } else {
            self.seen = None;
            self.refresh(cx);
        }
    }

    /// The list's outline at the cursor.
    fn place_ghost(&mut self, cx: &Context<'_>) {
        self.ghost = Preview::default();
        if self.updates_existing() || self.coverage.total == 0
            || (self.coverage.listed == 0 && !self.assign_codes)
        {
            return;
        }
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
        self.accept(Some(p), cx);
        true
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let step = if self.coverage.total == 0 {
            "çizimde kapı veya pencere yok"
        } else if self.coverage.listed == 0 && !self.assign_codes {
            "kapı ve pencereler kod bekliyor"
        } else if self.updates_existing() {
            "mevcut listeyi güncellemek için Enter"
        } else {
            "listenin sol üst köşesini belirtin"
        };
        let prompt = Prompt::new(LABEL, step).option_if(
            self.placed > 0,
            "Liste",
            "Y",
            if self.separate { "Yeni kopya" } else { "Mevcut listeyi güncelle" },
        );
        if self.coverage.coded < self.coverage.total {
            prompt.toggle("Kodları oluştur ve listele", "K", self.assign_codes)
        } else {
            prompt
        }
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.refresh(cx);
        cx.say(Level::Info, self.coverage.caption());
        Flow::Stay
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.refresh(cx);
        self.d.hover = Some(self.d.constrain(p, cx));
        self.place_ghost(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(Some(p), cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if text.trim().eq_ignore_ascii_case("K") && self.coverage.coded < self.coverage.total {
            self.assign_codes = !self.assign_codes;
            self.place_ghost(cx);
            return true;
        }
        if text.trim().eq_ignore_ascii_case("Y") && self.placed > 0 {
            self.separate = !self.separate;
            self.place_ghost(cx);
            return true;
        }
        match cx.typed_point(text, self.d.last(), self.d.hover) {
            Some(p) => {
                self.accept(Some(p), cx);
                true
            }
            None => false,
        }
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.refresh(cx);
        if self.updates_existing() {
            self.accept(None, cx);
            if !self.done {
                return Flow::Stay;
            }
        }
        Flow::Exit
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    fn preview(&self, _format: &crate::format::Format) -> Preview {
        Preview {
            tracking: self.d.tracking,
            tag: self.d.hover.map(|at| Tag {
                at,
                lines: vec![self.coverage.caption()],
            }),
            ..self.ghost.clone()
        }
    }

    fn finished(&self) -> bool {
        self.done
    }
}
