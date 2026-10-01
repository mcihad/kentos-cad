//! Kılavuz (docs/adr/0146 §7): the web's `LeaderTool`
//! (`apps/web/src/tools/leaderTool.ts`) on its `PointInputTool` base, step for step:
//!
//! - the first click (or typed point) is the arrow's tip, the next ones its
//!   vertices; Enter or a right click ends them, and a text field opens past
//!   the landing's end, on the side the last segment goes (`ViewChange::Text`);
//!   Enter there writes the leader with its note, Enter in the empty field
//!   writes it without one (the arrow alone), Esc goes back to the vertices;
//! - Ok (O) chooses the arrowhead from its menu or by its name, Yükseklik (Y)
//!   is Yazı's (paper mm, shared), Zemin (Z) fills the note's box, Geri (G)
//!   takes the last vertex back; they stay for as long as the app lives;
//! - Esc steps back: out of a question, then the leader being drawn, then
//!   out of the tool; a locked active layer is said at the tip's click.
//!
//! The preview is the core's layout (`geom::leader`): the arrowhead, the line
//! and the landing, the note's place as a dashed box. The leader is written
//! through `cad.entities.create` (docs/adr/0146 §6): the active layer, the
//! current colour and line weight, one undo step (“Kılavuz”).

use kentos_contracts::{CreateOperation, EntityGeometry, LeaderArrow};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::leader::{self, Head, LeaderLayout};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number, point_from_text};

use crate::Vec2;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, upper_tr};
use crate::text::fold_name;
use crate::tool::{
    Area, Context, Flow, Memory, OptionChoice, Pointer, Preview, Stroke, TextField, Tone, Tool,
    ViewChange,
};

/// The leader tool's id: its command is `tool.leader`.
pub const ID: &str = "leader";
pub const LABEL: &str = "Kılavuz";

/// The note's box at least this high on screen, logical pixels (the web's `Math.max(8, …)`).
const LEAST_BOX_PX: f64 = 8.0;

/// The arrowheads in Ok's menu: the value (none: the filled arrow, the
/// field's absence), its name as the prompt writes and the user types it,
/// its menu label and the web's icon. The web's `LEADER_ARROW_ROWS`.
pub const ARROWS: [(Option<LeaderArrow>, &str, &str, &str); 4] = [
    (None, "dolu", "Dolu", "leaderArrowFilled"),
    (Some(LeaderArrow::Open), "açık", "Açık", "leaderArrowOpen"),
    (Some(LeaderArrow::Dot), "nokta", "Nokta", "leaderArrowDot"),
    (Some(LeaderArrow::None), "yok", "Yok", "leaderArrowNone"),
];

/// An arrowhead's name: “dolu”, “açık”, “nokta”, “yok”.
pub fn arrow_name(a: Option<LeaderArrow>) -> &'static str {
    ARROWS.iter().find(|row| row.0 == a).map_or("dolu", |row| row.1)
}

/// An arrowhead's label at the head of a menu row or a cell: “Dolu”.
pub fn arrow_label(a: Option<LeaderArrow>) -> &'static str {
    ARROWS.iter().find(|row| row.0 == a).map_or("Dolu", |row| row.2)
}

/// An arrowhead's icon in the web's set (`ui/icons.ts`).
pub fn arrow_icon(a: Option<LeaderArrow>) -> &'static str {
    ARROWS
        .iter()
        .find(|row| row.0 == a)
        .map_or("leaderArrowFilled", |row| row.3)
}

/// The arrowhead a typed name is, with or without the Turkish marks; none
/// when it names none (the web's `leaderArrowFromName`).
pub fn arrow_from_name(typed: &str) -> Option<Option<LeaderArrow>> {
    let folded = fold_name(typed);
    ARROWS
        .iter()
        .find(|row| fold_name(row.1) == folded)
        .map(|row| row.0)
}

/// Where a leader's note stands and which point of it that is: past its
/// landing (docs/adr/0146 §2). One without a note is laid out as if it had
/// one, so that a field opens where its note will be (editing in place).
pub fn note_place(
    e: &kentos_contracts::Entity,
) -> Option<(Vec2, kentos_geometry_core::text::TextAlign)> {
    let mut s = kentos_native_application::geometry::shape(e);
    let Shape::Leader { text, .. } = &mut s else {
        return None;
    };
    text.get_or_insert_with(|| "Not".to_owned());
    let l = leader::layout_of(&s)?;
    Some((l.note_point?, l.note_align?))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// The tip and the vertices.
    #[default]
    Pts,
    /// Yükseklik: the note's paper height typed.
    Height,
    /// Ok: the arrowhead's name typed, or chosen from its menu.
    Arrow,
    /// The note's field is open.
    Typing,
}

