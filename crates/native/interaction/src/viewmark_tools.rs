//! Kesit, Görünüş, Detay, Döşeme deseni and Tavan ızgarası (docs/adr/0345 §5, plan/cad.md `CAD-ARCH-37`–`40`). The
//! tools say what was clicked and chosen; `cad.viewmarks.create` and `cad.spaces.finish` compute and write, one undo
//! step each, and their plan is the preview.
//!
//! - Kesit: the cut line's points, Enter ends it (two or more); Çevir (Ç) turns the look, Numara (N), Hedef (H).
//! - Görünüş: its point, then where it looks (Enter: north); Numara (N), Hedef (H).
//! - Detay: its circle's middle, its radius, then its bubble's place (Enter: up and to the right); Numara, Hedef.
//! - Döşeme deseni and Tavan ızgarası: a click inside a space writes its finish, the tool stays; Desen (D) and Aralık
//!   (A), Başlangıç (B) for the grid.
//!
//! Hedef steps through the project's targets (`cad.viewmarks.references`): none, the named views, the sheets' map
//! frames. A mark written with a number that ends in a digit or is one letter gives the next mark the next number.

use kentos_commands::{
    CommandResult, Entity, FinishGrid, FinishSurface, GridOrigin, NewViewMark, SpacesFinish,
    ViewMarkForm, ViewMarksCreate, ViewTarget,
};
use kentos_domain::Slot;
use kentos_geometry_core::geom::hatch_pattern::pattern_pieces;
use kentos_geometry_core::jsmath::{atan2, js_hypot};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{core_pattern, shape};
use kentos_native_application::viewmarks::references_of;
use kentos_native_application::{ExecutionContext, spaces_finish, viewmarks_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, TextGhost, Tone, Tool};

pub const SECTION_ID: &str = "viewSection";
pub const ELEVATION_ID: &str = "viewElevation";
pub const DETAIL_ID: &str = "viewDetail";
pub const FLOOR_ID: &str = "spaceFloor";
pub const CEILING_ID: &str = "spaceCeiling";

/// The most pieces a finish's preview draws.
const PREVIEW_PIECES: usize = 3000;

fn plan_words<P>(result: CommandResult<P>) -> Result<P, String> {
    match result {
        CommandResult::Completed { output, .. } => Ok(output),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        CommandResult::Queued { .. } | CommandResult::Cancelled => Err(String::new()),
    }
}

/// A mark drawn as the preview: its lines, circles and arrows as strokes, its words as ghost texts.
fn mark_drawn(e: &Entity, scale: f64, out: &mut Preview) {
    for piece in kentos_geometry_core::geom::arch::pieces(&shape(e), scale) {
        match &piece.shape {
            kentos_geometry_core::entity::Shape::Text {
                p, text, height, ..
            } => out.texts.push(TextGhost {
                p: *p,
                text: text.clone(),
                height: *height,
                rotation: 0.0,
                align: Some(kentos_geometry_core::text::TextAlign::MiddleCenter),
                mask: false,
                face: Default::default(),
                width_factor: 1.0,
            }),
            other => out
                .strokes
                .extend(Outline::of(other, None, 1.5, Tone::Accent).strokes),
        }
    }
}

/// The number after `mark`: the next letter of a single letter, one more of a trailing number; else the same.
pub fn next_mark(mark: &str) -> String {
    let chars: Vec<char> = mark.chars().collect();
    if let [c] = chars.as_slice()
        && c.is_ascii_uppercase()
        && *c != 'Z'
    {
        return char::from(*c as u8 + 1).to_string();
    }
    let digits = chars.iter().rev().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return mark.to_owned();
    }
    let (head, tail) = chars.split_at(chars.len() - digits);
    let n: u64 = tail.iter().collect::<String>().parse().unwrap_or(0);
    format!("{}{}", head.iter().collect::<String>(), n + 1)
}

/// What a mark tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Mark,
}

