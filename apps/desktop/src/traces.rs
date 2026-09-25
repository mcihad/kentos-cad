//! The interaction traces (`fixtures/interaction/v1`, docs/adr/0018) played
//! on the desktop: the files the web plays in a browser
//! (`apps/web/scripts/e2e/interaction.mjs`), unchanged, in the same three
//! variants (US keyboard, Turkish Q, a 2× screen).
//!
//! The player drives the real [`App`] with what its window would send, no
//! window needed:
//!
//! - keys are Iced key events a US or a Turkish Q keyboard produces (AltGr
//!   as Ctrl+Alt, as Windows reports it), through the app's own
//!   subscription (`keys::key_event`);
//! - the pointer is Iced mouse events through the drawing area's own
//!   gesture code (`viewport::gesture`), at the window pixel the world point
//!   falls on through the same camera, rounded to the screen's device pixels;
//! - while the command line has the keyboard, keys go to a model of the
//!   KentOS UI command line as `view.rs` configures it, which a test holds to
//!   the real widget;
//! - `run` is the command's message, as a ribbon button sends it;
//!   `saveAndReopen` is Ctrl+S and Ctrl+O with the file picker answered by a
//!   temporary file, and the app's own tasks run to their end.
//!
//! After each step the expectations are compared by the web runner's rules:
//! clicked points within `clickTolerance`, typed edges exactly, the scale
//! within a relative 1e-9. `kentos-cad snapshot --iz` plays a trace into an
//! image.

use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::futures::StreamExt;
use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};
use iced::time::Instant;
use iced::{Point, Rectangle, Task, event, mouse, window};
use serde::Deserialize;

use kentos_contracts::{DocumentSnapshotV1, Entity};
use kentos_render_wgpu::Vec2;

use crate::app::{App, Message, Picker};
use crate::document::Document;
use crate::keys;
use crate::viewport::{self, Gesture};

/// The traces' folder, next to the sources.
pub fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1")
}

