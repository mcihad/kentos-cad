//! The player: it drives the real [`App`] with what its window would send,
//! step by step, and reads back what a step can see (the web runner's
//! `act` and `observe`).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use iced::futures::StreamExt;
use iced::keyboard::Modifiers;
use iced::time::Instant;
use iced::{Point, Rectangle, Task, event, mouse, window};

use kentos_contracts::{DocumentSnapshotV1, Entity};
use kentos_render_wgpu::Vec2;

use crate::app::{App, Message, Picker};
use crate::document::Document;
use crate::keys;
use crate::marks::snap_name;
use crate::viewport::{self, Gesture};

use super::answers::Control;
use super::command_line::CommandLine;
use super::compare::compare;
use super::folder;
use super::format::{Step, Trace};
use super::keyboard::{Stroke, Variant, chord_stroke, stroke_for};

/// Where the drawing area is in the test runner's window (logical pixels): the
/// place a 1440 × 900 window gives it, with an odd size and a half-pixel
/// top so pixel rounding is exercised.
#[cfg(test)]
pub const AREA: Rectangle = Rectangle {
    x: 0.0,
    y: 123.5,
    width: 1119.0,
    height: 641.0,
};

/// The newest object as a step sees it (the web runner's `newest`).
#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    pub kind: String,
    /// A path's corners, a line's two ends, a point's place, an arc's start
    /// and end (counter-clockwise, as stored); absolute.
    pub pts: Vec<[f64; 2]>,
    pub bulges: Vec<f64>,
    /// A circle's or an arc's centre (absolute) and radius (docs/adr/0032).
    pub center: Option<[f64; 2]>,
    pub radius: Option<f64>,
    /// A text's content (docs/adr/0144 §7).
    pub text: Option<String>,
    /// A text's alignment's name (none: the left of the baseline), width
    /// factor and mask (docs/adr/0145).
    pub align: Option<String>,
    pub width_factor: Option<f64>,
    pub mask: Option<bool>,
    /// A text's turn in degrees.
    pub rotation: Option<f64>,
    /// A text's height on the ground (docs/adr/0175).
    pub height: Option<f64>,
    /// A leader's arrowhead's name; none the filled arrow (docs/adr/0146).
    pub arrow: Option<String>,
    /// A dimension's direction in degrees: a linear one's measured, an
    /// ordinate's axis (docs/adr/0147).
    pub angle: Option<f64>,
    /// The text beside it (a survey point's name), its attributes and a
    /// point's elevation (docs/adr/0152).
    pub label: Option<String>,
    pub attrs: std::collections::BTreeMap<String, String>,
    pub z: Option<f64>,
    /// The outer path's vertex elevations, a line's two ends', `None` none
    /// (docs/adr/0142, 0160); empty for other kinds.
    pub zs: Vec<Option<f64>>,
    /// An area's holes, in all its parts (docs/adr/0173 §5); none for other kinds.
    pub holes: Option<usize>,
    /// The slot of the object a linked text writes the label of (0 when no
    /// object of the drawing has its id) and its scale (docs/adr/0175 §4);
    /// none for a text of its own and other kinds.
    pub label_of: Option<u32>,
    pub label_scale: Option<f64>,
    /// Its own symbol, colour and line weight, and its layer's name (docs/adr/0176 §3).
    pub symbol: Option<String>,
    pub color: Option<String>,
    pub line_weight: Option<f64>,
    pub layer: String,
}

