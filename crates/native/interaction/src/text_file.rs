//! Metin dosyası yerleştir (docs/adr/0145 §6; the web's
//! `tools/textFileTool.ts` and `model/textFile.ts`):
//!
//! - as the tool starts it asks the host for a text file
//!   (`ViewChange::OpenTextFile`); the host gives back its name and bytes, or
//!   none (the picker cancelled), with [`Tool::file_given`];
//! - the file's lines ([`lines`]): a file of more than 1 MB, one that is not
//!   UTF-8, one of more than 10 000 lines or one of nothing but empty lines
//!   is refused, why said, and the tool leaves;
//! - then a point: each line becomes a text with Yazı's options (height,
//!   angle, alignment, width factor, mask; the session's `Memory`), the
//!   lines one under the other 1.5 heights apart down the texts' own up, an
//!   empty line keeping its place; one step “Metin dosyası yerleştir”
//!   (`cad.entities.create`'s `textFile`), and the tool leaves.
//!
//! While it waits for the point the lines' boxes follow the pointer.

use kentos_contracts::{CreateOperation, EntityGeometry};
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::drawing_font;

use crate::Vec2;
use crate::format::{Format, fixed, js_number};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::Prompt;
use crate::text::align_name;
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Stroke, Tone, Tool, ViewChange};

/// The tool's id: its command is `tool.placeTextFile`.
pub const ID: &str = "placeTextFile";
pub const LABEL: &str = "Metin dosyası yerleştir";

/// The largest file read, bytes (1 MB).
pub const MAX_BYTES: usize = 1024 * 1024;
/// The most lines placed.
pub const MAX_LINES: usize = 10_000;
/// The lines' spacing, in text heights.
const SPACING: f64 = 1.5;
/// The lines whose boxes the preview draws.
const PREVIEW_LINES: usize = 200;

/// Why a file is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    TooBig,
    NotUtf8,
    TooMany,
    Empty,
}

/// A refused file: why, and what to say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileError {
    pub kind: Refused,
    pub message: String,
}

