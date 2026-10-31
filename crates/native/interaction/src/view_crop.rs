//! Kırpma çerçevesi (docs/adr/0789 §2; Revit's crop region grips and Size Crop, Archicad's drawing frame): the named
//! views' areas drawn on the model as frames, the chosen one with nine grips: four corners, four edges' middles and the
//! middle. A grip dragged (pressed, moved, let go), or pressed, let go and pressed again where it goes, gives the view
//! its new area; a corner moves two edges, an edge one, the opposite ones staying; the middle moves the whole frame. The
//! view's twist turns the frame and its edges. A typed point while a grip is held goes there; `12x8` gives the width and
//! height about the middle; Öznitelikler's Genişlik and Yükseklik do so too. Each change is one `cad.views.edit` `area`
//! call, one undo step “Görünümün alanı”; the sheets' maps linked to the view follow. A click on another frame chooses
//! that view. Esc lets a held grip go, then the view, then the tool; Ctrl+Z with no grip held takes back the last area.

use kentos_commands::{NamedView, Vec2 as Wire, ViewsEdit, ViewsOp};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, views_edit};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Label, Marker, MarkerShape, Pointer, Preview, Stroke, Tone, Tool};
use crate::tool_settings::Setting;

/// The tool's id: its command is `tool.viewCrop`.
pub const ID: &str = "viewCrop";
pub const LABEL: &str = "Kırpma çerçevesi";
/// No named view to edit.
pub const NO_VIEWS: &str =
    "Projede adlı görünüm yok. Görünüm › Görünümler'de çizimi istediğiniz gibi gösterip bir görünüm kaydedin.";
/// A click on no frame.
pub const NO_FRAME: &str = "Tıklanan yerde bir görünümün çerçevesi yok; bir çerçevenin içine ya da kenarına tıklayın.";
/// How near a grip a press takes it, logical pixels.
const GRIP_REACH: f64 = 8.0;
/// A press that moves less than this before it is let go holds the grip for a second press, logical pixels.
const STILL: f64 = 3.0;
/// The least width or height a frame keeps, metres (1 mm).
const LEAST: f64 = 1.0e-3;

/// A view's area as the tool draws it: middle, width, height, twist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub center: Vec2,
    pub width: f64,
    pub height: f64,
    pub twist: f64,
}

impl Frame {
    pub fn of(v: &NamedView) -> Self {
        Self {
            center: Vec2::new(v.center.x, v.center.y),
            width: v.width,
            height: v.height,
            twist: v.twist,
        }
    }

    /// A point of the frame's own axes (x along its width) on the ground.
    fn ground(&self, u: f64, v: f64) -> Vec2 {
        let (s, c) = self.twist.sin_cos();
        Vec2::new(self.center.x + u * c - v * s, self.center.y + u * s + v * c)
    }

    /// A ground point in the frame's own axes.
    fn local(&self, p: Vec2) -> (f64, f64) {
        let (s, c) = self.twist.sin_cos();
        let (dx, dy) = (p.x - self.center.x, p.y - self.center.y);
        (dx * c + dy * s, -dx * s + dy * c)
    }

    /// Its four corners counter-clockwise from the bottom left (the frame's own).
    pub fn ring(&self) -> Vec<Vec2> {
        let (hw, hh) = (self.width / 2.0, self.height / 2.0);
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
            .iter()
            .map(|&(u, v)| self.ground(u, v))
            .collect()
    }

    /// Whether `p` is inside.
    fn holds(&self, p: Vec2) -> bool {
        let (u, v) = self.local(p);
        u.abs() <= self.width / 2.0 && v.abs() <= self.height / 2.0
    }

    /// Where a grip is on the ground.
    fn grip_at(&self, g: Grip) -> Vec2 {
        self.ground(f64::from(g.0) * self.width / 2.0, f64::from(g.1) * self.height / 2.0)
    }

    /// The frame with grip `g` moved from `from` to `to` (ground points): the opposite edges stay, the middle moves the
    /// whole frame; never narrower than 1 mm.
    pub fn dragged(&self, g: Grip, from: Vec2, to: Vec2) -> Frame {
        if g == Grip::MIDDLE {
            return Frame {
                center: Vec2::new(self.center.x + to.x - from.x, self.center.y + to.y - from.y),
                ..*self
            };
        }
        let (fu, fv) = self.local(from);
        let (tu, tv) = self.local(to);
        let (du, dv) = (tu - fu, tv - fv);
        let side = |sign: i8, half: f64, d: f64| -> (f64, f64) {
            if sign == 0 {
                return (0.0, 2.0 * half);
            }
            let s = f64::from(sign);
            let fixed = -s * half;
            let moved = s * half + d;
            let size = (s * (moved - fixed)).max(LEAST);
            (fixed + s * size / 2.0, size)
        };
        let (cu, w) = side(g.0, self.width / 2.0, du);
        let (cv, h) = side(g.1, self.height / 2.0, dv);
        Frame {
            center: self.ground(cu, cv),
            width: w,
            height: h,
            twist: self.twist,
        }
    }
}