impl Seen {
    /// An object of `doc` as a step sees it: a path's corners; a line's two
    /// ends; a point's or a text's place; an arc's start and end; a
    /// dimension's measured points (the web runner's `shape`).
    pub fn of(e: &Entity, doc: &kentos_domain::Document) -> Self {
        let (pts, bulges) = match e {
            Entity::Polygon(p) | Entity::Polyline(p) => (
                p.pts.iter().map(|v| [v.x, v.y]).collect(),
                p.bulges.clone().unwrap_or_default(),
            ),
            Entity::Line(l) => (vec![[l.a.x, l.a.y], [l.b.x, l.b.y]], Vec::new()),
            Entity::Point(p) => (vec![[p.p.x, p.p.y]], Vec::new()),
            Entity::Text(t) => (vec![[t.p.x, t.p.y]], Vec::new()),
            // A leader's vertices, the tip first (docs/adr/0146).
            Entity::Leader(l) => (l.pts.iter().map(|v| [v.x, v.y]).collect(), Vec::new()),
            Entity::Arc(a) => (
                [a.a0, a.a1]
                    .iter()
                    .map(|t| [a.c.x + a.r * t.cos(), a.c.y + a.r * t.sin()])
                    .collect(),
                Vec::new(),
            ),
            // A spline's fit points; an ellipse's axis ends (major, minor,
            // counter-clockwise) or an elliptical arc's start and end; a
            // construction line's point and one metre along it (docs/adr/0057).
            Entity::Spline(s) => (s.pts.iter().map(|v| [v.x, v.y]).collect(), Vec::new()),
            Entity::Ellipse(e) => {
                let at = |t: f64| {
                    let (m, r) = (e.major, e.ratio);
                    [
                        e.c.x + m.x * t.cos() - m.y * r * t.sin(),
                        e.c.y + m.y * t.cos() + m.x * r * t.sin(),
                    ]
                };
                let pts = if e.t0 == e.t1 {
                    (0..4)
                        .map(|i| at(f64::from(i) * std::f64::consts::FRAC_PI_2))
                        .collect()
                } else {
                    vec![at(e.t0), at(e.t1)]
                };
                (pts, Vec::new())
            }
            Entity::Xline(x) | Entity::Ray(x) => (
                vec![[x.p.x, x.p.y], [x.p.x + x.dir.x, x.p.y + x.dir.y]],
                Vec::new(),
            ),
            // A dimension's measured points, and an angle's vertex (docs/adr/0061).
            Entity::Dimension(d) => (
                [d.a, d.b].iter().chain(&d.c).map(|v| [v.x, v.y]).collect(),
                Vec::new(),
            ),
            // An insert's point, and where the definition's points one metre
            // east and north of its base go: turn, scale and mirror (docs/adr/0144).
            Entity::Insert(i) => {
                let (s, c) = i.rotation.sin_cos();
                let k = if i.mirror { -1.0 } else { 1.0 };
                let (x, y, m) = (i.p.x, i.p.y, i.scale);
                (
                    vec![
                        [x, y],
                        [x + m * c, y + m * s],
                        [x - m * k * s, y + m * k * c],
                    ],
                    Vec::new(),
                )
            }
            _ => (Vec::new(), Vec::new()),
        };
        let (center, radius) = match e {
            Entity::Circle(c) => (Some([c.c.x, c.c.y]), Some(c.r)),
            Entity::Arc(a) => (Some([a.c.x, a.c.y]), Some(a.r)),
            Entity::Ellipse(e) => (Some([e.c.x, e.c.y]), None),
            _ => (None, None),
        };
        Seen {
            kind: e.kind().to_owned(),
            pts,
            bulges,
            center,
            radius,
            text: match e {
                Entity::Text(t) => Some(t.text.clone()),
                Entity::Leader(l) => l.text.clone(),
                _ => None,
            },
            align: match e {
                Entity::Text(t) => t.align.map(|a| a.name().to_owned()),
                _ => None,
            },
            width_factor: match e {
                Entity::Text(t) => Some(t.width_factor.unwrap_or(1.0)),
                _ => None,
            },
            mask: match e {
                Entity::Text(t) => Some(t.mask),
                Entity::Leader(l) => Some(l.mask),
                Entity::Dimension(d) => Some(d.mask),
                _ => None,
            },
            rotation: match e {
                Entity::Text(t) => Some(t.rotation),
                Entity::Leader(l) => Some(l.rotation),
                _ => None,
            },
            height: match e {
                Entity::Text(t) => Some(t.height),
                _ => None,
            },
            arrow: match e {
                Entity::Leader(l) => l.arrow.map(|a| a.name().to_owned()),
                _ => None,
            },
            angle: match e {
                Entity::Dimension(d) => d.angle,
                _ => None,
            },
            label: e.base().label.clone(),
            attrs: e.base().attrs.clone(),
            z: match e {
                Entity::Point(p) => p.z,
                _ => None,
            },
            zs: match e {
                Entity::Line(l) => vec![l.za, l.zb],
                Entity::Polygon(p) | Entity::Polyline(p) => {
                    p.zs.clone().unwrap_or_else(|| vec![None; p.pts.len()])
                }
                _ => Vec::new(),
            },
            holes: match e {
                Entity::Polygon(p) => Some(
                    p.holes.as_ref().map_or(0, Vec::len)
                        + p.parts
                            .iter()
                            .flatten()
                            .map(|part| part.holes.as_ref().map_or(0, Vec::len))
                            .sum::<usize>(),
                ),
                _ => None,
            },
            label_of: match e {
                Entity::Text(t) => t.label_of.map(|id| {
                    doc.slot_of(kentos_domain::Uuid::from_bytes(id.0))
                        .map_or(0, |slot| slot.0)
                }),
                _ => None,
            },
            label_scale: match e {
                Entity::Text(t) => t.label_scale,
                _ => None,
            },
            symbol: e.base().symbol.clone(),
            color: e.base().color.clone(),
            line_weight: e.base().line_weight,
            layer: doc
                .layers()
                .get(&e.base().layer_id)
                .map_or_else(String::new, |l| l.name.clone()),
        }
    }
}