/// Kesit, Görünüş or Detay.
#[derive(Clone, Debug)]
pub struct ViewMarkTool {
    form: ViewMarkForm,
    pts: Vec<Vec2>,
    /// A detail's radius once given.
    radius: Option<f64>,
    flip: bool,
    mark: String,
    /// The target chosen: 0 none, then the project's targets in `cad.viewmarks.references`'s order.
    target: usize,
    /// The target's name as the prompt shows it.
    target_name: String,
    ask: Option<Ask>,
    hover: Option<Vec2>,
    drawn: Preview,
    problem: Option<String>,
}

impl ViewMarkTool {
    pub fn new(form: ViewMarkForm) -> Self {
        Self {
            form,
            pts: Vec::new(),
            radius: None,
            flip: false,
            mark: match form {
                ViewMarkForm::Section => "A".into(),
                _ => "1".into(),
            },
            target: 0,
            target_name: "Yok".into(),
            ask: None,
            hover: None,
            drawn: Preview::default(),
            problem: None,
        }
    }

    /// The project's targets and the one chosen (none when the list got shorter).
    fn targets(cx: &Context<'_>) -> Vec<(ViewTarget, String)> {
        references_of(cx.doc)
            .targets
            .into_iter()
            .map(|t| (t.target, t.name))
            .collect()
    }

    fn chosen(&self, cx: &Context<'_>) -> Option<(ViewTarget, String)> {
        self.target
            .checked_sub(1)
            .and_then(|k| Self::targets(cx).into_iter().nth(k))
    }

    /// The mark as the call would give it, with `last` for the point to come (the cursor's, or none).
    fn made(&self, last: Option<Vec2>, cx: &Context<'_>) -> Option<ViewMarksCreate> {
        let mut pts = self.pts.clone();
        let (mut radius, mut at, mut angle) = (None, None, None);
        match self.form {
            ViewMarkForm::Section => {
                pts.extend(last);
                if pts.len() < 2 {
                    return None;
                }
            }
            ViewMarkForm::Elevation => {
                let c = *pts.first()?;
                if let Some(p) = last
                    && js_hypot(p.x - c.x, p.y - c.y) > 0.0
                {
                    angle = Some(atan2(p.y - c.y, p.x - c.x).to_degrees());
                }
            }
            ViewMarkForm::Detail => {
                let c = *pts.first()?;
                radius = match (self.radius, last) {
                    (Some(r), _) => Some(r),
                    (None, Some(p)) => Some(js_hypot(p.x - c.x, p.y - c.y)),
                    (None, None) => None,
                };
                if self.radius.is_some() {
                    at = last.map(wire);
                }
                radius?;
            }
        }
        Some(ViewMarksCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            marks: vec![NewViewMark {
                form: self.form,
                points: points::wire_all(&pts),
                radius,
                at,
                angle,
                flip: self.flip,
                mark: self.mark.clone(),
                target: self.chosen(cx).map(|(t, _)| t),
                size: None,
            }],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.problem = None;
        if let ViewMarkForm::Section = self.form {
            self.drawn.path = self.pts.iter().copied().chain(self.hover).collect();
        }
        let Some(input) = self.made(self.hover, cx) else {
            return;
        };
        match plan_words(viewmarks_create::plan_in(cx.doc, &input)) {
            Ok(plan) => {
                for e in &plan.marks {
                    mark_drawn(e, cx.doc.settings().plot_scale, &mut self.drawn);
                }
            }
            Err(why) => self.problem = Some(why),
        }
    }

    fn write(&mut self, last: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(input) = self.made(last, cx) else {
            return;
        };
        let result = viewmarks_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            cx.say(
                Level::Success,
                "Görünüm işareti yazıldı; hedefi yeniden adlandırılınca yazısı da değişir.",
            );
            self.mark = next_mark(&self.mark);
        }
        self.pts.clear();
        self.radius = None;
        self.see(cx);
    }

