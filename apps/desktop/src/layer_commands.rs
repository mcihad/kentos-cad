//! The Katmanlar panel and its windows write the layer tree through the
//! product commands Python and MCP use (docs/adr/0248 §6): `cad.layers.edit`
//! (one undo step), `cad.layers.set` (the tree's own changes) and
//! `cad.layers.states`. A refusal is said in the command's words, its
//! warnings too; the panel says what it did itself.

use kentos_contracts::{
    CommandResult, LayerSetOperation, LayerStatesDone, LayersChanged, LayersEdit, LayersEdited,
    LayersSet, LayersStates,
};
use kentos_native_application::{ExecutionContext, layers_edit, layers_set, layers_states};

use crate::app::App;

impl App {
    /// The answer's output, its warnings said; its refusal said and none.
    fn answered<T>(&mut self, result: CommandResult<T>) -> Option<T> {
        match result {
            CommandResult::Completed { output, warnings } => {
                for w in warnings {
                    self.warn(w.message);
                }
                Some(output)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                None
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => None,
        }
    }

    /// `cad.layers.edit` on the open drawing.
    pub(crate) fn edit_layers(&mut self, input: LayersEdit) -> Option<LayersEdited> {
        let doc = self.document.as_mut()?;
        let result = layers_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        self.answered(result)
    }

    /// `cad.layers.set` on the open drawing.
    pub(crate) fn set_layers(&mut self, input: LayersSet) -> Option<LayersChanged> {
        let doc = self.document.as_mut()?;
        let result = layers_set::execute(&mut ExecutionContext::new(&mut doc.model), input);
        self.answered(result)
    }

    /// `cad.layers.states` on the open drawing.
    pub(crate) fn layer_states_command(&mut self, input: LayersStates) -> Option<LayerStatesDone> {
        let doc = self.document.as_mut()?;
        let result = layers_states::execute(&mut ExecutionContext::new(&mut doc.model), input);
        self.answered(result)
    }

    /// The eye: a node shown or hidden (`cad.layers.set`'s `visible`).
    pub(crate) fn toggle_layer_visible(&mut self, id: &str) {
        let Some(shown) = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .map(|n| n.visible)
        else {
            return;
        };
        let mut input = LayersSet::new(LayerSetOperation::Visible, Some(id.to_owned()));
        input.value = Some(!shown);
        self.set_layers(input);
    }

    /// The lock: a node locked or unlocked (`cad.layers.set`'s `locked`).
    pub(crate) fn toggle_layer_locked(&mut self, id: &str) {
        let Some(locked) = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .map(|n| n.locked)
        else {
            return;
        };
        let mut input = LayersSet::new(LayerSetOperation::Locked, Some(id.to_owned()));
        input.value = Some(!locked);
        self.set_layers(input);
    }

    /// Hepsini göster (`layer.showAll`).
    pub(crate) fn show_all_layers(&mut self) {
        self.set_layers(LayersSet::new(LayerSetOperation::ShowAll, None));
    }
}