// ── The trace format (`kentos.interaction-trace` v1, fixtures/interaction/README.md) ──
// Every field is named: a field the player does not know stops it, so a new
// one cannot be skipped unnoticed.

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Trace {
    pub format: String,
    pub version: u32,
    pub id: String,
    /// What the trace shows; the runner's report names it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub title: String,
    /// For the reader: where the trace comes from and the TODOS items it covers.
    #[allow(dead_code)]
    pub source: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    pub covers: Vec<String>,
    pub document: String,
    pub view: View,
    #[serde(default)]
    pub draft: DraftSpec,
    #[serde(default)]
    pub prefs: Prefs,
    pub click_tolerance: f64,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct View {
    pub center: [f64; 2],
    pub metres_per_pixel: f64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftSpec {
    #[serde(default)]
    snap: bool,
    #[serde(default)]
    grid: bool,
    #[serde(default)]
    ortho: bool,
    #[serde(default)]
    polar: bool,
    #[serde(default)]
    tracking: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Prefs {
    cursor_input: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Step {
    run: Option<String>,
    key: Option<String>,
    text: Option<String>,
    #[serde(rename = "move")]
    move_to: Option<[f64; 2]>,
    click: Option<[f64; 2]>,
    double_click: Option<[f64; 2]>,
    right_click: Option<[f64; 2]>,
    focus: Option<String>,
    save_and_reopen: Option<bool>,
    expect: Option<Expect>,
    /// For the reader; not checked.
    #[allow(dead_code)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Expect {
    tool: Option<String>,
    points: Option<usize>,
    options: Option<Vec<String>>,
    /// `null` (closed) and absent (not compared) differ.
    #[serde(default, deserialize_with = "present")]
    dynamic_input: Option<Option<String>>,
    command_line: Option<String>,
    entities: Option<usize>,
    newest: Option<Newest>,
    can_undo: Option<bool>,
    can_redo: Option<bool>,
    dirty: Option<bool>,
    log: Option<String>,
    metres_per_pixel: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Newest {
    kind: String,
    points: Option<Vec<[f64; 2]>>,
    edges: Option<Vec<[f64; 2]>>,
    arcs: Option<usize>,
}

/// A field that is there, even as `null`.
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

impl Trace {
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
        let trace: Trace =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if trace.format != "kentos.interaction-trace" || trace.version != 1 {
            return Err(format!("{}: v1 etkileşim izi değil", path.display()));
        }
        Ok(trace)
    }

    /// Every trace of the folder, by file name.
    #[cfg(test)]
    pub fn all() -> Result<Vec<Self>, String> {
        let dir = folder();
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| format!("{} okunamadı: {e}", dir.display()))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        paths.sort();
        paths.iter().map(|p| Self::read(p)).collect()
    }

    pub fn by_id(id: &str) -> Result<Self, String> {
        Self::read(&folder().join(format!("{id}.json")))
    }
}

// ── Variants ────────────────────────────────────────────────────────────────

/// Which keyboard types the characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Us,
    TurkishQ,
}

/// The web runner's variants: a keyboard and the screen's device pixels per logical pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variant {
    pub id: &'static str,
    pub layout: Layout,
    pub dpr: f32,
}

pub const VARIANTS: [Variant; 3] = [
    Variant {
        id: "us",
        layout: Layout::Us,
        dpr: 1.0,
    },
    Variant {
        id: "tr-q",
        layout: Layout::TurkishQ,
        dpr: 1.0,
    },
    Variant {
        id: "hidpi",
        layout: Layout::Us,
        dpr: 2.0,
    },
];

impl Variant {
    pub fn by_id(id: &str) -> Option<Self> {
        VARIANTS.into_iter().find(|v| v.id == id)
    }
}

// ── Keyboards ───────────────────────────────────────────────────────────────

/// One key as a keyboard reports it: the key without and with modifiers, its
/// position, the modifiers and the text it types.
struct Stroke {
    key: Key,
    modified: Key,
    code: Code,
    location: Location,
    modifiers: Modifiers,
    text: Option<String>,
}

fn character(c: &str) -> Key {
    Key::Character(c.into())
}

/// The key that types `ch` on `layout` (the web runner's `LAYOUTS` and `keyFor`).
fn stroke_for(ch: &str, layout: Layout) -> Result<Stroke, String> {
    let named = |named: Named, code: Code, text: Option<&str>| Stroke {
        key: Key::Named(named),
        modified: Key::Named(named),
        code,
        location: Location::Standard,
        modifiers: Modifiers::empty(),
        text: text.map(str::to_owned),
    };
    // Named keys, with the control characters a platform reports as their text.
    match ch {
        "Enter" => return Ok(named(Named::Enter, Code::Enter, Some("\r"))),
        "Esc" => return Ok(named(Named::Escape, Code::Escape, Some("\u{1b}"))),
        "Tab" => return Ok(named(Named::Tab, Code::Tab, Some("\t"))),
        "Backspace" => return Ok(named(Named::Backspace, Code::Backspace, Some("\u{8}"))),
        "Space" | " " => return Ok(named(Named::Space, Code::Space, Some(" "))),
        _ => {}
    }
    let plain = |base: &str, code: Code| Stroke {
        key: character(base),
        modified: character(ch),
        code,
        location: Location::Standard,
        modifiers: Modifiers::empty(),
        text: Some(ch.to_owned()),
    };
    let shifted = |base: &str, code: Code| Stroke {
        modifiers: Modifiers::SHIFT,
        ..plain(base, code)
    };
    let digit = |d: char| -> Option<Code> {
        Some(match d {
            '0' => Code::Digit0,
            '1' => Code::Digit1,
            '2' => Code::Digit2,
            '3' => Code::Digit3,
            '4' => Code::Digit4,
            '5' => Code::Digit5,
            '6' => Code::Digit6,
            '7' => Code::Digit7,
            '8' => Code::Digit8,
            '9' => Code::Digit9,
            _ => return None,
        })
    };
    let mut chars = ch.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return Err(format!("“{ch}” için tuş yok"));
    };
    if let Some(code) = digit(c) {
        return Ok(plain(ch, code));
    }
    let symbol = match (layout, c) {
        (Layout::Us, '.') => Some(plain(".", Code::Period)),
        (Layout::Us, ',') => Some(plain(",", Code::Comma)),
        (Layout::Us, ';') => Some(plain(";", Code::Semicolon)),
        (Layout::Us, '-') => Some(plain("-", Code::Minus)),
        (Layout::Us, '+') => Some(Stroke {
            location: Location::Numpad,
            ..plain("+", Code::NumpadAdd)
        }),
        (Layout::Us, '@') => Some(shifted("2", Code::Digit2)),
        (Layout::Us, '<') => Some(plain("<", Code::IntlBackslash)),
        // Turkish Q: + is Shift+4, − sits right of *, @ is AltGr+Q (Ctrl+Alt on Windows).
        (Layout::TurkishQ, '.') => Some(plain(".", Code::Slash)),
        (Layout::TurkishQ, ',') => Some(plain(",", Code::Backslash)),
        (Layout::TurkishQ, ';') => Some(shifted(",", Code::Backslash)),
        (Layout::TurkishQ, '-') => Some(plain("-", Code::Equal)),
        (Layout::TurkishQ, '+') => Some(shifted("4", Code::Digit4)),
        (Layout::TurkishQ, '@') => Some(Stroke {
            modifiers: Modifiers::CTRL | Modifiers::ALT,
            ..plain("q", Code::KeyQ)
        }),
        (Layout::TurkishQ, '<') => Some(plain("<", Code::IntlBackslash)),
        _ => None,
    };
    if let Some(stroke) = symbol {
        return Ok(stroke);
    }
    if c.is_alphabetic() {
        // An option letter is typed without Shift; the app compares upper case.
        let lower = match c {
            'I' => 'ı',
            'İ' => 'i',
            c => c.to_lowercase().next().unwrap_or(c),
        };
        let code = letter_code(lower, layout).ok_or(format!("“{ch}” için tuş yok"))?;
        let text = lower.to_string();
        return Ok(Stroke {
            key: character(&text),
            modified: character(&text),
            code,
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: Some(text),
        });
    }
    Err(format!("“{ch}” için tuş yok"))
}

