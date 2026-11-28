//! Seçilenleri kaydet in a CBS project (docs/adr/9339). A CBS project's data is its tables (docs/adr/9196 §2): a copy of
//! the selection whose layers held their objects in the project file could not be saved as it was opened (its first
//! Kaydet asked for a data store). So the copy's kind is asked before its file:
//!
//! - **CBS projesi ve GeoPackage** (the first): the selection's drawing, and the layers whose objects a CBS project keeps
//!   in tables moved by `cad.layers.migrate` into the GeoPackage beside the project file (`<ad>.gpkg`). The copy then
//!   saves at once, its rows read from its own GeoPackage. A table of the same name there is given a name of its own
//!   (Yeni tablo), made again (Üzerine yaz) or added to (Ekle). Every move is planned first; a refusal writes nothing,
//!   and a GeoPackage made for the copy is removed when the copy fails.
//! - **Çizim kopyası:** the objects stay in the project file, as before (docs/adr/0193 §1): for a CAD module, or to move
//!   them later.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{
    CallOp, CreateTarget, DocumentSnapshotV2, EntityId, ExportExisting, LayersMigrate,
};
use kentos_native_application::{ExecutionContext, registry};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// The copy's kind, as the control names it: with its tables (true) or the objects in the project file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind(pub bool);

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0 {
            "CBS projesi ve GeoPackage"
        } else {
            "Çizim kopyası"
        })
    }
}

/// What a table of the same name in the GeoPackage gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Same(pub ExportExisting);

impl fmt::Display for Same {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            ExportExisting::ReplaceLayer => "Üzerine yaz",
            ExportExisting::Append => "Ekle",
            _ => "Yeni tablo",
        })
    }
}

#[derive(Debug)]
pub struct State {
    pub tables: bool,
    /// A table there already: `Refuse` gives the new one a name of its own.
    pub same: ExportExisting,
    /// The layers whose objects go into tables, by name.
    pub layers: Vec<String>,
    uids: Vec<EntityId>,
    drawing: DocumentSnapshotV2,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tables(bool),
    Same(ExportExisting),
    /// Kaydet…: the file asked.
    Save,
    Picked(Option<PathBuf>),
}

fn event(e: Event) -> Message {
    message(Exchange::SelectionSave(e))
}

/// The selection's drawing, and the layers of it a CBS project keeps in tables (none in a CAD project).
pub(crate) fn held_layers(drawing: &DocumentSnapshotV2, uids: &[EntityId]) -> Vec<String> {
    let Ok(made) = kentos_domain::exchange::selection(drawing, uids, "seçim") else {
        return Vec::new();
    };
    let Ok(doc) = kentos_domain::Document::from_snapshot_v2(made) else {
        return Vec::new();
    };
    kentos_native_application::layer_kind::held_in_file(&doc)
        .into_iter()
        .map(|h| h.name)
        .collect()
}

impl App {
    /// The window of a CBS project's selection: its kind and a same-named table's fate asked.
    pub(crate) fn selection_save_open(
        &mut self,
        uids: Vec<EntityId>,
        drawing: DocumentSnapshotV2,
        layers: Vec<String>,
    ) {
        self.open_window(Window::SelectionSave(State {
            tables: true,
            same: ExportExisting::Refuse,
            layers,
            uids,
            drawing,
        }));
    }

