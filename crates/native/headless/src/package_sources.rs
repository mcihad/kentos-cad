//! Paketle's bound sources (docs/adr/0596): what `leaveOut` names, the layers of the links left out emptied in the
//! package's drawing, and a database table copied into a GeoPackage of the rows its link reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kentos_commands::encoding::sha256_reader_hex;
use kentos_commands::{CommandError, SourceLink, codes};
use kentos_domain::{Document, NewLayer, Slot, Uuid};
use kentos_native_application::sources_query::{StageFrom, stage};

use crate::file_commands::Refused;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Refused {
    Box::new(CommandError {
        code: code.to_owned(),
        message: message.into(),
        path: Some(path.to_owned()),
        ..Default::default()
    })
}

/// What `leaveOut` names, each checked against the drawing.
#[derive(Default)]
pub(crate) struct Leave {
    /// Bound layers' links (`kaynak-N`) and the objects their layers held.
    pub links: BTreeMap<String, u32>,
    /// Objects' lasting ids.
    pub objects: BTreeSet<String>,
    /// External references' names.
    pub xrefs: BTreeSet<String>,
}

impl Leave {
    /// The entries read; one that names no link, object or external reference is refused at its place.
    pub(crate) fn read(doc: &Document, entries: &[String]) -> Result<Self, Refused> {
        let mut out = Leave::default();
        for (i, entry) in entries.iter().enumerate() {
            if let Some(link) = doc.settings().sources.iter().find(|l| &l.id == entry) {
                let held = doc.by_layer_with_uids(&link.layer).count();
                out.links
                    .insert(entry.clone(), u32::try_from(held).unwrap_or(u32::MAX));
            } else if let Some(name) = entry.strip_prefix("xref:") {
                if !doc
                    .blocks()
                    .iter()
                    .any(|b| b.xref.is_some() && b.name == name)
                {
                    return Err(refuse(
                        codes::INVALID_INPUT,
                        format!(
                            "“{name}” adlı bir dış başvuru yok; dışarıda bırakılacak dış başvuruyu adıyla (xref:ad) verin."
                        ),
                        &format!("leaveOut[{i}]"),
                    ));
                }
                out.xrefs.insert(name.to_owned());
            } else if Uuid::parse_str(entry)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .is_some()
            {
                out.objects.insert(entry.clone());
            } else {
                return Err(refuse(
                    codes::INVALID_INPUT,
                    format!(
                        "“{entry}” bir bağın kimliği (kaynak-N), bir nesnenin kalıcı kimliği ya da bir dış başvuru (xref:ad) değil; cad.sources.describe bağları listeler."
                    ),
                    &format!("leaveOut[{i}]"),
                ));
            }
        }
        Ok(out)
    }

    /// The drawing with the layers of the links left out emptied (a copy; none when no link is left out).
    pub(crate) fn emptied(&self, doc: &Document) -> Option<Document> {
        if self.links.is_empty() {
            return None;
        }
        let mut copy = doc.reading_copy();
        let layers: Vec<String> = doc
            .settings()
            .sources
            .iter()
            .filter(|l| self.links.contains_key(&l.id))
            .map(|l| l.layer.clone())
            .collect();
        let held: Vec<Slot> = layers
            .iter()
            .flat_map(|layer| {
                copy.by_layer_with_uids(layer)
                    .map(|(u, _)| u)
                    .collect::<Vec<_>>()
            })
            .filter_map(|u| copy.slot_of(u))
            .collect();
        copy.remove(&held);
        Some(copy)
    }
}

/// A database table's copy (docs/adr/0596 §2): a GeoPackage in a temporary folder, named for the package.
pub(crate) struct DatabaseCopy {
    pub file: PathBuf,
    /// The table it holds.
    pub table: String,
    pub rows: u64,
    pub sha256: String,
}

/// The rows `link` reads (its area's only, for a part) written into a GeoPackage in `dir`, in the project's system with
/// their lasting ids. None when the link reads no row: there is no table to write. Refused, at `databases`, when the
/// database is not reached, when its table changed since the layer last matched it, or when it holds too many rows.
pub(crate) fn copy_database(
    doc: &Document,
    link: &SourceLink,
    dir: &Path,
) -> Result<Option<DatabaseCopy>, Refused> {
    let at = |mut e: Box<CommandError>| {
        e.path = Some("databases".into());
        e
    };
    let (staging, _) = stage(
        doc.settings(),
        &StageFrom::Layer(link),
        None,
        link.extent,
        None,
        "databases",
        &|| false,
    )
    .map_err(|e| {
        at(e.unwrap_or_else(|| {
            refuse(
                codes::FILE_UNWRITABLE,
                "Veritabanı okunurken durduruldu.",
                "databases",
            )
        }))
    })?;
    if kentos_native_application::sources::layer_state(doc, &link.layer).is_some_and(|s| s.changed)
    {
        return Err(refuse(
            codes::SOURCE_CHANGED,
            format!(
                "{} tablosu “{}” katmanı onunla son eşleştiğinden beri değişti; paket yapılmadı. Önce tablodan okuyun ya da uzlaştırın, sonra paketleyin.",
                link.table, link.layer
            ),
            "databases",
        ));
    }
    if staging.objects.is_empty() {
        return Ok(None);
    }
    let short = link
        .table
        .rsplit('.')
        .next()
        .unwrap_or(&link.table)
        .to_owned();
    let mut copy = doc.reading_copy();
    let mut new = NewLayer::layer(short.clone());
    new.id = Some(format!("paket-{}", link.id));
    new.fields = staging.fields.clone().unwrap_or_default();
    let fail = |why: String| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("{} tablosunun kopyası yazılamadı: {why}", link.table),
            "databases",
        )
    };
    let layer = copy.add_layer(new, None, false).map_err(|r| fail(r.0))?;
    // The bound layer's own objects carry the rows' lasting ids: they give way in the copy.
    let taken: Vec<Slot> = staging
        .objects
        .iter()
        .filter_map(|(u, _)| copy.slot_of(*u))
        .collect();
    copy.remove(&taken);
    let rows = staging.objects.len() as u64;
    let objects = staging
        .objects
        .into_iter()
        .map(|(u, mut e)| {
            e.base_mut().layer_id = layer.clone();
            (u, e)
        })
        .collect();
    copy.add_many_with_uids(objects, "Paketle")
        .map_err(|r| fail(r.0))?;
    let name: String = link
        .table
        .chars()
        .map(|c| {
            if matches!(c, '.' | '/' | '\\' | ':' | '\0') || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let file = dir.join(format!("{name}.gpkg"));
    let written =
        kentos_native_application::geopackage::write(&copy, std::slice::from_ref(&layer), &file)
            .map_err(fail)?;
    let table = written.tables.first().map_or(short, |t| t.table.clone());
    let (_, sha256) = std::fs::File::open(&file)
        .and_then(sha256_reader_hex)
        .map_err(|e| fail(e.to_string()))?;
    Ok(Some(DatabaseCopy {
        file,
        table,
        rows,
        sha256,
    }))
}

/// The folder a plan's database copies are written in, removed with them when the plan goes.
pub(crate) struct Scratch(pub PathBuf);

impl Scratch {
    pub(crate) fn new() -> Result<Self, Refused> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "kentos-paket-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| {
            refuse(
                codes::FILE_UNWRITABLE,
                format!("{} açılamadı: {e}", dir.display()),
                "path",
            )
        })?;
        Ok(Scratch(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