/// A grip by where it is on the frame: −1, 0 or 1 along the width and the height; (0, 0) the middle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grip(pub i8, pub i8);

impl Grip {
    pub const MIDDLE: Grip = Grip(0, 0);
    /// The nine grips: corners, edges' middles, the middle.
    pub const ALL: [Grip; 9] = [
        Grip(-1, -1),
        Grip(1, -1),
        Grip(1, 1),
        Grip(-1, 1),
        Grip(0, -1),
        Grip(1, 0),
        Grip(0, 1),
        Grip(-1, 0),
        Grip(0, 0),
    ];
}

/// A grip held: where it was taken, the frame then, and whether the pointer has moved since.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Held {
    grip: Grip,
    from: Vec2,
    screen: [f64; 2],
    start: Frame,
    moved: bool,
    /// Let go without moving: the next press puts it.
    waiting: bool,
}

#[derive(Default)]
pub struct ViewCrop {
    /// The project's views as the last event saw them.
    views: Vec<NamedView>,
    chosen: Option<String>,
    held: Option<Held>,
    /// The frame being dragged, as it would be written.
    draft: Option<Frame>,
    hover: Option<Vec2>,
    near: Option<Grip>,
    done: bool,
}

impl ViewCrop {
    pub fn new() -> Self {
        Self::default()
    }

    fn learn(&mut self, cx: &Context<'_>) {
        self.views = cx.doc.settings().named_views.clone();
        if let Some(name) = &self.chosen
            && !self.views.iter().any(|v| &v.name == name)
        {
            self.chosen = None;
            self.held = None;
            self.draft = None;
        }
    }

    fn chosen_view(&self) -> Option<&NamedView> {
        let name = self.chosen.as_deref()?;
        self.views.iter().find(|v| v.name == name)
    }

    /// The chosen view's frame as drawn now: the draft while a grip moves.
    fn frame(&self) -> Option<Frame> {
        self.draft.or_else(|| self.chosen_view().map(Frame::of))
    }

    /// The grip of the chosen frame within reach of the pointer, nearest first.
    fn grip_near(&self, p: &Pointer, cx: &Context<'_>) -> Option<Grip> {
        let f = self.chosen_view().map(Frame::of)?;
        let mut best: Option<(f64, Grip)> = None;
        for g in Grip::ALL {
            let at = cx.view.to_screen(f.grip_at(g));
            let d = (at[0] - p.screen[0]).hypot(at[1] - p.screen[1]);
            if d <= GRIP_REACH && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, g));
            }
        }
        best.map(|(_, g)| g)
    }

    /// The view whose frame is under `p`: the smallest holding it.
    fn frame_at(&self, p: Vec2) -> Option<String> {
        self.views
            .iter()
            .filter(|v| Frame::of(v).holds(p))
            .min_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
            .map(|v| v.name.clone())
    }

    /// Writes `f` as the chosen view's area through `cad.views.edit` (docs/adr/0789 §1).
    fn write(&mut self, f: Frame, cx: &mut Context<'_>) {
        let Some(name) = self.chosen.clone() else {
            return;
        };
        let result = views_edit::execute(
            &mut ExecutionContext::new(cx.doc),
            ViewsEdit {
                edit: ViewsOp::Area {
                    name: name.clone(),
                    center: Wire { x: f.center.x, y: f.center.y },
                    width: f.width,
                    height: f.height,
                    twist: None,
                },
                expected_revision: None,
            },
        );
        self.held = None;
        self.draft = None;
        if let Some(done) = points::written(result, cx) {
            if done.changed {
                let fm = cx.format();
                cx.say(
                    Level::Success,
                    format!("“{name}” görünümünün alanı: {} × {}.", fm.length(f.width), fm.length(f.height)),
                );
            }
            self.learn(cx);
        }
    }

    /// The held grip put at `to`.
    fn put(&mut self, to: Vec2, cx: &mut Context<'_>) {
        if let Some(h) = self.held {
            let f = h.start.dragged(h.grip, h.from, to);
            self.write(f, cx);
        }
    }

    /// `12x8`, `12×8`, `12*8`: the width and the height about the middle.
    fn typed_size(text: &str, cx: &Context<'_>) -> Option<(f64, f64)> {
        let t = js_trim(text);
        let (a, b) = t.split_once(['x', 'X', '×', '*'])?;
        let w = cx.typed_length(js_trim(a))?;
        let h = cx.typed_length(js_trim(b))?;
        (w > 0.0 && h > 0.0).then_some((w, h))
    }
}

