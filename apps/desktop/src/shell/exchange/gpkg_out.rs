//! GeoPackage dışa aktar's window (gis-usage-report GIS-007, docs/adr/1073): before the file is asked, what goes in —
//! Seçili nesneler, Etkin katman or Tüm proje — each with its objects and layers counted, so a field team sent one park
//! gets one park. Dışa aktar… asks where and writes through `file.export.gpkg` (its `uids` or `layers`).

use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::ExportGpkg;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message, Picker};

/// What the file takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Selection,
    Active,
    All,
}

/// A scope with its objects, as the segmented control shows it: “Seçili (1)”.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counted(pub Scope, pub usize);

impl fmt::Display for Counted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self.0 {
            Scope::Selection => "Seçili nesneler",
            Scope::Active => "Etkin katman",
            Scope::All => "Tüm proje",
        };
        write!(f, "{word} ({})", self.1)
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub scope: Scope,
    /// Tüm proje's empty layers whose type or fields make a table (a field template), written empty (GIS-010).
    pub empty: bool,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Scope(Scope),
    Empty,
    Save,
    Picked(Option<PathBuf>),
}

fn event(e: Event) -> Message {
    message(Exchange::GpkgOut(e))
}

/// A scope's objects and the layers they are on, with each layer's objects.
struct Reach {
    objects: usize,
    layers: BTreeSet<String>,
    per_layer: std::collections::BTreeMap<String, usize>,
    /// The objects a table has no geometry for (texts, dimensions, blocks…), by their kinds' words: counted, not written.
    left: std::collections::BTreeMap<&'static str, usize>,
}

/// The words of what the file takes and what it does not (docs/adr/9139).
pub(crate) mod words_of {
    pub const LEFT: &str = "Dosyaya gitmeyen: formlar, birleşik görünümler, çalışma alanı (görünüm, paftalar) ve yazı, \
                            ölçü, blok gibi geometrisi olmayan nesneler; bunları proje dosyası (.kcad) tutar.";
    pub const COPY: &str = "Bu, verinin bir kopyasıdır: proje bu dosyaya bağlanmaz. Projeyi verisiyle bir GeoPackage'ta \
                            saklamak için Dosya › Veri kaynağı; bir pakete kalıcı yeni bir tablo için Veri › Yeni katman › \
                            Türüyle.";
    pub fn taken(relations: usize) -> String {
        format!(
            "Dosyaya giden: her katman bir tablo (alanları türleriyle), katmanın stili{}.",
            if relations == 0 { String::new() } else { format!(" ve iki ucu da yazılan {relations} ilişki") }
        )
    }
}

impl App {
    /// The objects and layers a scope writes.
    fn gpkg_reach(&self, scope: Scope) -> Reach {
        let mut reach =
            Reach { objects: 0, layers: BTreeSet::new(), per_layer: Default::default(), left: Default::default() };
        let Some(doc) = &self.document else {
            return reach;
        };
        let doc = &doc.model;
        let mut take = |e: &kentos_commands::Entity| {
            // What the file takes as the writer does: an object a table has no geometry for is counted apart.
            if let Err(word) = kentos_native_application::geopackage::geometry_of(e) {
                *reach.left.entry(word).or_default() += 1;
                return;
            }
            reach.objects += 1;
            reach.layers.insert(e.base().layer_id.clone());
            *reach.per_layer.entry(e.base().layer_id.clone()).or_default() += 1;
        };
        match scope {
            Scope::Selection => self.selection.ids().iter().filter_map(|s| doc.get(*s)).for_each(&mut take),
            Scope::Active => doc.by_layer_with_uids(doc.layers().active()).for_each(|(_, e)| take(e)),
            Scope::All => doc.entities().for_each(&mut take),
        }
        reach
    }

    /// The empty layers a table can be made of (typed point, line or area, or with fields), by id.
    fn gpkg_template_ids(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let doc = &doc.model;
        doc.layers()
            .leaves()
            .iter()
            .filter(|n| doc.by_layer_with_uids(&n.id).next().is_none())
            .filter(|n| {
                matches!(
                    n.geometry,
                    Some(kentos_commands::LayerGeometry::Point | kentos_commands::LayerGeometry::Line | kentos_commands::LayerGeometry::Area)
                ) || !n.fields.is_empty()
            })
            .map(|n| n.id.clone())
            .collect()
    }

    /// Their names.
    fn gpkg_templates(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        self.gpkg_template_ids()
            .iter()
            .filter_map(|id| doc.model.layers().get(id).map(|n| n.name.clone()))
            .collect()
    }

    /// File › GeoPackage dışa aktar: the window, Seçili nesneler first when something is selected.
    pub(super) fn gpkg_out_open(&mut self) -> Task<Message> {
        let scope = if self.selection.ids().is_empty() { Scope::All } else { Scope::Selection };
        self.open_window(Window::GpkgOut(State { scope, empty: true, status: None }));
        Task::none()
    }