/// What a trace step can see (the web runner's `observe`).
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    pub tool: String,
    pub points: usize,
    /// The prompt's whole text.
    pub prompt: String,
    /// What the step wrote to the command line, in order, at every level.
    pub messages: Vec<String>,
    pub options: Vec<String>,
    pub dynamic_input: Option<String>,
    pub command_line: String,
    pub entities: usize,
    pub newest: Option<Seen>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub dirty: bool,
    pub log: Option<String>,
    pub metres_per_pixel: f64,
    /// The view's centre, absolute.
    pub view_center: [f64; 2],
    pub selected: Vec<u32>,
    pub hover: Option<u32>,
    /// The snap marker's kind, as the web names it.
    pub snap: Option<String>,
    /// Every object's id in the drawing's order.
    pub ids: Vec<u32>,
    /// Every object by its id (the `objects` expectation, docs/adr/0037).
    pub objects: BTreeMap<u32, Seen>,
    /// Object tracking's acquired points, absolute (docs/adr/0085).
    pub track_points: Vec<[f64; 2]>,
    /// The lock, absolute.
    pub track: Option<TrackSeen>,
    /// The open window's title (answers.rs).
    pub dialog: Option<String>,
    /// The digitizing locks' words (docs/adr/0166 §6).
    pub locks: Vec<String>,
    /// The active layer's groups and name (docs/adr/0176 §3).
    pub active_layer: Vec<String>,
    /// The layers and groups hidden or locked of their own, by their paths (docs/adr/0177 §1).
    pub hidden_layers: Vec<String>,
    pub locked_layers: Vec<String>,
    /// The colour and line weight new objects take now.
    pub current_color: Option<String>,
    pub current_weight: Option<f64>,
}

/// Object tracking's lock as a step sees it: the point and each line's origin and angle.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackSeen {
    pub point: [f64; 2],
    pub lines: Vec<([f64; 2], f64)>,
}

/// What takes a `shot` step's picture: the app, the picture's name, and
/// whether the command line has the keyboard.
pub type Shot<'s> = &'s mut dyn FnMut(&mut App, &str, bool);

/// Plays a trace on an app.
pub struct Player<'a> {
    pub app: &'a mut App,
    trace: &'a Trace,
    variant: Variant,
    origin: Vec2,
    area: Rectangle,
    gesture: Gesture,
    clock: Instant,
    /// The command line's own state: its keyboard, its suggestion list, its focus reports.
    line: CommandLine,
    file: PathBuf,
    /// Where Yazı's field over the drawing was open after the last step, and
    /// whether it opened since and nothing was typed in it yet: Artır's
    /// number in it is chosen, and typing replaces it (the widget's `select_all`).
    field_at: Option<Vec2>,
    field_fresh: bool,
    /// The pointer goes to the trace's point itself, not to the device pixel
    /// it falls in (`exact_pointer`).
    exact: bool,
}

