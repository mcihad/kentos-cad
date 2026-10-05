//! Katmanları birleştir and Kopyasını oluştur (docs/adr/0177 §3; the web's
//! `ui/layers/MergeLayersDialog.ts` and `app/layerActions.ts`).
//!
//! - Katmanları birleştir: a window with the drawing's layers (their paths
//!   and object counts, each with its box) and the target in a list (the one
//!   the Katmanlar panel's menu named, else the active layer). A locked layer
//!   and the target cannot be checked. Birleştir moves the sources' objects to
//!   the target through `cad.entities.set` and removes the sources, in one
//!   undo step “Katmanları birleştir”, and closes; a source that is the
//!   active layer gives that to the target first (a tree change, as the eye:
//!   outside the step). What keeps it from writing is said above the list.
//! - Kopyasını oluştur: a new layer last in the same group, named “<ad>
//!   kopyası” (one of its kind in the tree), with the same style and
//!   visibility, unlocked, and the copies of its objects through
//!   `cad.entities.create`; one undo step “Katmanı kopyala”. A locked layer is
//!   not copied (nor are its objects).
//!
//! Both change the layer tree: a cloud project without `project.edit`
//! refuses them ([`crate::layering::TREE_LOCKED`]).

use std::collections::BTreeSet;

use iced::widget::{Column, button, container, row, text};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{
    CommandResult, EntitiesCreate, EntitiesSetProperties, LayerNodeType, PropertiesOperation,
};
use kentos_domain::{NewLayer, Slot};
use kentos_interaction::Level;
use kentos_interaction::layer_move::new_object;
use kentos_native_application::{ExecutionContext, create, set};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog as Frame, Elided, VirtualList, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const MERGE_TITLE: &str = "Katmanları birleştir";
const TARGET: &str = "Hedef katman";
const MERGE: &str = "Birleştir";
const CANCEL: &str = "Vazgeç";

/// A row's height in the list of layers, logical pixels.
const ROW: f32 = 26.0;

/// The window: the target and the layers checked to go into it.
#[derive(Debug, Clone)]
pub struct Window {
    target: String,
    checked: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Target(String),
    /// A layer's box.
    Check(String, bool),
    Merge,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::LayerMerge(event)
}

/// What the window says above the list, each line with its kind, and
/// whether Birleştir may write (the web's `mergeSummary`).
pub(crate) struct Summary {
    pub ready: bool,
    pub lines: Vec<(Kind, String)>,
}

impl App {
    /// Katmanları birleştir's words for these sources and this target.
    pub(crate) fn merge_summary(&self, sources: &[String], target: &str) -> Summary {
        let refuse = |kind: Kind, text: String| Summary {
            ready: false,
            lines: vec![(kind, text)],
        };
        if let Some(locked) = self.tree_locked() {
            return refuse(Kind::Warn, locked.to_owned());
        }
        let Some(doc) = &self.document else {
            return refuse(Kind::Warn, "Açık çizim yok.".to_owned());
        };
        let layers = doc.model.layers();
        let Some(into) = layers
            .get(target)
            .filter(|n| n.kind == LayerNodeType::Layer)
        else {
            return refuse(
                Kind::Warn,
                "Hedef bir katman olmalı; listeden bir katman seçin.".to_owned(),
            );
        };
        if layers.is_locked(target) {
            return refuse(
                Kind::Warn,
                format!(
                    "“{}” katmanı kilitli; birleştirilemez. Kilidini Katmanlar panelinden açın.",
                    into.name
                ),
            );
        }
        let from: Vec<&String> = sources
            .iter()
            .filter(|id| {
                *id != target
                    && layers
                        .get(id)
                        .is_some_and(|n| n.kind == LayerNodeType::Layer)
            })
            .collect();
        if from.is_empty() {
            return refuse(
                Kind::Info,
                "Birleşecek katmanları işaretleyin: nesneleri hedef katmana geçer, katmanları silinir."
                    .to_owned(),
            );
        }
        let objects: usize = from.iter().map(|id| objects_on(&doc.model, id).len()).sum();
        let mut lines = vec![(
            Kind::Info,
            format!(
                "{} katman “{}” katmanına birleşir: {objects} nesne taşınır, katmanlar silinir. Tek adımda geri alınır.",
                from.len(),
                into.name
            ),
        )];
        let active = layers.active();
        if from.iter().any(|id| *id == active) {
            let name = layers.get(active).map_or(active, |n| n.name.as_str());
            lines.push((
                Kind::Info,
                format!(
                    "Etkin katman (“{name}”) birleşiyor: “{}” etkin katman olur.",
                    into.name
                ),
            ));
        }
        Summary { ready: true, lines }
    }

