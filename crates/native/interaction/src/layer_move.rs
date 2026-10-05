//! Katmanı eşle and Katmana kopyala (docs/adr/0177 §2; the web's
//! `tools/layerMoveTool.ts`), on the selection-first base
//! ([`crate::modify`]): the selected objects (picked first when nothing is
//! selected) go to, or are copied to, a target layer: the layer of an object
//! clicked, or the active one (Etkin katman, E).
//!
//! - One undo step named after the tool: Katmanı eşle through
//!   `cad.entities.set`'s layer, Katmana kopyala through
//!   `cad.entities.create` (the copies in place with every property, as
//!   Özgün koordinatlara yapıştır writes them), the copies selected.
//! - Objects on a locked layer are left out and counted; a locked target
//!   refuses and the tool waits for another; the commands' own warnings (a
//!   hidden target) are said.

use kentos_contracts::{EntitiesCreate, EntitiesSetProperties, NewObject, PropertiesOperation};
use kentos_domain::Slot;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, create, set};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer};

/// Katmanı eşle's id: its command is `tool.layerMatch`.
pub const MATCH_ID: &str = "layerMatch";
/// Katmana kopyala's id: its command is `tool.copyToLayer`.
pub const COPY_ID: &str = "copyToLayer";

/// Katmanı eşle or Katmana kopyala, after the selection.
#[derive(Clone, Debug)]
pub struct LayerMove {
    copy: bool,
    /// The hovered object's layer name, for the tag.
    target: Option<String>,
}

impl LayerMove {
    pub fn layer_match() -> Modify<Self> {
        Modify::with(Self {
            copy: false,
            target: None,
        })
    }

    pub fn copy_to_layer() -> Modify<Self> {
        Modify::with(Self {
            copy: true,
            target: None,
        })
    }

    fn name(&self) -> &'static str {
        if self.copy {
            "Katmana kopyala"
        } else {
            "Katmanı eşle"
        }
    }

    /// The layer of the object at `p`, or what is said when there is none.
    fn to_layer_at(&self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        let layer = cx
            .spatial
            .pick(p, cx.pick_tolerance())
            .and_then(|slot| cx.doc.get(slot))
            .map(|e| e.base().layer_id.clone());
        match layer {
            Some(layer) => self.to_layer(&layer, cx),
            None => {
                cx.say(
                    Level::Warn,
                    "Hedef katmanın bir nesnesine tıklayın ya da Etkin katman (E) seçin.",
                );
                Flow::Stay
            }
        }
    }

    /// Moves or copies the selection to `layer`, then the tool leaves; a
    /// locked target keeps it waiting.
    fn to_layer(&self, layer: &str, cx: &mut Context<'_>) -> Flow {
        let name = cx
            .doc
            .layers()
            .get(layer)
            .map_or_else(|| layer.to_owned(), |n| n.name.clone());
        if cx.doc.layers().is_locked(layer) {
            let verb = if self.copy {
                "kopyalanamaz"
            } else {
                "geçemez"
            };
            cx.say(
                Level::Warn,
                format!(
                    "“{name}” katmanı kilitli; nesneler oraya {verb}. Kilidini Katmanlar panelinden açın."
                ),
            );
            return Flow::Stay;
        }
        let doc = &*cx.doc;
        let all: Vec<Slot> = cx
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|s| doc.get(*s).is_some())
            .collect();
        let free: Vec<Slot> = all
            .iter()
            .copied()
            .filter(|s| doc.get(*s).is_some_and(|e| edge::unlocked(e, doc)))
            .collect();
        if free.len() < all.len() {
            cx.say(
                Level::Warn,
                format!(
                    "{} nesne kilitli katmanda olduğu için atlandı.",
                    all.len() - free.len()
                ),
            );
        }
        let objects: Vec<Slot> = if self.copy {
            free.clone()
        } else {
            free.iter()
                .copied()
                .filter(|s| cx.doc.get(*s).is_some_and(|e| e.base().layer_id != layer))
                .collect()
        };
        if objects.is_empty() {
            if !self.copy && !free.is_empty() {
                cx.say(
                    Level::Info,
                    format!("Seçili nesneler zaten “{name}” katmanında."),
                );
            }
        } else if self.copy {
            copy_to(&objects, layer, &name, cx);
        } else {
            move_to(&objects, layer, &name, cx);
        }
        Flow::Exit
    }
}