impl<'a> Player<'a> {
    /// Opens the trace's drawing in `app` and sets the view, the draft aids
    /// and the preferences (the web runner's `setUp`). `area` is where the
    /// drawing area is; `file` answers the file picker.
    pub fn new(
        app: &'a mut App,
        trace: &'a Trace,
        variant: Variant,
        area: Rectangle,
        file: PathBuf,
    ) -> Result<Self, String> {
        let d = &trace.draft;
        let text = std::fs::read_to_string(folder().join(&trace.document))
            .map_err(|e| format!("{}: {e}", trace.document))?;
        let snapshot =
            DocumentSnapshotV1::from_json(&text).map_err(|e| format!("{}: {e}", trace.document))?;
        // Opened as a new drawing: no path, so a save goes to the picker's file.
        let doc = Document::new(snapshot, None)?;
        let mut player = Self {
            app,
            trace,
            variant,
            origin: Vec2::new(trace.view.center[0], trace.view.center[1]),
            area,
            gesture: Gesture::default(),
            clock: Instant::now(),
            line: CommandLine::default(),
            file,
            field_at: None,
            field_fresh: false,
            exact: false,
        };
        player.app.picker = Picker::File(match &trace.open_file {
            Some(name) => super::folder().join(name),
            None => player.file.clone(),
        });
        // Object tracking's dwell passes on the player's clock (`rest`), never
        // on a thread's: a move that crosses a snap acquires nothing (docs/adr/0085).
        player.app.dwell_on_time = false;
        // Through the settings, as the web runner sets its signals: a toggle
        // later (F3, F8) starts from these, and the other drafting values
        // (snap kinds, apertures, polar step) are the settings' defaults.
        let refused = player.app.settings.choose(&[
            ("drafting.grid", d.grid.into()),
            ("drafting.ortho", d.ortho.into()),
            ("drafting.polar", d.polar.into()),
            ("drafting.snap", d.snap.into()),
            ("drafting.tracking", d.tracking.into()),
            ("drafting.topology", d.topology.into()),
            ("drafting.topologyPoints", d.topology_points.into()),
            (
                "drafting.overlap",
                d.overlap
                    .clone()
                    .unwrap_or_else(|| "allow".to_owned())
                    .into(),
            ),
            (
                "drafting.cursorInput",
                trace.prefs.cursor_input.unwrap_or(true).into(),
            ),
        ]);
        if !refused.is_empty() {
            return Err(format!("{}: ayarlar alınmadı: {refused:?}", trace.id));
        }
        player.app.overlap_layers = d.overlap_layers.clone();
        player.app.apply_settings();
        // The area reports its place first, as it does before any pointer event.
        player.mouse(mouse::Event::CursorLeft, mouse::Cursor::Unavailable)?;
        player.apply(Message::Opened(Some(Ok(Box::new(doc)))))?;
        let camera = &mut player.app.viewport.camera;
        camera.center = player.origin;
        camera.scale = 1.0 / trace.view.metres_per_pixel;
        Ok(player)
    }

    /// Puts the pointer on the trace's point itself, a fraction of a pixel as
    /// it may be, as the web's player does: a usage scenario's pictures of the
    /// two platforms then show what the same points give (`kentos-cad
    /// kullan`). The tests keep the screen's device pixels.
    pub fn exact_pointer(&mut self) {
        self.exact = true;
    }

    /// Plays the steps up to and including `last` (all when `None`);
    /// returns every expectation that did not hold, by step. A step that
    /// cannot be played ends the trace.
    pub fn play(&mut self, last: Option<usize>) -> Vec<String> {
        self.play_with(last, None)
    }

    /// Plays the whole trace, calling `shot` at each `shot` step with the app,
    /// the picture's name and whether the command line has the keyboard (so
    /// the picture can show its suggestion list); the expectations' problems.
    pub fn play_shots(&mut self, shot: Shot<'_>) -> Vec<String> {
        self.play_with(None, Some(shot))
    }

    fn play_with(&mut self, last: Option<usize>, mut shot: Option<Shot<'_>>) -> Vec<String> {
        let mut problems = Vec::new();
        for (i, step) in self.trace.steps.iter().enumerate() {
            if last.is_some_and(|last| i >= last) {
                break;
            }
            if let (Some(name), Some(take)) = (&step.shot, shot.as_mut()) {
                let line = self.line_has_keyboard();
                take(self.app, name, line);
                continue;
            }
            let label = format!("adım {} {}", i + 1, describe(step));
            let said = self.app.log.last_id();
            if let Err(error) = self.act(step) {
                problems.push(format!("{label}: {error}"));
                break;
            }
            if let Some(expect) = &step.expect {
                let mut seen = self.observe();
                seen.messages = self
                    .app
                    .log
                    .lines()
                    .filter(|line| line.id > said)
                    .map(|line| line.text.clone())
                    .collect();
                let bad = compare(expect, &seen, self.trace);
                if !bad.is_empty() {
                    problems.push(format!("{label}: {}", bad.join("; ")));
                }
            }
        }
        problems
    }

    fn act(&mut self, step: &Step) -> Result<(), String> {
        let done = self.act_step(step);
        let at = self.app.text_field.as_ref().map(|f| f.at);
        if at.is_some() && at != self.field_at {
            self.field_fresh = true;
        }
        self.field_at = at;
        done
    }

