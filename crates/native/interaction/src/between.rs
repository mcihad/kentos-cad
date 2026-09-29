//! Ara nokta (docs/adr/0140): points on the line between two points, three
//! ways. The ribbon starts a method by sending its letter right after the
//! tool, and the same letters work typed on the command line: `E`, `U`, `O`.
//!
//! - **Eşit aralık** (the default): the line in equal parts, the number of
//!   parts typed (2 to 10 000);
//! - **Uzaklıkla** (`U`): distances from the first point, metres, typed
//!   with commas between (`5, 12.5`);
//! - **Oranla** (`O`): fractions of the way from the first point to the
//!   second, from 0 to 1, typed with commas between (`0.25, 0.5`).
//!
//! The two points are clicked (snapping) or typed. Before the second one
//! the points to come follow the cursor; once both are in, they are shown at
//! the value kept from last time: a typed value writes at once, Enter writes
//! with the kept one. The count, the distances and the ratios stay for as
//! long as the app lives ([`crate::tool::Memory`]). The tool then asks for
//! the next two points; Esc steps back a point.
//!
//! The points go on the active layer through `cad.entities.create`, one undo
//! step (“Ekle”): “3 nokta kondu.” Where they fall is the shared core's
//! (`construct::points_between`).

use kentos_contracts::EntityGeometry;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::construct::{Between, points_between};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number, point_from_text};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, plain_number, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Stroke, Tag, Tone, Tool, Values,
};

/// The tool's id: its command is `tool.pointsBetween`.
pub const ID: &str = "pointsBetween";
pub const LABEL: &str = "Ara nokta";

/// Most points the tool places, and most the preview marks.
const MAX_POINTS: f64 = 10_000.0;
const MAX_SHOWN: usize = 2000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    Parts,
    Distances,
    Ratios,
}

/// Ara nokta.
#[derive(Clone, Debug)]
pub struct PointsBetween {
    d: Taken,
    method: Method,
    /// What the session remembered and the project's units, as of the last event.
    seen: Option<(Memory, Format)>,
}

