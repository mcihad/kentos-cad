//! Katman durumları (docs/adr/0177 §4; the web's `app/layerStates.ts` and
//! `ui/layers/LayerStatesDialog.ts`): named records of the layers'
//! visibility, and when asked their locks and styles, kept in the project's
//! settings (`layer_states`, KCAD schema 19), by the shared rules
//! (`kentos_domain::layer_states`).
//!
//! Saving, updating, renaming and removing one change the project's settings:
//! an edit, not an undo step; refused where the project's settings may not
//! change (a cloud project without `project.edit`). Applying one changes the
//! visibility and locks as the eye and the lock do (the tree's own changes,
//! the user's own) and the styles in one undo step “Katman durumu: <ad>”.
//!
//! The window lists the states, each with what it keeps and a mark when the
//! layers are in it now. A state is chosen by a click (its name and parts
//! come into the fields below), applied with Uygula, saved again from the
//! layers as they are with Güncelle, removed with Sil. Yeni durum kaydet saves
//! the layers as a new state under the name in Ad; Yeniden adlandır gives the
//! chosen state that name. The window stays open; Kapat closes it.

use iced::widget::{Column, button, container, row, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::LayerState;
use kentos_domain::layer_states::{self, LayerStateParts};
use kentos_interaction::Level;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const STATES_TITLE: &str = "Katman durumları";
/// Why the layer states may not change here, as the web says it.
pub const STATES_LOCKED: &str = "Bu projede katman durumlarını değiştirme yetkiniz yok (project.edit); proje sahibinden ya da yöneticisinden isteyin.";
const NAME: &str = "Ad";
const LOCKS: &str = "Kilitler";
const STYLES: &str = "Stiller";
const APPLY: &str = "Uygula";
const UPDATE: &str = "Güncelle";
const REMOVE: &str = "Sil";
const SAVE: &str = "Yeni durum kaydet";
const RENAME: &str = "Yeniden adlandır";
const CLOSE: &str = "Kapat";

/// What a state keeps, in words: “görünürlük”, “görünürlük ve stil”,
/// “görünürlük, kilit ve stil”.
pub fn parts_text(parts: LayerStateParts) -> String {
    let mut words = vec!["görünürlük"];
    if parts.locks {
        words.push("kilit");
    }
    if parts.styles {
        words.push("stil");
    }
    match words.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} ve {last}", rest.join(", ")),
        _ => words.join(""),
    }
}

/// The first “Durum n” no state is named.
pub fn next_state_name(states: &[LayerState]) -> String {
    (1..)
        .map(|n| format!("Durum {n}"))
        .find(|name| !states.iter().any(|s| s.name == *name))
        .unwrap_or_default()
}

/// The first “durum-n” no state has as its id.
fn next_state_id(states: &[LayerState]) -> String {
    (1..)
        .map(|n| format!("durum-{n}"))
        .find(|id| !states.iter().any(|s| s.id == *id))
        .unwrap_or_default()
}

/// Why `name` may not name a state (other than `this`), or none.
fn name_refused(states: &[LayerState], name: &str, this: Option<&str>) -> Option<String> {
    if name.is_empty() {
        return Some("Durumun adını yazın.".to_owned());
    }
    states
        .iter()
        .any(|s| s.name.trim() == name && Some(s.id.as_str()) != this)
        .then(|| {
            format!(
                "“{name}” adında bir katman durumu var; başka bir ad yazın ya da o durumu seçip Güncelle'ye basın."
            )
        })
}

/// The window: the chosen state and the fields below the list.
#[derive(Debug, Clone)]
pub struct Window {
    chosen: Option<String>,
    name: String,
    locks: bool,
    styles: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A state's row: it is chosen.
    Choose(String),
    /// Applies a state: the window's Uygula, or a state of Katman durumları ▾.
    Apply(String),
    Update,
    Remove,
    Save,
    Rename,
    Name(String),
    Locks(bool),
    Styles(bool),
    Close,
}

fn msg(event: Event) -> Message {
    Message::LayerStates(event)
}