/// Katmanı eşle's write: one `cad.entities.set` in the step “Katmanı eşle”.
fn move_to(objects: &[Slot], layer: &str, name: &str, cx: &mut Context<'_>) {
    let input = EntitiesSetProperties {
        uids: objects
            .iter()
            .filter_map(|s| cx.doc.uid(*s))
            .map(|u| u.to_string())
            .collect(),
        layer_id: Some(layer.to_owned()),
        color: None,
        line_weight: None,
        symbol: None,
        attrs: None,
        label: None,
        operation: PropertiesOperation::Layer,
        expected_revision: None,
        unlink: false,
    };
    let group = cx.doc.begin_group("Katmanı eşle");
    let result = set::execute(&mut ExecutionContext::new(cx.doc), input);
    match points::written(result, cx) {
        Some(out) => {
            cx.doc.end_group(group);
            cx.say(
                Level::Success,
                format!("{} nesne “{name}” katmanına geçti.", out.changed.len()),
            );
        }
        None => cx.doc.cancel_group(group),
    }
}

/// Katmana kopyala's write: one `cad.entities.create` in the step “Katmana
/// kopyala”, every copy with its look, attributes, label and symbol; the
/// copies become the selection.
fn copy_to(objects: &[Slot], layer: &str, name: &str, cx: &mut Context<'_>) {
    let new: Vec<NewObject> = objects
        .iter()
        .filter_map(|s| cx.doc.get(*s))
        .filter_map(|e| {
            Some(NewObject {
                geometry: edit_geometry(shape(e))?,
                color: e.base().color.clone(),
                line_weight: e.base().line_weight,
                attrs: Some(e.base().attrs.clone()),
                label: e.base().label.clone(),
                label_of: None,
                label_scale: None,
                symbol: e.base().symbol.clone(),
            })
        })
        .collect();
    let input = EntitiesCreate {
        layer_id: layer.to_owned(),
        objects: new,
        operation: None,
        expected_revision: None,
    };
    let group = cx.doc.begin_group("Katmana kopyala");
    let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
    let Some(output) = points::written(result, cx) else {
        cx.doc.cancel_group(group);
        return;
    };
    cx.doc.end_group(group);
    let slots: Vec<Slot> = output.ids.into_iter().map(Slot).collect();
    let n = slots.len();
    cx.selection.set(slots);
    cx.say(
        Level::Success,
        format!("{n} nesnenin kopyası “{name}” katmanına yazıldı."),
    );
}

impl Stages for LayerMove {
    fn id(&self) -> &'static str {
        if self.copy { COPY_ID } else { MATCH_ID }
    }

    fn label(&self) -> &'static str {
        self.name()
    }

    fn begin(&mut self, _cx: &mut Context<'_>) -> Flow {
        self.target = None;
        Flow::Stay
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self, _n: usize) -> Prompt {
        Prompt::new(self.name(), "hedef katmanın bir nesnesine tıklayın")
            .option("Etkin katman", "E")
    }

    /// A typed point: the object there names the layer.
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        self.to_layer_at(p, cx)
    }

    /// Etkin katman (E): the active layer is the target.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        if upper_tr(js_trim(text)) != "E" {
            return None;
        }
        let active = cx.doc.layers().active().to_owned();
        Some(self.to_layer(&active, cx))
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    fn tag(&self, _hover: Vec2, _format: &Format) -> Vec<String> {
        match &self.target {
            Some(name) => vec![
                format!("Hedef: {name}"),
                if self.copy {
                    "Tıklayın: kopyala"
                } else {
                    "Tıklayın: taşı"
                }
                .to_owned(),
            ],
            None => Vec::new(),
        }
    }

    /// A move lights the object under the cursor; a press takes its layer.
    fn pointer(&mut self, p: &Pointer, down: bool, cx: &mut Context<'_>) -> Option<Flow> {
        if down {
            return Some(self.to_layer_at(p.raw, cx));
        }
        let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
        cx.selection.set_hover(hit);
        self.target = hit.and_then(|slot| cx.doc.get(slot)).map(|e| {
            let id = &e.base().layer_id;
            cx.doc
                .layers()
                .get(id)
                .map_or_else(|| id.clone(), |n| n.name.clone())
        });
        None
    }
}