    fn act_step(&mut self, step: &Step) -> Result<(), String> {
        // An object template, as choosing it does (docs/adr/0176 §3).
        if let Some(id) = &step.template {
            return self.apply(Message::DrawTemplate(id.clone()));
        }
        // Şablonu uygula, as the panel's Seçili nesnelere uygula does (§6).
        if let Some(id) = &step.apply_template {
            return self.apply(Message::ApplyTemplate(id.clone()));
        }
        if let Some(id) = &step.run {
            let command = crate::catalog::catalog()
                .get(id)
                .ok_or(format!("{id}: böyle bir komut yok"))?;
            return self.apply(Message::Run(command.id));
        }
        if let Some(key) = &step.key {
            // Yazı's field over the drawing: Enter keeps what is typed (its text box's submit).
            if key == "Enter" && self.app.text_field.is_some() {
                return self.apply(Message::TextField(crate::text_field::Event::Keep));
            }
            // Backspace in it takes the chosen text away when it opened with one (Nokta's
            // Ad and Kod emptied, docs/adr/0152 §2), else the last character.
            if key == "Backspace"
                && let Some(open) = &self.app.text_field
            {
                let mut now = open.text.clone();
                if std::mem::take(&mut self.field_fresh) {
                    now.clear();
                } else {
                    now.pop();
                }
                return self.apply(Message::TextField(crate::text_field::Event::Input(now)));
            }
            return self.press(&chord_stroke(key, self.variant.layout)?);
        }
        if let Some(text) = &step.text {
            // Yazı's field takes what is typed; Artır's number, chosen as it opened, is typed over.
            if let Some(open) = &self.app.text_field {
                let now = if std::mem::take(&mut self.field_fresh) {
                    text.clone()
                } else {
                    format!("{}{text}", open.text)
                };
                return self.apply(Message::TextField(crate::text_field::Event::Input(now)));
            }
            for ch in text.chars() {
                // A capital typed into the value field is Shift and the letter, as a keyboard
                // types it (a point's name, docs/adr/0152 §4); on the drawing an option letter
                // goes without Shift (the web runner's rule).
                let stroke = if ch.is_uppercase() && self.app.field.is_some() {
                    chord_stroke(&format!("Shift+{ch}"), self.variant.layout)?
                } else {
                    stroke_for(&ch.to_string(), self.variant.layout)?
                };
                self.press(&stroke)?;
            }
            return Ok(());
        }
        if let Some(at) = step.move_to {
            let position = self.window_point(at)?;
            return self.cursor_to(position);
        }
        if let Some(at) = step.rest {
            // The web waits 500 ms, past the 350 ms dwell: the wait ends as its timer would.
            let position = self.window_point(at)?;
            self.cursor_to(position)?;
            self.clock += Duration::from_millis(500);
            if let Some(number) = self.app.tracking.dwell() {
                self.apply(Message::TrackDwell(number))?;
            }
            return Ok(());
        }
        if let Some(at) = step.click {
            return self.holding(step, |player| player.click(at, mouse::Button::Left));
        }
        if let Some([from, to]) = step.drag {
            return self.holding(step, |player| player.drag(from, to));
        }
        if let Some(at) = step.double_click {
            self.click(at, mouse::Button::Left)?;
            return self.click(at, mouse::Button::Left);
        }
        if let Some(at) = step.right_click {
            return self.click(at, mouse::Button::Right);
        }
        if let Some(target) = &step.focus {
            if target != "commandLine" {
                return Err(format!("bilinmeyen odak {target}"));
            }
            // The pointer goes down to the command line, off the drawing, and clicks it.
            let below = Point::new(
                self.area.x + self.area.width / 2.0,
                self.area.y + self.area.height + 20.0,
            );
            self.cursor_to(below)?;
            self.mouse(
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Cursor::Available(below),
            )?;
            if let Some(report) = self.line.click() {
                self.apply(report)?;
            }
            return self.mouse(
                mouse::Event::ButtonReleased(mouse::Button::Left),
                mouse::Cursor::Available(below),
            );
        }
        if step.save_and_reopen == Some(true) {
            self.press(&chord_stroke("Ctrl+S", self.variant.layout)?)?;
            return self.press(&chord_stroke("Ctrl+O", self.variant.layout)?);
        }
        if let Some(title) = &step.dialog {
            return self.answer(title, step);
        }
        if step.expect.is_some() || step.shot.is_some() {
            return Ok(());
        }
        Err("adımda eylem yok".to_owned())
    }

