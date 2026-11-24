//! KentOS Mimari's start (docs/adr/8007): its wall tools begin with a hatched style instead of "Yok", and a new project
//! opens on the Mimari tab (its level, Zemin kat, comes with the project: `kentos_project::new_project`). Before, the
//! first wall of a new building was two thin lines with no level, drawn after a click on the Mimari tab.
//!
//! Katman anahtarı (docs/adr/8008): KentOS Mimari writes each Mimari object on its kind's EK-4 layer; the tool panel's
//! Katman row (tool_settings.rs) says which, and turns the key off (Etkin katman) or on (Türüne göre) for the session.

use crate::app::App;

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
        // Each Mimari object on its kind's EK-4 layer (docs/adr/8008).
        self.memory.layer_keys = true;
        // Blok ekle's Duvara hizala on (docs/adr/8024): a fixture by a wall stands its back on the face, its front
        // toward the room (docs/adr/0702); H turns it off.
        self.memory.block_to_wall = true;
        if let Some(style) = kentos_commands::builtin_wall_styles()
            .into_iter()
            .find(|s| s.id == FIRST_WALL_STYLE)
        {
            kentos_interaction::wall::take_style(&mut self.memory, &style);
        }
    }
}

#[cfg(test)]
#[path = "../tests/arch_start_tests.rs"]
mod tests;
