//! Ölçekle: the web's `ScaleTool` (`apps/web/src/tools/modifyTools.ts`) on
//! its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0037):
//!
//! - the objects, the base point, then a typed factor; or lengths: a point
//!   shows the reference length from the base, the next the new length;
//! - Referans (R): the reference length between two points (or typed), then
//!   the new length shown from the base or typed;
//! - Kopya (K): the originals stay and scaled copies are added.
//!
//! Written through `cad.entities.transform` (a scale about the base), one
//! undo step, “Ölçekle”; says “2 nesne 2.0000 faktörüyle ölçeklendi.” and
//! leaves. A factor or length not above zero warns and is not taken. The
//! ghosts and the factor beside the cursor are the web's.

use kentos_contracts::Transform;
use kentos_geometry_core::geom::affine::{Affine, scaling};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::editing::scale_factor;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::{Format, fixed};
use crate::log::Level;
use crate::modify::{Modify, Stages, transform_selection};
use crate::points::{SAME, plain_number, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow};

/// The scale tool's id: its command is `tool.scale`.
pub const ID: &str = "scale";
pub const LABEL: &str = "Ölçekle";

/// The scale tool's stages.
#[derive(Clone, Debug, Default)]
pub struct Scale {
    base: Option<Vec2>,
    /// The point that showed the reference length from the base.
    reference: Option<Vec2>,
    copy: bool,
    ref_mode: bool,
    /// Referans: its first end, then its length once known.
    ref_from: Option<Vec2>,
    ref_length: Option<f64>,
}

impl Scale {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The factor with the cursor at `hover` (the web's `factor`).
    fn factor(&self, hover: Vec2) -> Option<f64> {
        let base = self.base?;
        if self.ref_mode {
            return self
                .ref_length
                .filter(|l| *l != 0.0)
                .map(|l| scale_factor(base, hover, l));
        }
        self.reference
            .map(|r| scale_factor(base, hover, dist(base, r)))
    }

    /// Scales by `f` and leaves; a factor that is not a positive number is not taken.
    fn scale(&mut self, f: f64, cx: &mut Context<'_>) -> Flow {
        let Some(base) = self.base.filter(|_| f > 0.0 && f.is_finite()) else {
            return Flow::Stay;
        };
        let t = Transform::Scale {
            center: wire(base),
            factor: f,
        };
        if let Some(n) = transform_selection(t, self.copy, cx) {
            let copy = if self.copy { " (kopya)" } else { "" };
            let line = format!("{n} nesne {} faktörüyle ölçeklendi{copy}.", fixed(f, 4));
            cx.say(Level::Success, line);
        }
        Flow::Exit
    }
}

impl Stages for Scale {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self, _cx: &mut Context<'_>) -> Flow {
        self.base = None;
        self.reference = None;
        self.ref_from = None;
        self.ref_mode = false;
        self.ref_length = None;
        Flow::Stay
    }

    fn anchor(&self) -> Option<Vec2> {
        if self.ref_mode && self.ref_length.is_none() {
            self.ref_from
        } else {
            self.base
        }
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let copy = if self.copy { "açık" } else { "kapalı" };
        if self.base.is_none() {
            return Prompt::new(LABEL, "temel noktayı belirtin");
        }
        if self.ref_mode && self.ref_length.is_none() {
            let step = if self.ref_from.is_some() {
                "referans uzunluğun ikinci ucunu gösterin"
            } else {
                "referans uzunluğun ilk ucunu gösterin ya da uzunluğu yazın"
            };
            return Prompt::new(LABEL, step);
        }
        if self.ref_mode {
            return Prompt::new(LABEL, "yeni uzunluğu temel noktadan gösterin ya da yazın")
                .option_with("Kopya", "K", copy);
        }
        if self.reference.is_none() {
            return Prompt::new(
                LABEL,
                "ölçek faktörünü yazın ya da referans uzunluk için bir nokta gösterin",
            )
            .option("Referans", "R")
            .option_with("Kopya", "K", copy);
        }
        Prompt::new(LABEL, "yeni uzunluğu gösterin ya da faktör yazın")
            .option_with("Kopya", "K", copy)
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        let Some(base) = self.base else {
            self.base = Some(p);
            return Flow::Stay;
        };
        if self.ref_mode {
            if let Some(length) = self.ref_length {
                return self.scale(scale_factor(base, p, length), cx);
            }
            match self.ref_from {
                None => self.ref_from = Some(p),
                Some(from) if dist(from, p) > SAME => self.ref_length = Some(dist(from, p)),
                Some(_) => {}
            }
            return Flow::Stay;
        }
        match self.reference {
            None => {
                if dist(base, p) > SAME {
                    self.reference = Some(p);
                }
                Flow::Stay
            }
            Some(r) => self.scale(scale_factor(base, p, dist(base, r)), cx),
        }
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        self.base?;
        let t = upper_tr(js_trim(text));
        if t == "K" {
            self.copy = !self.copy;
            return Some(Flow::Stay);
        }
        if t == "R" && !self.ref_mode && self.reference.is_none() {
            self.ref_mode = true;
            return Some(Flow::Stay);
        }
        let n = plain_number(text)?;
        if n <= 0.0 {
            let why = if self.ref_mode {
                "Uzunluk sıfırdan büyük olmalı."
            } else {
                "Ölçek faktörü sıfırdan büyük olmalı."
            };
            cx.say(Level::Warn, why);
            return Some(Flow::Stay);
        }
        match (self.ref_mode, self.ref_length) {
            (true, None) => {
                self.ref_length = Some(n);
                self.ref_from = None;
                Some(Flow::Stay)
            }
            (true, Some(length)) => Some(self.scale(n / length, cx)),
            (false, _) => Some(self.scale(n, cx)),
        }
    }

    fn preview(&self, hover: Vec2) -> Option<Affine> {
        let base = self.base?;
        self.factor(hover)
            .filter(|f| *f > 0.0)
            .map(|f| scaling(f, base))
    }

    fn tag(&self, hover: Vec2, _format: &Format) -> Vec<String> {
        // The web shows it unless it is 0 (or NaN).
        self.factor(hover)
            .filter(|f| *f != 0.0 && !f.is_nan())
            .map(|f| vec![format!("Faktör {}", fixed(f, 4))])
            .unwrap_or_default()
    }
}