/// Where a letter is: its Latin key, or a Turkish Q letter's own key.
fn letter_code(lower: char, layout: Layout) -> Option<Code> {
    if layout == Layout::TurkishQ {
        let turkish = match lower {
            'ı' => Some(Code::KeyI),
            'i' => Some(Code::Quote),
            'ş' => Some(Code::Semicolon),
            'ğ' => Some(Code::BracketLeft),
            'ü' => Some(Code::BracketRight),
            'ö' => Some(Code::Comma),
            'ç' => Some(Code::Period),
            _ => None,
        };
        if turkish.is_some() {
            return turkish;
        }
    }
    let ascii = match lower {
        'ı' => 'i',
        'ç' => 'c',
        'ğ' => 'g',
        'ö' => 'o',
        'ş' => 's',
        'ü' => 'u',
        c => c,
    };
    Some(match ascii.to_ascii_uppercase() {
        'A' => Code::KeyA,
        'B' => Code::KeyB,
        'C' => Code::KeyC,
        'D' => Code::KeyD,
        'E' => Code::KeyE,
        'F' => Code::KeyF,
        'G' => Code::KeyG,
        'H' => Code::KeyH,
        'I' => Code::KeyI,
        'J' => Code::KeyJ,
        'K' => Code::KeyK,
        'L' => Code::KeyL,
        'M' => Code::KeyM,
        'N' => Code::KeyN,
        'O' => Code::KeyO,
        'P' => Code::KeyP,
        'Q' => Code::KeyQ,
        'R' => Code::KeyR,
        'S' => Code::KeyS,
        'T' => Code::KeyT,
        'U' => Code::KeyU,
        'V' => Code::KeyV,
        'W' => Code::KeyW,
        'X' => Code::KeyX,
        'Y' => Code::KeyY,
        'Z' => Code::KeyZ,
        _ => return None,
    })
}

