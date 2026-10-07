//! The style engine on the desktop (docs/STYLE.md, docs/adr/0090): the
//! style library the drawing and the style windows read, the drawing's
//! styled layers for the drawing area (`scene.rs`), the pictures the GPU
//! atlas draws its images from (`images.rs`, `svg.rs`), symbol pictures for
//! the windows (`thumbs.rs`), the Katman stili window (`layer_style/`,
//! docs/adr/0091), Stil yöneticisi (`manager/`, docs/adr/0092), Lejant
//! (`legend/`, docs/adr/0093), Sembol tasarımcısı (`designer/`, its form
//! controls in `fields.rs`, docs/adr/0094) and SVG çizim düzenleyicisi
//! (`svgedit/`, docs/adr/0095).
//!
//! The library holds the system symbols that ship with KentOS, the user's
//! own (Kitaplığım, kept in a .kstil file of the user's data folder,
//! `user_library.rs`) and the open project's (`ProjectStyles` in the
//! drawing, so everyone who opens the project sees them). The web keeps the
//! same three sources (`style/library.ts`).

pub mod designer;
pub mod fields;
pub mod images;
pub mod layer_style;
pub mod legend;
pub mod manager;
#[cfg(test)]
mod perf;
pub mod scene;
#[cfg(test)]
mod scene_tests;
#[cfg(test)]
mod screens;
pub mod svg;
pub mod svgedit;
pub mod thumbs;
pub mod user_library;

use std::path::Path;
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
    /// The open Stil yöneticisi window.
    pub manager: Option<manager::Manager>,
    /// The open Lejant window.
    pub legend: Option<legend::LegendWindow>,
    /// The open Sembol tasarımcısı.
    pub designer: Option<designer::Designer>,
    /// The open SVG çizim düzenleyicisi.
    pub svg_editor: Option<svgedit::SvgEditor>,
    /// Where Kitaplığım is kept (nowhere until the program names its folder, and in tests).
    user_file: user_library::UserLibrary,
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
            manager: None,
            legend: None,
            designer: None,
            svg_editor: None,
            user_file: user_library::UserLibrary::default(),
            project: None,
        }
    }

    /// Kitaplığım from its file in `folder`; what to say when the file could not be read.
    pub fn open_user_library(&mut self, folder: &Path) -> Option<String> {
        let (file, opened) = user_library::UserLibrary::open(folder);
        self.user_file = file;
        self.library
            .load(Source::User, &opened.items, &opened.categories);
        opened.problem
    }

    /// After an edit of `source` in the library: Kitaplığım is written to its
    /// file, the project's part into the drawing (an edit, not an undo step:
    /// the library keeps no undo, as on the web). What to say when it could
    /// not be kept.
    pub fn changed(
        &mut self,
        source: Source,
        doc: Option<&mut kentos_domain::Document>,
    ) -> Option<String> {
        match source {
            Source::System => None,
            Source::User => self.user_file.save(&self.library).err().map(|e| {
                format!("Kitaplığım kaydedilemedi ({e}); değişiklik yalnız bu oturumda duruyor. Klasörün yazılabilir olduğunu denetleyin.")
            }),
            Source::Project => {
                let Some(doc) = doc else {
                    return Some("Açık çizim yok: projenin kitaplığı kaydedilemedi.".into());
                };
                let (items, categories) = self.library.dump(Source::Project);
                let styles = ProjectStyles { items, categories };
                doc.set_styles(styles.clone());
                // The drawing now holds what the library holds: not read back.
                self.project = Some(styles);
                None
            }
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
            view_build: kentos_native_style::View {
                fills: settings.bool("graphics.fills"),
                area_edges: settings.bool("graphics.areaEdges"),
            },
            view_colors: kentos_native_style::color::ViewColors {
                mode: kentos_native_style::color::ColorMode::from_key(
                    &settings.text("graphics.colorMode"),
                ),
                opaque: !settings.bool("graphics.transparency"),
            },
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

    /// Renk kipi from its command (Görünüm kipleri, docs/adr/0195 §3).
    pub(crate) fn choose_color_mode(&mut self, id: &str) {
        let (value, words) = match id {
            "view.colorMode.mono" => ("mono", "Tek renk"),
            "view.colorMode.gray" => ("gray", "Gri"),
            _ => ("color", "Renkli"),
        };
        let _ = self
            .settings
            .choose(&[("graphics.colorMode", serde_json::Value::from(value))]);
        self.apply_settings();
        self.output(format!("Renk kipi: {words}"));
    }
}