impl PointsBetween {
    pub fn new() -> Self {
        Self {
            d: Taken::default(),
            method: Method::Parts,
            seen: None,
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    fn format(&self) -> Format {
        self.seen.map(|(_, f)| f).unwrap_or_default()
    }

    /// The points between `a` and `b` at the kept value, or why there are
    /// none: a short reason for the tag and the whole sentence.
    fn compute(&self, a: Vec2, b: Vec2) -> Result<Vec<Vec2>, (&'static str, String)> {
        let (m, f) = (self.memory(), self.format());
        let length = dist(a, b);
        if length < points::SAME {
            return Err((
                "Nokta çakışıyor",
                "İki nokta çakışıyor; ikinci noktayı ilkinden ayrı bir yere koyun.".into(),
            ));
        }
        let how = match self.method {
            Method::Parts => Between::Parts(f64::from(m.between_parts)),
            Method::Distances => {
                let list = m.between_distances.as_slice();
                if list.is_empty() {
                    return Err(("Uzaklık yazın", "Uzaklıkları yazın, ör. 5, 12.5.".into()));
                }
                if let Some(d) = list.iter().find(|d| !(0.0..=length).contains(*d)) {
                    return Err((
                        "Uzaklık dışarıda",
                        format!(
                            "{} uzaklığı iki nokta arasının ({}) dışında kalıyor; 0 ile {} arasında yazın.",
                            f.length(*d),
                            f.length(length),
                            f.length(length)
                        ),
                    ));
                }
                Between::Distances(list.to_vec())
            }
            Method::Ratios => {
                let list = m.between_ratios.as_slice();
                if list.is_empty() {
                    return Err(("Oran yazın", "Oranları yazın, ör. 0.25, 0.5.".into()));
                }
                if let Some(t) = list.iter().find(|t| !(0.0..=1.0).contains(*t)) {
                    return Err((
                        "Oran dışarıda",
                        format!(
                            "{t} oranı 0 ile 1 arasında değil; iki noktanın arasında kalan oranlar yazın."
                        ),
                    ));
                }
                Between::Ratios(list.to_vec())
            }
        };
        points_between(a, b, &how)
            .filter(|p| !p.is_empty())
            .ok_or_else(|| {
                (
                    "Parça yok",
                    "Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.".to_owned(),
                )
            })
    }

    /// A point given (the web's `accept`): the first two are kept; with both
    /// in, a click has nothing to say, so it is left.
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.see(cx);
        if self.d.pts.len() >= 2 {
            return;
        }
        if self.d.last().is_some_and(|q| dist(q, p) < points::SAME) {
            cx.say(
                Level::Warn,
                "İkinci nokta ilkiyle çakışıyor; başka bir yer gösterin.",
            );
            return;
        }
        self.d.begin(p, cx);
        self.d.pts.push(p);
    }

    /// Writes the points at the kept value and starts over; a refusal is
    /// said and the two points stay.
    fn write(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
        let (Some(&a), Some(&b)) = (self.d.pts.first(), self.d.pts.get(1)) else {
            return;
        };
        let pts = match self.compute(a, b) {
            Ok(pts) => pts,
            Err((_, why)) => {
                cx.say(Level::Warn, why);
                return;
            }
        };
        let n = pts.len();
        let objects = pts
            .into_iter()
            .map(|p| EntityGeometry::Point {
                p: wire(p),
                z: None,
            })
            .collect();
        if points::write_objects(
            objects,
            Some(kentos_contracts::CreateOperation::PointsBetween),
            cx,
        )
        .is_some()
        {
            cx.say(Level::Success, format!("{n} nokta kondu."));
        }
        self.d.reset();
    }

    /// The value typed for the running method; false when it is not one.
    fn value(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match self.method {
            Method::Parts => {
                let Some(n) = plain_number(text) else {
                    return false;
                };
                if n.fract() != 0.0 || !(2.0..=MAX_POINTS).contains(&n) {
                    cx.say(
                        Level::Warn,
                        "Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.",
                    );
                    return true;
                }
                cx.memory.between_parts = n as u32;
            }
            Method::Distances | Method::Ratios => {
                let Some(list) = number_list(text) else {
                    return false;
                };
                let Some(values) = Values::from_slice(&list) else {
                    cx.say(
                        Level::Warn,
                        format!("En çok {} değer yazılabilir.", Values::MAX),
                    );
                    return true;
                };
                if self.method == Method::Distances {
                    cx.memory.between_distances = values;
                } else {
                    cx.memory.between_ratios = values;
                }
            }
        }
        self.write(cx);
        true
    }

    /// The methods other than the running one, as options.
    fn options(&self, prompt: Prompt) -> Prompt {
        let prompt = prompt.then();
        match self.method {
            Method::Parts => prompt.option("Uzaklıkla", "U").option("Oranla", "O"),
            Method::Distances => prompt.option("Eşit aralık", "E").option("Oranla", "O"),
            Method::Ratios => prompt.option("Eşit aralık", "E").option("Uzaklıkla", "U"),
        }
    }

    /// What the kept value is, for the prompt: `4 parça`, `5, 12.5 m`, `0.25, 0.5`.
    fn kept(&self) -> String {
        let m = self.memory();
        match self.method {
            Method::Parts => format!("{} parça", m.between_parts),
            Method::Distances if m.between_distances.is_empty() => "uzaklık yok".to_owned(),
            Method::Distances if m.between_distances.as_slice().len() > 3 => {
                format!("{} uzaklık", m.between_distances.as_slice().len())
            }
            Method::Distances => format!(
                "{} m",
                join(m.between_distances.as_slice(), crate::format::js_number)
            ),
            Method::Ratios if m.between_ratios.is_empty() => "oran yok".to_owned(),
            Method::Ratios if m.between_ratios.as_slice().len() > 3 => {
                format!("{} oran", m.between_ratios.as_slice().len())
            }
            Method::Ratios => join(m.between_ratios.as_slice(), crate::format::js_number),
        }
    }
}

impl Default for PointsBetween {
    fn default() -> Self {
        Self::new()
    }
}

/// Numbers with commas, semicolons or spaces between them; none for anything
/// else, and for none at all.
fn number_list(text: &str) -> Option<Vec<f64>> {
    let list: Option<Vec<f64>> = js_trim(text)
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(parse_number)
        .collect();
    list.filter(|l| !l.is_empty())
}

fn join(list: &[f64], one: impl Fn(f64) -> String) -> String {
    list.iter().map(|v| one(*v)).collect::<Vec<_>>().join(", ")
}

impl Tool for PointsBetween {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let kept = self.kept();
        let prompt = match (self.d.pts.len(), self.method) {
            (0, _) => Prompt::new(LABEL, "ilk noktayı belirtin").note(kept),
            (1, _) => Prompt::new(LABEL, "ikinci noktayı belirtin").note(kept),
            (_, Method::Parts) => {
                Prompt::new(LABEL, format!("parça sayısını yazın (Enter: {kept})"))
            }
            (_, Method::Distances) => {
                Prompt::new(LABEL, format!("uzaklıkları virgülle yazın (Enter: {kept})"))
            }
            (_, Method::Ratios) => Prompt::new(
                LABEL,
                format!("0 ile 1 arası oranları yazın (Enter: {kept})"),
            ),
        };
        self.options(prompt)
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn accepts_points(&self) -> bool {
        self.d.pts.len() < 2
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.d.pts.len() >= 2 {
            return false;
        }
        self.accept(p, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let point = self.d.constrain(p, cx);
        self.accept(point, cx);
    }

    /// The method letters; with both points in, the value of the running
    /// method; before that, a point.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        match upper_tr(js_trim(text)).as_str() {
            "E" => self.method = Method::Parts,
            "U" => self.method = Method::Distances,
            "O" => self.method = Method::Ratios,
            _ if self.d.pts.len() >= 2 => return self.value(text, cx),
            _ => {
                let Some(p) =
                    point_from_text(text, self.d.last(), self.d.hover, |d| cx.track_along(d))
                else {
                    return false;
                };
                self.accept(p, cx);
                return true;
            }
        }
        self.see(cx);
        true
    }