impl App {
    /// Why the layer states may not change in the open drawing, or none.
    pub(crate) fn states_locked(&self) -> Option<&'static str> {
        self.tree_locked().map(|_| STATES_LOCKED)
    }

    /// The open drawing's layer states.
    pub(crate) fn layer_states(&self) -> &[LayerState] {
        self.document
            .as_ref()
            .map_or(&[], |d| d.model.settings().layer_states.as_slice())
    }

    /// Whether the drawing's layers are in the state now (the menu's mark).
    pub(crate) fn state_matches(&self, state: &LayerState) -> bool {
        self.document
            .as_ref()
            .is_some_and(|d| layer_states::matches(d.model.layers().nodes(), state))
    }

    /// The project's states replaced by `states`: an edit, not an undo step.
    fn write_states(&mut self, states: Vec<LayerState>) {
        if let Some(doc) = self.document.as_mut() {
            let mut settings = doc.model.settings().clone();
            settings.layer_states = states;
            doc.model.set_settings(settings);
        }
    }

    /// Saves the tree as a new state named `name` keeping `parts`; its id, or
    /// none when refused (said).
    pub(crate) fn save_layer_state(
        &mut self,
        name: &str,
        parts: LayerStateParts,
    ) -> Option<String> {
        if let Some(locked) = self.states_locked() {
            self.warn(locked);
            return None;
        }
        let doc = self.document.as_ref()?;
        let mut states = doc.model.settings().layer_states.clone();
        let name = name.trim();
        if let Some(refused) = name_refused(&states, name, None) {
            self.warn(refused);
            return None;
        }
        let state = layer_states::capture(
            doc.model.layers().nodes(),
            &next_state_id(&states),
            name,
            parts,
        );
        let id = state.id.clone();
        states.push(state);
        self.write_states(states);
        self.say(
            Level::Success,
            format!("“{name}” katman durumu kaydedildi: {}.", parts_text(parts)),
        );
        Some(id)
    }

    /// Saves the tree at once as “Durum n” with its visibility and styles
    /// (`layer.stateSave`, the panel's Yeni durum kaydet).
    pub(crate) fn quick_save_layer_state(&mut self) -> Option<String> {
        let name = next_state_name(self.layer_states());
        self.save_layer_state(
            &name,
            LayerStateParts {
                locks: false,
                styles: true,
            },
        )
    }

    /// The state `id` saved again from the tree as it is now, keeping `parts`.
    fn update_layer_state(&mut self, id: &str, parts: LayerStateParts) -> bool {
        if let Some(locked) = self.states_locked() {
            self.warn(locked);
            return false;
        }
        let Some(doc) = self.document.as_ref() else {
            return false;
        };
        let mut states = doc.model.settings().layer_states.clone();
        let Some(at) = states.iter().position(|s| s.id == id) else {
            return false;
        };
        let name = states[at].name.clone();
        states[at] = layer_states::capture(doc.model.layers().nodes(), id, &name, parts);
        self.write_states(states);
        self.say(
            Level::Success,
            format!(
                "“{name}” katman durumu şimdiki hâlle güncellendi: {}.",
                parts_text(parts)
            ),
        );
        true
    }

    /// The state `id` named `name`.
    fn rename_layer_state(&mut self, id: &str, name: &str) -> bool {
        if let Some(locked) = self.states_locked() {
            self.warn(locked);
            return false;
        }
        let mut states = self.layer_states().to_vec();
        let Some(at) = states.iter().position(|s| s.id == id) else {
            return false;
        };
        let old = states[at].name.clone();
        let name = name.trim();
        if name == old {
            return true;
        }
        if let Some(refused) = name_refused(&states, name, Some(id)) {
            self.warn(refused);
            return false;
        }
        states[at].name = name.to_owned();
        self.write_states(states);
        self.say(
            Level::Success,
            format!("“{old}” katman durumunun adı “{name}” oldu."),
        );
        true
    }

    /// Removes the state `id`.
    fn remove_layer_state(&mut self, id: &str) -> bool {
        if let Some(locked) = self.states_locked() {
            self.warn(locked);
            return false;
        }
        let mut states = self.layer_states().to_vec();
        let Some(at) = states.iter().position(|s| s.id == id) else {
            return false;
        };
        let old = states.remove(at);
        self.write_states(states);
        self.say(
            Level::Success,
            format!("“{}” katman durumu silindi.", old.name),
        );
        true
    }

    /// Applies the state `id`: the visibility and locks of the nodes it names
    /// that the tree still has become what it kept (the tree's own changes),
    /// their styles too in one undo step “Katman durumu: <ad>”; what it
    /// changed is said, and its nodes the drawing no longer has.
    pub(crate) fn apply_layer_state(&mut self, id: &str) {
        let Some(state) = self.layer_states().iter().find(|s| s.id == id).cloned() else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let model = &mut doc.model;
        let c = layer_states::changes(model.layers().nodes(), &state);
        for (node, visible) in &c.visible {
            model.set_layer_visible(node, *visible);
        }
        for (node, _) in &c.locked {
            model.toggle_layer_locked(node);
        }
        if !c.styles.is_empty() {
            let label = format!("Katman durumu: {}", state.name);
            let group = model.begin_group(&label);
            for (node, style) in &c.styles {
                model.set_layer_style(node, style.clone(), &label);
            }
            model.end_group(group);
        }
        let mut parts = Vec::new();
        if !c.visible.is_empty() {
            parts.push(format!("{} görünürlük", c.visible.len()));
        }
        if !c.locked.is_empty() {
            parts.push(format!("{} kilit", c.locked.len()));
        }
        if !c.styles.is_empty() {
            parts.push(format!("{} stil", c.styles.len()));
        }
        if parts.is_empty() {
            self.say(
                Level::Info,
                format!("Katmanlar zaten “{}” durumunda.", state.name),
            );
        } else {
            self.say(
                Level::Success,
                format!(
                    "“{}” katman durumu uygulandı: {} değişti.",
                    state.name,
                    parts.join(", ")
                ),
            );
        }
        if c.missing > 0 {
            self.say(
                Level::Info,
                format!(
                    "Durumdaki {} katman ya da grup artık çizimde yok; atlandı.",
                    c.missing
                ),
            );
        }
    }

    /// Katman durumları (`layer.states`): the window.
    pub(crate) fn open_layer_states(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.layer_states_window = Some(Window {
            chosen: None,
            name: next_state_name(self.layer_states()),
            locks: false,
            styles: true,
        });
        self.dialog = Some(Dialog::LayerStates);
    }

    /// Chooses the state `id`: its name and parts come into the fields.
    fn choose_state(&mut self, id: Option<String>) {
        let state = id
            .as_deref()
            .and_then(|id| self.layer_states().iter().find(|s| s.id == id))
            .cloned();
        let Some(w) = self.layer_states_window.as_mut() else {
            return;
        };
        if let Some(state) = state {
            let parts = layer_states::parts_of(&state);
            w.name = state.name;
            w.locks = parts.locks;
            w.styles = parts.styles;
        }
        w.chosen = id;
    }

    pub(crate) fn layer_states_event(&mut self, event: Event) -> Task<Message> {
        let parts = self.layer_states_window.as_ref().map(|w| LayerStateParts {
            locks: w.locks,
            styles: w.styles,
        });
        let chosen = self
            .layer_states_window
            .as_ref()
            .and_then(|w| w.chosen.clone());
        match event {
            Event::Choose(id) => self.choose_state(Some(id)),
            Event::Apply(id) => self.apply_layer_state(&id),
            Event::Update => {
                if let (Some(id), Some(parts)) = (chosen, parts) {
                    self.update_layer_state(&id, parts);
                }
            }
            Event::Remove => {
                if let Some(id) = chosen
                    && self.remove_layer_state(&id)
                {
                    let name = next_state_name(self.layer_states());
                    if let Some(w) = self.layer_states_window.as_mut() {
                        w.chosen = None;
                        w.name = name;
                    }
                }
            }
            Event::Save => {
                let name = self
                    .layer_states_window
                    .as_ref()
                    .map(|w| w.name.clone())
                    .unwrap_or_default();
                if let Some(parts) = parts
                    && let Some(id) = self.save_layer_state(&name, parts)
                {
                    self.choose_state(Some(id));
                }
            }
            Event::Rename => {
                let name = self
                    .layer_states_window
                    .as_ref()
                    .map(|w| w.name.clone())
                    .unwrap_or_default();
                if let Some(id) = chosen {
                    self.rename_layer_state(&id, &name);
                }
            }
            Event::Name(name) => {
                if let Some(w) = self.layer_states_window.as_mut() {
                    w.name = name;
                }
            }
            Event::Locks(on) => {
                if let Some(w) = self.layer_states_window.as_mut() {
                    w.locks = on;
                }
            }
            Event::Styles(on) => {
                if let Some(w) = self.layer_states_window.as_mut() {
                    w.styles = on;
                }
            }
            Event::Close => {
                self.layer_states_window = None;
                self.dialog = None;
            }
        }
        // A state the window had chosen and that is gone (removed elsewhere) is chosen no more.
        let gone = self
            .layer_states_window
            .as_ref()
            .and_then(|w| w.chosen.clone())
            .is_some_and(|id| !self.layer_states().iter().any(|s| s.id == id));
        if gone && let Some(w) = self.layer_states_window.as_mut() {
            w.chosen = None;
        }
        Task::none()
    }

    pub(crate) fn layer_states_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.layer_states_window else {
            return iced::widget::text("").into();
        };
        let locked = self.states_locked();
        let chosen = w.chosen.as_deref();
        let mut list = Column::new().spacing(1).padding(2);
        if self.layer_states().is_empty() {
            list = list.push(
                container(label::caption(
                    "Kayıtlı durum yok. Aşağıya bir ad yazıp Yeni durum kaydet’e basın.",
                ))
                .padding(10),
            );
        }
        for state in self.layer_states() {
            let now = self.state_matches(state);
            let mark: Element<'_, Message> = if now {
                icon(Icon::Check).size(14.0).tone(Tone::Accent).into()
            } else {
                iced::widget::text("").into()
            };
            let name = Elided::new(state.name.clone())
                .size(typography::body())
                .font(typography::ui())
                .width(Fill);
            let parts = label::caption(parts_text(layer_states::parts_of(state))).style(
                |theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(theme).muted),
                },
            );
            let face = row![container(mark).width(typography::scaled(18.0)), name, parts]
                .spacing(6)
                .align_y(Center);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(state.id.clone())))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(chosen == Some(state.id.as_str()))),
            );
        }
        let list = container(iced::widget::scrollable(list).height(typography::scaled(180.0)))
            .style(style::container::field_box)
            .width(Fill);
        let editable = locked.is_none();
        let actions = row![
            words::secondary(APPLY, chosen.map(|id| msg(Event::Apply(id.to_owned())))),
            words::secondary(
                UPDATE,
                chosen.filter(|_| editable).map(|_| msg(Event::Update))
            ),
            words::secondary(
                REMOVE,
                chosen.filter(|_| editable).map(|_| msg(Event::Remove))
            ),
        ]
        .spacing(8);
        let name = text_input("Durumun adı", &w.name)
            .on_input(|t| msg(Event::Name(t)))
            .padding([5, 8])
            .style(style::field::input);
        let checks = row![
            words::check(w.locks, LOCKS, Some(msg(Event::Locks(!w.locks)))),
            words::check(w.styles, STYLES, Some(msg(Event::Styles(!w.styles)))),
        ]
        .spacing(18);
        let saving = row![
            words::primary(SAVE, editable.then(|| msg(Event::Save))),
            words::secondary(
                RENAME,
                chosen.filter(|_| editable).map(|_| msg(Event::Rename))
            ),
        ]
        .spacing(8);
        let mut body = Column::new().spacing(12);
        if let Some(why) = locked {
            body = body.push(words::summary(vec![words::text_line(Kind::Warn, why)]));
        }
        let body = body
            .push(words::field("Kayıtlı durumlar", list, None))
            .push(actions)
            .push(words::field(NAME, focus_ring(name), None))
            .push(words::field(
                "Kaydedilecekler",
                checks,
                Some("Görünürlük her durumda kaydedilir.".to_owned()),
            ))
            .push(saving);
        overlay::modal(
            Frame::new(STATES_TITLE)
                .push(body)
                .action(words::secondary(CLOSE, Some(msg(Event::Close))))
                .width(520.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): a
    /// state's row by its name, Ad, Kilitler, Stiller and the buttons.
    pub(crate) fn layer_states_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.layer_states_window else {
            return Err(format!("{STATES_TITLE} penceresi açık değil"));
        };
        let editable = self.states_locked().is_none();
        let chosen = w.chosen.clone();
        Ok(match control {
            Control::Fill(NAME, text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Check(LOCKS, on) => (w.locks != on).then(|| msg(Event::Locks(on))),
            Control::Check(STYLES, on) => (w.styles != on).then(|| msg(Event::Styles(on))),
            Control::Press(APPLY) => chosen.map(|id| msg(Event::Apply(id))),
            Control::Press(UPDATE) => chosen.filter(|_| editable).map(|_| msg(Event::Update)),
            Control::Press(REMOVE) => chosen.filter(|_| editable).map(|_| msg(Event::Remove)),
            Control::Press(SAVE) => editable.then(|| msg(Event::Save)),
            Control::Press(RENAME) => chosen.filter(|_| editable).map(|_| msg(Event::Rename)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(words) => match self.layer_states().iter().find(|s| s.name == words) {
                Some(s) => Some(msg(Event::Choose(s.id.clone()))),
                None => {
                    return Err(format!(
                        "“{STATES_TITLE}” penceresinde “{words}” düğmesi yok"
                    ));
                }
            },
            other => return Err(format!("“{STATES_TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
mod tests {
    use kentos_domain::layer_states::LayerStateParts;

    use crate::app::Message;
    use crate::files_testing::{app_with_drawing, last_said};

    #[test]
    fn the_parts_are_said_as_the_web_says_them() {
        let text = |locks, styles| super::parts_text(LayerStateParts { locks, styles });
        assert_eq!(text(false, false), "görünürlük");
        assert_eq!(text(false, true), "görünürlük ve stil");
        assert_eq!(text(true, false), "görünürlük ve kilit");
        assert_eq!(text(true, true), "görünürlük, kilit ve stil");
    }

    /// The sample drawing: Kadastro / Parsel (active), Kadastro / Bina
    /// (locked), Çizim (hidden).
    #[test]
    fn a_state_keeps_the_styles_and_gives_them_back_in_one_undo_step() {
        let mut app = app_with_drawing();
        let parts = LayerStateParts {
            locks: true,
            styles: true,
        };
        let id = app.save_layer_state("İlk hâl", parts).expect("saved");
        assert_eq!(
            last_said(&app),
            "“İlk hâl” katman durumu kaydedildi: görünürlük, kilit ve stil."
        );
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        let parsel = doc.layers().nodes()[0].children[0].id.clone();
        let mut style = doc.layers().get(&parsel).expect("Parsel").style.clone();
        let before = style.clone();
        style.color = "#4F8EF7".to_owned();
        doc.set_layer_style(&parsel, style, "Katman stili");
        doc.toggle_layer_locked(&parsel);
        let _ = app.update(Message::LayerStates(super::Event::Apply(id)));
        assert_eq!(
            last_said(&app),
            "“İlk hâl” katman durumu uygulandı: 1 kilit, 1 stil değişti."
        );
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        assert_eq!(doc.layers().get(&parsel).expect("Parsel").style, before);
        assert!(!doc.layers().get(&parsel).expect("Parsel").locked);
        // The styles came back in one undo step of the state's name.
        assert_eq!(doc.undo().as_deref(), Some("Katman durumu: İlk hâl"));
        // The name may not be taken twice.
        assert!(app.save_layer_state(" İlk hâl ", parts).is_none());
        assert_eq!(
            last_said(&app),
            "“İlk hâl” adında bir katman durumu var; başka bir ad yazın ya da o durumu seçip Güncelle'ye basın."
        );
    }
}
