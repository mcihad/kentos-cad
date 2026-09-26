//! Taşı and Kopyala: the web's `MoveTool` (`apps/web/src/tools/modifyTools.ts`)
//! on its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0037):
//!
//! - the objects (the selection, or picked first), a base point, then the
//!   target: clicked, or typed (`@dY,dX` from the base, a distance along the
//!   cursor);
//! - Taşı moves the selection there and leaves; Kopyala leaves a copy at
//!   every point until a confirm (Bitir, Enter).
//!
//! Written through `cad.entities.transform` (a displacement), one undo step
//! per move or copy, “Taşı” or “Kopyala”; says “2 nesne taşındı: ΔY 12.500
//! ΔX -7.250”. The ghosts and the length beside the cursor are the web's.

use kentos_contracts::Transform;
use kentos_geometry_core::geom::affine::{Affine, translation};
use kentos_geometry_core::geometry::dist;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages, transform_selection};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow};

/// The move tool's id: its command is `tool.move`.
pub const MOVE_ID: &str = "move";
pub const MOVE_LABEL: &str = "Taşı";
/// The copy tool's id: its command is `tool.copy`.
pub const COPY_ID: &str = "copy";
pub const COPY_LABEL: &str = "Kopyala";

/// The move tool's stages; with `copy`, the copy tool's.
#[derive(Clone, Debug, Default)]
pub struct Move {
    copy: bool,
    base: Option<Vec2>,
}

impl Move {
    /// Taşı.
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// Kopyala.
    pub fn copy_tool() -> Modify<Self> {
        Modify::with(Self {
            copy: true,
            base: None,
        })
    }
}

impl Stages for Move {
    fn id(&self) -> &'static str {
        if self.copy { COPY_ID } else { MOVE_ID }
    }

    fn label(&self) -> &'static str {
        if self.copy { COPY_LABEL } else { MOVE_LABEL }
    }

    fn begin(&mut self) {
        self.base = None;
    }

    fn anchor(&self) -> Option<Vec2> {
        self.base
    }

    fn prompt(&self, n: usize) -> Prompt {
        let label = self.label();
        match self.base {
            None => Prompt::new(label, format!("{n} nesne için temel noktayı belirtin")),
            Some(_) if self.copy => {
                Prompt::new(label, "kopyanın yerini belirtin ya da @dY,dX yazın")
                    .option("Bitir", "Enter")
            }
            Some(_) => Prompt::new(label, "hedef noktayı belirtin ya da @dY,dX yazın"),
        }
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        let Some(base) = self.base else {
            self.base = Some(p);
            return Flow::Stay;
        };
        let (dx, dy) = (p.x - base.x, p.y - base.y);
        if let Some(n) = transform_selection(Transform::Move { dx, dy }, self.copy, cx) {
            let f = cx.format();
            let done = if self.copy {
                "kopyalandı"
            } else {
                "taşındı"
            };
            let line = format!(
                "{n} nesne {done}: ΔY {}  ΔX {}",
                f.length_bare(dx),
                f.length_bare(dy)
            );
            cx.say(Level::Success, line);
        }
        if self.copy { Flow::Stay } else { Flow::Exit }
    }

    fn typed(&mut self, _text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }

    fn preview(&self, hover: Vec2) -> Option<Affine> {
        self.base
            .map(|base| translation(hover.x - base.x, hover.y - base.y))
    }

    fn tag(&self, hover: Vec2, format: &Format) -> Vec<String> {
        self.base
            .map(|base| vec![format.length(dist(base, hover))])
            .unwrap_or_default()
    }
}