    fn step(&self) -> &'static str {
        match (self.form, self.pts.len(), self.radius) {
            (ViewMarkForm::Section, 0, _) => "kesme çizgisinin ilk noktasına tıklayın",
            (ViewMarkForm::Section, _, _) => "sonraki noktaya tıklayın, Enter ile bitirin",
            (ViewMarkForm::Elevation, 0, _) => "işaretin yerine tıklayın",
            (ViewMarkForm::Elevation, _, _) => "bakış yönüne tıklayın, Enter ile kuzeye",
            (ViewMarkForm::Detail, 0, _) => "detayın ortasına tıklayın",
            (ViewMarkForm::Detail, _, None) => "detayın sınırına tıklayın ya da yarıçapı yazın",
            (ViewMarkForm::Detail, _, Some(_)) => "balonun yerine tıklayın, Enter ile sağ üste",
        }
    }
}

impl Tool for ViewMarkTool {
    fn id(&self) -> &'static str {
        match self.form {
            ViewMarkForm::Section => SECTION_ID,
            ViewMarkForm::Elevation => ELEVATION_ID,
            ViewMarkForm::Detail => DETAIL_ID,
        }
    }

    fn label(&self) -> &'static str {
        self.form.label()
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snaps(&self) -> bool {
        true
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn prompt(&self) -> Prompt {
        if self.ask == Some(Ask::Mark) {
            return Prompt::new(
                self.form.label(),
                "işaretin numarasını yazın (boş: paftadaki penceresinin sırası)",
            )
            .option("Geri", "G");
        }
        let mut p = Prompt::new(self.form.label(), self.step()).option_with(
            "Numara",
            "N",
            if self.mark.is_empty() {
                "Sıra".to_owned()
            } else {
                self.mark.clone()
            },
        );
        if self.form == ViewMarkForm::Section {
            p = p.option_with(
                "Çevir",
                "Ç",
                if self.flip { "Sağa" } else { "Sola" }.to_owned(),
            );
        }
        p.option_with("Hedef", "H", self.target_name.clone())
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.click(p.world, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.typed(text, cx)
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            return true;
        }
        if self.radius.take().is_some() || self.pts.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        match (self.form, self.pts.len()) {
            (_, 0) => Flow::Exit,
            (ViewMarkForm::Section, 1) => Flow::Stay,
            (ViewMarkForm::Detail, _) if self.radius.is_none() => Flow::Stay,
            _ => {
                self.write(None, cx);
                Flow::Stay
            }
        }
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
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

impl ViewMarkTool {
    /// A point given, by a click or typed.
    fn click(&mut self, at: Vec2, cx: &mut Context<'_>) {
        match (self.form, self.pts.len(), self.radius) {
            (ViewMarkForm::Section, _, _) => {
                if self.pts.last() != Some(&at) {
                    self.pts.push(at);
                }
            }
            (_, 0, _) => self.pts.push(at),
            (ViewMarkForm::Elevation, _, _) => return self.write(Some(at), cx),
            (ViewMarkForm::Detail, _, None) => {
                let c = self.pts[0];
                let r = js_hypot(at.x - c.x, at.y - c.y);
                if r > 0.0 {
                    self.radius = Some(r);
                }
            }
            (ViewMarkForm::Detail, _, Some(_)) => return self.write(Some(at), cx),
        }
        self.see(cx);
    }

    /// A key or a value typed.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if self.ask == Some(Ask::Mark) {
            if key != "G" {
                self.mark = js_trim(text).to_owned();
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "N" => self.ask = Some(Ask::Mark),
            "Ç" | "C" if self.form == ViewMarkForm::Section => {
                self.flip = !self.flip;
                self.see(cx);
            }
            "H" => {
                let all = Self::targets(cx);
                self.target = (self.target + 1) % (all.len() + 1);
                self.target_name = self.chosen(cx).map_or_else(|| "Yok".to_owned(), |(_, name)| name);
                self.see(cx);
            }
            _ => {
                // A detail's radius typed.
                if self.form == ViewMarkForm::Detail
                    && self.pts.len() == 1
                    && self.radius.is_none()
                    && let Some(r) = cx.typed_length(text)
                    && r > 0.0
                {
                    self.radius = Some(r);
                    self.see(cx);
                    return true;
                }
                if let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover) {
                    self.click(p, cx);
                    return true;
                }
                return false;
            }
        }
        true
    }
}

