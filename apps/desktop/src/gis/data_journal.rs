//! Bekleyen veri düzenlemeleri (docs/adr/9111 §3): what a saved project leaves out of its file, kept on this device beside
//! it and given back when it is opened again.
//!
//! - A file keeps a data layer's new rows as its objects (docs/adr/9103 §2) but not the rows taken from the table, nor the
//!   keys deleted without being taken: those are the table's data, personal data among it (6698 md. 12), and a file goes
//!   from hand to hand. They are written here, in the user's own folder, readable by the user alone, a file per project
//!   (its place: the `.kcad` file, or the project in a data source). A save of the project writes its journal, or removes
//!   it when nothing waits.
//! - Opened again, the project gets them back (`Document::restore_pending`); a save to the table checks them as it checks
//!   any taken row (docs/adr/9104 §3): a row someone changed since asks first.
//! - The journal follows the tables: what is written to a table or let go after the project was saved leaves the journal,
//!   and the file's new rows among it are named, so that the project opened again does not write them a second time
//!   (`Document::drop_left`).
//! - A copy (Kopyasını kaydet), a cloud project and a recovery copy keep no journal: the drawing stays with its own file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kentos_commands::Entity;
use kentos_domain::{PendingLayer, PendingRow, Uuid};
use serde::{Deserialize, Serialize};

use crate::app::App;
use crate::document::{Document, StoredSource};

/// The journal's format.
const VERSION: u32 = 1;

/// Where the journals are kept: a folder of the device; none in a test unless one is given.
#[derive(Debug, Clone, Default)]
pub(crate) struct Journals {
    dir: Option<PathBuf>,
}

impl Journals {
    /// The user's folder for the program's state: `%LOCALAPPDATA%\kentos-cad\veri-oturumlari` on Windows,
    /// `~/Library/Application Support/kentos-cad/veri-oturumlari` on macOS, else `$XDG_STATE_HOME/kentos-cad/veri-oturumlari`
    /// (`~/.local/state/…`).
    pub(crate) fn device() -> Self {
        let env = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
        let base = if cfg!(target_os = "windows") {
            env("LOCALAPPDATA").or_else(|| env("APPDATA"))
        } else if cfg!(target_os = "macos") {
            env("HOME").map(|h| h.join("Library").join("Application Support"))
        } else {
            env("XDG_STATE_HOME").or_else(|| env("HOME").map(|h| h.join(".local").join("state")))
        };
        Self { dir: base.map(|b| b.join("kentos-cad").join("veri-oturumlari")) }
    }

    #[cfg(test)]
    pub(crate) fn at(dir: &Path) -> Self {
        Self { dir: Some(dir.to_path_buf()) }
    }

    /// A project's journal file: the place's FNV-1a hash, which does not change between the program's versions.
    pub(crate) fn file(&self, place: &str) -> Option<PathBuf> {
        let hash = place.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
        Some(self.dir.as_ref()?.join(format!("{hash:016x}.json")))
    }

    fn read(&self, place: &str) -> Result<Option<Written>, String> {
        let Some(file) = self.file(place) else {
            return Ok(None);
        };
        match std::fs::read(&file) {
            Ok(bytes) => serde_json::from_slice::<Written>(&bytes)
                .map(Some)
                .map_err(|e| format!("{}: {e}", file.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", file.display())),
        }
    }

    /// Writes the journal (to a file of its own first, renamed over the last one), or removes it when nothing waits.
    fn write(&self, kept: &Kept, project: Option<Uuid>) -> std::io::Result<()> {
        let Some(file) = self.file(&kept.place) else {
            return Ok(());
        };
        if kept.layers.is_empty() && kept.gone.is_empty() {
            return match std::fs::remove_file(&file) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            };
        }
        let dir = file.parent().unwrap_or(Path::new("."));
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(dir)?;
        let written = Written::of(kept, project);
        let bytes = serde_json::to_vec(&written).map_err(std::io::Error::other)?;
        let part = file.with_extension("json.yaziliyor");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        // Its rows are personal data: the user's alone (6698 md. 12).
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        {
            use std::io::Write as _;
            let mut out = options.open(&part)?;
            out.write_all(&bytes)?;
            out.sync_all()?;
        }
        std::fs::rename(&part, &file)
    }

    fn remove(&self, place: &str) {
        if let Some(file) = self.file(place) {
            let _ = std::fs::remove_file(file);
        }
    }
}

/// A project's journal as an open drawing keeps it in step with its tables.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Kept {
    /// The project's place: its file, or its project in a data source.
    pub place: String,
    /// The rows its file left out when it was saved or opened, less what was written or let go since.
    pub layers: Vec<PendingLayer>,
    /// The file's objects that were written or let go before it was opened (its journal said so).
    pub gone: Vec<Uuid>,
    /// `Document::data_left`'s length when the file was saved or opened: what left after is gone from the file's rows.
    pub left_from: usize,
    /// `Document::data_written` the journal is in step with.
    pub written: u64,
}

