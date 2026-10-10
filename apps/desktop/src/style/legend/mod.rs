//! Lejant on the desktop (docs/STYLE.md §7, docs/adr/0093; the web's
//! `ui/style/LegendDialog.ts`): what the drawing's symbols mean, layer by
//! layer, the rows from `kentos_native_style::legend` (held with the web's
//! to `fixtures/style/v1/legend.json`). Layers can be left out; “PNG olarak
//! kaydet” draws the legend on white paper with black ink, whatever the
//! theme, at twice its size (`sheet.rs`).

mod sheet;
#[cfg(test)]
mod tests;
mod view;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::Arc;

use iced::Task;
use kentos_contracts::Entity;
use kentos_native_style::legend::{
    LegendGroup, LegendLayer, LegendSources, legend_layers, legend_layout, legend_of, texts,
};
use kentos_native_style::library::StyleLibrary;
use serde_json::Value;

use crate::app::{App, Dialog, Message};

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    VisibleOnly(bool),
    Headings(bool),
    /// A layer in the legend (true) or left out.
    Layer(String, bool),
    Save,
    /// The picture written (its path's words), or why not; none when the save was cancelled.
    Saved(Option<Result<String, String>>),
    Close,
}

pub(crate) fn ev(e: Event) -> Message {
    Message::Legend(e)
}

/// A line of the window's list: a layer's heading or one of its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Head(usize),
    Entry(usize, usize),
}

/// The Lejant window.
pub struct LegendWindow {
    pub visible_only: bool,
    pub headings: bool,
    /// The layers left out of the legend.
    pub left: BTreeSet<String>,
    said: Option<(String, bool)>,
    /// A picture being drawn and written.
    pub saving: bool,
    /// The groups, built again only when the drawing, the library or the choice changed.
    cache: RefCell<Option<(Key, Arc<Vec<LegendGroup>>)>>,
}

/// What the groups depend on: the drawing's generation, the library's version, visible only.
type Key = (u64, u64, bool);

impl Default for LegendWindow {
    fn default() -> Self {
        LegendWindow {
            visible_only: true,
            headings: true,
            left: BTreeSet::new(),
            said: None,
            saving: false,
            cache: RefCell::new(None),
        }
    }
}

/// The drawing and the library, as the legend reads them.
struct Sources<'a> {
    doc: &'a kentos_domain::Document,
    library: &'a StyleLibrary,
}

impl LegendSources for Sources<'_> {
    fn entities(&self, layer: &str) -> Vec<&Entity> {
        self.doc.by_layer(layer).collect()
    }

    fn symbol(&self, id: &str) -> Option<Value> {
        self.library.symbol(id).cloned()
    }

    fn item_name(&self, id: &str) -> Option<String> {
        self.library.get(id).map(|(i, _)| i.name().to_owned())
    }
}

impl LegendWindow {
    pub fn said(&self) -> Option<&(String, bool)> {
        self.said.as_ref()
    }

    pub fn say(&mut self, text: impl Into<String>, warn: bool) {
        self.said = Some((text.into(), warn));
    }

    /// The legend's groups (every layer's, the left out ones too), the top of the list first.
    pub fn groups(
        &self,
        doc: &kentos_domain::Document,
        library: &StyleLibrary,
    ) -> Arc<Vec<LegendGroup>> {
        let key = (doc.generation(), library.version(), self.visible_only);
        if let Some((k, groups)) = &*self.cache.borrow()
            && *k == key
        {
            return groups.clone();
        }
        let layers = doc.layers();
        let leaves: Vec<(LegendLayer<'_>, bool)> = layers
            .leaves()
            .into_iter()
            .map(|n| {
                (
                    LegendLayer {
                        id: &n.id,
                        name: &n.name,
                        style: &n.style,
                    },
                    layers.is_visible(&n.id),
                )
            })
            .collect();
        let src = Sources { doc, library };
        let groups = Arc::new(legend_of(&legend_layers(&leaves, self.visible_only), &src));
        *self.cache.borrow_mut() = Some((key, groups.clone()));
        groups
    }

    /// The groups the picture shows: the ones not left out.
    pub fn shown(&self, groups: &[LegendGroup]) -> Vec<LegendGroup> {
        groups
            .iter()
            .filter(|g| !self.left.contains(&g.layer_id))
            .cloned()
            .collect()
    }

    /// The footer's count: the rows of the layers shown (`${n} satır`).
    pub fn rows(&self, groups: &[LegendGroup]) -> usize {
        groups
            .iter()
            .filter(|g| !self.left.contains(&g.layer_id))
            .map(|g| g.entries.len())
            .sum()
    }
}

/// The picture PNG olarak kaydet would write, drawn here and now (pictures for the owner).
#[cfg(test)]
pub(crate) fn picture(app: &App) -> Option<Result<Vec<u8>, String>> {
    Some(app.legend_sheet()?.and_then(|s| sheet::png(&s)))
}

/// The list's lines: each group's heading, then its rows.
pub fn lines(groups: &[LegendGroup]) -> Vec<Line> {
    let mut out = Vec::new();
    for (g, group) in groups.iter().enumerate() {
        out.push(Line::Head(g));
        out.extend((0..group.entries.len()).map(|e| Line::Entry(g, e)));
    }
    out
}

impl App {
    /// `style.legend`: the drawing's legend.
    pub(crate) fn open_legend(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.styles.legend = Some(LegendWindow::default());
        self.dialog = Some(Dialog::Legend);
    }

