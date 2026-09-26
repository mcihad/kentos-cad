//! Döndür: the web's `RotateTool` (`apps/web/src/tools/modifyTools.ts`) on
//! its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0037):
//!
//! - the objects, the centre, then the angle: typed in degrees
//!   (counter-clockwise), or shown with a point (the direction from the
//!   centre);
//! - Referans (R): the current direction shown with two points or typed as
//!   an angle, then the new direction shown or typed; the selection turns by
//!   the difference (a building onto a road);
//! - Kopya (K): the originals stay and turned copies are added.
//!
//! Written through `cad.entities.transform` (a rotation about the centre),
//! one undo step, “Döndür”; says “2 nesne 90.0000° döndürüldü (kopya).” and
//! leaves. The ghosts and the angle beside the cursor are the web's.

use kentos_contracts::Transform;
use kentos_geometry_core::geom::affine::{Affine, rotation};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::PI;
use kentos_geometry_core::tools::drawing::direction_angle;
use kentos_geometry_core::tools::editing::rotation_angle;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::{Format, degrees, fixed};
use crate::log::Level;
use crate::modify::{Modify, Stages, transform_selection};
use crate::points::{SAME, plain_number, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow};

/// The rotate tool's id: its command is `tool.rotate`.
pub const ID: &str = "rotate";
pub const LABEL: &str = "Döndür";

/// The rotate tool's stages.
#[derive(Clone, Debug, Default)]
pub struct Rotate {
    base: Option<Vec2>,
    copy: bool,
    /// Referans: its first point, then its angle (radians) once known.
    ref_from: Option<Vec2>,
    ref_angle: Option<f64>,
    ref_mode: bool,
}

impl Rotate {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The turn with the cursor at `hover`: none on the centre, or while the
    /// reference is still being shown.
    fn turn(&self, hover: Vec2) -> Option<f64> {
        let base = self.base?;
        if dist(base, hover) < SAME || (self.ref_mode && self.ref_angle.is_none()) {
            return None;
        }
        Some(rotation_angle(base, hover, self.ref_angle.unwrap_or(0.0)))
    }

    fn rotate(&mut self, angle: f64, cx: &mut Context<'_>) -> Flow {
        let Some(base) = self.base else {
            return Flow::Stay;
        };
        let t = Transform::Rotate {
            center: wire(base),
            angle,
        };
        if let Some(n) = transform_selection(t, self.copy, cx) {
            let copy = if self.copy { " (kopya)" } else { "" };
            let line = format!("{n} nesne {}° döndürüldü{copy}.", fixed(degrees(angle), 4));
            cx.say(Level::Success, line);
        }
        Flow::Exit
    }
}

impl Stages for Rotate {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self) {
        self.base = None;
        self.ref_mode = false;
        self.ref_from = None;
        self.ref_angle = None;
    }

    fn anchor(&self) -> Option<Vec2> {
        if self.ref_mode && self.ref_angle.is_none() {
            self.ref_from
        } else {
            self.base
        }
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let copy = if self.copy { "açık" } else { "kapalı" };
        if self.base.is_none() {
            return Prompt::new(LABEL, "dönme merkezini belirtin");
        }
        if self.ref_mode && self.ref_angle.is_none() {
            let step = if self.ref_from.is_some() {
                "referans doğrultunun ikinci noktasını gösterin"
            } else {
                "referans doğrultunun ilk noktasını gösterin ya da referans açıyı yazın"
            };
            return Prompt::new(LABEL, step);
        }
        if self.ref_mode {
            return Prompt::new(LABEL, "yeni doğrultuyu gösterin ya da yeni açıyı yazın")
                .option_with("Kopya", "K", copy);
        }
        Prompt::new(
            LABEL,
            "açıyı yazın (derece, saat yönü tersine) ya da bir nokta gösterin",
        )
        .option("Referans", "R")
        .option_with("Kopya", "K", copy)
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        let Some(base) = self.base else {
            self.base = Some(p);
            return Flow::Stay;
        };
        if self.ref_mode && self.ref_angle.is_none() {
            match self.ref_from {
                None => self.ref_from = Some(p),
                Some(from) if dist(from, p) > SAME => {
                    self.ref_angle = Some(direction_angle(from, p));
                }
                Some(_) => {}
            }
            return Flow::Stay;
        }
        if dist(base, p) < SAME {
            return Flow::Stay;
        }
        let angle = rotation_angle(base, p, self.ref_angle.unwrap_or(0.0));
        self.rotate(angle, cx)
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        self.base?;
        let t = upper_tr(js_trim(text));
        if t == "K" {
            self.copy = !self.copy;
            return Some(Flow::Stay);
        }
        if t == "R" && !self.ref_mode {
            self.ref_mode = true;
            return Some(Flow::Stay);
        }
        let n = plain_number(text)?;
        let rad = (n * PI) / 180.0;
        if self.ref_mode && self.ref_angle.is_none() {
            self.ref_angle = Some(rad);
            self.ref_from = None;
            return Some(Flow::Stay);
        }
        Some(self.rotate(rad - self.ref_angle.unwrap_or(0.0), cx))
    }

    fn preview(&self, hover: Vec2) -> Option<Affine> {
        let base = self.base?;
        self.turn(hover).map(|a| rotation(a, base))
    }

    fn tag(&self, hover: Vec2, _format: &Format) -> Vec<String> {
        self.turn(hover)
            .map(|a| vec![format!("Açı {}°", fixed(degrees(a), 2))])
            .unwrap_or_default()
    }
}