/// The leader tool.
#[derive(Clone, Debug, Default)]
pub struct Leader {
    d: Taken,
    stage: Stage,
    /// The tip, then the vertices.
    pts: Vec<Vec2>,
    /// The vertices the open field writes with.
    typing: Vec<Vec2>,
    /// The note's box height in metres at the last move: the note's own, at
    /// least 8 px on screen.
    box_height: f64,
    /// The note's height in metres, as of the last call.
    height: f64,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
}

/// The active layer's lock sentence, when it is locked (the drawing tools' words).
fn active_locked(cx: &Context<'_>) -> Option<String> {
    let layers = cx.doc.layers();
    let id = layers.active();
    layers.is_locked(id).then(|| {
        let name = layers.get(id).map_or(id, |n| n.name.as_str());
        format!(
            "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
        )
    })
}

/// Paper millimetres as metres at the project's plot scale (the web's `paper`).
fn paper(mm: f64, cx: &Context<'_>) -> f64 {
    mm / 1000.0 * cx.doc.settings().plot_scale
}

/// `açık` or `kapalı`, as the prompt says a switch.
fn on_off(on: bool) -> &'static str {
    if on { "açık" } else { "kapalı" }
}

/// The core's layout of a leader through `pts` with this arrowhead, its note's place when `note`.
fn layout(pts: &[Vec2], height: f64, arrow: Option<LeaderArrow>, note: bool) -> Option<LeaderLayout> {
    leader::layout_of(&Shape::Leader {
        pts: pts.to_vec(),
        text: note.then(|| "Not".to_owned()),
        height,
        rotation: 0.0,
        arrow: arrow.map(|a| a.name().to_owned()),
        mask: None,
    })
}

impl Leader {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
        self.height = paper(cx.memory.text_height_mm, cx);
    }

    fn memory(&self) -> Memory {
        self.seen.map_or_else(Memory::default, |(m, _)| m)
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if self.stage != Stage::Pts {
            return false;
        }
        match key {
            "O" => self.stage = Stage::Arrow,
            "Y" => self.stage = Stage::Height,
            "Z" => cx.memory.leader_mask = !cx.memory.leader_mask,
            "G" if !self.pts.is_empty() => {
                self.pts.pop();
            }
            _ => return false,
        }
        true
    }

    /// A typed or chosen arrowhead: kept, and the tool waits for its
    /// vertices again; a word that names none is said.
    fn take_arrow(&mut self, typed: &str, cx: &mut Context<'_>) -> bool {
        match arrow_from_name(typed) {
            Some(a) => {
                cx.memory.leader_arrow = a;
                self.stage = Stage::Pts;
            }
            None => cx.say(
                Level::Warn,
                format!(
                    "“{typed}” bir ok başı adı değil. Ok başını menüden seçin ya da adını yazın: dolu, açık, nokta, yok."
                ),
            ),
        }
        true
    }

    /// A point given (the web's `accept`, then `onPoint`).
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        points::echo(p, cx);
        if self.pts.is_empty() {
            // A first point starts a new object: what was written before is the drawing's to undo.
            self.d.reset();
        }
        if self.stage != Stage::Pts {
            return;
        }
        if self.pts.is_empty() {
            // A locked layer is said at the tip's click, not after the note is typed.
            if let Some(line) = active_locked(cx) {
                cx.say(Level::Warn, line);
                return;
            }
        }
        // A click on the last vertex adds none.
        if self.pts.last() == Some(&p) {
            return;
        }
        self.pts.push(p);
    }

    /// The note's field, past the landing's end on the note's side.
    fn open_note(&mut self, cx: &mut Context<'_>) {
        let height = paper(cx.memory.text_height_mm, cx);
        let Some(l) = layout(&self.pts, height, cx.memory.leader_arrow, true) else {
            return;
        };
        let Some(at) = l.note_point else {
            return;
        };
        self.typing = self.pts.clone();
        self.stage = Stage::Typing;
        cx.view_changes.push(ViewChange::Text(TextField {
            at,
            height,
            rotation: 0.0,
            align: l
                .note_align
                .and_then(|a| kentos_contracts::TextAlign::from_name(a.name())),
            width_factor: 1.0,
            initial: None,
            placeholder: Some("Notu yazın"),
            hint: Some("Enter: ekle · boş Enter: notsuz · Esc: köşelere dön"),
            empty: true,
        }));
    }

    /// The leader through the typed vertices, with its note when one was
    /// typed, in one step (“Kılavuz”); the tool waits for the next one.
    fn write(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        let m = *cx.memory;
        // The defaults are no fields: a filled arrow, no mask (a mask without a note masks nothing).
        let geometry = EntityGeometry::Leader {
            pts: self.typing.iter().map(|p| points::wire(*p)).collect(),
            text: text.map(str::to_owned),
            height: paper(m.text_height_mm, cx),
            rotation: 0.0,
            arrow: m.leader_arrow,
            mask: m.leader_mask && text.is_some(),
        };
        if let Some(out) =
            points::write_objects(vec![geometry], Some(CreateOperation::Leader), cx)
            && let Some(&id) = out.ids.first()
        {
            self.d.note(id, cx);
            cx.say(
                Level::Success,
                match text {
                    Some(t) => format!("Kılavuz eklendi: “{t}”"),
                    None => "Kılavuz eklendi (notsuz).".to_owned(),
                },
            );
        }
        self.reset();
    }

    fn reset(&mut self) {
        self.stage = Stage::Pts;
        self.pts.clear();
        self.typing.clear();
    }
}