/// A trace's key (`Enter`, `G`, `-`, `+`, `Ctrl+Z`) as a stroke: a chord
/// types nothing but the control character a platform reports for it.
fn chord_stroke(chord: &str, layout: Layout) -> Result<Stroke, String> {
    let (mods, name) = match chord {
        "+" => (Vec::new(), "+"),
        _ => {
            let mut parts: Vec<&str> = chord.split('+').collect();
            let name = parts.pop().unwrap_or(chord);
            (parts, name)
        }
    };
    let mut stroke = stroke_for(name, layout)?;
    for m in mods {
        stroke.modifiers |= match m {
            "Ctrl" => Modifiers::CTRL,
            "Alt" => Modifiers::ALT,
            "Shift" => Modifiers::SHIFT,
            other => return Err(format!("bilinmeyen değiştirici tuş {other}")),
        };
    }
    if stroke.modifiers.control() && !matches!(stroke.key, Key::Named(_)) {
        // Ctrl+letter: the ASCII control character (Ctrl+Z is U+001A).
        stroke.text = stroke
            .text
            .as_deref()
            .and_then(|t| t.chars().next())
            .filter(char::is_ascii_alphabetic)
            .map(|c| char::from((c.to_ascii_lowercase() as u8) - b'a' + 1).to_string());
    }
    Ok(stroke)
}

impl Stroke {
    fn event(&self) -> iced::Event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: self.key.clone(),
            modified_key: self.modified.clone(),
            physical_key: Physical::Code(self.code),
            location: self.location,
            modifiers: self.modifiers,
            text: self.text.as_deref().map(Into::into),
            repeat: false,
        })
    }
}

// ── The command line as view.rs configures it ───────────────────────────────

/// What the KentOS UI command line sends for a key press while its text box
/// has the keyboard and no suggestion list is open (`value` is its text):
/// `None` when it lets the key through to the subscription. The widget's
/// own behaviour is the reference: `the_command_line_model_is_the_widget`.
pub fn line_messages(value: &str, event: &iced::Event) -> Option<Vec<Message>> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key,
        modifiers,
        text,
        ..
    }) = event
    else {
        return None;
    };
    let plain = !modifiers.command() && !modifiers.alt();
    match key {
        Key::Named(Named::Escape) if plain => Some(if value.is_empty() {
            vec![Message::CommandCancelled, Message::CommandFocus(false)]
        } else {
            vec![Message::CommandInput(String::new())]
        }),
        Key::Named(Named::Enter) => Some(vec![Message::CommandSubmitted]),
        Key::Named(Named::Space) if plain => Some(vec![Message::CommandSubmitted]),
        Key::Named(Named::Backspace) => {
            let mut value = value.to_owned();
            value.pop();
            Some(vec![Message::CommandInput(value)])
        }
        _ => text
            .as_deref()
            .and_then(|t| t.chars().next())
            .filter(|c| !c.is_control())
            .map(|c| vec![Message::CommandInput(format!("{value}{c}"))]),
    }
}

// ── The player ──────────────────────────────────────────────────────────────

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

