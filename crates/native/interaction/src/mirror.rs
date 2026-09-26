//! Aynala: the web's `MirrorTool` (`apps/web/src/tools/modifyTools.ts`) on
//! its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0037):
//!
//! - the objects, then the two points of the axis;
//! - Kaynağı sil (S): off, mirrored copies are added and the originals
//!   stay; on, the originals themselves turn over, in place.
//!
//! Written through `cad.entities.transform` (a reflection across the axis;
//! copies unless Kaynağı sil is on), one undo step, “Aynala”; says “2
//! nesnenin simetriği kopya olarak oluşturuldu.” and leaves. Mirrored text
//! stays readable and arcs stay counter-clockwise: the shared core's rules.

use kentos_contracts::Transform;
use kentos_geometry_core::geom::affine::{Affine, mirror};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::log::Level;
use crate::modify::{Modify, Stages, transform_selection};
use crate::points::{SAME, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow};

/// The mirror tool's id: its command is `tool.mirror`.
pub const ID: &str = "mirror";
pub const LABEL: &str = "Aynala";

/// The mirror tool's stages.
#[derive(Clone, Debug, Default)]
pub struct Mirror {
    p1: Option<Vec2>,
    /// Kaynağı sil: the originals turn over in place instead of being copied.
    erase_source: bool,
}

impl Mirror {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }
}

impl Stages for Mirror {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self) {
        self.p1 = None;
    }

    fn anchor(&self) -> Option<Vec2> {
        self.p1
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let step = if self.p1.is_some() {
            "simetri ekseninin ikinci noktasını belirtin"
        } else {
            "simetri ekseninin ilk noktasını belirtin"
        };
        let erase = if self.erase_source { "evet" } else { "hayır" };
        Prompt::new(LABEL, step).option_with("Kaynağı sil", "S", erase)
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        let Some(p1) = self.p1 else {
            self.p1 = Some(p);
            return Flow::Stay;
        };
        if dist(p1, p) < SAME {
            return Flow::Stay;
        }
        let t = Transform::Mirror {
            a: wire(p1),
            b: wire(p),
        };
        if let Some(n) = transform_selection(t, !self.erase_source, cx) {
            let how = if self.erase_source {
                "alındı; kaynaklar yerinde değiştirildi"
            } else {
                "kopya olarak oluşturuldu"
            };
            cx.say(Level::Success, format!("{n} nesnenin simetriği {how}."));
        }
        Flow::Exit
    }

    fn typed(&mut self, text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        if upper_tr(js_trim(text)) != "S" {
            return None;
        }
        self.erase_source = !self.erase_source;
        Some(Flow::Stay)
    }

    fn preview(&self, hover: Vec2) -> Option<Affine> {
        let p1 = self.p1?;
        (dist(p1, hover) > SAME).then(|| mirror(p1, hover))
    }
}