    /// The window pixel a trace point (east, north from the view's centre)
    /// falls on, rounded to the screen's device pixels (not with
    /// `exact_pointer`). A point off the drawing area stops the trace instead
    /// of missing silently.
    fn window_point(&self, [de, dn]: [f64; 2]) -> Result<Point, String> {
        let world = Vec2::new(self.origin.x + de, self.origin.y + dn);
        let [x, y] = self.app.viewport.camera.world_to_screen(world);
        let dpr = f64::from(self.variant.dpr);
        let exact = self.exact;
        let round = |v: f64| if exact { v } else { (v * dpr).round() / dpr };
        let window = Point::new(
            round(f64::from(self.area.x) + x) as f32,
            round(f64::from(self.area.y) + y) as f32,
        );
        if !self.area.contains(window) {
            return Err(format!(
                "[{de}, {dn}] çizim alanının dışında ({}, {} px); izin noktalarını README'deki kutuda tutun",
                window.x, window.y
            ));
        }
        Ok(window)
    }

    fn cursor_to(&mut self, position: Point) -> Result<(), String> {
        self.mouse(
            mouse::Event::CursorMoved { position },
            mouse::Cursor::Available(position),
        )
    }

    fn click(&mut self, at: [f64; 2], button: mouse::Button) -> Result<(), String> {
        let position = self.window_point(at)?;
        self.cursor_to(position)?;
        let cursor = mouse::Cursor::Available(position);
        self.mouse(mouse::Event::ButtonPressed(button), cursor)?;
        // A click anywhere outside the command line's text box takes its keyboard.
        if let Some(report) = self.line.click_elsewhere() {
            self.apply(report)?;
        }
        // A right press shorter than the hold that would open the command menu.
        if button == mouse::Button::Right {
            self.clock += Duration::from_millis(40);
        }
        self.mouse(mouse::Event::ButtonReleased(button), cursor)?;
        self.clock += Duration::from_millis(40);
        Ok(())
    }

