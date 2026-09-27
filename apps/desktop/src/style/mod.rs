//! The style engine on the desktop (docs/STYLE.md, docs/adr/0090): the
//! style library the drawing and the style windows read, the drawing's
//! styled layers for the drawing area (`scene.rs`), the pictures the GPU
//! atlas draws its images from (`images.rs`, `svg.rs`), symbol pictures for
//! the windows (`thumbs.rs`) and the Katman stili window (`layer_style/`,
//! docs/adr/0091).
//!
//! The library holds the system symbols that ship with KentOS, the user's
//! own and the open project's (`ProjectStyles` in the drawing, so everyone
//! who opens the project sees them). The web keeps the same three sources
//! (`style/library.ts`).

pub mod images;
pub mod layer_style;
#[cfg(test)]
mod perf;
pub mod scene;
#[cfg(test)]
mod screens;
pub mod svg;
pub mod thumbs;

use std::sync::Arc;

use kentos_contracts::ProjectStyles;
use kentos_native_style::library::{Source, StyleLibrary};

/// The style library and what drawing with it needs.
pub struct Styles {
    pub library: StyleLibrary,
    /// The atlas's pictures, shared with the drawing area's frames.
    pub images: Arc<images::Images>,
    /// The symbol pictures of the style windows.
    pub thumbs: thumbs::Thumbs,
    /// The open Katman stili window.
    pub layer_style: Option<layer_style::LayerStyleWindow>,
    /// The project styles last loaded, so an unchanged drawing is not read again.
    project: Option<ProjectStyles>,
}

impl std::fmt::Debug for Styles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Styles")
            .field("version", &self.library.version())
            .finish_non_exhaustive()
    }
}

impl Default for Styles {
    fn default() -> Self {
        Self::new()
    }
}

impl Styles {
    /// The system library, and no user or project items yet.
    pub fn new() -> Styles {
        Styles {
            library: kentos_native_style::system::library(),
            images: Arc::new(images::Images::new()),
            thumbs: thumbs::Thumbs::default(),
            layer_style: None,
            project: None,
        }
    }

    /// What the drawing area draws the styled layers with: the typed settings'
    /// `graphics.symbolSize` and `graphics.lineWeights` (the web's `prefs`).
    pub fn styling<'a>(
        &'a self,
        spatial: &'a kentos_interaction::Spatial,
        settings: &crate::settings::Settings,
    ) -> crate::viewport::Styling<'a> {
        crate::viewport::Styling {
            spatial,
            styles: self,
            screen_symbols: settings.text("graphics.symbolSize") == "screen",
            line_weights: settings.bool("graphics.lineWeights"),
        }
    }

    /// Follows the open drawing's own symbols (its `ProjectStyles`); cheap when they did not change.
    /// Called only when the drawing changed (the app's `follow_document`).
    pub fn follow_project(&mut self, styles: Option<&ProjectStyles>) {
        if self.project.as_ref() == styles {
            return;
        }
        let empty = ProjectStyles::default();
        let s = styles.unwrap_or(&empty);
        self.library.load(Source::Project, &s.items, &s.categories);
        self.project = styles.cloned();
    }
}

impl crate::app::App {
    /// Sembol boyutu: symbols at the plot scale or at a fixed size on the screen (`view.symbols.*`).
    pub(crate) fn choose_symbol_size(&mut self, id: &str) {
        let screen = id == "view.symbols.screen";
        let value = if screen { "screen" } else { "plot" };
        let _ = self
            .settings
            .choose(&[("graphics.symbolSize", serde_json::Value::from(value))]);
        self.apply_settings();
        self.output(if screen {
            "Semboller: ekranda sabit"
        } else {
            "Semboller: çizim ölçeğinde"
        });
    }
}
