//! Hizala: the web's `AlignTool` (`apps/web/src/tools/arrangeTools.ts`) on
//! its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0047), AutoCAD's ALIGN:
//!
//! - the objects, then the first source point and the first target point: a
//!   confirm here only moves the objects, the one onto the other;
//! - then the second source point and the second target: the objects turn
//!   so that the source direction lies along the target direction, and with
//!   Ölçekle (Ö) they are scaled so that the one length becomes the other.
//!
//! Source or target points that lose their direction (within a nanometre
//! of the first pair's) are said, and the second pair is asked for again.
//! Written through the product command `cad.entities.transform` (an
//! alignment), one undo step, “Hizala”; says “2 nesne hizalandı ve
//! ölçeklendi.” and leaves. The ghosts are the web's: the alignment with the
//! cursor as the point being given.

use kentos_contracts::Transform;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::editing::align_transform;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::log::Level;
use crate::modify::{Modify, Stages, transform_selection};
use crate::points::{SAME, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow};

/// The align tool's id: its command is `tool.align`.
pub const ID: &str = "align";
pub const LABEL: &str = "Hizala";

/// The align tool's stages.
#[derive(Clone, Debug, Default)]
pub struct Align {
    /// Source, target, source, target, as far as given.
    pts: Vec<Vec2>,
    /// Ölçekle, as the session remembers it (the web's `AlignTool.scale`), seen at every event.
    scale: bool,
}

impl Align {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The points as the transform command's alignment: the first pair, and
    /// the second when it is given; none before the first pair.
    fn alignment(&self) -> Option<Transform> {
        let (source, target) = match self.pts.as_slice() {
            [s, t, ..] => (wire(*s), wire(*t)),
            _ => return None,
        };
        Some(match self.pts.as_slice() {
            [_, _, s2, t2] => Transform::Align {
                source,
                target,
                source2: Some(wire(*s2)),
                target2: Some(wire(*t2)),
                scale: self.scale.then_some(true),
            },
            _ => Transform::Align {
                source,
                target,
                source2: None,
                target2: None,
                scale: None,
            },
        })
    }

    fn finish(&mut self, cx: &mut Context<'_>) -> Flow {
        let alignment = self.alignment();
        let Some(alignment) =
            alignment.filter(|_| align_transform(&self.pts, self.scale).is_some())
        else {
            cx.say(
                Level::Warn,
                "Kaynak ya da hedef noktaları çakışıyor; hizalama yapılamaz.",
            );
            self.pts.truncate(2);
            return Flow::Stay;
        };
        if let Some(n) = transform_selection(alignment, false, cx) {
            let scaled = if self.pts.len() == 4 && self.scale {
                " ve ölçeklendi"
            } else {
                ""
            };
            cx.say(Level::Success, format!("{n} nesne hizalandı{scaled}."));
        }
        Flow::Exit
    }
}

impl Stages for Align {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.pts.clear();
        self.scale = cx.memory.align_scale;
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.scale = cx.memory.align_scale;
    }

    /// A source point given: the target is measured from it.
    fn anchor(&self) -> Option<Vec2> {
        if self.pts.len().is_multiple_of(2) {
            None
        } else {
            self.pts.last().copied()
        }
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let scale = if self.scale { "evet" } else { "hayır" };
        match self.pts.len() {
            0 => Prompt::new(LABEL, "birinci kaynak noktasını gösterin"),
            1 => Prompt::new(LABEL, "birinci hedef noktasını gösterin"),
            2 => Prompt::new(
                LABEL,
                "ikinci kaynak noktasını gösterin ya da yalnızca taşımak için sağ tıklayın",
            )
            .option_with("Ölçekle", "Ö", scale),
            _ => Prompt::new(LABEL, "ikinci hedef noktasını gösterin").option_with(
                "Ölçekle",
                "Ö",
                scale,
            ),
        }
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        // A source point on the target just given is let go (the web's `point`).
        if let Some(last) = self.pts.last()
            && dist(*last, p) < SAME
            && self.pts.len().is_multiple_of(2)
        {
            return Flow::Stay;
        }
        self.pts.push(p);
        if self.pts.len() == 4 {
            return self.finish(cx);
        }
        Flow::Stay
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        let t = upper_tr(js_trim(text));
        if t != "Ö" && t != "O" {
            return None;
        }
        cx.memory.align_scale = !cx.memory.align_scale;
        self.scale = cx.memory.align_scale;
        Some(Flow::Stay)
    }

    /// With the first pair given, a confirm only moves; otherwise the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.pts.len() == 2 {
            return self.finish(cx);
        }
        Flow::Exit
    }

    fn preview(&self, hover: Vec2) -> Option<Affine> {
        let mut pts = self.pts.clone();
        if !pts.len().is_multiple_of(2) {
            pts.push(hover);
        }
        align_transform(&pts, self.scale)
    }
}
