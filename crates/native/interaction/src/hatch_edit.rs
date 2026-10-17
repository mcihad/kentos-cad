//! Taramayı düzenle (AutoCAD's HATCHEDIT, docs/adr/0329 §4): click a hatch, then, as the option chosen says:
//!
//! - Ada ekle (E): each closed object clicked becomes one of its islands;
//! - Ada çıkar (Ç): the island clicked is hatched over again (a tied hatch's by its object, another's by the hole the
//!   click is in);
//! - Yeniden seç (S): a click finds its region again as Tarama would (Tarama's Sınır, Adalar, Yazılar, İlişkili and
//!   Boşluk);
//! - Yeniden oluştur (O): its boundary is written at once as an area or, with Biçim (B), as polylines; it then
//!   follows them;
//! - İlişkiyi kopar (K): it follows nothing from then on.
//!
//! Every change is one undo step of `cad.hatches.edit` named after it; the hatch stays picked for the next. Enter or
//! Esc drops the hatch; with none, Enter leaves.

use kentos_contracts::{
    BoundaryForm, Entity, HatchEdit as Edit, HatchRegion, HatchWay, HatchesEdit, TrimBoundaries,
};
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, hatches_edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::hatch_options as options;
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tone, Tool};

/// The tool's id: its command is `tool.hatchEdit`.
pub const ID: &str = "hatchEdit";
pub const LABEL: &str = "Taramayı düzenle";

/// What a click does once a hatch is picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    AddIslands,
    RemoveIslands,
    Reselect,
    Recreate,
    Untie,
    /// Hizala (docs/adr/0331 §2): the hatches clicked continue the picked one's lines.
    Align,
}

impl Mode {
    /// The modes in the order the memory keeps them.
    pub const ALL: [Mode; 6] = [
        Mode::AddIslands,
        Mode::RemoveIslands,
        Mode::Reselect,
        Mode::Recreate,
        Mode::Untie,
        Mode::Align,
    ];

    fn key(self) -> &'static str {
        match self {
            Mode::AddIslands => "E",
            Mode::RemoveIslands => "Ç",
            Mode::Reselect => "S",
            Mode::Recreate => "O",
            Mode::Untie => "K",
            Mode::Align => "H",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Mode::AddIslands => "Ada ekle",
            Mode::RemoveIslands => "Ada çıkar",
            Mode::Reselect => "Yeniden seç",
            Mode::Recreate => "Yeniden oluştur",
            Mode::Untie => "İlişkiyi kopar",
            Mode::Align => "Hizala",
        }
    }

    /// What the prompt asks for with a hatch picked.
    fn asks(self) -> &'static str {
        match self {
            Mode::AddIslands => "ada olacak kapalı nesneye tıklayın",
            Mode::RemoveIslands => "çıkarılacak adaya tıklayın",
            Mode::Reselect => "taramanın yeni bölgesinin içine tıklayın",
            Mode::Recreate | Mode::Untie => "Enter'la başka taramaya geçin",
            Mode::Align => "çizgileri bu taramanınkileri sürdürecek taramalara tıklayın",
        }
    }
}

pub struct HatchEditor {
    picked: Option<Slot>,
    drawn: Preview,
    /// The memory's mode and form as last seen (the prompt reads them).
    mode: Mode,
    polyline: bool,
    asks: &'static str,
}

impl Default for HatchEditor {
    fn default() -> Self {
        Self::new()
    }
}

fn mode(cx: &Context<'_>) -> Mode {
    Mode::ALL[usize::from(cx.memory.hatch_edit_mode).min(Mode::ALL.len() - 1)]
}

impl HatchEditor {
    pub fn new() -> Self {
        Self {
            picked: None,
            drawn: Preview::default(),
            mode: Mode::AddIslands,
            polyline: false,
            asks: Mode::AddIslands.asks(),
        }
    }

    /// The memory's mode and form, for the prompt.
    fn see(&mut self, cx: &Context<'_>) {
        self.mode = mode(cx);
        self.polyline = cx.memory.hatch_recreate_polyline;
        self.asks = self.mode.asks();
    }

    fn uid(&self, cx: &Context<'_>) -> Option<String> {
        self.picked.and_then(|s| cx.doc.uid(s)).map(|u| u.to_string())
    }

