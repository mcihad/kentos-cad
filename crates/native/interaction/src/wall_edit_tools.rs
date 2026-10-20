//! The wall edits by a click (docs/adr/0340 §6, plan/cad.md `CAD-ARCH-07`, `10`): each writes through
//! `cad.walls.edit` in one undo step “Duvarı düzenle”; the command computes, the tool only says what was clicked.
//!
//! - Duvarı böl: each click on a wall splits it there.
//! - Duvarları birleştir: the walls selected first, or clicked one by one, made one on Enter.
//! - Duvarı uzat: the wall to reach first, then each click on a wall runs its nearer end on to it.
//! - Duvarı buda: the cutting wall first, then each click on a wall keeps the part clicked.
//! - Duvarı çevir: the walls selected first, or each one clicked, the other way round.
//! - Stili değiştir: a click on a wall names its style; Stil (S) gives every wall of it (Tümü, T) or the one clicked
//!   another style.
//! - Duvar birleşimi: a click near a wall's end picks that end; Otomatik (O), Yok (Y), Gönye (G), Alın (A), Geçen
//!   (E) set its join, Temizlik (T) the wall's cleanup, Sıfırla (R) both ends and the cleanup back to the rule.

use kentos_commands::{Entity, WallEnd, WallJoin, WallsEdit, WallsEdited, WallsOperation};
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, walls_edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};

pub const SPLIT_ID: &str = "wallSplit";
pub const JOIN_ID: &str = "wallJoin";
pub const EXTEND_ID: &str = "wallExtend";
pub const TRIM_ID: &str = "wallTrim";
pub const FLIP_ID: &str = "wallFlip";
pub const RESTYLE_ID: &str = "wallRestyle";
pub const JOINS_ID: &str = "wallJoins";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Split,
    Join,
    Extend,
    Trim,
    Flip,
    Restyle,
    Joins,
}

impl Action {
    fn id(self) -> &'static str {
        match self {
            Action::Split => SPLIT_ID,
            Action::Join => JOIN_ID,
            Action::Extend => EXTEND_ID,
            Action::Trim => TRIM_ID,
            Action::Flip => FLIP_ID,
            Action::Restyle => RESTYLE_ID,
            Action::Joins => JOINS_ID,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Action::Split => "Duvarı böl",
            Action::Join => "Duvarları birleştir",
            Action::Extend => "Duvarı uzat",
            Action::Trim => "Duvarı buda",
            Action::Flip => "Duvarı çevir",
            Action::Restyle => "Stili değiştir",
            Action::Joins => "Duvar birleşimi",
        }
    }

    /// Whether the first click names a wall the next ones act against (the wall to reach, the cutting wall, the style).
    fn targets(self) -> bool {
        matches!(self, Action::Extend | Action::Trim | Action::Restyle)
    }
}

/// The joins Duvar birleşimi offers, with their keys and icons.
const JOINS: [(WallJoin, &str, &str); 5] = [
    (WallJoin::Auto, "O", "wallJoinAuto"),
    (WallJoin::None, "Y", "wallJoinNone"),
    (WallJoin::Miter, "G", "wallJoinMiter"),
    (WallJoin::Butt, "A", "wallJoinButt"),
    (WallJoin::Through, "E", "wallJoinThrough"),
];

#[derive(Clone, Debug)]
pub struct WallEditTool {
    action: Action,
    /// The wall the clicks act against (Uzat, Buda), or whose style is replaced (Stili değiştir).
    target: Option<Slot>,
    /// Birleştir's walls so far, in click order.
    gathered: Vec<Slot>,
    /// Birleşim's wall and end (none: a closed wall, its cleanup only).
    end: Option<(Slot, Option<WallEnd>)>,
    /// Stili değiştir: every wall of the style, or the one clicked.
    all: bool,
    hover: Option<(Slot, Vec2)>,
    /// The walls picked, ringed: the target dashed, the gathered and the chosen end's wall solid, the end's square;
    /// drawn again after each event (`see`).
    drawn: Preview,
    /// The styles Stil offers: the project's and KentOS's own, by name.
    offer: Vec<String>,
    done: bool,
}

impl WallEditTool {
    fn with(action: Action) -> Self {
        Self {
            action,
            target: None,
            gathered: Vec::new(),
            end: None,
            all: true,
            hover: None,
            drawn: Preview::default(),
            offer: Vec::new(),
            done: false,
        }
    }