impl Tool for Leader {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        self.see(cx);
        true
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let memory = self.memory();
        match self.stage {
            Stage::Height => Prompt::new(
                LABEL,
                "kâğıt üzerindeki not yüksekliğini mm olarak yazın (Yazı ile ortak)",
            ),
            Stage::Arrow => Prompt::new(
                LABEL,
                "ok başını seçin ya da adını yazın: dolu, açık, nokta, yok",
            )
            .option_with("Ok", "O", arrow_name(memory.leader_arrow)),
            Stage::Typing => Prompt::new(
                LABEL,
                "notu kolun ucuna yazın; Enter ekler, boş Enter notsuz ekler, Esc köşelere döner",
            ),
            Stage::Pts => {
                let ask = match self.pts.len() {
                    0 => "okun ucuna tıklayın",
                    1 => "sonraki köşeye tıklayın",
                    _ => "sonraki köşeye tıklayın; Enter ya da sağ tık notu yazdırır",
                };
                let prompt = Prompt::new(LABEL, ask)
                    .option_with("Ok", "O", arrow_name(memory.leader_arrow))
                    .option_with(
                        "Yükseklik",
                        "Y",
                        format!("{} mm", js_number(memory.text_height_mm)),
                    )
                    .option_with("Zemin", "Z", on_off(memory.leader_mask));
                if self.pts.is_empty() {
                    prompt
                } else {
                    prompt.option("Geri", "G")
                }
            }
        }
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, tracking) = points::constrain(self.pts.last().copied(), p, cx);
        self.d.tracking = tracking;
        self.d.hover = Some(point);
        self.box_height =
            paper(cx.memory.text_height_mm, cx).max(cx.view.world_length(LEAST_BOX_PX));
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, _) = points::constrain(self.pts.last().copied(), p, cx);
        self.accept(point, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        let done = if self.option(&upper_tr(t), cx) {
            true
        } else {
            match self.stage {
                Stage::Arrow => self.take_arrow(t, cx),
                Stage::Height => match parse_number(t) {
                    Some(n) if n > 0.0 => {
                        cx.memory.text_height_mm = n;
                        self.stage = Stage::Pts;
                        true
                    }
                    _ => false,
                },
                Stage::Typing => false,
                Stage::Pts => {
                    match point_from_text(text, self.pts.last().copied(), self.d.hover, |d| {
                        cx.track_along(d)
                    }) {
                        Some(p) => {
                            self.accept(p, cx);
                            true
                        }
                        None => false,
                    }
                }
            }
        };
        self.see(cx);
        done
    }

    /// Ok's menu: the four arrowheads, while the tool waits for a vertex or for one.
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key != "O" || !matches!(self.stage, Stage::Pts | Stage::Arrow) {
            return Vec::new();
        }
        let chosen = self.memory().leader_arrow;
        ARROWS
            .iter()
            .map(|&(a, typed, label, icon)| OptionChoice {
                label,
                typed,
                icon,
                checked: a == chosen,
            })
            .collect()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key != "O" || !matches!(self.stage, Stage::Pts | Stage::Arrow) {
            return false;
        }
        let taken = self.take_arrow(typed, cx);
        self.see(cx);
        taken
    }

    /// The field's answer: the note (an empty one: the arrow alone), or none
    /// (Esc): back to the vertices.
    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if self.stage != Stage::Typing {
            return;
        }
        match text.map(js_trim) {
            Some("") => self.write(None, cx),
            Some(note) => self.write(Some(note), cx),
            None => self.stage = Stage::Pts,
        }
        self.see(cx);
    }

    /// Enter or a right click: the vertices end and the note's field opens;
    /// with the tip alone it is said; with none the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let flow = match self.stage {
            Stage::Typing => Flow::Stay,
            Stage::Height | Stage::Arrow => {
                self.stage = Stage::Pts;
                Flow::Stay
            }
            Stage::Pts => match self.pts.len() {
                0 => Flow::Exit,
                1 => {
                    cx.say(
                        Level::Warn,
                        "Kılavuzun en az 2 köşesi olur: okun ucundan sonra bir köşeye daha tıklayın.",
                    );
                    Flow::Stay
                }
                _ => {
                    self.open_note(cx);
                    Flow::Stay
                }
            },
        };
        self.see(cx);
        flow
    }

    /// Esc, one step back: out of a question, then the leader being drawn;
    /// with nothing drawn the tool leaves.
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        match self.stage {
            Stage::Height | Stage::Arrow | Stage::Typing => {
                self.stage = Stage::Pts;
                true
            }
            Stage::Pts if !self.pts.is_empty() => {
                self.reset();
                true
            }
            Stage::Pts => false,
        }
    }

    /// Ctrl+Z: Geri first (the last vertex), then the leader just written.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.stage == Stage::Pts && self.pts.pop().is_some() {
            return true;
        }
        self.d.undo_last_made(cx)
    }

    /// The leader as it will be, through the pointer: its arrowhead, line
    /// and landing, and its note's place as a dashed box.
    fn preview(&self, _format: &Format) -> Preview {
        let mut pts = if self.stage == Stage::Typing {
            self.typing.clone()
        } else {
            self.pts.clone()
        };
        if self.stage == Stage::Pts
            && let Some(h) = self.d.hover
        {
            pts.push(h);
        }
        if pts.len() < 2 {
            return Preview::default();
        }
        let memory = self.memory();
        let Some(l) = layout(&pts, self.height, memory.leader_arrow, true) else {
            return Preview::default();
        };
        let accent = |pts: Vec<Vec2>, closed: bool| Stroke {
            pts,
            closed,
            dash: None,
            width: 1.5,
            tone: Tone::Accent,
        };
        let mut strokes = vec![accent(leader::drawn_path(&pts, &l), false)];
        let mut areas = Vec::new();
        if let Head::Open { lines } = &l.head {
            strokes.push(accent(lines.to_vec(), false));
        } else if let Some(ring) = leader::head_ring(&l.head) {
            areas.push(Area {
                rings: vec![ring],
                fill: 1.0,
                width: 1.0,
                dash: None,
                fill_tone: Tone::Accent,
            });
        }
        // The note's place, four of its heights wide, unless its field stands there.
        if self.stage != Stage::Typing
            && let (Some(at), Some(align)) = (l.note_point, l.note_align)
        {
            let h = self.box_height;
            let w = h * 4.0;
            let x0 = if align.along() > 0.5 { at.x - w } else { at.x };
            strokes.push(Stroke {
                pts: vec![
                    Vec2::new(x0, at.y - h / 2.0),
                    Vec2::new(x0 + w, at.y - h / 2.0),
                    Vec2::new(x0 + w, at.y + h / 2.0),
                    Vec2::new(x0, at.y + h / 2.0),
                ],
                closed: true,
                dash: Some([3.0, 3.0]),
                width: 1.0,
                tone: Tone::Accent,
            });
        }
        Preview {
            strokes,
            areas,
            ..Preview::default()
        }
    }
}