    /// Katmanları birleştir (`layer.merge`): the window, its target the
    /// given layer (Katmanlar's menu) or the active one.
    pub(crate) fn open_layer_merge(&mut self, target: Option<String>) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let layers = doc.model.layers();
        let target = target
            .filter(|id| {
                layers
                    .get(id)
                    .is_some_and(|n| n.kind == LayerNodeType::Layer)
            })
            .unwrap_or_else(|| layers.active().to_owned());
        self.layer_merge = Some(Window {
            target,
            checked: BTreeSet::new(),
        });
        self.dialog = Some(Dialog::LayerMerge);
    }

    pub(crate) fn layer_merge_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.layer_merge.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Target(id) => {
                w.checked.remove(&id);
                w.target = id;
            }
            Event::Check(id, on) => {
                if on {
                    w.checked.insert(id);
                } else {
                    w.checked.remove(&id);
                }
            }
            Event::Merge => {
                let (sources, target) = self.merge_sources();
                if self.merge_layers(&sources, &target) {
                    self.layer_merge = None;
                    self.dialog = None;
                }
            }
            Event::Cancel => {
                self.layer_merge = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// The window's checked layers in the tree's order, and its target.
    fn merge_sources(&self) -> (Vec<String>, String) {
        let (Some(w), Some(doc)) = (&self.layer_merge, &self.document) else {
            return (Vec::new(), String::new());
        };
        let sources = doc
            .model
            .layers()
            .leaves()
            .iter()
            .filter(|l| w.checked.contains(&l.id) && l.id != w.target)
            .map(|l| l.id.clone())
            .collect();
        (sources, w.target.clone())
    }

    /// Moves the sources' objects to the target and removes the sources, one
    /// undo step; says what it did or why not. Whether it wrote.
    pub(crate) fn merge_layers(&mut self, sources: &[String], target: &str) -> bool {
        let said = self.merge_summary(sources, target);
        if !said.ready {
            if let Some((_, text)) = said.lines.into_iter().next() {
                self.warn(text);
            }
            return false;
        }
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let model = &mut doc.model;
        let layers = model.layers();
        let into = layers
            .get(target)
            .map_or_else(|| target.to_owned(), |n| n.name.clone());
        let from: Vec<String> = sources
            .iter()
            .filter(|id| {
                *id != target
                    && layers
                        .get(id)
                        .is_some_and(|n| n.kind == LayerNodeType::Layer)
            })
            .cloned()
            .collect();
        if let Some(locked) = from.iter().find(|id| layers.is_locked(id)) {
            let name = layers
                .get(locked)
                .map_or(locked.as_str(), |n| n.name.as_str());
            let text = format!(
                "“{name}” katmanı kilitli; birleştirilemez. Kilidini Katmanlar panelinden açın."
            );
            self.warn(text);
            return false;
        }
        let uids: Vec<String> = from
            .iter()
            .flat_map(|id| objects_on(model, id))
            .filter_map(|slot| model.uid(slot))
            .map(|uid| uid.to_string())
            .collect();
        if from.iter().any(|id| id == model.layers().active()) {
            model.set_active_layer(target);
        }
        let group = model.begin_group("Katmanları birleştir");
        let mut warnings = Vec::new();
        let moved = uids.len();
        if !uids.is_empty() {
            let input = EntitiesSetProperties {
                uids,
                layer_id: Some(target.to_owned()),
                color: None,
                line_weight: None,
                symbol: None,
                attrs: None,
                label: None,
                operation: PropertiesOperation::Layer,
                expected_revision: None,
                unlink: false,
            };
            match set::execute(&mut ExecutionContext::new(model), input) {
                CommandResult::Completed { warnings: w, .. } => {
                    warnings.extend(w.into_iter().map(|w| w.message));
                }
                other => {
                    model.cancel_group(group);
                    self.warn(refusal(other, "Nesneler taşınamadı."));
                    return false;
                }
            }
        }
        for id in &from {
            if let Err(refused) = model.remove_layer(id) {
                model.cancel_group(group);
                self.warn(refused.to_string());
                return false;
            }
        }
        model.end_group(group);
        for text in warnings {
            self.warn(text);
        }
        self.say(
            Level::Success,
            format!(
                "{} katman “{into}” katmanına birleştirildi: {moved} nesne taşındı.",
                from.len()
            ),
        );
        true
    }

    /// Kopyasını oluştur (`layer.duplicate`): the layer `id`, or the active
    /// one, copied with its objects.
    pub(crate) fn duplicate_layer(&mut self, id: Option<String>) {
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return;
        };
        let model = &mut doc.model;
        let layers = model.layers();
        let id = id.unwrap_or_else(|| layers.active().to_owned());
        let Some(source) = layers
            .get(&id)
            .filter(|n| n.kind == LayerNodeType::Layer)
            .cloned()
        else {
            self.warn("Kopyası oluşturulacak bir katman seçin; grubun kopyası oluşturulmaz.");
            return;
        };
        if layers.is_locked(&id) {
            let text = format!(
                "“{}” katmanı kilitli; kopyası oluşturulmaz. Kilidini Katmanlar panelinden açın.",
                source.name
            );
            self.warn(text);
            return;
        }
        let name = model
            .layers()
            .unique_name(&format!("{} kopyası", source.name));
        let objects: Vec<kentos_contracts::NewObject> = objects_on(model, &id)
            .into_iter()
            .filter_map(|slot| model.get(slot).and_then(new_object))
            .collect();
        let group = model.begin_group("Katmanı kopyala");
        let mut new = NewLayer::layer(name.clone());
        new.visible = source.visible;
        new.style = source.style.clone();
        let layer = match model.add_layer(new, Some(&id), false) {
            Ok(layer) => layer,
            Err(refused) => {
                model.cancel_group(group);
                self.warn(refused.to_string());
                return;
            }
        };
        let mut made = 0;
        let mut warnings = Vec::new();
        if !objects.is_empty() {
            let input = EntitiesCreate {
                layer_id: layer,
                objects,
                operation: None,
                expected_revision: None,
            };
            match create::execute(&mut ExecutionContext::new(model), input) {
                CommandResult::Completed {
                    output,
                    warnings: w,
                } => {
                    made = output.ids.len();
                    warnings.extend(w.into_iter().map(|w| w.message));
                }
                other => {
                    model.cancel_group(group);
                    self.warn(refusal(other, "Nesneler kopyalanamadı."));
                    return;
                }
            }
        }
        model.end_group(group);
        for text in warnings {
            self.warn(text);
        }
        self.say(
            Level::Success,
            format!(
                "“{}” katmanı “{name}” olarak kopyalandı: {made} nesne.",
                source.name
            ),
        );
    }

    pub(crate) fn layer_merge_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.layer_merge, &self.document) else {
            return text("").into();
        };
        let layers = doc.model.layers();
        let leaves: Vec<String> = layers.leaves().iter().map(|l| l.id.clone()).collect();
        let counts: Vec<usize> = leaves
            .iter()
            .map(|id| objects_on(&doc.model, id).len())
            .collect();
        let choices: Vec<Choice> = leaves
            .iter()
            .map(|id| Choice::new(self.layer_path(id)))
            .collect();
        let selected = leaves.iter().position(|id| *id == w.target);
        let pick_leaves = leaves.clone();
        let pick = Select::new(choices, selected, move |i| {
            msg(Event::Target(
                pick_leaves.get(i).cloned().unwrap_or_default(),
            ))
        });
        let target = container(words::field(TARGET, pick, None)).width(Fill);
        let sources: Vec<String> = leaves
            .iter()
            .filter(|id| w.checked.contains(*id) && **id != w.target)
            .cloned()
            .collect();
        let said = self.merge_summary(&sources, &w.target);
        let lines = said
            .lines
            .iter()
            .map(|(kind, line)| words::text_line(*kind, line.clone()))
            .collect();

        let head = row![
            container(text("")).width(typography::scaled(28.0)),
            container(label::caption("Katman")).width(Fill),
            container(label::caption("Nesne")).width(typography::scaled(56.0)),
        ]
        .spacing(8)
        .padding([4, 8]);
        let rows: Vec<(String, String, bool, bool, usize)> = leaves
            .iter()
            .zip(&counts)
            .map(|(id, n)| {
                (
                    id.clone(),
                    self.layer_path(id),
                    layers.is_locked(id),
                    *id == w.target,
                    *n,
                )
            })
            .collect();
        let checked = w.checked.clone();
        let list = VirtualList::new(rows.len(), typography::scaled(ROW), move |i| {
            let (id, path, locked, is_target, n) = &rows[i];
            let blocked = *locked || *is_target;
            let on = !blocked && checked.contains(id);
            let boxed = check_box(
                if on { Check::Checked } else { Check::Unchecked },
                (!blocked).then(|| msg(Event::Check(id.clone(), !on))),
            );
            let quiet = blocked;
            let words = if *is_target {
                format!("{path} (hedef)")
            } else {
                path.clone()
            };
            let name = Elided::new(words)
                .size(typography::caption())
                .font(typography::ui())
                .style(move |theme: &iced::Theme| {
                    let t = Tokens::of(theme);
                    iced::widget::text::Style {
                        color: Some(if quiet { t.muted } else { t.text }),
                    }
                })
                .width(Fill);
            let name = if *locked {
                row![icon(Icon::Lock).size(12.0).tone(Tone::Muted), name]
                    .spacing(4)
                    .align_y(Center)
                    .width(Fill)
            } else {
                row![name].width(Fill)
            };
            let line = button(row![name].align_y(Center))
                .on_press_maybe((!blocked).then(|| msg(Event::Check(id.clone(), !on))))
                .padding(0)
                .style(style::button::ghost)
                .width(Fill);
            row![
                container(boxed)
                    .width(typography::scaled(28.0))
                    .center_x(typography::scaled(28.0)),
                line,
                container(label::caption(n.to_string())).width(typography::scaled(56.0)),
            ]
            .spacing(8)
            .padding([0, 8])
            .height(typography::scaled(ROW))
            .align_y(Center)
            .into()
        })
        .height(typography::scaled(240.0));
        let table = container(Column::new().push(head).push(list))
            .style(style::container::field_box)
            .width(Fill);
        let body = Column::new()
            .spacing(12)
            .push(target)
            .push(words::summary(lines))
            .push(table);
        overlay::modal(
            Frame::new(MERGE_TITLE)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(MERGE, said.ready.then(|| msg(Event::Merge))))
                .width(560.0),
            msg(Event::Cancel),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): a
    /// layer's box by its path, Birleştir and Vazgeç.
    pub(crate) fn layer_merge_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let (Some(w), Some(doc)) = (&self.layer_merge, &self.document) else {
            return Err(format!("{MERGE_TITLE} penceresi açık değil"));
        };
        let layers = doc.model.layers();
        Ok(match control {
            Control::Check(words, on) => {
                let Some(leaf) = layers
                    .leaves()
                    .into_iter()
                    .find(|l| self.layer_path(&l.id) == words)
                else {
                    return Err(format!("“{MERGE_TITLE}” penceresinde “{words}” kutusu yok"));
                };
                let blocked = layers.is_locked(&leaf.id) || leaf.id == w.target;
                let now = w.checked.contains(&leaf.id);
                (!blocked && now != on).then(|| msg(Event::Check(leaf.id.clone(), on)))
            }
            Control::Press(MERGE) => {
                let (sources, target) = self.merge_sources();
                self.merge_summary(&sources, &target)
                    .ready
                    .then(|| msg(Event::Merge))
            }
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{MERGE_TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The objects on a layer, in the drawing's order.
fn objects_on(doc: &kentos_domain::Document, layer: &str) -> Vec<Slot> {
    doc.entities()
        .filter(|e| e.base().layer_id == layer)
        .map(|e| Slot(e.base().id))
        .collect()
}

/// A command's refusal in its words, or `fallback`.
fn refusal<T>(result: CommandResult<T>, fallback: &str) -> String {
    match result {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => error.message,
        _ => fallback.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use crate::app::Message;
    use crate::files_testing::{app_with_drawing, last_said};

    /// The sample drawing: Kadastro / Parsel (active, one object), Kadastro /
    /// Bina (locked, three), Çizim (hidden, nine).
    fn layer_of(app: &crate::app::App, slot: u32) -> String {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.get(kentos_domain::Slot(slot))
            .map(|e| e.base().layer_id.clone())
            .unwrap_or_default()
    }

    #[test]
    fn merging_the_active_layer_makes_the_target_active_and_undoes_in_one_step() {
        let mut app = app_with_drawing();
        let parcel = {
            let doc = &app.document.as_ref().expect("a drawing").model;
            doc.entities()
                .find(|e| e.base().layer_id == "parsel")
                .map(|e| e.base().id)
                .expect("the parcel")
        };
        assert!(app.merge_layers(&["parsel".to_owned()], "cizim"));
        let layers = app.document.as_ref().expect("a drawing").model.layers();
        assert_eq!(layers.active(), "cizim");
        assert!(layers.get("parsel").is_none(), "the source goes");
        assert_eq!(layer_of(&app, parcel), "cizim");
        assert_eq!(
            last_said(&app),
            "1 katman “Çizim” katmanına birleştirildi: 1 nesne taşındı."
        );
        let _ = app.update(Message::Run("edit.undo"));
        let layers = app.document.as_ref().expect("a drawing").model.layers();
        assert!(layers.get("parsel").is_some(), "back with its object");
        assert_eq!(layer_of(&app, parcel), "parsel");
        // The active layer is the tree's own change: undo leaves it.
        assert_eq!(layers.active(), "cizim");
    }

    #[test]
    fn a_locked_layer_neither_merges_nor_is_copied() {
        let mut app = app_with_drawing();
        assert!(!app.merge_layers(&["bina".to_owned()], "parsel"));
        assert_eq!(
            last_said(&app),
            "“Bina” katmanı kilitli; birleştirilemez. Kilidini Katmanlar panelinden açın."
        );
        app.duplicate_layer(Some("bina".to_owned()));
        assert_eq!(
            last_said(&app),
            "“Bina” katmanı kilitli; kopyası oluşturulmaz. Kilidini Katmanlar panelinden açın."
        );
    }

    #[test]
    fn copies_take_names_of_their_own() {
        let mut app = app_with_drawing();
        app.duplicate_layer(Some("parsel".to_owned()));
        app.duplicate_layer(Some("parsel".to_owned()));
        assert_eq!(
            last_said(&app),
            "“Parsel” katmanı “Parsel kopyası 2” olarak kopyalandı: 1 nesne."
        );
        let layers = app.document.as_ref().expect("a drawing").model.layers();
        let names: Vec<&str> = layers
            .leaves()
            .into_iter()
            .map(|l| l.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "Parsel",
                "Bina",
                "Parsel kopyası",
                "Parsel kopyası 2",
                "Çizim"
            ]
        );
    }
}