/// What a trace step can see (the web runner's `observe`).
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    pub tool: String,
    pub points: usize,
    pub options: Vec<String>,
    pub dynamic_input: Option<String>,
    pub command_line: String,
    pub entities: usize,
    /// The newest object's kind, corners (absolute) and bulges.
    pub newest: Option<(String, Vec<[f64; 2]>, Vec<f64>)>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub dirty: bool,
    pub log: Option<String>,
    pub metres_per_pixel: f64,
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
    /// Whether the command line's text box has the keyboard (the widget's state).
    line_focused: bool,
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
        if d.snap || d.grid || d.tracking {
            return Err(format!(
                "{}: kenet, ızgara ve kenet izlemesi masaüstünde henüz yok; iz oynatılamaz",
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
            line_focused: false,
            file,
        };
        player.app.picker = Picker::File(player.file.clone());
        player.app.draft.ortho = d.ortho;
        player.app.draft.polar = d.polar.then_some(90.0);
        player.app.cursor_input = trace.prefs.cursor_input.unwrap_or(true);
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
            return self.click(at, mouse::Button::Left);
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
            if !self.line_focused {
                self.line_focused = true;
                self.apply(Message::CommandFocus(true))?;
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
        if self.line_focused {
            self.line_focused = false;
            self.apply(Message::CommandFocus(false))?;
        }
        // A right press shorter than the hold that would open the command menu.
        if button == mouse::Button::Right {
            self.clock += Duration::from_millis(40);
        }
        self.mouse(mouse::Event::ButtonReleased(button), cursor)?;
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
        let taken = if self.line_focused {
            line_messages(&self.app.command_input, &event)
        } else {
            None
        };
        match taken {
            Some(messages) => {
                for message in messages {
                    if let Message::CommandFocus(focused) = message {
                        self.line_focused = focused;
                    }
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
    fn apply(&mut self, message: Message) -> Result<(), String> {
        let task = self.app.update(message);
        self.run(task)
    }

    /// Runs a task: its messages go back to the app. A widget operation
    /// (a focus change) is not played: it stops the trace rather than let
    /// the command line's state go wrong unseen.
    fn run(&mut self, task: Task<Message>) -> Result<(), String> {
        let Some(stream) = iced_runtime::task::into_stream(task) else {
            return Ok(());
        };
        let actions: Vec<_> = iced::futures::executor::block_on(stream.collect());
        for action in actions {
            match action {
                iced_runtime::Action::Output(message) => self.apply(message)?,
                iced_runtime::Action::Widget(_) => {
                    return Err(
                        "iz, oynatıcının izlemediği bir odak değişikliği üretti (widget işlemi)"
                            .to_owned(),
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// What the step can see, read from the app (the web runner's `observe`).
    pub fn observe(&self) -> Observation {
        let app = &*self.app;
        let doc = app.document.as_ref();
        let newest = doc
            .and_then(|d| d.model.entities().max_by_key(|e| e.base().id))
            .map(|e| {
                let (pts, bulges) = match e {
                    Entity::Polygon(p) | Entity::Polyline(p) => (
                        p.pts.iter().map(|v| [v.x, v.y]).collect(),
                        p.bulges.clone().unwrap_or_default(),
                    ),
                    _ => (Vec::new(), Vec::new()),
                };
                (e.kind().to_owned(), pts, bulges)
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
        format!("click {}", pair(at))
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

/// Differences between what a step expects and what the app shows, by the
/// web runner's rules; empty when it matches.
pub fn compare(expect: &Expect, got: &Observation, trace: &Trace) -> Vec<String> {
    let mut bad = Vec::new();
    let mut check = |name: &str, same: bool, have: String, want: String| {
        if !same {
            bad.push(format!("{name}: {have}, beklenen {want}"));
        }
    };
    if let Some(want) = &expect.tool {
        check("tool", &got.tool == want, got.tool.clone(), want.clone());
    }
    if let Some(want) = expect.points {
        check(
            "points",
            got.points == want,
            got.points.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = &expect.options {
        check(
            "options",
            &got.options == want,
            format!("{:?}", got.options),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.dynamic_input {
        check(
            "dynamicInput",
            &got.dynamic_input == want,
            format!("{:?}", got.dynamic_input),
            format!("{want:?}"),
        );
    }
    if let Some(want) = &expect.command_line {
        check(
            "commandLine",
            &got.command_line == want,
            format!("{:?}", got.command_line),
            format!("{want:?}"),
        );
    }
    if let Some(want) = expect.entities {
        check(
            "entities",
            got.entities == want,
            got.entities.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = expect.can_undo {
        check(
            "canUndo",
            got.can_undo == want,
            got.can_undo.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = expect.can_redo {
        check(
            "canRedo",
            got.can_redo == want,
            got.can_redo.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = expect.dirty {
        check(
            "dirty",
            got.dirty == want,
            got.dirty.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = &expect.log {
        check(
            "log",
            got.log.as_ref() == Some(want),
            format!("{:?}", got.log),
            want.clone(),
        );
    }
    if let Some(want) = expect.metres_per_pixel {
        check(
            "metresPerPixel",
            (got.metres_per_pixel - want).abs() <= want * 1e-9,
            got.metres_per_pixel.to_string(),
            want.to_string(),
        );
    }
    if let Some(want) = &expect.newest {
        bad.extend(compare_newest(want, got, trace));
    }
    bad
}

fn compare_newest(want: &Newest, got: &Observation, trace: &Trace) -> Vec<String> {
    let mut bad = Vec::new();
    let Some((kind, pts, bulges)) = &got.newest else {
        return vec![format!("newest: yok, beklenen {}", want.kind)];
    };
    if *kind != want.kind {
        return vec![format!("newest: {kind}, beklenen {}", want.kind)];
    }
    let [ox, oy] = trace.view.center;
    if let Some(points) = &want.points {
        // Clicked points come from screen pixels: within the trace's tolerance.
        let near = pts.len() == points.len()
            && pts.iter().zip(points).all(|([x, y], [wx, wy])| {
                (x - ox - wx).hypot(y - oy - wy) <= trace.click_tolerance
            });
        if !near {
            let relative: Vec<[f64; 2]> = pts.iter().map(|[x, y]| [x - ox, y - oy]).collect();
            bad.push(format!(
                "newest.points: {relative:?}, beklenen {points:?} (±{} m)",
                trace.click_tolerance
            ));
        }
    }
    if let Some(arcs) = want.arcs {
        let have = bulges.iter().filter(|b| **b != 0.0).count();
        if have != arcs {
            bad.push(format!("newest.arcs: {have}, beklenen {arcs}"));
        }
    }
    if let Some(edges) = &want.edges {
        // Typed values are exact: consecutive corner differences, not rounded.
        let have: Vec<[f64; 2]> = pts
            .windows(2)
            .map(|w| [w[1][0] - w[0][0], w[1][1] - w[0][1]])
            .collect();
        if have != *edges {
            bad.push(format!("newest.edges: {have:?}, beklenen {edges:?}"));
        }
    }
    bad
}

/// A temporary file for a trace's saves.
pub fn scratch_file(trace: &Trace, variant: Variant) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kentos-iz-{}-{}-{}.kcad",
        std::process::id(),
        trace.id,
        variant.id
    ))
}

/// Plays a whole trace on a new app; its problems, empty when it passes.
#[cfg(test)]
pub fn play_trace(trace: &Trace, variant: Variant) -> Result<Vec<String>, String> {
    let (mut app, _) = App::boot(None);
    let mut player = Player::new(&mut app, trace, variant, AREA, scratch_file(trace, variant))?;
    Ok(player.play(None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size as IcedSize;
    use kentos_ui::snapshot::Snapshot;

    /// Every trace × every variant, as `pnpm e2e:interaction` plays them on the web.
    #[test]
    fn every_trace_passes_in_every_variant() {
        let traces = Trace::all().expect("the traces read");
        assert!(traces.len() >= 4, "the four traces are there");
        let mut report = Vec::new();
        let mut failed = 0;
        for variant in VARIANTS {
            for trace in &traces {
                let problems = play_trace(trace, variant).unwrap_or_else(|e| vec![e]);
                let mark = if problems.is_empty() { "✓" } else { "✗" };
                report.push(format!(
                    "{mark} [{}] {}: {}",
                    variant.id, trace.id, trace.title
                ));
                for p in &problems {
                    report.push(format!("  {p}"));
                }
                if !problems.is_empty() {
                    failed += 1;
                }
            }
        }
        println!("{}", report.join("\n"));
        assert_eq!(failed, 0, "\n{}", report.join("\n"));
    }

    #[test]
    fn a_point_off_the_drawing_area_stops_the_trace() {
        let mut trace = Trace::by_id("polygon-accept").expect("reads");
        trace.steps.truncate(1);
        trace.steps.push(Step {
            run: None,
            key: None,
            text: None,
            move_to: None,
            click: Some([500.0, 0.0]),
            double_click: None,
            right_click: None,
            focus: None,
            save_and_reopen: None,
            expect: None,
            note: None,
        });
        let problems = play_trace(&trace, VARIANTS[0]).expect("starts");
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].contains("çizim alanının dışında"),
            "{problems:?}"
        );
    }

    /// Keys typed and pressed on the real KentOS UI command line, as `view.rs`
    /// configures it, give what `line_messages` says, and the widget takes
    /// exactly those keys: the others reach the subscription.
    #[test]
    fn the_command_line_model_is_the_widget() {
        let (mut app, _) = App::boot(None);
        let trace = Trace::by_id("polygon-keys").expect("reads");
        let mut player = Player::new(
            &mut app,
            &trace,
            VARIANTS[1],
            AREA,
            scratch_file(&trace, VARIANTS[1]),
        )
        .expect("opens");
        let _ = player.apply(Message::Run("tool.polygon"));
        let app = &mut *player.app;
        let size = IcedSize::new(
            1600.0,
            kentos_ui::widget::command_line::height(kentos_ui::widget::command_line::LINES, false),
        );
        let mut ui = Snapshot::software(size).expect("the software renderer");
        let mut deliver = |app: &mut App, event: iced::Event| {
            let (messages, status) = ui.deliver(app.command_line(), &event);
            for m in &messages {
                let _ = app.update(m.clone());
            }
            (messages, status)
        };
        // Click the text box: it takes the keyboard and says so.
        let input = Point::new(1200.0, size.height - 16.0);
        let _ = deliver(
            app,
            iced::Event::Mouse(mouse::Event::CursorMoved { position: input }),
        );
        let (messages, _) = deliver(
            app,
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        );
        assert!(
            matches!(messages.as_slice(), [Message::CommandFocus(true)]),
            "the click focuses the text box: {messages:?}"
        );
        let _ = deliver(
            app,
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        );
        let keys = [
            "1",
            "2",
            "Esc",
            "4",
            "8",
            "7",
            "Backspace",
            "7",
            ",",
            "4",
            "@",
            "+",
            "-",
            ".",
            "Space",
            "Tab",
            "Ctrl+Z",
            "Esc",
        ];
        for key in keys {
            let stroke = chord_stroke(key, Layout::TurkishQ).expect("a key");
            let event = stroke.event();
            let want = line_messages(&app.command_input, &event);
            let (got, status) = deliver(app, event);
            match want {
                Some(want) => {
                    assert_eq!(format!("{got:?}"), format!("{want:?}"), "{key}");
                    assert_eq!(status, event::Status::Captured, "{key} is the text box's");
                }
                None => {
                    assert!(got.is_empty(), "{key}: {got:?}");
                    assert_eq!(
                        status,
                        event::Status::Ignored,
                        "{key} goes on to the subscription"
                    );
                }
            }
        }
        assert!(
            !app.line_focused,
            "Esc on the empty line let the keyboard go"
        );
        assert_eq!(app.session.tool_id(), "select", "and ended the command");
    }
}
