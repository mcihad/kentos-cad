//! Kutupsal dizi: the web's `PolarArrayTool` (`apps/web/src/tools/arrangeTools.ts`)
//! on its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0047):
//!
//! - the objects, then the centre, clicked or typed;
//! - then the options, the copies showing: Adet (N) the count, the
//!   originals among them (2 to 1000); Açı (A) the angle to fill in degrees
//!   (360 a full turn, minus clockwise); Nesneleri döndür (D) whether the
//!   copies turn. A number typed without an option is the count;
//! - a confirm writes it (after N or A, a confirm leaves the question).
//!
//! Written through the product command `cad.entities.array` (polar), one
//! undo step, “Kutupsal dizi”; says “Kutupsal dizi: 6 adet, 360° içinde, 5
//! yeni nesne.” and leaves. Copies that do not turn are placed by the middle
//! of the box of the objects it copies (the locked ones left out), as the
//! command places them; the geometry store measures it.

use kentos_contracts::ArrayLayout;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::tools::editing::polar_array_transforms;
use kentos_geometry_core::tools::point_input::midpoint;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};

use crate::Vec2;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::modify::{Modify, Stages, array_selection};
use crate::points::wire;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory};

/// The polar array tool's id: its command is `tool.arrayPolar`.
pub const ID: &str = "arrayPolar";
pub const LABEL: &str = "Kutupsal dizi";

/// The value a typed number answers (the web's `ask`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Count,
    Fill,
}

/// The polar array tool's stages.
#[derive(Clone, Debug, Default)]
pub struct Polar {
    centre: Option<Vec2>,
    ask: Option<Ask>,
    /// What the session remembers (the web's `PolarArrayTool.last`), seen at every event.
    last: Memory,
    /// The copies' affines with the centre given, from the last event.
    copies: Vec<Affine>,
}

impl Polar {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }
}

/// The middle of the box of the selected objects the array copies (those on
/// locked layers left out), from the geometry store (the web's `centreOf`).
fn middle(cx: &Context<'_>) -> Vec2 {
    let doc = &*cx.doc;
    let ids: Vec<f64> = cx
        .selection
        .ids()
        .iter()
        .filter(|slot| {
            doc.get(**slot)
                .is_some_and(|e| !doc.layers().is_locked(&e.base().layer_id))
        })
        .map(|slot| f64::from(slot.0))
        .collect();
    let b = cx.spatial.store().extent(Some(&ids)).unwrap_or(Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 0.0,
        max_y: 0.0,
    });
    midpoint(Vec2::new(b.min_x, b.min_y), Vec2::new(b.max_x, b.max_y))
}

impl Stages for Polar {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self, _cx: &mut Context<'_>) -> Flow {
        self.centre = None;
        self.ask = None;
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.last = *cx.memory;
        self.copies = match self.centre {
            Some(c) => polar_array_transforms(
                c,
                f64::from(self.last.polar_count),
                self.last.polar_fill,
                self.last.polar_rotate,
                middle(cx),
            ),
            None => Vec::new(),
        };
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let l = &self.last;
        match (self.centre, self.ask) {
            (None, _) => Prompt::new(LABEL, "dizinin merkezini gösterin"),
            (Some(_), Some(Ask::Count)) => Prompt::new(
                LABEL,
                "toplam adedi yazın (seçilen dahil, 2 ile 1000 arası)",
            ),
            (Some(_), Some(Ask::Fill)) => Prompt::new(
                LABEL,
                "doldurma açısını derece olarak yazın (360 tam tur; eksi saat yönünde)",
            ),
            (Some(_), None) => Prompt::new(LABEL, "uygulamak için sağ tıklayın")
                .option_with("Adet", "N", l.polar_count.to_string())
                .option_with("Açı", "A", format!("{}°", js_number(l.polar_fill)))
                .option_with(
                    "Nesneleri döndür",
                    "D",
                    if l.polar_rotate { "evet" } else { "hayır" },
                ),
        }
    }

    fn point(&mut self, p: Vec2, _cx: &mut Context<'_>) -> Flow {
        if self.centre.is_none() {
            self.centre = Some(p);
        }
        Flow::Stay
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        self.centre?;
        let t = upper_tr(js_trim(text));
        match t.as_str() {
            "N" => self.ask = Some(Ask::Count),
            "A" => self.ask = Some(Ask::Fill),
            "D" => cx.memory.polar_rotate = !cx.memory.polar_rotate,
            _ => {
                let n = parse_number(text)?;
                if self.ask == Some(Ask::Fill) {
                    if n.abs() < 1e-9 || n.abs() > 360.0 {
                        cx.say(
                            Level::Warn,
                            "Doldurma açısı 0 ile ±360 derece arasında olmalı.",
                        );
                        return Some(Flow::Stay);
                    }
                    cx.memory.polar_fill = n;
                } else {
                    if n.fract() != 0.0 || !(2.0..=1000.0).contains(&n) {
                        cx.say(Level::Warn, "Adet 2 ile 1000 arasında bir tam sayı olmalı.");
                        return Some(Flow::Stay);
                    }
                    // A whole number from 2 to 1000: exact as u32.
                    cx.memory.polar_count = n as u32;
                }
                self.ask = None;
            }
        }
        Some(Flow::Stay)
    }

    /// Before the centre a typed point is the centre; after it, only the
    /// options and numbers are read (the web's `input`).
    fn typed_points(&self) -> bool {
        self.centre.is_none()
    }

    /// A confirm leaves a question, writes the array with the centre given,
    /// and leaves without one.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.ask.is_some() {
            self.ask = None;
            return Flow::Stay;
        }
        let Some(c) = self.centre else {
            return Flow::Exit;
        };
        let (count, fill, rotate) = (
            cx.memory.polar_count,
            cx.memory.polar_fill,
            cx.memory.polar_rotate,
        );
        let layout = ArrayLayout::Polar {
            center: wire(c),
            count,
            fill,
            rotate,
        };
        if let Some(n) = array_selection(layout, cx) {
            let line = format!(
                "Kutupsal dizi: {count} adet, {}° içinde, {n} yeni nesne.",
                js_number(fill)
            );
            cx.say(Level::Success, line);
        }
        Flow::Exit
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    fn previews(&self, _hover: Option<Vec2>) -> Vec<Affine> {
        self.copies.clone()
    }

    fn tag(&self, _hover: Vec2, _format: &Format) -> Vec<String> {
        match self.centre {
            Some(_) => vec![format!(
                "{} adet · {}°",
                self.last.polar_count,
                js_number(self.last.polar_fill)
            )],
            None => Vec::new(),
        }
    }
}