    pub(crate) fn legend_event(&mut self, event: Event) -> Task<Message> {
        let Some(window) = &mut self.styles.legend else {
            return Task::none();
        };
        match event {
            Event::VisibleOnly(on) => window.visible_only = on,
            Event::Headings(on) => window.headings = on,
            Event::Layer(id, on) => {
                if on {
                    window.left.remove(&id);
                } else {
                    window.left.insert(id);
                }
            }
            Event::Close => {
                self.styles.legend = None;
                self.dialog = None;
            }
            Event::Saved(outcome) => {
                window.saving = false;
                match outcome {
                    Some(Ok(_)) => window.say(texts::SAVED, false),
                    Some(Err(why)) => window.say(why, true),
                    // The save dialog was closed: nothing happened.
                    None => {}
                }
            }
            Event::Save => return self.save_legend(),
        }
        Task::none()
    }

    /// What PNG olarak kaydet draws, or why nothing: no rows, or more than a PNG holds.
    fn legend_sheet(&self) -> Option<Result<sheet::Sheet, String>> {
        let (Some(window), Some(doc)) = (&self.styles.legend, &self.document) else {
            return None;
        };
        let groups = window.groups(&doc.model, &self.styles.library);
        let shown = window.shown(&groups);
        if shown.is_empty() {
            return Some(Err(texts::NO_ROWS.to_owned()));
        }
        let layout = legend_layout(&shown, window.headings, doc.name());
        if layout.height > sheet::MAX_HEIGHT {
            return Some(Err(format!(
                "Lejant PNG için çok uzun: {} satır, en çok {} satır olur. Bazı katmanları dışarıda bırakın ya da başlıkları kapatın.",
                layout.rows.len(),
                sheet::max_lines()
            )));
        }
        let entries: Vec<_> = shown.into_iter().flat_map(|g| g.entries).collect();
        Some(Ok(sheet::Sheet {
            layout,
            scales: entries.iter().map(|e| e.px_per_mm).collect(),
            symbols: entries.into_iter().map(|e| e.symbol).collect(),
            library: self.styles.library.clone(),
            images: self.styles.images.clone(),
        }))
    }

    /// PNG olarak kaydet: where to, then the picture drawn off the window's thread.
    fn save_legend(&mut self) -> Task<Message> {
        if self.styles.legend.as_ref().is_none_or(|w| w.saving) {
            return Task::none();
        }
        let sheet = match self.legend_sheet() {
            Some(Ok(sheet)) => sheet,
            Some(Err(why)) => {
                if let Some(window) = &mut self.styles.legend {
                    window.say(why, true);
                }
                return Task::none();
            }
            None => return Task::none(),
        };
        if let Some(window) = &mut self.styles.legend {
            window.saving = true;
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Lejantı kaydet")
                    .add_filter("PNG görüntüsü", &["png"])
                    .set_file_name(texts::FILE)
                    .save_file()
                    .await?;
                let mut path = file.path().to_path_buf();
                if path.extension().is_none() {
                    path.set_extension("png");
                }
                // The picture is drawn on its own device, off the window's thread.
                let (tx, rx) = iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let _ = tx.send(sheet::png(&sheet));
                });
                let written = match rx.await {
                    Ok(Ok(bytes)) => std::fs::write(&path, bytes)
                        .map(|()| path.display().to_string())
                        .map_err(|e| {
                            format!(
                                "Lejant yazılamadı ({}): {e}. Başka bir klasör seçin.",
                                path.display()
                            )
                        }),
                    Ok(Err(why)) => Err(why),
                    Err(_) => Err("Lejant çizilemedi: çizim işi yarıda kaldı.".to_owned()),
                };
                Some(written)
            },
            |outcome| ev(Event::Saved(outcome)),
        )
    }
}
