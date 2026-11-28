//! The Katmanlar panel and its windows write the layer tree through the
//! product commands Python and MCP use (docs/adr/0248 §6): `cad.layers.edit`
//! (one undo step), `cad.layers.set` (the tree's own changes) and
//! `cad.layers.states`. A refusal is said in the command's words, its
//! warnings too; the panel says what it did itself.

use kentos_commands::{
    CallOp, ChangesetApply, ChangesetStep, CommandError, CommandResult, LayerSetOperation,
    LayerStatesDone, LayersChanged, LayersEdit, LayersEdited, LayersSet, LayersStates,
};
use kentos_native_application::{
    ExecutionContext, changeset, layers_edit, layers_set, layers_states, registry,
};

use crate::app::App;

/// A command's answer without its output: what [`App::answered`] says of it is all a Kaydet needs.
fn done<T>(result: CommandResult<T>) -> CommandResult<()> {
    match result {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed {
            output: (),
            warnings,
        },
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::Cancelled => CommandResult::Cancelled,
    }
}

impl App {
    /// The answer's output, its warnings said; its refusal said and none.
    pub fn answered<T>(&mut self, result: CommandResult<T>) -> Option<T> {
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

    /// A window's Kaydet that holds several edits of one command (docs/adr/9401 §4). One edit goes in by its own
    /// command. Several go in as one change (`cad.changeset.apply`) or not at all: a refused edit takes the others
    /// back, the project's settings too (a loop of the command's own calls left the earlier ones written). A refusal
    /// is said in the edit's own words. Whether the edits went in; none to write is none refused.
    pub(crate) fn save_together<I: serde::Serialize>(
        &mut self,
        label: &str,
        command: &str,
        inputs: &[I],
    ) -> bool {
        let mut steps = Vec::with_capacity(inputs.len());
        for input in inputs {
            match serde_json::to_value(input) {
                Ok(input) => steps.push(ChangesetStep {
                    command: command.to_owned(),
                    version: None,
                    input,
                    name: None,
                }),
                Err(e) => {
                    self.warn(format!(
                        "Değişiklik hazırlanamadı ({e}); hiçbir şey yazılmadı."
                    ));
                    return false;
                }
            }
        }
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let cx = &mut ExecutionContext::new(&mut doc.model);
        let result: CommandResult<()> = match steps.len() {
            0 => return true,
            1 => {
                let input = steps.remove(0).input;
                let answer = registry::run(cx, command, CallOp::Execute, input)
                    .map_err(|unrun| unrun.message)
                    .and_then(|answer| {
                        serde_json::from_value::<CommandResult<serde_json::Value>>(answer)
                            .map_err(|e| format!("Komutun cevabı okunamadı: {e}"))
                    });
                match answer {
                    Ok(result) => done(result),
                    Err(why) => {
                        self.warn(why);
                        return false;
                    }
                }
            }
            _ => {
                // The step's own words: the user sent edits, not a set's numbered steps.
                let own = |error: CommandError| match changeset::step_refusal(&error) {
                    Some((_, own)) => own,
                    None => error,
                };
                let set = ChangesetApply {
                    model_scope: None,
                    steps,
                    label: Some(label.to_owned()),
                    expected_revision: None,
                };
                match changeset::execute(cx, set) {
                    CommandResult::Failed { error } => CommandResult::Failed { error: own(error) },
                    CommandResult::Conflict { error } => {
                        CommandResult::Conflict { error: own(error) }
                    }
                    CommandResult::NeedsInput { error } => {
                        CommandResult::NeedsInput { error: own(error) }
                    }
                    other => done(other),
                }
            }
        };
        self.answered(result).is_some()
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

    /// Yazdır: a node printed or not (`cad.layers.set`'s `plot`, docs/adr/0260 §4).
    pub(crate) fn toggle_layer_plot(&mut self, id: &str) {
        let Some(printed) = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .map(|n| n.plot != Some(false))
        else {
            return;
        };
        let mut input = LayersSet::new(LayerSetOperation::Plot, Some(id.to_owned()));
        input.value = Some(!printed);
        self.set_layers(input);
    }

    /// Mobilde ▸ (docs/adr/9198): the node's place on the mobile clients; a choice its kind has not is the command's to
    /// refuse, in its words.
    pub fn set_layer_mobile(&mut self, id: &str, mobile: kentos_commands::LayerMobile) {
        let mut input = LayersSet::new(LayerSetOperation::Mobile, Some(id.to_owned()));
        input.mobile = Some(mobile);
        self.set_layers(input);
    }

    /// The mobile behaviours `node` may take here (docs/adr/9198 §2.2): to edit only a CBS project's data layer.
    pub(crate) fn mobile_choices(
        &self,
        node: &kentos_commands::LayerNode,
    ) -> &'static [kentos_launch::Mobile] {
        match self.document.as_ref() {
            Some(d) => kentos_native_application::layer_kind::mobile_choices(&d.model, node),
            None => &[],
        }
    }

    /// Dondur / Çöz (docs/adr/0311 §1): the node's freeze turned over; the
    /// active layer's refusal said in the command's words.
    pub(crate) fn toggle_layer_frozen(&mut self, id: &str) {
        let Some(frozen) = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .map(|n| n.frozen == Some(true))
        else {
            return;
        };
        let mut input = LayersSet::new(LayerSetOperation::Frozen, Some(id.to_owned()));
        input.value = Some(!frozen);
        self.set_layers(input);
    }

    /// Bütün katmanları çöz (`layer.thawAll`, docs/adr/0311 §1).
    pub(crate) fn thaw_all_layers(&mut self) {
        self.set_layers(LayersSet::new(LayerSetOperation::ThawAll, None));
    }

    /// Hepsini göster (`layer.showAll`).
    pub(crate) fn show_all_layers(&mut self) {
        self.set_layers(LayersSet::new(LayerSetOperation::ShowAll, None));
    }
}