impl Tool for ViewCrop {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let step = match (&self.chosen, &self.held) {
            (None, _) => "düzenlenecek görünümün çerçevesine tıklayın",
            (Some(_), Some(h)) if h.waiting => "tutamacın yeni yerine tıklayın ya da noktayı yazın",
            (Some(_), Some(_)) => "tutamacı sürükleyip bırakın",
            (Some(_), None) => "bir tutamacı sürükleyin ya da en×boy yazın (12x8)",
        };
        Prompt::new(LABEL, step).note("Esc: bırak")
    }

    fn point_count(&self) -> usize {
        usize::from(self.chosen.is_some()) + usize::from(self.held.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.learn(cx);
        if self.views.is_empty() {
            cx.say(Level::Warn, NO_VIEWS);
            self.done = true;
            return Flow::Exit;
        }
        self.chosen = cx
            .memory
            .view_crop
            .take()
            .and_then(|i| self.views.get(i).map(|v| v.name.clone()))
            .or_else(|| (self.views.len() == 1).then(|| self.views[0].name.clone()));
        Flow::Stay
    }

    /// Snaps only while a grip moves: an edge can come to a wall's corner.
    fn snaps(&self) -> bool {
        self.held.is_some()
    }

    fn cursor(&self) -> Cursor {
        if self.held.is_some() || self.near.is_some() {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

    fn accepts_points(&self) -> bool {
        self.held.is_some()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.put(p, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        // An undo or another window may have changed the views since.
        if self.held.is_none() {
            self.learn(cx);
        }
        self.hover = Some(p.world);
        self.near = if self.held.is_none() { self.grip_near(p, cx) } else { None };
        if let Some(h) = self.held.as_mut() {
            if (p.screen[0] - h.screen[0]).hypot(p.screen[1] - h.screen[1]) > STILL {
                h.moved = true;
            }
            self.draft = Some(h.start.dragged(h.grip, h.from, p.world));
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.learn(cx);
        if let Some(h) = self.held
            && h.waiting
        {
            self.put(p.world, cx);
            return;
        }
        if let Some(g) = self.grip_near(p, cx)
            && let Some(start) = self.chosen_view().map(Frame::of)
        {
            // A grip's own point is where it is taken: the frame moves by the pointer's travel from it.
            self.held = Some(Held {
                grip: g,
                from: start.grip_at(g),
                screen: p.screen,
                start,
                moved: false,
                waiting: false,
            });
            self.draft = Some(start);
            return;
        }
        match self.frame_at(p.raw) {
            Some(name) => {
                self.chosen = Some(name);
                self.draft = None;
            }
            None => cx.say(Level::Warn, NO_FRAME),
        }
    }

    fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(h) = self.held else {
            return;
        };
        if h.waiting {
            return;
        }
        if h.moved {
            self.put(p.world, cx);
        } else if let Some(h) = self.held.as_mut() {
            h.waiting = true;
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.learn(cx);
        if self.chosen.is_none() {
            return false;
        }
        if self.held.is_some() {
            let Some(p) = cx.typed_point(text, self.held.map(|h| h.from), self.hover) else {
                return false;
            };
            self.put(p, cx);
            return true;
        }
        let Some((w, h)) = Self::typed_size(text, cx) else {
            return false;
        };
        if let Some(f) = self.frame() {
            self.write(Frame { width: w, height: h, ..f }, cx);
        }
        true
    }

    fn settings(&self, f: &Format) -> Vec<Setting> {
        let names: Vec<(&str, &str)> = self.views.iter().map(|v| (v.name.as_str(), v.name.as_str())).collect();
        let chosen = self.chosen.clone().unwrap_or_default();
        let mut rows = vec![Setting::choice("view", "Görünüm", crate::tool_settings::choices(&names, &chosen))];
        if let Some(fr) = self.frame() {
            rows.push(Setting::text("width", "Genişlik", f.length(fr.width)));
            rows.push(Setting::text("height", "Yükseklik", f.length(fr.height)));
            rows.push(Setting::hint(
                "note",
                "Alan",
                "Tutamaçları sürükleyin: köşe iki kenarı, kenar ortası bir kenarı, orta bütün çerçeveyi taşır. Bağlı paftalar izler.",
            ));
        }
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.learn(cx);
        match key {
            "view" if self.views.iter().any(|v| v.name == value) => {
                self.chosen = Some(value.to_owned());
                self.held = None;
                self.draft = None;
                true
            }
            "width" | "height" => {
                let (Some(size), Some(fr)) = (cx.typed_length(value), self.frame()) else {
                    cx.say(Level::Warn, "Uzunluğu bir sayıyla yazın; örneğin 12 ya da 1200 cm.");
                    return false;
                };
                if size <= 0.0 {
                    cx.say(Level::Warn, "Genişlik ve yükseklik 0'dan büyük olmalı.");
                    return false;
                }
                let next = if key == "width" {
                    Frame { width: size, ..fr }
                } else {
                    Frame { height: size, ..fr }
                };
                self.write(next, cx);
                true
            }
            _ => false,
        }
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.held.is_some() {
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Esc: a held grip first (the frame as it was), then the view.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx) || self.chosen.take().is_some()
    }

    /// Ctrl+Z: a held grip lets go; else the drawing's undo takes back the last area written.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.draft = None;
        self.held.take().is_some()
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, f: &Format) -> Preview {
        let mut out = Preview::default();
        // Every view's frame, faint, views of one area under one label; the chosen one solid with its grips.
        let mut others: Vec<(Frame, Vec<&str>)> = Vec::new();
        for v in &self.views {
            if Some(&v.name) == self.chosen.as_ref() {
                continue;
            }
            let fr = Frame::of(v);
            match others.iter_mut().find(|(f, _)| *f == fr) {
                Some((_, names)) => names.push(&v.name),
                None => others.push((fr, vec![&v.name])),
            }
        }
        for (fr, names) in others {
            out.strokes
                .push(Stroke::dashed(fr.ring(), true, [6.0, 4.0]).tone(Tone::Snap));
            out.labels.push(Label {
                at: fr.ring()[3],
                text: names.join(", "),
                offset: [4.0, -14.0],
                tone: Tone::Snap,
            });
        }
        let Some(fr) = self.frame() else {
            return out;
        };
        if self.draft.is_some()
            && let Some(was) = self.chosen_view().map(Frame::of)
        {
            out.strokes
                .push(Stroke::dashed(was.ring(), true, [3.0, 3.0]).tone(Tone::Snap));
        }
        out.strokes
            .push(Stroke::solid(fr.ring(), true).width(2.0).tone(Tone::Accent));
        let held = self.held.map(|h| h.grip);
        for g in Grip::ALL {
            let hot = held == Some(g) || self.near == Some(g);
            out.markers.push(Marker {
                at: fr.grip_at(g),
                shape: if g == Grip::MIDDLE { MarkerShape::Ring(5.0) } else { MarkerShape::Square(4.0) },
                tone: if hot { Tone::Danger } else { Tone::Accent },
            });
        }
        let name = self.chosen.clone().unwrap_or_default();
        out.labels.push(Label {
            at: fr.ring()[3],
            text: format!("{name}: {} × {}", f.length(fr.width), f.length(fr.height)),
            offset: [4.0, -14.0],
            tone: Tone::Accent,
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(twist: f64) -> Frame {
        Frame { center: Vec2::new(10.0, 20.0), width: 8.0, height: 4.0, twist }
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a.x - b.x).hypot(a.y - b.y) < 1e-9
    }

    /// An edge moves alone, the opposite one stays; a corner moves two; the middle moves the whole frame.
    #[test]
    fn grips_move_their_edges() {
        let f = frame(0.0);
        let e = f.dragged(Grip(1, 0), f.grip_at(Grip(1, 0)), Vec2::new(16.0, 25.0));
        assert_eq!((e.width, e.height), (10.0, 4.0));
        assert!(near(e.center, Vec2::new(11.0, 20.0)));
        let c = f.dragged(Grip(-1, 1), f.grip_at(Grip(-1, 1)), Vec2::new(5.0, 23.0));
        assert_eq!((c.width, c.height), (9.0, 5.0));
        assert!(near(c.center, Vec2::new(9.5, 20.5)));
        let m = f.dragged(Grip::MIDDLE, f.grip_at(Grip::MIDDLE), Vec2::new(12.0, 21.0));
        assert!(near(m.center, Vec2::new(12.0, 21.0)) && m.width == 8.0);
    }

    /// A turned view's edges are its own axes: with a quarter turn its width runs north.
    #[test]
    fn a_turned_frame_moves_along_its_axes() {
        let f = frame(std::f64::consts::FRAC_PI_2);
        let right = f.grip_at(Grip(1, 0));
        assert!(near(right, Vec2::new(10.0, 24.0)));
        let e = f.dragged(Grip(1, 0), right, Vec2::new(10.0, 26.0));
        assert!((e.width - 10.0).abs() < 1e-9 && (e.height - 4.0).abs() < 1e-9);
        assert!(near(e.center, Vec2::new(10.0, 21.0)));
    }

    /// An edge pulled past its opposite one leaves a frame 1 mm wide, never a negative one.
    #[test]
    fn a_frame_never_turns_inside_out() {
        let f = frame(0.0);
        let e = f.dragged(Grip(1, 0), f.grip_at(Grip(1, 0)), Vec2::new(0.0, 20.0));
        assert!((e.width - LEAST).abs() < 1e-12);
        assert!(near(e.center, Vec2::new(6.0 + LEAST / 2.0, 20.0)));
    }
}