    /// Runs a pointer action with the step's keys held: the window reports
    /// the modifiers before and after, as for a key.
    fn holding(
        &mut self,
        step: &Step,
        act: impl FnOnce(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        let shift = step.shift == Some(true);
        if shift {
            self.apply(Message::Modifiers(Modifiers::SHIFT))?;
        }
        let done = act(self);
        if shift {
            self.apply(Message::Modifiers(Modifiers::empty()))?;
        }
        done
    }

    /// Plays step `index` (from 0) halfway, for an image: a drag stops with
    /// the button still down at its end, the box showing. Other steps play whole.
    pub fn halfway(&mut self, index: usize) -> Result<(), String> {
        let step = self
            .trace
            .steps
            .get(index)
            .ok_or(format!("{}. adım yok", index + 1))?;
        let Some([from, to]) = step.drag else {
            return self.act(step);
        };
        let shift = step.shift == Some(true);
        if shift {
            self.apply(Message::Modifiers(Modifiers::SHIFT))?;
        }
        let a = self.window_point(from)?;
        let b = self.window_point(to)?;
        self.cursor_to(a)?;
        self.mouse(
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Cursor::Available(a),
        )?;
        self.cursor_to(b)
    }

    /// The left button down at `from`, moved through the middle to `to`,
    /// released there (the web runner's `drag`).
    fn drag(&mut self, from: [f64; 2], to: [f64; 2]) -> Result<(), String> {
        let a = self.window_point(from)?;
        let middle = self.window_point([(from[0] + to[0]) / 2.0, (from[1] + to[1]) / 2.0])?;
        let b = self.window_point(to)?;
        self.cursor_to(a)?;
        self.mouse(
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Cursor::Available(a),
        )?;
        if let Some(report) = self.line.click_elsewhere() {
            self.apply(report)?;
        }
        self.cursor_to(middle)?;
        self.cursor_to(b)?;
        self.mouse(
            mouse::Event::ButtonReleased(mouse::Button::Left),
            mouse::Cursor::Available(b),
        )?;
        self.clock += Duration::from_millis(40);
        Ok(())
    }

    /// A `dialog` step: the window titled `title` is open; its fields are
    /// filled, its boxes set and its button pressed, each through the
    /// message the control sends (answers.rs).
    fn answer(&mut self, title: &str, step: &Step) -> Result<(), String> {
        let open = self.app.dialog_title();
        if open.as_deref() != Some(title) {
            let open = open.map_or_else(|| "yok".to_owned(), |t| format!("“{t}”"));
            return Err(format!("“{title}” penceresi açık değil (açık: {open})"));
        }
        let fills = step.fill.iter().flat_map(|f| f.0.iter());
        let mut controls: Vec<Control<'_>> = fills
            .map(|(label, text)| Control::Fill(label, text))
            .collect();
        let checks = step.check.iter().flat_map(|c| c.0.iter());
        controls.extend(checks.map(|(words, on)| Control::Check(words, *on)));
        controls.extend(step.press.as_deref().map(Control::Press));
        for control in controls {
            if let Some(message) = self.app.dialog_control(control)? {
                self.apply(message)?;
            }
        }
        Ok(())
    }

    /// A mouse event through the drawing area's own gesture code.
    fn mouse(&mut self, event: mouse::Event, cursor: mouse::Cursor) -> Result<(), String> {
        self.clock += Duration::from_millis(1);
        let event = iced::Event::Mouse(event);
        if let Some((Some(e), _)) =
            viewport::gesture(&mut self.gesture, &event, self.area, cursor, self.clock)
        {
            self.apply(Message::Viewport(e))?;
        }
        Ok(())
    }

    /// A key press: to the command line when it has the keyboard, else (and
    /// when it lets the key through) to the app's subscription. The modifier
    /// state goes before and after, as the window reports it.
    fn press(&mut self, stroke: &Stroke) -> Result<(), String> {
        if !stroke.modifiers.is_empty() {
            self.apply(Message::Modifiers(stroke.modifiers))?;
        }
        let event = stroke.event();
        let taken = if self.line.has_keyboard() {
            self.line.key(self.app, &event)
        } else {
            None
        };
        match taken {
            Some(messages) => {
                for message in messages {
                    self.apply(message)?;
                }
            }
            None => {
                if let Some(message) =
                    keys::key_event(event, event::Status::Ignored, window::Id::unique())
                {
                    self.apply(message)?;
                }
            }
        }
        if !stroke.modifiers.is_empty() {
            self.apply(Message::Modifiers(Modifiers::empty()))?;
        }
        Ok(())
    }

    /// Gives the app a message and runs the task it returns to its end.
    pub(super) fn apply(&mut self, message: Message) -> Result<(), String> {
        let task = self.app.update(message);
        self.run(task)
    }

    /// Runs a task: its messages go back to the app; a widget operation
    /// runs on the command line's model, which reports a focus change to the
    /// app as the widget does. An operation the model cannot follow stops the
    /// trace rather than let the command line's state go wrong unseen.
    fn run(&mut self, task: Task<Message>) -> Result<(), String> {
        let Some(mut stream) = iced_runtime::task::into_stream(task) else {
            return Ok(());
        };
        // One action at a time, as the runtime takes them: the task of an
        // operation that reports back (`widget::operate`) ends only once its
        // operation has run and been dropped.
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            match action {
                iced_runtime::Action::Output(message) => self.apply(message)?,
                iced_runtime::Action::Widget(mut operation) => {
                    let report = self.line.operate(operation.as_mut())?;
                    drop(operation);
                    if let Some(report) = report {
                        self.apply(report)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Whether the command line's text box has the keyboard now (the snapshot shows it so).
    pub fn line_has_keyboard(&self) -> bool {
        self.line.has_keyboard()
    }

    /// What the step can see, read from the app (the web runner's `observe`).
    pub fn observe(&self) -> Observation {
        let app = &*self.app;
        let doc = app.document.as_ref();
        let newest = doc.and_then(|d| {
            d.model
                .entities()
                .max_by_key(|e| e.base().id)
                .map(|e| Seen::of(e, &d.model))
        });
        let objects = doc.map_or_else(BTreeMap::new, |d| {
            d.model
                .entities()
                .map(|e| (e.base().id, Seen::of(e, &d.model)))
                .collect()
        });
        Observation {
            tool: app.session.tool_id().to_owned(),
            points: app.session.point_count(),
            prompt: app.prompt().text(),
            messages: Vec::new(),
            options: app.prompt().keys().into_iter().map(str::to_owned).collect(),
            dynamic_input: app.field.as_ref().map(|f| f.text.clone()),
            locks: app.locks.words(&app.format()),
            command_line: app.command_input.clone(),
            entities: doc.map_or(0, Document::entity_count),
            newest,
            can_undo: doc.is_some_and(|d| d.model.can_undo()),
            can_redo: doc.is_some_and(|d| d.model.can_redo()),
            dirty: doc.is_some_and(Document::dirty),
            log: app.last_level.map(|l| l.as_str().to_owned()),
            metres_per_pixel: 1.0 / app.viewport.camera.scale,
            view_center: [app.viewport.camera.center.x, app.viewport.camera.center.y],
            selected: app.selection.ids().iter().map(|s| s.0).collect(),
            hover: app.selection.hover().map(|s| s.0),
            snap: app.snap.map(|s| snap_name(s.kind).to_owned()),
            ids: doc.map_or_else(Vec::new, |d| {
                d.model.entities().map(|e| e.base().id).collect()
            }),
            objects,
            track_points: app.tracking.points().iter().map(|p| [p.x, p.y]).collect(),
            // The web reads the lock only while no snap wins over it.
            track: app
                .tracking
                .track()
                .filter(|_| app.snap.is_none())
                .map(|t| TrackSeen {
                    point: [t.point.x, t.point.y],
                    lines: t
                        .lines
                        .iter()
                        .map(|l| ([l.origin.x, l.origin.y], l.angle))
                        .collect(),
                }),
            dialog: app.dialog_title(),
            active_layer: doc.map_or_else(Vec::new, |d| {
                let layers = d.model.layers();
                let mut names = Vec::new();
                let mut at = layers.get(layers.active());
                while let Some(node) = at {
                    names.insert(0, node.name.clone());
                    at = layers.parent(&node.id);
                }
                names
            }),
            hidden_layers: doc
                .map_or_else(Vec::new, |d| layer_paths(d.model.layers(), |n| !n.visible)),
            locked_layers: doc
                .map_or_else(Vec::new, |d| layer_paths(d.model.layers(), |n| n.locked)),
            current_color: app.draft.color_text(),
            current_weight: app.draft.line_weight,
        }
    }
}

/// The paths of the tree's nodes that `which` takes, in tree order.
fn layer_paths(
    layers: &kentos_domain::LayerTree,
    which: impl Fn(&kentos_contracts::LayerNode) -> bool,
) -> Vec<String> {
    fn walk(
        nodes: &[kentos_contracts::LayerNode],
        layers: &kentos_domain::LayerTree,
        which: &dyn Fn(&kentos_contracts::LayerNode) -> bool,
        out: &mut Vec<String>,
    ) {
        for node in nodes {
            if which(node) {
                out.push(layers.path(&node.id));
            }
            walk(&node.children, layers, which, out);
        }
    }
    let mut out = Vec::new();
    walk(layers.nodes(), layers, &which, &mut out);
    out
}

impl Drop for Player<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
    }
}

/// A step's action for the report, without its expectations and note.
fn describe(step: &Step) -> String {
    let pair = |[a, b]: [f64; 2]| format!("[{a}, {b}]");
    if let Some(id) = &step.template {
        format!("template {id}")
    } else if let Some(id) = &step.apply_template {
        format!("applyTemplate {id}")
    } else if let Some(id) = &step.run {
        format!("run {id}")
    } else if let Some(key) = &step.key {
        format!("key {key}")
    } else if let Some(text) = &step.text {
        format!("text {text:?}")
    } else if let Some(at) = step.move_to {
        format!("move {}", pair(at))
    } else if let Some(at) = step.rest {
        format!("rest {}", pair(at))
    } else if let Some(at) = step.click {
        let held = if step.shift == Some(true) {
            "Shift+"
        } else {
            ""
        };
        format!("{held}click {}", pair(at))
    } else if let Some([from, to]) = step.drag {
        let held = if step.shift == Some(true) {
            "Shift+"
        } else {
            ""
        };
        format!("{held}drag {} → {}", pair(from), pair(to))
    } else if let Some(at) = step.double_click {
        format!("doubleClick {}", pair(at))
    } else if let Some(at) = step.right_click {
        format!("rightClick {}", pair(at))
    } else if let Some(target) = &step.focus {
        format!("focus {target}")
    } else if step.save_and_reopen.is_some() {
        "saveAndReopen".to_owned()
    } else if let Some(title) = &step.dialog {
        let press = step
            .press
            .as_deref()
            .map_or_else(String::new, |p| format!(" → {p}"));
        format!("dialog {title}{press}")
    } else if let Some(name) = &step.shot {
        format!("shot {name}")
    } else {
        "beklenti".to_owned()
    }
}
