//! Yol boyunca dizi (docs/adr/0140): copies of the selected objects along a
//! path, on the selection-first base ([`crate::modify`]):
//!
//! - the objects, selected before or after, then the path: a click on a
//!   line, an arc, a circle or a polyline (on any layer; it is only read).
//!   The path is drawn, with where the copies start;
//! - then the copies show along it, live. A typed number is the count (2 to
//!   10 000, the originals among them, the copies spread from the path's
//!   start to its end), or with Aralık (A) the spacing in metres from the
//!   start, the count then filling the path as far as it goes; Adet (N)
//!   goes back to the count. Hizala (H) turns the copies to the path's
//!   direction (on by default). All four stay for as long as the app lives
//!   ([`crate::tool::Memory`]);
//! - Enter or a quick right click writes; Esc lets go of the path, then leaves.
//!
//! Written through `cad.entities.array` (a path layout), one undo step,
//! “Yol boyunca dizi”: “Yol boyunca dizi: 5 adet, 8 yeni nesne.” The copies'
//! places are the shared core's (`path_array_transforms`), the same the
//! command computes.

use kentos_contracts::{ArrayLayout, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::ops::path::{path_of, point_at_s};
use kentos_geometry_core::tools::editing::path_array_transforms;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages, array_selection};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Tone};

/// The tool's id: its command is `tool.arrayPath`.
pub const ID: &str = "arrayPath";
pub const LABEL: &str = "Yol boyunca dizi";

/// Most places along a path.
const MOST: u32 = 10_000;

/// What a path can be: a line, an arc, a circle or a polyline.
fn follows(e: &Entity, _: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Arc(_) | Entity::Circle(_) | Entity::Polyline(_)
    )
}

/// What the copies were made from, to know when they are stale.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Key {
    generation: u64,
    slot: Slot,
    count: u32,
    spacing: Option<f64>,
    align: bool,
}

/// Yol boyunca dizi's stages.
#[derive(Clone, Debug, Default)]
pub struct ArrayPath {
    path: Option<Slot>,
    /// What the session remembers and the project's units, seen at every event.
    last: Memory,
    format: Format,
    /// The count and spacing the copies are laid out with, and their affines.
    plan: Option<(Key, Vec<Affine>)>,
    /// The path drawn, for the overlay.
    outline: Outline,
    start: Option<Vec2>,
}

impl ArrayPath {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The count and the spacing for the path `shape`: the typed count, or
    /// with Aralık as many places as the spacing fits along it.
    fn layout(&self, shape: &kentos_geometry_core::entity::Shape) -> Option<(u32, Option<f64>)> {
        let m = &self.last;
        if !m.path_by_spacing {
            return Some((m.path_count, None));
        }
        let s = m.path_spacing;
        let length = path_of(shape)?.length;
        if s.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || !(length / s).is_finite() {
            return None;
        }
        // The places that fit, tried down from the most: the core decides what fits.
        let mut n = ((length / s + 1e-9).floor() + 1.0).min(f64::from(MOST)) as u32;
        while n >= 2 {
            if path_array_transforms(shape, f64::from(n), Some(s), m.path_align).is_some() {
                return Some((n, Some(s)));
            }
            n -= 1;
        }
        None
    }

    /// The copies along the picked path, kept until something they depend on changes.
    fn refresh(&mut self, cx: &Context<'_>) {
        self.last = *cx.memory;
        self.format = cx.format();
        let Some(slot) = self.path else {
            self.plan = None;
            self.outline = Outline::default();
            self.start = None;
            return;
        };
        let Some(path) = cx.doc.get(slot).map(shape) else {
            self.path = None;
            self.plan = None;
            return;
        };
        let generation = cx.doc.generation();
        let laid = self.layout(&path);
        let key = laid.map(|(count, spacing)| Key {
            generation,
            slot,
            count,
            spacing,
            align: self.last.path_align,
        });
        if self.plan.as_ref().map(|(k, _)| *k) == key && key.is_some() {
            return;
        }
        self.plan = laid.zip(key).and_then(|((count, spacing), key)| {
            let affines =
                path_array_transforms(&path, f64::from(count), spacing, self.last.path_align)?;
            Some((key, affines))
        });
        self.outline = Outline::of(&path, None, 2.0, Tone::Snap);
        self.start = path_of(&path).map(|p| point_at_s(&p, 0.0));
    }

    /// The count the copies are laid out with, when they are.
    fn count(&self) -> Option<u32> {
        self.plan.as_ref().map(|(k, _)| k.count)
    }

    /// A path picked by a click: not one of the objects to copy.
    fn choose(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick(p, cx, follows) else {
            cx.say(
                Level::Warn,
                "Yol olarak bir çizgiye, yaya, daireye ya da çoklu çizgiye tıklayın.",
            );
            return;
        };
        if cx.selection.ids().contains(&slot) {
            cx.say(
                Level::Warn,
                "Yol, kopyalanacak nesnelerden biri olamaz; başka bir nesneye tıklayın.",
            );
            return;
        }
        let long = cx
            .doc
            .get(slot)
            .and_then(|e| path_of(&shape(e)))
            .map(|p| p.length);
        if long.is_none_or(|l| l < 1e-9) {
            cx.say(
                Level::Warn,
                "Bu yolun uzunluğu yok; başka bir nesneye tıklayın.",
            );
            return;
        }
        self.path = Some(slot);
        cx.selection.set_hover(None);
        self.refresh(cx);
    }