    /// Both points in: writes at the kept value. One point: it is let go.
    /// None: the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        match self.d.pts.len() {
            0 => Flow::Exit,
            1 => {
                self.d.pts.clear();
                Flow::Stay
            }
            _ => {
                self.write(cx);
                Flow::Stay
            }
        }
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.d.pts.pop().is_some()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(true, cx)
    }

    /// The line between the points with the points to come on it; before the
    /// second point, the cursor stands for it.
    fn preview(&self, _format: &Format) -> Preview {
        let Some(&a) = self.d.pts.first() else {
            return Preview::default();
        };
        let Some(b) = self.d.pts.get(1).copied().or(self.d.hover) else {
            return Preview::default();
        };
        let mut preview = Preview {
            strokes: vec![Stroke::dashed(vec![a, b], false, [5.0, 3.0])],
            markers: [a, b]
                .into_iter()
                .map(|at| Marker {
                    at,
                    shape: MarkerShape::Ring(7.5),
                    tone: Tone::Snap,
                })
                .collect(),
            tracking: self.d.tracking,
            ..Preview::default()
        };
        let f = self.format();
        let tag = match self.compute(a, b) {
            Ok(pts) => {
                let n = pts.len();
                let mut lines = vec![format!("{n} nokta")];
                if self.method == Method::Parts && n > 0 {
                    lines.push(format!("aralık {}", f.length(dist(a, b) / (n + 1) as f64)));
                }
                if n <= MAX_SHOWN {
                    preview.markers.extend(pts.into_iter().map(|at| Marker {
                        at,
                        shape: MarkerShape::Ring(4.5),
                        tone: Tone::Accent,
                    }));
                }
                lines
            }
            Err((short, _)) => {
                preview.tag_tone = Tone::Danger;
                vec![short.to_owned()]
            }
        };
        preview.tag = self.d.hover.map(|at| Tag { at, lines: tag });
        preview
    }
}