/// Whether the point is inside the rings by the even-odd rule.
fn inside(rings: &[Vec<Vec2>], p: Vec2) -> bool {
    let mut odd = false;
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                odd = !odd;
            }
        }
    }
    odd
}

// ── Döşeme deseni and Tavan ızgarası ───────────────────────────────────────────────────────────────────────────

/// What a finish tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FinishAsk {
    Pattern,
    Spacing,
}

/// Döşeme deseni or Tavan ızgarası.
#[derive(Clone, Debug)]
pub struct FinishTool {
    surface: FinishSurface,
    pattern: Option<String>,
    dx: f64,
    dy: f64,
    origin: GridOrigin,
    ask: Option<FinishAsk>,
    space: Option<Slot>,
    hover: Option<Vec2>,
    drawn: Preview,
    problem: Option<String>,
}

impl FinishTool {
    pub fn new(surface: FinishSurface) -> Self {
        Self {
            surface,
            pattern: None,
            dx: 0.6,
            dy: 0.6,
            origin: GridOrigin::Center,
            ask: None,
            space: None,
            hover: None,
            drawn: Preview::default(),
            problem: None,
        }
    }

    /// The space under the point: the smallest whose rings hold it, on a shown layer.
    fn space_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        let mut best: Option<(f64, Slot)> = None;
        for e in cx.doc.entities() {
            let Entity::Space(s) = e else { continue };
            if !cx.doc.layers().is_visible(&e.base().layer_id) {
                continue;
            }
            let rings: Vec<Vec<Vec2>> = s
                .space
                .parts
                .iter()
                .flat_map(|p| std::iter::once(&p.outer).chain(&p.holes))
                .map(|r| r.pts.iter().map(|q| Vec2::new(q.x, q.y)).collect())
                .collect();
            if inside(&rings, at) {
                let a = kentos_native_application::spaces::area_of(&s.space.parts);
                if best.is_none_or(|(b, _)| a < b) {
                    best = Some((a, Slot(e.base().id)));
                }
            }
        }
        best.map(|(_, s)| s)
    }

    fn made(&self, cx: &Context<'_>) -> Option<SpacesFinish> {
        let uid = cx.doc.uid(self.space?)?.to_string();
        let ceiling = self.surface == FinishSurface::Ceiling;
        Some(SpacesFinish {
            uids: vec![uid],
            surface: self.surface,
            pattern: if ceiling { None } else { self.pattern.clone() },
            scale: None,
            grid: ceiling.then_some(FinishGrid {
                dx: self.dx,
                dy: self.dy,
                angle: 0.0,
                origin: self.origin,
            }),
            layer_id: None,
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.problem = None;
        let Some(input) = self.made(cx) else {
            return;
        };
        match plan_words(spaces_finish::plan_in(cx.doc, &input)) {
            Ok(plan) => {
                for e in &plan.hatches {
                    let Entity::Hatch(h) = e else { continue };
                    let ring: Vec<Vec2> = h.ring.iter().map(|q| Vec2::new(q.x, q.y)).collect();
                    let holes: Vec<Vec<Vec2>> = h
                        .holes
                        .iter()
                        .flatten()
                        .map(|r| r.iter().map(|q| Vec2::new(q.x, q.y)).collect())
                        .collect();
                    let pieces = pattern_pieces(&ring, &holes, &core_pattern(&h.pattern), PREVIEW_PIECES);
                    if !pieces.capped {
                        self.drawn.hatch.extend(pieces.segments);
                    }
                    self.drawn.ring = Some(ring);
                }
            }
            Err(why) => self.problem = Some(why),
        }
    }

    fn title(&self) -> &'static str {
        match self.surface {
            FinishSurface::Floor => "Döşeme deseni",
            FinishSurface::Ceiling => "Tavan ızgarası",
        }
    }
}