    fn typed_number(&mut self, n: f64, cx: &mut Context<'_>) {
        if self.last.path_by_spacing {
            if n > 0.0 {
                cx.memory.path_spacing = n;
            } else {
                cx.say(Level::Warn, "Aralık sıfırdan büyük olmalı.");
            }
        } else if n.fract() != 0.0 || !(2.0..=f64::from(MOST)).contains(&n) {
            cx.say(
                Level::Warn,
                "Adet 2 ile 10 000 arasında bir tam sayı olmalı.",
            );
        } else {
            // A whole number from 2 to 10 000: exact as u32.
            cx.memory.path_count = n as u32;
        }
    }
}

impl Stages for ArrayPath {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.path = None;
        self.refresh(cx);
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.refresh(cx);
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let l = &self.last;
        let f = self.format;
        if self.path.is_none() {
            return Prompt::new(LABEL, "yolu seçin: çizgi, yay, daire ya da çoklu çizgi");
        }
        let align = if l.path_align { "evet" } else { "hayır" };
        // The count is in the tag by the cursor; the prompt keeps to what fits a narrow window.
        let prompt = if l.path_by_spacing {
            Prompt::new(
                LABEL,
                format!("aralığı yazın (Enter: {})", f.length(l.path_spacing)),
            )
            .then()
            .option("Adet", "N")
        } else {
            Prompt::new(LABEL, format!("adedi yazın (Enter: {} adet)", l.path_count))
                .then()
                .option("Aralık", "A")
        };
        prompt
            .option_with("Hizala", "H", align)
            .option("Uygula", "Enter")
    }

    /// A click picks the path (or another); a move lights the object it would pick.
    fn pointer(&mut self, p: &Pointer, down: bool, cx: &mut Context<'_>) -> Option<Flow> {
        if !down {
            if self.path.is_none() {
                edge::hover(p, cx, follows);
            }
            return None;
        }
        self.choose(p, cx);
        Some(Flow::Stay)
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    /// Adet (N), Aralık (A), Hizala (H), and a number for the one that rules.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        match upper_tr(js_trim(text)).as_str() {
            "N" => cx.memory.path_by_spacing = false,
            "A" => cx.memory.path_by_spacing = true,
            "H" => cx.memory.path_align = !cx.memory.path_align,
            _ => {
                let n = parse_number(text)?;
                if self.path.is_none() {
                    cx.say(Level::Warn, "Önce dizinin yolunu seçin.");
                } else {
                    self.typed_number(n, cx);
                }
            }
        }
        self.refresh(cx);
        Some(Flow::Stay)
    }

    /// The path is picked, or a point is not read: only counts, spacings and options.
    fn typed_points(&self) -> bool {
        false
    }

    /// Writes the array along the path; with none picked, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let (Some(slot), Some(count)) = (self.path, self.count()) else {
            if self.path.is_none() {
                return Flow::Exit;
            }
            let f = self.format;
            let text = if self.last.path_by_spacing {
                format!(
                    "{} aralık yola sığmıyor; dizi oluşmuyor. Daha küçük bir aralık yazın.",
                    f.length(self.last.path_spacing)
                )
            } else {
                "Yolun boyu yetmiyor; dizi oluşmuyor.".to_owned()
            };
            cx.say(Level::Warn, text);
            return Flow::Stay;
        };
        let spacing = self.plan.as_ref().and_then(|(k, _)| k.spacing);
        let layout = ArrayLayout::Path {
            path: edge::uid(cx.doc, slot),
            count,
            spacing,
            align: cx.memory.path_align,
        };
        let Some(n) = array_selection(layout, cx) else {
            return Flow::Stay;
        };
        cx.say(
            Level::Success,
            format!("Yol boyunca dizi: {count} adet, {n} yeni nesne."),
        );
        Flow::Exit
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    fn previews(&self, _hover: Option<Vec2>) -> Vec<Affine> {
        self.plan
            .as_ref()
            .map(|(_, affines)| affines.clone())
            .unwrap_or_default()
    }

    fn tag(&self, _hover: Vec2, format: &Format) -> Vec<String> {
        let Some((key, _)) = &self.plan else {
            return if self.path.is_some() {
                vec!["Sığmıyor".to_owned()]
            } else {
                Vec::new()
            };
        };
        let mut lines = vec![format!("{} adet", key.count)];
        if let Some(s) = key.spacing {
            lines.push(format!("aralık {}", format.length(s)));
        }
        lines
    }

    /// Esc lets go of the path.
    fn back(&mut self, cx: &mut Context<'_>) -> bool {
        if self.path.take().is_none() {
            return false;
        }
        self.refresh(cx);
        true
    }

    /// The path drawn, and a ring where the copies start.
    fn overlay(&self) -> Preview {
        Preview {
            strokes: self.outline.strokes.clone(),
            markers: self
                .start
                .map(|at| Marker {
                    at,
                    shape: MarkerShape::Ring(6.0),
                    tone: Tone::Snap,
                })
                .into_iter()
                .collect(),
            ..Preview::default()
        }
    }
}
