//! Döndür: the web's `RotateTool` (`apps/web/src/tools/modifyTools.ts`) on
//! its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0037):
//!
//! - the objects, the centre, then the angle: typed in the project's way
//!   (docs/adr/0291: a CAD project's degrees counter-clockwise, a CBS
//!   project's grads clockwise, its angle unit either way), or shown with a
//!   point (the direction from the centre);
//! - Referans (R): the current direction shown with two points or typed as
//!   a direction (a CBS project's a semt), then the new direction shown from
//!   the centre, typed, or with Noktalar (N) shown with two points; the
//!   selection turns by the difference (a building onto a road);
//! - Kopya (K): the originals stay and turned copies are added.
//!
//! Written through `cad.entities.transform` (a rotation about the centre),
//! one undo step, “Döndür”; says “2 nesne 90.0000° döndürüldü (kopya).” in
//! the project's way and leaves. The ghosts are the web's.

use kentos_contracts::Transform;
use kentos_geometry_core::geom::affine::{Affine, rotation};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::drawing::direction_angle;
use kentos_geometry_core::tools::editing::rotation_angle;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::Format;
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
    /// Noktalar (N): the new direction shown with two points; its first once given.
    new_points: bool,
    new_from: Option<Vec2>,
    /// The project's units and ways, as of the last event.
    format: Option<Format>,
}

impl Rotate {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The turn with the cursor at `hover`: none on the centre, or while the
    /// reference is still being shown.
    fn turn(&self, hover: Vec2) -> Option<f64> {
        let base = self.base?;
        if self.new_points {
            let from = self.new_from?;
            return (dist(from, hover) > SAME)
                .then(|| direction_angle(from, hover) - self.ref_angle.unwrap_or(0.0));
        }
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
            let line = format!("{n} nesne {} döndürüldü{copy}.", cx.format().turn(angle));
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

    fn begin(&mut self, _cx: &mut Context<'_>) -> Flow {
        self.base = None;
        self.ref_mode = false;
        self.ref_from = None;
        self.ref_angle = None;
        self.new_points = false;
        self.new_from = None;
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.format = Some(cx.format());
    }

    fn anchor(&self) -> Option<Vec2> {
        if self.ref_mode && self.ref_angle.is_none() {
            self.ref_from
        } else if self.new_points {
            self.new_from.or(self.base)
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
        if self.new_points {
            let step = if self.new_from.is_some() {
                "yeni doğrultunun ikinci noktasını gösterin"
            } else {
                "yeni doğrultunun ilk noktasını gösterin"
            };
            return Prompt::new(LABEL, step);
        }
        if self.ref_mode {
            return Prompt::new(LABEL, "yeni doğrultuyu gösterin ya da yeni açıyı yazın")
                .option("Noktalar", "N")
                .option_with("Kopya", "K", copy);
        }
        let words = self.format.map_or_else(
            || "derece, saat yönü tersine".to_owned(),
            |f| f.turn_words(),
        );
        Prompt::new(
            LABEL,
            format!("açıyı yazın ({words}) ya da bir nokta gösterin"),
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
        if self.new_points {
            return match self.new_from {
                None => {
                    self.new_from = Some(p);
                    Flow::Stay
                }
                Some(from) if dist(from, p) > SAME => {
                    let turn = direction_angle(from, p) - self.ref_angle.unwrap_or(0.0);
                    self.rotate(turn, cx)
                }
                Some(_) => Flow::Stay,
            };
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
        if t == "N" && self.ref_mode && self.ref_angle.is_some() && !self.new_points {
            self.new_points = true;
            return Some(Flow::Stay);
        }
        let n = plain_number(text)?;
        // The project's way (docs/adr/0291): a turn in its unit and direction; Referans's are directions.
        let f = cx.format();
        if self.ref_mode && self.ref_angle.is_none() {
            self.ref_angle = Some(f.direction_from_typed(n));
            self.ref_from = None;
            return Some(Flow::Stay);
        }
        let turn = match self.ref_angle {
            Some(reference) => f.direction_from_typed(n) - reference,
            None => f.turn_from_typed(n),
        };
        Some(self.rotate(turn, cx))
    }

    fn preview(&self, hover: Vec2) -> Option<Affine> {
        let base = self.base?;
        self.turn(hover).map(|a| rotation(a, base))
    }

    fn tag(&self, hover: Vec2, format: &Format) -> Vec<String> {
        self.turn(hover)
            .map(|a| vec![format!("Açı {}", format.turn(a))])
            .unwrap_or_default()
    }
}
