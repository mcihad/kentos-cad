//! KentOS Mimari's start (docs/adr/3007): its wall tools begin with a hatched style instead of "Yok", and a new project
//! opens on the Mimari tab (its level, Zemin kat, comes with the project: `kentos_project::new_project`). Before, the
//! first wall of a new building was two thin lines with no level, drawn after a click on the Mimari tab.

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
        if let Some(style) = kentos_commands::builtin_wall_styles()
            .into_iter()
            .find(|s| s.id == FIRST_WALL_STYLE)
        {
            kentos_interaction::wall::take_style(&mut self.memory, &style);
        }
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
