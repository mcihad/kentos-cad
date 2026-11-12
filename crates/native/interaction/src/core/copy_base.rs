//! Taban noktasıyla kopyala (AutoCAD's COPYBASE, docs/adr/0309) on the [`crate::modify`] base: the objects (the
//! selection, or picked first), then the base point, clicked or typed. The copy (`cad.entities.copy` with that base)
//! goes to the host's clipboard ([`ViewChange::Copied`]); the drawing does not change and the tool leaves.

use kentos_domain::Slot;
use kentos_geometry_core::geom::affine::Affine;

use crate::Vec2;
use crate::clipboard::copy_of;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, ViewChange};

/// The tool's id: its command is `tool.copyBase`.
pub const ID: &str = "copyBase";
pub const LABEL: &str = "Taban noktasıyla kopyala";

/// The tool's one stage: the base point.
#[derive(Clone, Copy, Debug, Default)]
pub struct CopyBase;

impl CopyBase {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self)
    }
}

impl Stages for CopyBase {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        Prompt::new(LABEL, "taban noktasına tıklayın ya da Y,X yazın")
    }

    /// The base point: the copy goes to the clipboard, the tool leaves.
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        let slots: Vec<Slot> = cx.selection.ids().to_vec();
        if let Some(clip) = copy_of(&slots, Some(p), cx) {
            let n = clip.objects.len();
            cx.view_changes.push(ViewChange::Copied(Box::new(clip)));
            let at = cx.format().point(p);
            cx.say(
                Level::Success,
                format!("{n} nesne panoya kopyalandı; taban noktası {at}."),
            );
        }
        Flow::Exit
    }

    fn typed(&mut self, _text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }
}