    /// The rings and the styles offered, as the drawing is now.
    fn see(&mut self, cx: &Context<'_>) {
        self.offer = crate::wall::offered(cx.doc.settings())
            .into_iter()
            .map(|s| s.name)
            .collect();
        let mut out = Preview::default();
        let mut ring = |slot: Slot, dash: Option<[f32; 2]>| {
            if let Some(e) = cx.doc.get(slot) {
                let o = Outline::of(&shape(e), dash, 2.5, Tone::Accent);
                out.strokes.extend(o.strokes);
                out.marks.extend(o.marks);
            }
        };
        if let Some(t) = self.target {
            ring(t, Some([6.0, 4.0]));
        }
        for s in &self.gathered {
            ring(*s, None);
        }
        if let Some((slot, end)) = self.end {
            ring(slot, None);
            if let (Some(Entity::Wall(w)), Some(end)) = (cx.doc.get(slot), end) {
                let p = match end {
                    WallEnd::Start => w.wall.pts[0],
                    WallEnd::End => w.wall.pts[w.wall.pts.len() - 1],
                };
                out.squares.push(Vec2::new(p.x, p.y));
            }
        }
        self.drawn = out;
    }

    pub fn split() -> Self {
        Self::with(Action::Split)
    }
    pub fn join() -> Self {
        Self::with(Action::Join)
    }
    pub fn extend() -> Self {
        Self::with(Action::Extend)
    }
    pub fn trim() -> Self {
        Self::with(Action::Trim)
    }
    pub fn flip() -> Self {
        Self::with(Action::Flip)
    }
    pub fn restyle() -> Self {
        Self::with(Action::Restyle)
    }
    pub fn joins() -> Self {
        Self::with(Action::Joins)
    }

    fn step(&self) -> &'static str {
        match (self.action, self.target.is_some()) {
            (Action::Split, _) => "duvarı bölünecek yerine tıklayın",
            (Action::Join, _) => "uç uca duvarlara tıklayın",
            (Action::Extend, false) => "uzatılacak duvarların varacağı duvara tıklayın",
            (Action::Extend, true) => "uzatılacak duvara ucunun yanından tıklayın",
            (Action::Trim, false) => "kesen duvara tıklayın",
            (Action::Trim, true) => "budanacak duvarın kalacak parçasına tıklayın",
            (Action::Flip, _) => "çevrilecek duvara tıklayın",
            (Action::Restyle, false) => "stili değişecek duvara tıklayın",
            (Action::Restyle, true) => "yeni stili seçin",
            (Action::Joins, _) if self.end.is_some() => {
                "birleşimi seçin ya da başka bir uca tıklayın"
            }
            (Action::Joins, _) => "birleşimi değişecek duvar ucuna tıklayın",
        }
    }

    fn uid(cx: &Context<'_>, slot: Slot) -> Option<String> {
        cx.doc.uid(slot).map(|u| u.to_string())
    }

    /// The wall under the pointer, or none (said when `say`).
    fn wall_at(&self, at: Vec2, cx: &mut Context<'_>, say: bool) -> Option<Slot> {
        let slot = cx
            .spatial
            .hits(at, cx.pick_tolerance())
            .into_iter()
            .find(|s| matches!(cx.doc.get(*s), Some(Entity::Wall(_))));
        if slot.is_none() && say {
            cx.say(
                Level::Warn,
                format!("{}: bir duvara tıklayın.", self.action.label()),
            );
        }
        slot
    }

    fn run(
        &self,
        uids: Vec<String>,
        operation: WallsOperation,
        cx: &mut Context<'_>,
    ) -> Option<WallsEdited> {
        let input = WallsEdit {
            uids,
            operation,
            expected_revision: None,
        };
        let result = walls_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        points::written(result, cx)
    }

    /// The end of the wall in `slot` nearer `at`; none for a closed wall.
    fn end_near(cx: &Context<'_>, slot: Slot, at: Vec2) -> Option<WallEnd> {
        let Some(Entity::Wall(w)) = cx.doc.get(slot) else {
            return None;
        };
        if w.wall.closed {
            return None;
        }
        let d = |p: &kentos_commands::Vec2| (p.x - at.x).hypot(p.y - at.y);
        let (first, last) = (&w.wall.pts[0], &w.wall.pts[w.wall.pts.len() - 1]);
        Some(if d(last) <= d(first) {
            WallEnd::End
        } else {
            WallEnd::Start
        })
    }

    fn write_joins(&mut self, operation: WallsOperation, cx: &mut Context<'_>) {
        let Some((slot, _)) = self.end else {
            cx.say(Level::Warn, "Önce bir duvar ucuna tıklayın.");
            return;
        };
        let Some(uid) = Self::uid(cx, slot) else {
            return;
        };
        if self.run(vec![uid], operation, cx).is_some() {
            cx.say(Level::Success, "Duvar birleşimi değişti.");
        }
    }

    fn join_mode(&mut self, mode: WallJoin, cx: &mut Context<'_>) {
        let Some((_, end)) = self.end else {
            cx.say(Level::Warn, "Önce bir duvar ucuna tıklayın.");
            return;
        };
        let Some(end) = end else {
            cx.say(
                Level::Warn,
                "Kapalı duvarın ucu yok; yalnız Temizlik değişir.",
            );
            return;
        };
        let (start, end) = match end {
            WallEnd::Start => (Some(mode), None),
            WallEnd::End => (None, Some(mode)),
        };
        self.write_joins(
            WallsOperation::Joins {
                start,
                end,
                cleanup: None,
            },
            cx,
        );
    }

    fn give_style(&mut self, to: &str, cx: &mut Context<'_>) {
        let Some(slot) = self.target else { return };
        let Some(Entity::Wall(w)) = cx.doc.get(slot) else {
            return;
        };
        let Some(from) = w.wall.style.clone() else {
            cx.say(
                Level::Warn,
                "Duvarın stili yok; stilsiz duvara Öznitelikler'den stil verin.",
            );
            return;
        };
        let uids = if self.all {
            Vec::new()
        } else {
            Self::uid(cx, slot).into_iter().collect()
        };
        if let Some(done) = self.run(
            uids,
            WallsOperation::ReplaceStyle {
                from,
                to: to.to_owned(),
            },
            cx,
        ) {
            cx.say(
                Level::Success,
                format!("{}: {} duvar.", self.action.label(), done.walls.len()),
            );
            self.target = None;
        }
    }
}