    pub(super) fn gpkg_out_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Scope(scope) => {
                if let Some(Window::GpkgOut(s)) = &mut self.exchange {
                    s.scope = scope;
                    s.status = None;
                }
                Task::none()
            }
            Event::Empty => {
                if let Some(Window::GpkgOut(s)) = &mut self.exchange {
                    s.empty = !s.empty;
                }
                Task::none()
            }
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Picked(Some(path.clone()))));
                }
                let suggested = self.export_name(".gpkg");
                Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("GeoPackage dışa aktar")
                            .add_filter("GeoPackage", &["gpkg"])
                            .set_file_name(suggested)
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension("gpkg");
                        }
                        Some(path)
                    },
                    |p| event(Event::Picked(p)),
                )
            }
            Event::Picked(None) => Task::none(),
            Event::Picked(Some(path)) => {
                let Some(Window::GpkgOut(s)) = &self.exchange else {
                    return Task::none();
                };
                let scope = s.scope;
                let empty = s.empty && scope == Scope::All;
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let model = &doc.model;
                let (layers, uids) = match scope {
                    Scope::All => (Vec::new(), Vec::new()),
                    Scope::Active => (vec![model.layers().active().to_owned()], Vec::new()),
                    Scope::Selection => (
                        Vec::new(),
                        self.selection
                            .ids()
                            .iter()
                            .filter_map(|s| model.uid(*s))
                            .map(|u| u.to_string())
                            .collect(),
                    ),
                };
                let input = ExportGpkg {
                    path: path.display().to_string(),
                    layers,
                    uids,
                    empty_layers: empty,
                    overwrite: true,
                    time: self.export_time(),
                };
                if self.gpkg_write(&path, &input) {
                    self.exchange = None;
                } else if let Some(Window::GpkgOut(s)) = &mut self.exchange {
                    s.status = self.log.lines().last().map(|l| (true, l.text.clone()));
                }
                Task::none()
            }
        }
    }

    /// The summary's lines: what is written (each layer with its objects), what goes into the file with it and what
    /// does not, and that it is a copy (docs/adr/9139). A warning first when the scope writes nothing.
    fn gpkg_out_lines(&self, reach: &Reach) -> Vec<(bool, String)> {
        if reach.objects == 0 && reach.layers.is_empty() {
            return vec![(true, "Bu kapsamda yazılacak nesne yok.".to_owned())];
        }
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let layers = doc.model.layers();
        let names: Vec<String> = reach
            .layers
            .iter()
            .filter_map(|id| {
                let n = layers.get(id)?;
                Some(match reach.per_layer.get(id) {
                    Some(count) => format!("{} ({count})", n.name),
                    None => format!("{} (boş tablo)", n.name),
                })
            })
            .collect();
        // The project's relations whose two layers both go: the file's relation profile takes them.
        let relations = doc
            .model
            .settings()
            .relations
            .iter()
            .filter(|r| reach.layers.contains(&r.parent) && reach.layers.contains(&r.child))
            .count();
        let mut lines = vec![
            (false, format!("{} nesne, {} katman yazılır: {}.", reach.objects, reach.layers.len(), names.join(", "))),
            (false, words_of::taken(relations)),
        ];
        // The objects of the scope a table has no geometry for: how many of each kind stay out.
        if !reach.left.is_empty() {
            let kinds: Vec<String> = reach.left.iter().map(|(word, n)| format!("{n} {word}")).collect();
            lines.push((true, format!("Geometrisi olmadığı için yazılmayan: {}.", kinds.join(", "))));
        }
        lines.push((false, words_of::LEFT.to_owned()));
        lines.push((false, words_of::COPY.to_owned()));
        lines
    }

    /// The summary's lines for the scope chosen (a test's question).
    #[cfg(test)]
    pub(crate) fn gpkg_out_summary(&self, scope: Scope) -> Vec<String> {
        self.gpkg_out_lines(&self.gpkg_reach(scope)).into_iter().map(|(_, t)| t).collect()
    }

    pub(super) fn gpkg_out_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let counted = |scope| Counted(scope, self.gpkg_reach(scope).objects);
        let selected = !self.selection.ids().is_empty();
        let mut body = Column::new().spacing(12).push(words::field(
            "Kapsam",
            Segmented::new_with(
                [counted(Scope::Selection), counted(Scope::Active), counted(Scope::All)],
                counted(s.scope),
                |c: Counted| event(Event::Scope(c.0)),
                move |c: Counted| c.0 != Scope::Selection || selected,
            ),
            Some("Seçili nesneler yalnız seçimi, kendi katmanlarının tablolarına yazar; Etkin katman o katmanın bütün nesnelerini; Tüm proje nesnesi olan her katmanı.".to_owned()),
        ));
        let templates = self.gpkg_templates();
        if s.scope == Scope::All && !templates.is_empty() {
            body = body.push(words::field(
                "Boş katmanlar",
                words::check(
                    s.empty,
                    format!("Şeması hazır boş katmanları da yaz ({})", templates.len()),
                    Some(event(Event::Empty)),
                ),
                Some(format!(
                    "Türü ya da alanları tanımlı ama nesnesi olmayan katmanlar ({}) boş tablo olarak, sütunlarıyla yazılır: saha ekibi ilk kaydı yeni sütun açmadan girer.",
                    templates.join(", ")
                )),
            ));
        }
        if let Some(row) = self.export_time_row() {
            body = body.push(row);
        }
        let mut reach = self.gpkg_reach(s.scope);
        if s.scope == Scope::All && s.empty {
            reach.layers.extend(self.gpkg_template_ids());
        }
        let lines = self.gpkg_out_lines(&reach)
            .into_iter()
            .map(|(warn, text)| words::text_line(if warn { Line::Warn } else { Line::Info }, text))
            .collect::<Vec<_>>();
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(if *error { Line::Error } else { Line::Info }, text.clone()));
        }
        overlay::blocking(
            Dialog::new("GeoPackage dışa aktar")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("Dışa aktar…", (!reach.layers.is_empty()).then(|| event(Event::Save))))
                .width(640.0)
                .max_height(720.0)
                .on_close(Some(message(Exchange::Close))),
        )
    }
}