/// A `.kcad` file's place.
pub(crate) fn file_place(path: &Path) -> String {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()).display().to_string()
}

/// A project's place in a data source.
pub(crate) fn source_place(at: &StoredSource) -> String {
    format!("kaynak:{}|{}|{}", at.address, at.schema.as_deref().unwrap_or(""), at.name)
}

/// An open drawing's place, when it has one a journal goes with: its file or its project in a source; a cloud project
/// and a drawing without a file have none.
pub(crate) fn place_of(doc: &Document) -> Option<String> {
    if doc.cloud_source().is_some() {
        return None;
    }
    doc.path.as_deref().map(file_place).or_else(|| doc.stored_source().map(source_place))
}

/// What a save of `model` to `place` keeps beside it: taken when the save starts, with the drawing it writes.
pub(crate) fn kept_for(model: &kentos_domain::Document, place: String) -> Kept {
    Kept {
        place,
        layers: model.pending_rows(),
        gone: Vec::new(),
        left_from: model.data_left().len(),
        written: model.data_written(),
    }
}

/// The journal as written.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Written {
    version: u32,
    /// The project's persistent id: the journal of another project once at the same place is not given to this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project: Option<String>,
    /// When it was written (milliseconds since the epoch).
    written_at: u64,
    #[serde(default)]
    layers: Vec<WrittenLayer>,
    /// The file's objects its tables hold now or that were let go.
    #[serde(default)]
    gone: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WrittenLayer {
    layer: String,
    #[serde(default)]
    taken: Vec<WrittenRow>,
    #[serde(default)]
    deleted: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WrittenRow {
    uid: String,
    key: i64,
    row: Entity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    now: Option<Entity>,
}

impl Written {
    fn of(kept: &Kept, project: Option<Uuid>) -> Self {
        Self {
            version: VERSION,
            project: project.map(|p| p.to_string()),
            written_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64),
            layers: kept
                .layers
                .iter()
                .map(|p| WrittenLayer {
                    layer: p.layer.clone(),
                    taken: p
                        .taken
                        .iter()
                        .map(|r| WrittenRow { uid: r.uid.to_string(), key: r.key, row: r.row.clone(), now: r.now.clone() })
                        .collect(),
                    deleted: p.deleted.clone(),
                })
                .collect(),
            gone: kept.gone.iter().map(ToString::to_string).collect(),
        }
    }

    /// Its layers and the file's objects gone; a row whose id does not read is left out.
    fn parts(self) -> (Vec<PendingLayer>, Vec<Uuid>) {
        let layers = self
            .layers
            .into_iter()
            .map(|l| PendingLayer {
                layer: l.layer,
                taken: l
                    .taken
                    .into_iter()
                    .filter_map(|r| {
                        Some(PendingRow { uid: Uuid::parse_str(&r.uid).ok()?, key: r.key, row: r.row, now: r.now })
                    })
                    .collect(),
                deleted: l.deleted,
            })
            .collect();
        let gone = self.gone.iter().filter_map(|u| Uuid::parse_str(u).ok()).collect();
        (layers, gone)
    }
}

impl App {
    /// A project saved to its place: its journal written beside it (or removed when nothing waits), and kept by its
    /// drawing to follow its tables. What to say of it, when anything: said after the save's own sentence.
    pub(crate) fn journal_saved(&mut self, session: u64, kept: Kept) -> Option<(kentos_interaction::Level, String)> {
        let journals = self.gis.data_journal.clone();
        let doc = match self.document.as_mut().filter(|d| d.session == session) {
            Some(doc) => Some(doc),
            None => self.doc_tabs.find_mut(session),
        }?;
        let project = doc.model.project_id();
        let result = journals.write(&kept, project);
        let waiting = kept.layers.iter().map(|p| p.taken.len() + p.deleted.len()).sum::<usize>();
        doc.kept = Some(kept);
        match result {
            Ok(()) if waiting > 0 => Some((
                kentos_interaction::Level::Info,
                format!(
                    "Tabloya yazılmamış {waiting} satır değişikliği projeyle bu cihazda saklandı; proje açılınca geri gelir."
                ),
            )),
            Ok(()) => None,
            Err(e) => Some((
                kentos_interaction::Level::Warn,
                format!(
                    "Bekleyen veri düzenlemeleri bu cihazda saklanamadı ({e}). Proje kaydedildi; tabloya yazılmamış alınan ve silinen satırlar proje yeniden açılınca gelmez."
                ),
            )),
        }
    }