    pub(super) fn selection_save_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Tables(on) => {
                if let Some(Window::SelectionSave(s)) = &mut self.exchange {
                    s.tables = on;
                }
            }
            Event::Same(same) => {
                if let Some(Window::SelectionSave(s)) = &mut self.exchange {
                    s.same = same;
                }
            }
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Picked(Some(path.clone()))));
                }
                let name = self
                    .document
                    .as_ref()
                    .map(|d| d.model.name().to_owned())
                    .unwrap_or_default();
                let suggested = format!(
                    "{} - seçim.kcad",
                    if name.is_empty() {
                        "Çizim"
                    } else {
                        name.as_str()
                    }
                );
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Seçilenleri dosyaya kaydet")
                            .add_filter("KentOS projesi (.kcad)", &["kcad"])
                            .set_file_name(suggested)
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| event(Event::Picked(path)),
                );
            }
            Event::Picked(None) => {}
            Event::Picked(Some(path)) => {
                let Some(Window::SelectionSave(s)) = self.exchange.take() else {
                    return Task::none();
                };
                self.close_exchange();
                let State {
                    tables,
                    same,
                    uids,
                    drawing,
                    ..
                } = s;
                return off_thread(
                    move || {
                        if tables {
                            write_tables(&drawing, &uids, &path, same)
                        } else {
                            write_selection(&drawing, &uids, &path)
                        }
                    },
                    Exchange::SelectionSaved,
                );
            }
        }
        Task::none()
    }

    pub(super) fn selection_saved(&mut self, done: Result<String, String>) -> Task<Message> {
        match done {
            Ok(said) => self.say(kentos_interaction::Level::Success, said),
            Err(why) => self.warn(why),
        }
        Task::none()
    }

    pub(super) fn selection_save_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let kinds = Segmented::new([Kind(true), Kind(false)], Kind(s.tables), |k: Kind| {
            event(Event::Tables(k.0))
        });
        let mut body = Column::new().spacing(12).push(words::field(
            "Kopyanın türü",
            kinds,
            Some(if s.tables {
                "Seçilenler, kaydedeceğiniz proje dosyasıyla aynı adlı GeoPackage'a (<ad>.gpkg, yanında) tablo olarak yazılır; proje dosyası onları okur. Açılınca doğrudan kaydedilir.".to_owned()
            } else {
                "Nesneler proje dosyasında kalır: CAD modüllerinde açmak için. CBS'de açılınca ilk Kaydet onları bir veri deposuna taşımayı ister.".to_owned()
            }),
        ));
        if s.tables {
            let same = [
                ExportExisting::Refuse,
                ExportExisting::ReplaceLayer,
                ExportExisting::Append,
            ]
            .map(Same);
            body = body.push(words::field(
                "GeoPackage'da aynı adlı tablo varsa",
                Segmented::new(same, Same(s.same), |x: Same| event(Event::Same(x.0))),
                Some(
                    match s.same {
                        ExportExisting::ReplaceLayer => "Tablo silinip seçilenlerle yeniden yapılır; GeoPackage'ın öbür tabloları kalır.",
                        ExportExisting::Append => "Seçilenler tablonun satırlarının ardına eklenir; katman bütün tabloyu okur.",
                        _ => "Seçilenler yeni bir tabloya yazılır (adının sonuna _2, _3 …); var olan tablo değişmez.",
                    }
                    .to_owned(),
                ),
            ));
        }
        let mut lines = vec![words::text_line(
            Line::Info,
            format!(
                "{} nesne seçili. Tabloya yazılacak katmanlar: {}.",
                s.uids.len(),
                s.layers.join(", ")
            ),
        )];
        if s.tables {
            lines.push(words::text_line(
                Line::Info,
                "Ölçü, blok ve tarama gibi tabloya yazılamayan nesne varsa hiçbir şey yazılmaz; onlar için Çizim kopyası'nı seçin.",
            ));
        }
        body = body.push(words::summary(lines));
        overlay::blocking(
            Dialog::new("Seçilenleri kaydet")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("Kaydet…", Some(event(Event::Save))))
                .width(620.0)
                .max_height(560.0)
                .on_close(Some(message(Exchange::Close))),
        )
    }
}

