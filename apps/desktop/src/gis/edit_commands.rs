//! The ribbon's Düzenleme panel (docs/adr/9301): the active data layer's edit session opened and closed, every edited
//! layer's changes saved to their sources, let go after one question, or looked at in Katman değişiklikleri. They are
//! the session's own controls (data_edits.rs) as commands, so the ribbon, Komut ara and the command line reach them; the
//! layer's right-click menu has none of them (the owner's word of 24 November 2026: “katman sağ tuş menüsünden
//! kaldırılmalı hem edit hem de kayıt”).

use iced::Task;
use kentos_commands::LayerNode;

use crate::app::{App, Message};
use crate::data_edits::Event;

/// Düzenlemeyi başlat or bitir on the active layer.
pub(crate) const TOGGLE: &str = "layer.edit";
/// Değişiklikleri kaydet: every edited layer's changes to their sources.
pub(crate) const SAVE: &str = "layer.editSave";
/// Değişiklikleri geri al: every edited layer's changes let go, asked first.
pub(crate) const ROLLBACK: &str = "layer.editRollback";
/// Değişiklikleri incele…: Katman değişiklikleri.
pub(crate) const REVIEW: &str = "layer.editReview";
/// The panel's commands.
pub(crate) const COMMANDS: [&str; 4] = [TOGGLE, SAVE, ROLLBACK, REVIEW];

impl App {
    /// The active layer, with its data source's read-only flag when it is a data layer (none: not a data layer).
    fn edit_target(&self) -> Option<(&LayerNode, Option<bool>)> {
        let layers = self.document.as_ref()?.model.layers();
        let node = layers.get(layers.active())?;
        Some((node, node.data.as_ref().map(|d| d.read_only)))
    }

    /// Whether an edited layer has changes to save or let go.
    fn edits_waiting(&self) -> bool {
        !self.data_modified_layers(&[]).is_empty()
    }

    /// A command of the panel run; none for any other command.
    pub(crate) fn edit_command(&mut self, id: &str) -> Option<Task<Message>> {
        let task = match id {
            TOGGLE => {
                if let Some(why) = self.edit_why(TOGGLE) {
                    self.warn(why);
                    return Some(Task::none());
                }
                let (node, _) = self.edit_target()?;
                let layer = node.id.clone();
                let editing = self.document.as_ref().is_some_and(|d| d.model.editing(&layer));
                self.data_edits_event(if editing { Event::Stop(layer) } else { Event::Start(layer) })
            }
            SAVE => self.data_edits_event(Event::Save(Vec::new())),
            ROLLBACK => {
                self.ask_data_rollback();
                Task::none()
            }
            // On the active layer when its rows wait, else on the first that waits.
            REVIEW => {
                let active = self
                    .edit_target()
                    .map(|(node, _)| node.id.clone())
                    .filter(|layer| self.data_waiting().iter().any(|w| &w.layer == layer));
                self.update(crate::data_pending::open_at_message(active))
            }
            _ => return None,
        };
        Some(task)
    }

    /// Whether a command of the panel can run; none for any other command.
    pub(crate) fn edit_available(&self, id: &str) -> Option<bool> {
        Some(match id {
            TOGGLE => self.edit_why(TOGGLE).is_none(),
            SAVE | ROLLBACK => self.edits_waiting() && !self.data_saving(),
            REVIEW => !self.data_waiting().is_empty(),
            _ => return None,
        })
    }

    /// Why a command of the panel cannot run, said in its tip; none when it can, or for any other command.
    pub(crate) fn edit_why(&self, id: &str) -> Option<String> {
        if !COMMANDS.contains(&id) {
            return None;
        }
        if self.document.is_none() {
            return Some("Açık proje yok.".to_owned());
        }
        match id {
            TOGGLE => match self.edit_target() {
                None => Some("Etkin katman yok: Katmanlar panelinde düzenlenecek veri katmanına bir kez basın.".to_owned()),
                Some((node, None)) => Some(format!(
                    "“{}” bir veri katmanı değil: düzenleme oturumu bir veri kaynağının tablosunu okuyan katmanındır. \
                     Katmanlar panelinde düzenlenecek veri katmanına bir kez basın.",
                    node.name
                )),
                Some((node, Some(true))) => Some(format!(
                    "“{}” salt okunur bağlantıdan geliyor: düzenlemeye açılmaz, kaynağına yazılmaz.",
                    node.name
                )),
                Some(_) if self.data_saving() => {
                    Some("Katman değişiklikleri tabloya yazılıyor; yazma bitince yeniden deneyin.".to_owned())
                }
                Some(_) => None,
            },
            SAVE | ROLLBACK if self.data_saving() => {
                Some("Katman değişiklikleri tabloya yazılıyor; yazma bitince yeniden deneyin.".to_owned())
            }
            SAVE | ROLLBACK if !self.edits_waiting() => {
                Some("Düzenlenen katmanlarda kaydedilmemiş değişiklik yok.".to_owned())
            }
            REVIEW if self.data_waiting().is_empty() => Some("Kaynağına yazılmamış satır yok.".to_owned()),
            _ => None,
        }
    }

    /// Whether the active layer is being edited: Düzenlemeyi başlat's lit state; none for any other command.
    pub(crate) fn edit_checked(&self, id: &str) -> Option<bool> {
        (id == TOGGLE).then(|| {
            self.edit_target()
                .is_some_and(|(node, _)| self.document.as_ref().is_some_and(|d| d.model.editing(&node.id)))
        })
    }

    /// The ribbon's face of Düzenlemeyi başlat while the active layer is being edited: Düzenlemeyi bitir, on two lines as
    /// the large button writes it (its first line, and so its width, the same); none for any other command.
    pub(crate) fn edit_face(&self, id: &str) -> Option<&'static str> {
        (id == TOGGLE).then(|| {
            if self.edit_checked(TOGGLE) == Some(true) { "Düzenlemeyi\nbitir" } else { "Düzenlemeyi\nbaşlat" }
        })
    }
}