    /// The picked hatch outlined.
    fn outline(&mut self, cx: &Context<'_>) {
        self.drawn = match self.picked.and_then(|s| cx.doc.get(s)) {
            // Its ring and each hole closed, apart: the core's outline runs a hatch's holes on from its ring.
            Some(Entity::Hatch(h)) => {
                let ring = |pts: &[kentos_contracts::Vec2]| {
                    let pts: Vec<Vec2> = pts.iter().map(|q| Vec2::new(q.x, q.y)).collect();
                    Stroke::solid(pts, true).width(2.5).tone(Tone::Snap)
                };
                let mut strokes = vec![ring(&h.ring)];
                strokes.extend(h.holes.iter().flatten().map(|r| ring(r)));
                Preview {
                    strokes,
                    ..Preview::default()
                }
            }
            Some(e) => {
                let o = Outline::of(&kentos_native_application::geometry::shape(e), None, 2.5, Tone::Snap);
                Preview {
                    strokes: o.strokes,
                    marks: o.marks,
                    ..Preview::default()
                }
            }
            None => Preview::default(),
        };
    }

    /// Writes `edit` through `cad.hatches.edit`, says what it did.
    fn apply(&mut self, edit: Edit, cx: &mut Context<'_>) {
        let words = match &edit {
            Edit::AddIslands { .. } => "Ada eklendi.",
            Edit::RemoveIslands { .. } => "Ada çıkarıldı; yeniden tarandı.",
            Edit::Reselect { .. } => "Taramanın bölgesi yeniden bulundu.",
            Edit::Recreate { .. } => "Taramanın sınırı yeniden oluşturuldu.",
            Edit::Untie { .. } => "Taramanın ilişkisi koptu; artık bir şeyi izlemiyor.",
            Edit::Pattern { .. } => "Taramanın deseni değişti.",
            Edit::Align { .. } => "Tarama hizalandı; çizgileri seçilen taramanınkileri sürdürüyor.",
        };
        let input = HatchesEdit {
            edit,
            expected_revision: None,
        };
        let result = {
            let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
            hatches_edit::execute(&mut ecx, input)
        };
        let Some(out) = points::written(result, cx) else {
            return;
        };
        if out.changed.is_empty() && out.created.is_empty() {
            cx.say(Level::Info, "Tarama değişmedi.");
        } else {
            cx.say(Level::Success, words);
        }
        self.outline(cx);
    }

    /// The region a click asks for with Tarama's options (docs/adr/0254 §7, 0329).
    fn region_at(p: Vec2, cx: &Context<'_>) -> HatchRegion {
        let m = *cx.memory;
        let view = cx.view.visible();
        HatchRegion::Point {
            at: wire(p),
            way: Some(if m.hatch_by_lines {
                HatchWay::Lines
            } else {
                HatchWay::Object
            }),
            boundaries: m.hatch_by_lines.then(|| TrimBoundaries::Area {
                min: wire(Vec2::new(view.min_x, view.min_y)),
                max: wire(Vec2::new(view.max_x, view.max_y)),
            }),
            boundary_layer: None,
            islands: Some(m.hatch_islands),
            texts: Some(m.hatch_texts),
            associative: Some(m.hatch_assoc),
            also_at: Vec::new(),
            island_style: options::island_style(&m),
            gap: (m.hatch_by_lines && m.hatch_gap > 0.0).then_some(m.hatch_gap),
        }
    }

    /// The hole of the picked hatch a point is in, by its place.
    fn hole_at(&self, p: Vec2, cx: &Context<'_>) -> Option<u32> {
        let Some(Entity::Hatch(h)) = self.picked.and_then(|s| cx.doc.get(s)) else {
            return None;
        };
        h.holes.iter().flatten().position(|ring| {
            let pts: Vec<Vec2> = ring.iter().map(|q| Vec2::new(q.x, q.y)).collect();
            kentos_geometry_core::geometry::point_in_polygon(p, &pts)
        })
        .map(|k| k as u32)
    }