/// The selection written as a CBS project and its GeoPackage at `path` (the GeoPackage beside it, `<stem>.gpkg`): what
/// to say, or why not (then nothing is left of the copy).
pub fn write_tables(
    drawing: &DocumentSnapshotV2,
    uids: &[EntityId],
    path: &Path,
    same: ExportExisting,
) -> Result<String, String> {
    let stem = path
        .file_stem()
        .map_or_else(|| "Çizim".to_owned(), |s| s.to_string_lossy().into_owned());
    let made = kentos_domain::exchange::selection(drawing, uids, &stem)
        .map_err(|why| format!("Seçilenler yazılamadı: {why}"))?;
    let objects = made.entities.len();
    let mut doc = kentos_domain::Document::from_snapshot_v2(made)
        .map_err(|why| format!("Seçilenler yazılamadı: {why}"))?;
    let gpkg = path.with_extension("gpkg");
    let existed = gpkg.exists();
    let mut taken: BTreeSet<String> = if existed {
        kentos_data::registry()
            .source(&gpkg.display().to_string())
            .and_then(|s| s.tables())
            .map(|t| t.into_iter().map(|t| t.name).collect())
            .map_err(|e| format!("“{}” okunamadı: {e}", gpkg.display()))?
    } else {
        BTreeSet::new()
    };
    let mut inputs = Vec::new();
    for held in kentos_native_application::layer_kind::held_in_file(&doc) {
        let base = kentos_native_application::layer_export::table_name_of(&held.name);
        let (table, existing) = match (taken.contains(&base), same) {
            (false, _) => (base, None),
            (true, ExportExisting::ReplaceLayer | ExportExisting::Append) => (base, Some(same)),
            (true, _) => (
                (2..)
                    .map(|n| format!("{base}_{n}"))
                    .find(|t| !taken.contains(t))
                    .unwrap_or(base),
                None,
            ),
        };
        taken.insert(table.clone());
        inputs.push(LayersMigrate {
            layer: held.id,
            target: CreateTarget::Gpkg,
            path: Some(gpkg.display().to_string()),
            table: Some(table),
            existing,
            ..LayersMigrate::default()
        });
    }
    let run = |doc: &mut kentos_domain::Document, input: &LayersMigrate, op: CallOp| {
        let value = serde_json::to_value(input).unwrap_or_default();
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.layers.migrate",
            op,
            value,
        )
        .map_err(|e| format!("{e:?}"))?;
        if answer["status"] == "completed" {
            Ok(answer["output"]["table"]
                .as_str()
                .unwrap_or_default()
                .to_owned())
        } else {
            Err(answer["error"]["message"]
                .as_str()
                .unwrap_or("taşınamadı")
                .to_owned())
        }
    };
    // Every move planned first: a refusal writes nothing.
    for input in &inputs {
        run(&mut doc, input, CallOp::Plan).map_err(|why| {
            format!("Seçilenler CBS projesi olarak yazılamadı: {why} Hiçbir dosya yazılmadı.")
        })?;
    }
    let undo = |why: String| {
        if !existed {
            kentos_data::registry().forget(&gpkg.display().to_string());
            let _ = std::fs::remove_file(&gpkg);
        }
        why
    };
    let mut tables = Vec::new();
    for input in &inputs {
        let table = run(&mut doc, input, CallOp::Execute)
            .map_err(|why| undo(format!("Seçilenler CBS projesi olarak yazılamadı: {why}")))?;
        tables.push(table);
    }
    let mut snapshot = doc.to_snapshot_v2();
    kentos_native_application::source_places::with_relative_layers(&mut snapshot.layers, path);
    crate::saving::write_watched(
        &snapshot,
        path,
        &AtomicBool::new(false),
        &mut |_| {},
        &crate::saving::Faults::NONE,
    )
    .map_err(|e| {
        undo(match e {
            crate::saving::SaveError::Failed(why) => format!("Seçilenler yazılamadı: {why}"),
            crate::saving::SaveError::Stopped => {
                "Seçilenler yazılamadı: kayıt durduruldu.".to_owned()
            }
        })
    })?;
    let name = |p: &Path| {
        p.file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
    };
    Ok(format!(
        "Seçilenler CBS projesi olarak kaydedildi: {objects} nesne → “{}”; verisi “{}” içinde ({}). Açık çizim değişmedi.",
        name(path),
        name(&gpkg),
        tables.join(", ")
    ))
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map_or_else(|| "Çizim".to_owned(), |s| s.to_string_lossy().into_owned())
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// The selection's drawing written to `path` (docs/adr/0193 §1): what to say, or why not.
pub fn write_selection(
    drawing: &DocumentSnapshotV2,
    uids: &[EntityId],
    path: &Path,
) -> Result<String, String> {
    let made = kentos_domain::exchange::selection(drawing, uids, &stem(path))
        .map_err(|why| format!("Seçilenler yazılamadı: {why}"))?;
    crate::saving::write_watched(
        &made,
        path,
        &AtomicBool::new(false),
        &mut |_| {},
        &crate::saving::Faults::NONE,
    )
    .map_err(|e| match e {
        crate::saving::SaveError::Failed(why) => format!("Seçilenler yazılamadı: {why}"),
        crate::saving::SaveError::Stopped => "Seçilenler yazılamadı: kayıt durduruldu.".to_owned(),
    })?;
    let (hatches, dimensions) = lost_ties(drawing, uids);
    let mut lost = Vec::new();
    if hatches > 0 {
        lost.push(format!("{hatches} tarama"));
    }
    if dimensions > 0 {
        lost.push(format!("{dimensions} ölçü"));
    }
    let lost = if lost.is_empty() {
        String::new()
    } else {
        format!(
            " Kaynak nesneleri seçilmediği için {} dosyada bağsız (ilişkisiz) kaldı; bağ kalsın istiyorsanız kaynaklarını da seçip yeniden kaydedin.",
            lost.join(" ve ")
        )
    };
    Ok(format!(
        "Seçilenler kaydedildi: {} nesne, {} blok → “{}”. Açık çizim değişmedi.{lost}",
        made.entities.len(),
        made.blocks.len(),
        name_of(path)
    ))
}

/// How many of the chosen hatches and dimensions are tied to objects not chosen (their ties do not go into the file):
/// a hatch by its boundary objects, a dimension by the objects its points are on.
pub(crate) fn lost_ties(drawing: &DocumentSnapshotV2, uids: &[EntityId]) -> (usize, usize) {
    use kentos_commands::Entity;
    let chosen: std::collections::HashSet<String> = uids.iter().map(|u| u.to_string()).collect();
    // Every “uid” string under a value (a dimension's references).
    fn named(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    match (k.as_str(), x) {
                        ("uid", serde_json::Value::String(u)) => out.push(u.clone()),
                        _ => named(x, out),
                    }
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| named(x, out)),
            _ => {}
        }
    }
    let (mut hatches, mut dimensions) = (0, 0);
    for (e, uid) in drawing.entities.iter().zip(&drawing.uids) {
        if !chosen.contains(&uid.to_string()) {
            continue;
        }
        match e {
            Entity::Hatch(h) => {
                if h.assoc.as_ref().is_some_and(|a| {
                    a.outer
                        .iter()
                        .chain(&a.bounds)
                        .chain(&a.islands)
                        .any(|id| !chosen.contains(&id.to_string()))
                }) {
                    hatches += 1;
                }
            }
            Entity::Dimension(d) => {
                if let Some(refs) = &d.refs {
                    let mut ids = Vec::new();
                    named(&serde_json::to_value(refs).unwrap_or_default(), &mut ids);
                    if ids.iter().any(|id| !chosen.contains(id)) {
                        dimensions += 1;
                    }
                }
            }
            _ => {}
        }
    }
    (hatches, dimensions)
}
