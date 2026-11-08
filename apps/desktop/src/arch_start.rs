//! KentOS Mimari's start (docs/adr/3007): its wall tools begin with a hatched style instead of "Yok", and a new project
//! opens on the Mimari tab (its level, Zemin kat, comes with the project: `kentos_project::new_project`). Before, the
//! first wall of a new building was two thin lines with no level, drawn after a click on the Mimari tab.
//!
//! Katman anahtarı (docs/adr/3008): KentOS Mimari writes each Mimari object on its kind's EK-4 layer; the tool panel's
//! Katman row says which, and turns the key off (Etkin katman) or on (Türüne göre) for the session.

use crate::app::{App, Message};
use kentos_ui::widget::Menu;

/// The style KentOS Mimari's wall tools start with: KentOS's exterior wall (iç sıva, tuğla, ısı yalıtımı, dış sıva;
/// 25 cm), drawn poşeli at 1:100. A built-in style: the project takes it with the first wall (docs/adr/0339 §3).
pub(crate) const FIRST_WALL_STYLE: &str = "kentos-dis-duvar-25";

impl App {
    /// The Mimari program's wall tools take [`FIRST_WALL_STYLE`] and its thickness, as choosing it under Stil does.
    /// Other programs keep "Yok" and 0.20 m.
    pub(crate) fn start_architecture_tools(&mut self) {
        if self.program != Some(kentos_launch::Module::Architecture) {
            return;
        }
        // Each Mimari object on its kind's EK-4 layer (docs/adr/3008).
        self.memory.layer_keys = true;
        // Blok ekle's Duvara hizala on (docs/adr/3024): a fixture by a wall stands its back on the face, its front
        // toward the room (docs/adr/0702); H turns it off.
        self.memory.block_to_wall = true;
        if let Some(style) = kentos_commands::builtin_wall_styles()
            .into_iter()
            .find(|s| s.id == FIRST_WALL_STYLE)
        {
            kentos_interaction::wall::take_style(&mut self.memory, &style);
        }
    }

    /// The tool panel's Katman (docs/adr/3008): the layer the running tool writes on; its menu turns the layer key on
    /// (Türüne göre, EK-4) or off (Etkin katman). None for a tool that writes no Mimari object.
    pub(crate) fn layer_key_setting(&self) -> Option<iced::Element<'_, Message>> {
        let shown = self.session.layer_shown()?.to_owned();
        let keyed = self.memory.layer_keys;
        let active = self.document.as_ref().map_or_else(String::new, |d| {
            let layers = d.model.layers();
            layers
                .get(layers.active())
                .map_or_else(|| layers.active().to_owned(), |l| l.name.clone())
        });
        Some(kentos_ui::widget::property_grid::choice(
            shown,
            None,
            move || {
                Menu::new()
                    .header("Katman")
                    .radio("Türüne göre (EK-4)", keyed, Some(Message::LayerKeys(true)))
                    .radio(
                        format!("Etkin katman ({active})"),
                        !keyed,
                        Some(Message::LayerKeys(false)),
                    )
            },
        ))
    }

    /// Katman's menu: the layer key on or off for the session; the running tool sees it at once.
    pub(crate) fn choose_layer_keys(&mut self, on: bool) {
        self.memory.layer_keys = on;
        let _ = self.with_tool(|session, cx| {
            session.document_changed(cx);
            session.see_layer(cx);
        });
        self.output(if on {
            "Katman: mimari nesneler türlerinin EK-4 katmanına yazılır (MM-DUVA-TUGLA, MM-KAPI-GENEL …)."
        } else {
            "Katman: mimari nesneler etkin katmana yazılır."
        });
    }

    /// A new project opens on its program's own tab: KentOS Mimari's on Mimari, where Duvar is. Other programs, and
    /// a file opened, keep the tab as it was.
    pub(crate) fn open_new_project_on_program_tab(&mut self) {
        if let Some(tab) = self.program.and_then(crate::view::program_tab) {
            self.tab = tab;
            self.ribbon_context = false;
        }
    }
}

#[cfg(test)]
#[path = "arch_start_tests.rs"]
mod tests;