    /// A click with a hatch picked, as the mode says.
    fn act(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(uid) = self.uid(cx) else { return };
        match mode(cx) {
            Mode::AddIslands => match crate::selectable::pick(cx, p.raw).and_then(|s| cx.doc.uid(s)) {
                Some(u) => self.apply(
                    Edit::AddIslands {
                        uid,
                        uids: vec![u.to_string()],
                    },
                    cx,
                ),
                None => cx.say(Level::Warn, "Ada olacak kapalı bir nesneye tıklayın."),
            },
            Mode::RemoveIslands => {
                let tied = matches!(self.picked.and_then(|s| cx.doc.get(s)), Some(Entity::Hatch(h)) if h.assoc.is_some());
                let edit = if tied {
                    crate::selectable::pick(cx, p.raw)
                        .and_then(|s| cx.doc.uid(s))
                        .map(|u| Edit::RemoveIslands {
                            uid,
                            uids: vec![u.to_string()],
                            holes: Vec::new(),
                        })
                } else {
                    self.hole_at(p.raw, cx).map(|k| Edit::RemoveIslands {
                        uid,
                        uids: Vec::new(),
                        holes: vec![k],
                    })
                };
                match edit {
                    Some(e) => self.apply(e, cx),
                    None => cx.say(
                        Level::Warn,
                        if tied {
                            "Taramanın adası olan nesneye tıklayın."
                        } else {
                            "Taramanın bir deliğinin içine tıklayın."
                        },
                    ),
                }
            }
            Mode::Reselect => {
                let region = Self::region_at(p.world, cx);
                self.apply(Edit::Reselect { uid, region }, cx);
            }
            Mode::Align => {
                let other = crate::selectable::candidates(cx, p.raw)
                    .into_iter()
                    .filter(|s| Some(*s) != self.picked)
                    .find(|s| matches!(cx.doc.get(*s), Some(Entity::Hatch(_))))
                    .and_then(|s| cx.doc.uid(s));
                match other {
                    Some(u) => self.apply(
                        Edit::Align {
                            uids: vec![u.to_string()],
                            reference: uid,
                        },
                        cx,
                    ),
                    None => cx.say(Level::Warn, "Hizalanacak başka bir taramaya tıklayın."),
                }
            }
            Mode::Recreate | Mode::Untie => {}
        }
    }

    /// What picking a hatch does in a mode that needs no further click.
    fn at_once(&mut self, cx: &mut Context<'_>) {
        let Some(uid) = self.uid(cx) else { return };
        match mode(cx) {
            Mode::Recreate => {
                let form = if cx.memory.hatch_recreate_polyline {
                    BoundaryForm::Polyline
                } else {
                    BoundaryForm::Area
                };
                self.apply(
                    Edit::Recreate {
                        uid,
                        form: Some(form),
                        layer_id: None,
                        associative: Some(true),
                    },
                    cx,
                );
            }
            Mode::Untie => self.apply(Edit::Untie { uids: vec![uid] }, cx),
            _ => {}
        }
    }
}

impl Tool for HatchEditor {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn snaps(&self) -> bool {
        false
    }

    fn point_count(&self) -> usize {
        0
    }

    fn prompt(&self) -> Prompt {
        let mut prompt = match self.picked {
            None => Prompt::new(LABEL, "düzenlenecek taramaya tıklayın"),
            Some(_) => Prompt::new(LABEL, self.asks),
        }
        .note(format!("kip: {}", self.mode.label()))
        .then();
        for m in Mode::ALL {
            prompt = prompt.option(m.label(), m.key());
        }
        if self.mode == Mode::Recreate {
            prompt = prompt.option_with("Biçim", "B", if self.polyline { "çoklu çizgi" } else { "alan" });
        }
        prompt
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let hit = crate::selectable::hover(cx, p.raw);
        cx.selection.set_hover(hit);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.picked.is_some() {
            self.act(p, cx);
            self.see(cx);
            return;
        }
        // The click's candidates, the hatch among them (its edge is often an area's too).
        let hatch = crate::selectable::candidates(cx, p.raw)
            .into_iter()
            .find(|s| matches!(cx.doc.get(*s), Some(Entity::Hatch(_))));
        let Some(slot) = hatch else {
            cx.say(Level::Warn, "Kilitsiz katmandaki bir taramaya tıklayın.");
            return;
        };
        self.picked = Some(slot);
        cx.selection.set_hover(None);
        self.outline(cx);
        self.at_once(cx);
        self.see(cx);
    }

    /// A mode's letter chooses it (Yeniden oluştur and İlişkiyi kopar act at once on a picked hatch); Biçim turns the
    /// form.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if key == "B" && mode(cx) == Mode::Recreate {
            cx.memory.hatch_recreate_polyline = !cx.memory.hatch_recreate_polyline;
            self.see(cx);
            return true;
        }
        let Some(k) = Mode::ALL.iter().position(|m| m.key() == key) else {
            return false;
        };
        cx.memory.hatch_edit_mode = k as u8;
        self.see(cx);
        if self.picked.is_some() {
            self.at_once(cx);
        }
        true
    }

    /// Drops the hatch; with none, the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.picked.take().is_some() {
            self.drawn = Preview::default();
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.picked.take().is_some() {
            self.drawn = Preview::default();
            return true;
        }
        false
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