impl Tool for FinishTool {
    fn id(&self) -> &'static str {
        match self.surface {
            FinishSurface::Floor => FLOOR_ID,
            FinishSurface::Ceiling => CEILING_ID,
        }
    }

    fn label(&self) -> &'static str {
        self.title()
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
        match self.ask {
            Some(FinishAsk::Pattern) => {
                return Prompt::new(self.title(), "desenin adını yazın (NET, ANSI31, BRICK …; boş: mahalin kaplaması)")
                    .option("Geri", "G");
            }
            Some(FinishAsk::Spacing) => {
                return Prompt::new(self.title(), "ızgaranın aralığını yazın (0.6 ya da 0.6x1.2)").option("Geri", "G");
            }
            None => {}
        }
        let p = Prompt::new(self.title(), "kaplanacak mahalin içine tıklayın");
        match self.surface {
            FinishSurface::Floor => p.option_with(
                "Desen",
                "D",
                self.pattern.clone().unwrap_or_else(|| "Kaplamaya göre".into()),
            ),
            FinishSurface::Ceiling => p
                .option_with("Aralık", "A", format!("{} × {}", self.dx, self.dy))
                .option_with(
                    "Başlangıç",
                    "B",
                    match self.origin {
                        GridOrigin::Center => "Orta",
                        GridOrigin::Corner => "Köşe",
                    }
                    .to_owned(),
                ),
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        let space = Self::space_at(p.raw, cx);
        if space != self.space {
            self.space = space;
            cx.selection.set_hover(space);
            self.see(cx);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.space = Self::space_at(p.raw, cx);
        let Some(input) = self.made(cx) else {
            cx.say(Level::Info, "Burada mahal yok; bir mahalin içine tıklayın.");
            return;
        };
        let result = spaces_finish::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            cx.say(
                Level::Success,
                format!("{} yazıldı; mahal değişince o da değişir.", self.title()),
            );
        }
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        match self.ask.take() {
            Some(FinishAsk::Pattern) => {
                if key != "G" {
                    let t = js_trim(text);
                    self.pattern = (!t.is_empty()).then(|| t.to_owned());
                }
                self.see(cx);
                return true;
            }
            Some(FinishAsk::Spacing) => {
                if key != "G" {
                    let parts: Vec<f64> = key
                        .split(['X', '×', ' ', '*'])
                        .filter(|s| !s.is_empty())
                        .filter_map(|s| cx.typed_length(s))
                        .collect();
                    match parts.as_slice() {
                        [d] if *d > 0.0 => (self.dx, self.dy) = (*d, *d),
                        [a, b] if *a > 0.0 && *b > 0.0 => (self.dx, self.dy) = (*a, *b),
                        _ => cx.say(Level::Warn, "Aralık okunamadı: 0.6 ya da 0.6x1.2 yazın."),
                    }
                }
                self.see(cx);
                return true;
            }
            None => {}
        }
        match (self.surface, key.as_str()) {
            (FinishSurface::Floor, "D") => self.ask = Some(FinishAsk::Pattern),
            (FinishSurface::Ceiling, "A") => self.ask = Some(FinishAsk::Spacing),
            (FinishSurface::Ceiling, "B") => {
                self.origin = match self.origin {
                    GridOrigin::Center => GridOrigin::Corner,
                    GridOrigin::Corner => GridOrigin::Center,
                };
                self.see(cx);
            }
            _ => return false,
        }
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.ask.take().is_some()
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
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