impl Tool for WallEditTool {
    fn id(&self) -> &'static str {
        self.action.id()
    }

    fn label(&self) -> &'static str {
        self.action.label()
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    /// Duvarı böl takes the snapped point (a face's middle, a vertex); the others pick.
    fn snaps(&self) -> bool {
        self.action == Action::Split
    }

    fn prompt(&self) -> Prompt {
        let prompt = Prompt::new(self.action.label(), self.step());
        match self.action {
            Action::Join => prompt
                .note(format!("{} duvar", self.gathered.len()))
                .option("Birleştir", "Enter"),
            Action::Restyle if self.target.is_some() => {
                prompt.option("Stil", "S").toggle("Tümü", "T", self.all)
            }
            Action::Restyle => prompt.toggle("Tümü", "T", self.all),
            Action::Joins if self.end.is_some() => {
                let mut p = prompt;
                for (j, key, _) in JOINS {
                    p = p.option(j.label(), key);
                }
                p.option("Temizlik", "T")
                    .option("Sıfırla", "R")
                    .option("Bitir", "Enter")
            }
            _ => prompt.option("Bitir", "Enter"),
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.target.is_some()) + self.gathered.len()
    }

    /// Walls selected first: Birleştir joins them, Çevir turns them, and the tool leaves.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        if !matches!(self.action, Action::Join | Action::Flip) {
            self.see(cx);
            return Flow::Stay;
        }
        let walls: Vec<String> = cx
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(cx.doc.get(**s), Some(Entity::Wall(_))))
            .filter_map(|s| Self::uid(cx, *s))
            .collect();
        let enough = if self.action == Action::Join { 2 } else { 1 };
        if walls.len() < enough {
            self.see(cx);
            return Flow::Stay;
        }
        let op = if self.action == Action::Join {
            WallsOperation::Join
        } else {
            WallsOperation::Flip
        };
        if let Some(done) = self.run(walls, op, cx) {
            cx.say(
                Level::Success,
                format!("{}: {} duvar.", self.action.label(), done.walls.len()),
            );
            cx.selection.clear();
        }
        Flow::Exit
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let hit = self.wall_at(p.raw, cx, false);
        cx.selection.set_hover(hit);
        self.hover = hit.map(|s| (s, p.raw));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = self.wall_at(p.raw, cx, true) else {
            return;
        };
        let Some(uid) = Self::uid(cx, slot) else {
            return;
        };
        match self.action {
            Action::Split => {
                let at = points::wire(p.world);
                if self
                    .run(vec![uid], WallsOperation::Split { at }, cx)
                    .is_some()
                {
                    cx.say(Level::Success, "Duvar bölündü.");
                }
            }
            Action::Join => {
                if !self.gathered.contains(&slot) {
                    self.gathered.push(slot);
                }
            }
            Action::Flip => {
                if self.run(vec![uid], WallsOperation::Flip, cx).is_some() {
                    cx.say(Level::Success, "Duvar çevrildi.");
                }
            }
            Action::Extend | Action::Trim | Action::Restyle if self.target.is_none() => {
                if self.action == Action::Restyle
                    && !matches!(cx.doc.get(slot), Some(Entity::Wall(w)) if w.wall.style.is_some())
                {
                    cx.say(
                        Level::Warn,
                        "Duvarın stili yok; stilli bir duvara tıklayın.",
                    );
                    return;
                }
                self.target = Some(slot);
            }
            Action::Extend | Action::Trim => {
                let Some(to) = self.target.and_then(|t| Self::uid(cx, t)) else {
                    return;
                };
                if Some(slot) == self.target {
                    cx.say(
                        Level::Warn,
                        "Bu duvar varılacak duvarın kendisi; başka bir duvara tıklayın.",
                    );
                    return;
                }
                let op = if self.action == Action::Extend {
                    WallsOperation::Extend {
                        to,
                        end: Self::end_near(cx, slot, p.raw),
                    }
                } else {
                    WallsOperation::Trim {
                        to,
                        keep: Some(points::wire(p.raw)),
                    }
                };
                if self.run(vec![uid], op, cx).is_some() {
                    let said = if self.action == Action::Extend {
                        "Duvar uzatıldı."
                    } else {
                        "Duvar budandı."
                    };
                    cx.say(Level::Success, said);
                }
            }
            Action::Restyle => self.target = Some(slot),
            Action::Joins => self.end = Some((slot, Self::end_near(cx, slot, p.raw))),
        }
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let taken = self.typed(text, cx);
        self.see(cx);
        taken
    }

    fn takes_words(&self) -> bool {
        self.action == Action::Restyle && self.target.is_some()
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if self.action != Action::Restyle || key != "S" {
            return Vec::new();
        }
        self.offer
            .iter()
            .map(|n| OptionChoice {
                typed: n.clone(),
                label: n.clone(),
                icon: None,
                preview: None,
                checked: false,
                command: None,
            })
            .collect()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if self.action == Action::Restyle && key == "S" {
            self.give_style(typed, cx);
            self.see(cx);
            return true;
        }
        false
    }

    /// Enter: Birleştir joins what it gathered; the others leave.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.action == Action::Join && self.gathered.len() >= 2 {
            let uids: Vec<String> = self
                .gathered
                .iter()
                .filter_map(|s| Self::uid(cx, *s))
                .collect();
            if let Some(done) = self.run(uids, WallsOperation::Join, cx) {
                cx.say(
                    Level::Success,
                    format!(
                        "Duvarlar birleşti: {} duvar tek duvar oldu.",
                        done.removed.len() + 1
                    ),
                );
                self.gathered.clear();
            }
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Esc: the gathered walls, the target or the chosen end go first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.gathered.is_empty() || self.target.is_some() || self.end.is_some() {
            self.gathered.clear();
            self.target = None;
            self.end = None;
            self.see(cx);
            return true;
        }
        false
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let took = self.gathered.pop().is_some()
            || (self.action.targets() && self.target.take().is_some());
        if took {
            self.see(cx);
        }
        took
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            tag: self.hover.map(|(_, at)| Tag {
                at,
                lines: vec![self.action.label().to_owned(), self.step().to_owned()],
            }),
            ..self.drawn.clone()
        }
    }
}

impl WallEditTool {
    /// Typed text: Stili değiştir's Tümü and a style's name, Duvar birleşimi's keys.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        match self.action {
            Action::Restyle if key == "T" => {
                self.all = !self.all;
                true
            }
            Action::Restyle if self.target.is_some() && !key.is_empty() && key != "S" => {
                self.give_style(js_trim(text), cx);
                true
            }
            Action::Joins => {
                if let Some((j, _, _)) = JOINS.iter().find(|(_, k, _)| *k == key) {
                    self.join_mode(*j, cx);
                    return true;
                }
                match key.as_str() {
                    "T" => {
                        let on = self
                            .end
                            .and_then(|(s, _)| match cx.doc.get(s) {
                                Some(Entity::Wall(w)) => Some(w.wall.cleanup),
                                _ => None,
                            })
                            .unwrap_or(true);
                        self.write_joins(
                            WallsOperation::Joins {
                                start: None,
                                end: None,
                                cleanup: Some(!on),
                            },
                            cx,
                        );
                        true
                    }
                    "R" => {
                        self.write_joins(WallsOperation::ResetJoins, cx);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}