    /// A project just read: what its file left out given back from its journal, and the file's rows that were written or
    /// let go after it was saved taken out (see the module).
    pub(crate) fn journal_opened(&mut self, doc: &mut Document) {
        let Some(place) = place_of(doc) else {
            return;
        };
        let journals = self.gis.data_journal.clone();
        let mut kept = Kept { place: place.clone(), ..Kept::default() };
        let read = journals.read(&place);
        let finish = |doc: &mut Document, mut kept: Kept| {
            kept.left_from = doc.model.data_left().len();
            kept.written = doc.model.data_written();
            doc.kept = Some(kept);
        };
        let written = match read {
            Ok(Some(written)) => written,
            Ok(None) => return finish(doc, kept),
            Err(why) => {
                self.warn(format!("Bekleyen veri düzenlemeleri okunamadı ({why}); proje onlarsız açıldı."));
                return finish(doc, kept);
            }
        };
        if let (Some(theirs), Some(ours)) = (written.project.as_deref(), doc.model.project_id())
            && theirs != ours.to_string()
        {
            journals.remove(&place);
            self.warn(
                "Bu yerde saklanan bekleyen veri düzenlemeleri başka bir projeye aitti (dosyanın yerine başka bir proje kaydedilmiş); kullanılmadı ve silindi.",
            );
            return finish(doc, kept);
        }
        let (layers, gone) = written.parts();
        let dropped = doc.model.drop_left(&gone);
        let said = doc.model.restore_pending(layers);
        kept.layers = doc.model.pending_rows();
        kept.gone = gone;
        let back: Vec<String> = kept
            .layers
            .iter()
            .map(|p| {
                let c = doc.model.data_changes(&p.layer);
                let name = doc.model.layers().get(&p.layer).map_or(p.layer.as_str(), |n| n.name.as_str());
                format!("{name}: {}", crate::data_edits::counts_words(c.added.len(), c.changed.len(), c.deleted.len()))
            })
            .collect();
        finish(doc, kept);
        if !back.is_empty() {
            self.say(
                kentos_interaction::Level::Info,
                format!(
                    "Bekleyen veri düzenlemeleri geri geldi ({}). Tabloya yazmak için Katmanlar'da katmanın sağ tuşundan Değişiklikleri kaydet'i seçin; başkasının o arada değiştirdiği satırlar kaydederken sorulur.",
                    back.join("; ")
                ),
            );
        }
        if dropped > 0 {
            self.output(format!(
                "Proje kaydedildikten sonra tabloya yazılan ya da bırakılan {dropped} yeni satır çizime alınmadı: tabloya ikinci kez yazılmaz."
            ));
        }
        for why in said {
            self.warn(why);
        }
    }

    /// After every message: the open project's journal follows its tables once a save to a table or a letting go
    /// changed them.
    pub(crate) fn journal_tasks(&mut self) {
        let journals = self.gis.data_journal.clone();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let model = &doc.model;
        let Some(kept) = doc.kept.as_mut() else {
            return;
        };
        let written = model.data_written();
        if written == kept.written {
            return;
        }
        kept.written = written;
        for p in &mut kept.layers {
            let layer = p.layer.clone();
            p.taken.retain_mut(|r| match model.taken_row(&layer, r.uid) {
                // Still taken: a save that wrote it while it changed keeps it against the row as written.
                Some(row) if model.taken_key(&layer, r.uid) == Some(r.key) => {
                    r.row = (*row).clone();
                    true
                }
                _ => false,
            });
            let still: BTreeSet<i64> = model.deleted_keys(&layer).into_iter().collect();
            p.deleted.retain(|k| still.contains(k));
        }
        kept.layers.retain(|p| !p.is_empty());
        let left = model.data_left().get(kept.left_from..).unwrap_or_default();
        let mut gone = kept.gone.clone();
        gone.extend(left.iter().copied().filter(|u| !kept.gone.contains(u)));
        let mut written_kept = kept.clone();
        written_kept.gone = gone;
        let project = model.project_id();
        if let Err(e) = journals.write(&written_kept, project) {
            self.warn(format!("Bekleyen veri düzenlemelerinin günlüğü güncellenemedi ({e})."));
        }
    }
}