/// The lines of the file `name` (the web's `textFileLines`), each to become a
/// text, trimmed as JavaScript trims, an empty one keeping its place. Lines
/// end at `\r\n`, `\n` or `\r`; a last line break ends the last line; a byte
/// order mark is no letter. fixtures/text/v1/file.json holds both platforms
/// to the rule.
pub fn lines(name: &str, bytes: &[u8]) -> Result<Vec<String>, FileError> {
    let refuse = |kind, message: String| Err(FileError { kind, message });
    if bytes.len() > MAX_BYTES {
        return refuse(
            Refused::TooBig,
            format!(
                "“{name}” 1 MB'tan büyük ({} MB); en çok 1 MB okunur. Dosyayı bölüp yeniden deneyin.",
                fixed(bytes.len() as f64 / MAX_BYTES as f64, 1)
            ),
        );
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return refuse(
            Refused::NotUtf8,
            format!("“{name}” UTF-8 değil. Dosyayı UTF-8 olarak kaydedip yeniden deneyin."),
        );
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    // Split as JavaScript's split(/\r\n|\n|\r/), then a last empty piece dropped.
    let mut pieces: Vec<&str> = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                pieces.push(&text[start..i]);
                i += if bytes.get(i + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                start = i;
            }
            b'\n' => {
                pieces.push(&text[start..i]);
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    pieces.push(&text[start..]);
    if pieces.last() == Some(&"") {
        pieces.pop();
    }
    if pieces.len() > MAX_LINES {
        return refuse(
            Refused::TooMany,
            format!(
                "“{name}” {} satır; en çok {MAX_LINES} satır yerleştirilir. Dosyayı bölüp yeniden deneyin.",
                pieces.len()
            ),
        );
    }
    let trimmed: Vec<String> = pieces.iter().map(|l| js_trim(l).to_owned()).collect();
    if trimmed.iter().all(String::is_empty) {
        return refuse(
            Refused::Empty,
            format!("“{name}” boş: yerleştirilecek satır yok."),
        );
    }
    Ok(trimmed)
}

/// A file read: its name, lines and each line's width at a height of 1.
#[derive(Clone, Debug)]
struct Loaded {
    name: String,
    lines: Vec<String>,
    widths: Vec<f64>,
}

/// The tool.
#[derive(Clone, Debug, Default)]
pub struct PlaceTextFile {
    d: Taken,
    file: Option<Loaded>,
    done: bool,
    /// What the session remembered, as of the last call.
    seen: Option<Memory>,
    /// The project's plot scale, as of the last call: paper millimetres to metres.
    scale: f64,
}

/// Paper millimetres as metres at the project's plot scale.
fn paper(mm: f64, cx: &Context<'_>) -> f64 {
    mm / 1000.0 * cx.doc.settings().plot_scale
}

impl PlaceTextFile {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some(*cx.memory);
        self.scale = cx.doc.settings().plot_scale;
    }

    /// The lines written from `p`, one step; the tool leaves.
    fn place(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(file) = &self.file else {
            return;
        };
        points::echo(p, cx);
        let layers = cx.doc.layers();
        let active = layers.active();
        if layers.is_locked(active) {
            let name = layers.get(active).map_or(active, |n| n.name.as_str());
            cx.say(
                Level::Warn,
                format!(
                    "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
                ),
            );
            return;
        }
        let m = *cx.memory;
        let height = paper(m.text_height_mm, cx);
        let r = m.text_angle.to_radians();
        // Down the texts' own up: each line 1.5 heights under the one before.
        let down = Vec2::new(r.sin(), -r.cos());
        let geometries: Vec<EntityGeometry> = file
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
            .map(|(i, line)| {
                let step = SPACING * height * i as f64;
                EntityGeometry::Text {
                    p: points::wire(Vec2::new(p.x + down.x * step, p.y + down.y * step)),
                    text: line.clone(),
                    height,
                    rotation: m.text_angle,
                    align: m.text_align,
                    width_factor: (m.text_width_factor != 1.0).then_some(m.text_width_factor),
                    mask: m.text_mask,
                }
            })
            .collect();
        let n = geometries.len();
        if points::write_objects(geometries, Some(CreateOperation::TextFile), cx).is_some() {
            cx.say(
                Level::Success,
                format!("“{}”: {n} satır yazı olarak yerleştirildi.", file.name),
            );
        }
        self.done = true;
    }
}

impl Tool for PlaceTextFile {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let Some(file) = &self.file else {
            return Prompt::new(LABEL, "metin dosyasını seçin");
        };
        let m = self.seen.unwrap_or_default();
        let texts = file.lines.iter().filter(|l| !l.is_empty()).count();
        Prompt::new(LABEL, "ilk satırın başlangıcına tıklayın")
            .note(format!("“{}”: {texts} yazı", file.name))
            .then()
            .note(format!(
                "Yazı'nın seçenekleriyle: {} mm, {}°, {}",
                js_number(m.text_height_mm),
                js_number(
                    fixed(m.text_angle, 4)
                        .parse::<f64>()
                        .unwrap_or(m.text_angle)
                ),
                align_name(m.text_align)
            ))
    }

    fn point_count(&self) -> usize {
        0
    }

    /// The file is asked for as the tool starts.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        cx.view_changes.push(ViewChange::OpenTextFile);
        Flow::Stay
    }

    fn file_given(&mut self, file: Option<(&str, &[u8])>, cx: &mut Context<'_>) {
        let Some((name, bytes)) = file else {
            self.done = true;
            return;
        };
        match lines(name, bytes) {
            Ok(lines) => {
                let font = drawing_font(cx.doc.settings().drawing_font);
                // An empty line has no box: the core measures an empty text
                // as a replacement mark, so that it can be picked.
                let widths = lines
                    .iter()
                    .map(|text| {
                        if text.is_empty() {
                            return 0.0;
                        }
                        TextPlace {
                            p: Vec2::new(0.0, 0.0),
                            text,
                            height: 1.0,
                            rotation: 0.0,
                            align: None,
                            width_factor: None,
                        }
                        .width(font)
                    })
                    .collect();
                self.file = Some(Loaded {
                    name: name.to_owned(),
                    lines,
                    widths,
                });
            }
            Err(e) => {
                cx.say(Level::Warn, e.message);
                self.done = true;
            }
        }
        self.see(cx);
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let point = self.d.constrain(p, cx);
        self.place(point, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.file.is_none() {
            return false;
        }
        let Some(p) = cx.typed_point(text, None, self.d.hover) else {
            return false;
        };
        self.place(p, cx);
        self.see(cx);
        true
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Each line's box about the pointer, as the texts will stand (the first 200).
    fn preview(&self, _format: &Format) -> Preview {
        let (Some(h), Some(file)) = (self.d.hover, &self.file) else {
            return Preview::default();
        };
        let m = self.seen.unwrap_or_default();
        let (along, up) = m.text_align.map_or((0.0, 0.0), |a| (a.along(), a.up()));
        let height = m.text_height_mm / 1000.0 * self.scale;
        let r = m.text_angle.to_radians();
        let (dx, dy) = (r.cos(), r.sin());
        // A point `x` along the texts' baseline and `y` up from it, counted from the pointer.
        let at = |x: f64, y: f64| Vec2::new(h.x + dx * x - dy * y, h.y + dy * x + dx * y);
        let strokes = file
            .widths
            .iter()
            .take(PREVIEW_LINES)
            .enumerate()
            .filter(|(_, w)| **w > 0.0)
            .map(|(i, w)| {
                let width = w * height * m.text_width_factor;
                let (x0, y0) = (-along * width, -up * height - SPACING * height * i as f64);
                Stroke {
                    pts: vec![
                        at(x0, y0),
                        at(x0 + width, y0),
                        at(x0 + width, y0 + height),
                        at(x0, y0 + height),
                    ],
                    closed: true,
                    dash: Some([3.0, 3.0]),
                    width: 1.0,
                    tone: Tone::Accent,
                }
            })
            .collect();
        Preview {
            strokes,
            ..Preview::default()
        }
    }
}
