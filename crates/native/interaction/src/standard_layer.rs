//! The standard layers some tools write to, whatever layer is active:
//! Parsel oluştur's `parsel`, Kot noktası's `kot` (docs/adr/0067). A drawing
//! without one (an imported plan, an older file) gets it as a new project
//! has it (`kentos_project::new_project::standard_layer`), in the undo step of
//! the object written to it; nothing stays when that object is refused.

use kentos_domain::{Group, NewLayer, labels};

use crate::log::Level;
use crate::tool::Context;

/// A standard layer opened for the object about to be written: its group
/// holds the layer and, next, the object.
pub struct Opened {
    group: Group,
    name: String,
    what: &'static str,
}

/// Opens standard layer `id` when the drawing lacks it, for `what` (“parsel”,
/// “kot noktası”); none when it is there. Err when the document refuses the
/// layer, the refusal said.
pub fn open_if_missing(
    id: &str,
    what: &'static str,
    cx: &mut Context<'_>,
) -> Result<Option<Opened>, ()> {
    open_in_step(id, what, labels::ADD, cx)
}

/// `open_if_missing`, the layer and the object one undo step named `step`
/// (the command's own name for it; docs/adr/0175 §3).
pub fn open_in_step(
    id: &str,
    what: &'static str,
    step: &str,
    cx: &mut Context<'_>,
) -> Result<Option<Opened>, ()> {
    if cx.doc.layers().get(id).is_some() {
        return Ok(None);
    }
    let settings = cx.doc.settings();
    let cad = settings.project_type() == Some(kentos_contracts::Workspace::Cad);
    let (name, style) =
        match kentos_project::new_project::standard_layer(id, settings.plot_scale, cad) {
            Some(node) => (node.name, node.style),
            None => (id.to_owned(), NewLayer::layer(id).style),
        };
    let group = cx.doc.begin_group(step);
    let layer = NewLayer {
        id: Some(id.to_owned()),
        style,
        ..NewLayer::layer(name.clone())
    };
    match cx.doc.add_layer(layer, None, false) {
        Ok(_) => Ok(Some(Opened { group, name, what })),
        Err(refusal) => {
            cx.doc.cancel_group(group);
            cx.say(Level::Warn, refusal.to_string());
            Err(())
        }
    }
}

/// The name layer `id` has, or the one it is opened with when the drawing lacks it.
pub fn name_of(id: &str, cx: &Context<'_>) -> String {
    if let Some(layer) = cx.doc.layers().get(id) {
        return layer.name.clone();
    }
    let settings = cx.doc.settings();
    let cad = settings.project_type() == Some(kentos_contracts::Workspace::Cad);
    kentos_project::new_project::standard_layer(id, settings.plot_scale, cad)
        .map_or_else(|| id.to_owned(), |node| node.name)
}

impl Opened {
    /// The object was written: the layer and the object are one undo step, and it is said.
    pub fn keep(self, cx: &mut Context<'_>) {
        cx.doc.end_group(self.group);
        cx.say(
            Level::Info,
            format!(
                "“{}” katmanı çizimde yoktu; {} için açıldı.",
                self.name, self.what
            ),
        );
    }

    /// The object was refused: the layer goes too.
    pub fn drop(self, cx: &mut Context<'_>) {
        cx.doc.cancel_group(self.group);
    }
}
