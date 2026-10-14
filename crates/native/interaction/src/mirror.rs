//! Aynala: the web's `MirrorTool` (`apps/web/src/tools/modifyTools.ts`) on
//! its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0037):
//!
//! - the objects, then the two points of the axis;
//! - Kaynağı sil (S): off, mirrored copies are added and the originals
//!   stay; on, the originals themselves turn over, in place;
//! - Yazılar (Y): okunur (AutoCAD's MIRRTEXT = 0, the default) or aynalı
//!   (docs/adr/0292), kept for the session with the grip mode's.
//!
//! Written through `cad.entities.transform` (a reflection across the axis;
//! copies unless Kaynağı sil is on), one undo step, “Aynala”; says “2
//! nesnenin simetriği kopya olarak oluşturuldu.” and leaves. The texts are
//! the policy's, arcs stay counter-clockwise: the shared core's rules.

use kentos_contracts::Transform;
use kentos_geometry_core::geom::affine::{Affine, mirror};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::grip_edit::{mirror_text, texts_word};
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
    /// Yazılar (docs/adr/0292): the texts turn with the mirror; the session's.
    text_mirrored: bool,
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

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.p1 = None;
        self.text_mirrored = cx.memory.mirror_text_mirrored;
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.text_mirrored = cx.memory.mirror_text_mirrored;
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
        Prompt::new(LABEL, step)
            .option_with("Kaynağı sil", "S", erase)
            .option_with("Yazılar", "Y", texts_word(self.text_mirrored))
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
            text: Some(mirror_text(self.text_mirrored)),
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

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        match upper_tr(js_trim(text)).as_str() {
            "S" => self.erase_source = !self.erase_source,
            "Y" => {
                self.text_mirrored = !self.text_mirrored;
                cx.memory.mirror_text_mirrored = self.text_mirrored;
            }
            _ => return None,
        }
        Some(Flow::Stay)
    }

    fn preview(&self, hover: Vec2) -> Option<Affine> {
        let p1 = self.p1?;
        (dist(p1, hover) > SAME).then(|| mirror(p1, hover))
    }
}
