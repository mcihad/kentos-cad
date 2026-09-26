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
}

impl Seen {
    /// An object as a step sees it: a path's corners; a line's two ends; a
    /// point's place; an arc's start and end (the web runner's `shape`).
    pub fn of(e: &Entity) -> Self {
        let (pts, bulges) = match e {
            Entity::Polygon(p) | Entity::Polyline(p) => (
                p.pts.iter().map(|v| [v.x, v.y]).collect(),
                p.bulges.clone().unwrap_or_default(),
            ),
            Entity::Line(l) => (vec![[l.a.x, l.a.y], [l.b.x, l.b.y]], Vec::new()),
            Entity::Point(p) => (vec![[p.p.x, p.p.y]], Vec::new()),
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
        }
    }
}

/// What a trace step can see (the web runner's `observe`).
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    pub tool: String,
    pub points: usize,
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
}

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
        if d.grid || d.tracking {
            return Err(format!(
                "{}: ızgara ve nesne izleme masaüstünde henüz yok; iz oynatılamaz",
                trace.id
            ));
        }
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
        };
        player.app.picker = Picker::File(player.file.clone());
        // Through the settings, as the web runner sets its signals: a toggle
        // later (F3, F8) starts from these, and the other drafting values
        // (snap kinds, apertures, polar step) are the settings' defaults.
        let refused = player.app.settings.choose(&[
            ("drafting.ortho", d.ortho.into()),
            ("drafting.polar", d.polar.into()),
            ("drafting.snap", d.snap.into()),
            (
                "drafting.cursorInput",
                trace.prefs.cursor_input.unwrap_or(true).into(),
            ),
        ]);
        if !refused.is_empty() {
            return Err(format!("{}: ayarlar alınmadı: {refused:?}", trace.id));
        }
        player.app.apply_settings();
        // The area reports its place first, as it does before any pointer event.
        player.mouse(mouse::Event::CursorLeft, mouse::Cursor::Unavailable)?;
        player.apply(Message::Opened(Some(Ok(Box::new(doc)))))?;
        let camera = &mut player.app.viewport.camera;
        camera.center = player.origin;
        camera.scale = 1.0 / trace.view.metres_per_pixel;
        Ok(player)
    }

    /// Plays the steps up to and including `last` (all when `None`);
    /// returns every expectation that did not hold, by step. A step that
    /// cannot be played ends the trace.
    pub fn play(&mut self, last: Option<usize>) -> Vec<String> {
        let mut problems = Vec::new();
        for (i, step) in self.trace.steps.iter().enumerate() {
            if last.is_some_and(|last| i >= last) {
                break;
            }
            let label = format!("adım {} {}", i + 1, describe(step));
            if let Err(error) = self.act(step) {
                problems.push(format!("{label}: {error}"));
                break;
            }
            if let Some(expect) = &step.expect {
                let bad = compare(expect, &self.observe(), self.trace);
                if !bad.is_empty() {
                    problems.push(format!("{label}: {}", bad.join("; ")));
                }
            }
        }
        problems
    }

    fn act(&mut self, step: &Step) -> Result<(), String> {
        if let Some(id) = &step.run {
            let command = crate::catalog::catalog()
                .get(id)
                .ok_or(format!("{id}: böyle bir komut yok"))?;
            return self.apply(Message::Run(command.id));
        }
        if let Some(key) = &step.key {
            return self.press(&chord_stroke(key, self.variant.layout)?);
        }
        if let Some(text) = &step.text {
            for ch in text.chars() {
                self.press(&stroke_for(&ch.to_string(), self.variant.layout)?)?;
            }
            return Ok(());
        }
        if let Some(at) = step.move_to {
            let position = self.window_point(at)?;
            return self.cursor_to(position);
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
        if step.expect.is_some() {
            return Ok(());
        }
        Err("adımda eylem yok".to_owned())
    }

    /// The window pixel a trace point (east, north from the view's centre)
    /// falls on, rounded to the screen's device pixels. A point off the
    /// drawing area stops the trace instead of missing silently.
    fn window_point(&self, [de, dn]: [f64; 2]) -> Result<Point, String> {
        let world = Vec2::new(self.origin.x + de, self.origin.y + dn);
        let [x, y] = self.app.viewport.camera.world_to_screen(world);
        let dpr = f64::from(self.variant.dpr);
        let round = |v: f64| (v * dpr).round() / dpr;
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
        let newest = doc
            .and_then(|d| d.model.entities().max_by_key(|e| e.base().id))
            .map(Seen::of);
        let objects = doc.map_or_else(BTreeMap::new, |d| {
            d.model
                .entities()
                .map(|e| (e.base().id, Seen::of(e)))
                .collect()
        });
        Observation {
            tool: app.session.tool_id().to_owned(),
            points: app.session.point_count(),
            options: app
                .session
                .prompt()
                .keys()
                .into_iter()
                .map(str::to_owned)
                .collect(),
            dynamic_input: app.field.as_ref().map(|f| f.text.clone()),
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
        }
    }
}

impl Drop for Player<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
    }
}

/// A step's action for the report, without its expectations and note.
fn describe(step: &Step) -> String {
    let pair = |[a, b]: [f64; 2]| format!("[{a}, {b}]");
    if let Some(id) = &step.run {
        format!("run {id}")
    } else if let Some(key) = &step.key {
        format!("key {key}")
    } else if let Some(text) = &step.text {
        format!("text {text:?}")
    } else if let Some(at) = step.move_to {
        format!("move {}", pair(at))
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
    } else {
        "beklenti".to_owned()
    }
}
