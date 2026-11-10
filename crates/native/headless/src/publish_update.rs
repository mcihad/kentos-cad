//! Update publication (docs/adr/1320 §3): `file.publish.sheets` with `previous` reads an earlier delivery folder and
//! carries each sheet whose page draws now what it drew then (equal context-free digests) and whose PDF still has its
//! delivered bytes, byte for byte, with its own date; the rest are made again. And the page source record (§2) every
//! publication writes into `yayin.json`, which `file.publish.status` compares with the drawing now.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use kentos_commands::{
    PreviousPublication, PublishSheets, SheetPublishIssue, SheetPublishMode, SheetPublishPin,
    SheetPublishScope, SheetPublishedFile, SheetPublishedPage, SheetSourceChanges,
    encoding::sha256_hex,
};
use kentos_domain::Document;
use kentos_native_application::sheets_sources;
use kentos_sheet::SheetBook;

use crate::file_commands::Refused;
use crate::publish::{issue, refuse};

/// The earlier delivery as read.
pub(crate) struct Previous {
    pub report: PreviousPublication,
    dir: PathBuf,
    pages: Vec<SheetPublishedPage>,
    files: Vec<SheetPublishedFile>,
    scope: Vec<SheetPublishScope>,
    issues: Vec<SheetPublishIssue>,
    options: serde_json::Value,
}

/// A page carried from the earlier delivery: its bytes, file and page records and its own non-error findings.
pub(crate) struct Carried {
    pub bytes: Vec<u8>,
    pub file: SheetPublishedFile,
    pub page: SheetPublishedPage,
    pub issues: Vec<SheetPublishIssue>,
}

/// `previous` read and checked against the input; none without it.
pub(crate) fn read(input: &PublishSheets, target: &Path) -> Result<Option<Previous>, Refused> {
    let Some(path) = &input.previous else {
        return Ok(None);
    };
    let dir = PathBuf::from(path);
    if path.trim().is_empty() || !dir.is_absolute() {
        return Err(refuse(
            "invalid_file_path",
            format!(
                "“{path}” mutlak bir klasör yolu değil; güncellenecek teslim klasörünün tam yolunu verin."
            ),
            "previous",
        ));
    }
    if dir == target {
        return Err(refuse(
            "invalid_input",
            "Güncelleme yeni bir klasöre yayınlanır, önceki teslim olduğu gibi kalır; başka bir teslim klasörü verin.",
            "path",
        ));
    }
    if input.mode != SheetPublishMode::Separate {
        return Err(refuse(
            "invalid_input",
            "Önceki teslimden güncelleme yalnız ayrı PDF'lerle yapılır: tek PDF'te değişmeyen sayfa ayrı korunamaz. Her paftayı ayrı PDF seçin.",
            "previous",
        ));
    }
    if input.resume || input.resumable {
        return Err(refuse(
            "invalid_input",
            "Güncelleme yayını devam ettirilebilir yayınla birlikte yapılamaz; ikisinden birini seçin.",
            "previous",
        ));
    }
    let at = dir.join("yayin.json");
    let bytes = std::fs::read(&at).ok();
    let manifest = bytes
        .as_ref()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        .filter(|m| m["format"] == "kentos.publish/1");
    let (Some(bytes), Some(m)) = (bytes, manifest) else {
        return Err(refuse(
            "no_previous",
            format!(
                "{path} klasöründe okunabilir bir yayın (yayin.json) yok; güncellenecek teslimin klasörünü verin."
            ),
            "previous",
        ));
    };
    let read = |k: &str| m[k].clone();
    Ok(Some(Previous {
        report: PreviousPublication {
            path: path.clone(),
            sha256: sha256_hex(&bytes),
            date: m["date"].as_str().unwrap_or_default().to_owned(),
            source: serde_json::from_value::<SheetPublishPin>(read("source")).unwrap_or_default(),
        },
        dir,
        pages: serde_json::from_value(read("pages")).unwrap_or_default(),
        files: serde_json::from_value(read("files")).unwrap_or_default(),
        scope: serde_json::from_value(read("scope")).unwrap_or_default(),
        issues: serde_json::from_value(read("issues")).unwrap_or_default(),
        options: read("options"),
    }))
}

impl Previous {
    /// Whether its pages may be carried into a publication with `input`'s options from `source`: a page digest
    /// covers the content stream and the names of its faces and pictures, not the page's geo-reference, its pictures'
    /// pixels or a resolution, so those must be the earlier delivery's (docs/adr/1320 §3).
    pub(crate) fn same_output(&self, input: &PublishSheets, source: &SheetPublishPin) -> bool {
        let o = &self.options;
        o["geo"].as_bool().unwrap_or(true) == input.geo
            && o["layers"].as_bool().unwrap_or(true) == input.layers
            // An archival file has its own metadata and output intent (docs/adr/8037).
            && o["pdfa"].as_bool().unwrap_or(false) == input.pdfa
            && o["dpi"] == serde_json::json!(input.dpi)
            && self.report.source.resources_sha256 == source.resources_sha256
    }

    /// The earlier page of `sheet` when it draws what `digest` draws and its file, alone in it, still has the
    /// delivered bytes; named `name` in the new delivery.
    pub(crate) fn carry(&self, name: &str, sheet: &str, digest: &str) -> Option<Carried> {
        if digest.is_empty() {
            return None;
        }
        let page = self
            .pages
            .iter()
            .find(|p| p.sheet == sheet && p.digest == digest)?;
        let file = self.files.iter().find(|f| {
            f.name == page.file
                && f.sheets.len() == 1
                && f.sheets[0] == sheet
                && !f.sha256.is_empty()
        })?;
        let bytes = std::fs::read(self.dir.join(&file.name)).ok()?;
        if sha256_hex(&bytes) != file.sha256 {
            return None;
        }
        let mut page = page.clone();
        page.file = name.to_owned();
        page.date = page.date.or_else(|| Some(self.report.date.clone()));
        Some(Carried {
            file: SheetPublishedFile {
                name: name.to_owned(),
                sheets: vec![sheet.to_owned()],
                bytes: bytes.len() as u64,
                sha256: file.sha256.clone(),
            },
            bytes,
            page,
            issues: self
                .issues
                .iter()
                .filter(|i| i.sheet == sheet && !i.error)
                .cloned()
                .collect(),
        })
    }

    /// A warning when `sheet`, made again because its page changed, still has the revision its earlier page was
    /// delivered with: a changed sheet under an unchanged revision reads as the delivered one (ISO 7200's revision
    /// index; docs/adr/1320 §3).
    pub(crate) fn revision_note(
        &self,
        sheet: &str,
        digest: &str,
        now: Option<&SheetPublishScope>,
    ) -> Option<SheetPublishIssue> {
        let page = self.pages.iter().find(|p| p.sheet == sheet)?;
        if page.digest.is_empty() || page.digest == digest {
            return None;
        }
        let was = self.scope.iter().find(|s| s.sheet == sheet)?;
        let now = now?;
        (was.revision == now.revision).then(|| {
            issue(
                sheet,
                "revision_unchanged",
                if now.revision.is_empty() {
                    format!("“{}” paftası önceki teslimden sonra değişti ve revizyonu yok.", now.name)
                } else {
                    format!(
                        "“{}” paftası önceki teslimden sonra değişti ama revizyonu önceki teslimdekiyle aynı (“{}”).",
                        now.name, now.revision
                    )
                },
                false,
                "Pafta bilgisinde revizyonu ilerletin; değişiklik bu teslime ait değilse paftayı seçimden çıkarın.".into(),
            )
        })
    }
}

/// The page source record of `yayin.json` (docs/adr/1320 §2): each page's source objects by persistent id (null when
/// they cannot be listed), every listed object's content digest, each sheet record's digest and the project parts'.
pub(crate) fn source_record(
    doc: &Document,
    book: &SheetBook,
    chosen: &[String],
) -> serde_json::Value {
    let Ok(pages) = sheets_sources::page_sources(doc, chosen) else {
        return serde_json::Value::Null;
    };
    let objects =
        sheets_sources::object_digests(doc, pages.iter().flat_map(|(_, u)| u.iter().flatten()));
    let sheets: serde_json::Map<String, serde_json::Value> = chosen
        .iter()
        .filter_map(|id| Some((id.clone(), sheets_sources::sheet_digest(book, id)?.into())))
        .collect();
    serde_json::json!({
        "digest": sheets_sources::OBJECT_DIGEST,
        "pages": pages.iter().map(|(sheet, uids)| serde_json::json!({"sheet": sheet, "objects": uids})).collect::<Vec<_>>(),
        "objects": objects,
        "sheets": sheets,
        "parts": sheets_sources::part_digests(doc, Some(book)),
    })
}

/// The published sources of `ids`' pages against the drawing now (docs/adr/1320 §2), by sheet; a sheet whose
/// published or current sources are not listed has none. `resources` says the linked pictures' versions changed.
pub(crate) fn changes(
    doc: &Document,
    book: &SheetBook,
    record: &serde_json::Value,
    ids: &[String],
    resources: bool,
) -> Vec<(String, SheetSourceChanges)> {
    if record.is_null() || record["digest"] != sheets_sources::OBJECT_DIGEST {
        return Vec::new();
    }
    let Ok(now) = sheets_sources::page_sources(doc, ids) else {
        return Vec::new();
    };
    let was_digest = |u: &str| record["objects"][u].as_str().map(str::to_owned);
    let now_parts = sheets_sources::part_digests(doc, Some(book));
    let mut parts: Vec<String> = now_parts
        .iter()
        .filter(|(k, v)| record["parts"][k.as_str()].as_str() != Some(v.as_str()))
        .map(|(k, _)| k.clone())
        .collect();
    if resources {
        parts.push("resources".into());
    }
    parts.sort();
    // Each object listed now and then digested once, whichever pages show it.
    let digests = sheets_sources::object_digests(
        doc,
        now.iter()
            .flat_map(|(_, u)| u.iter().flatten())
            .filter(|u| !record["objects"][u.as_str()].is_null())
            .collect::<std::collections::BTreeSet<_>>(),
    );
    let mut out = Vec::new();
    for (sheet, listed) in now {
        let Some(listed) = listed else { continue };
        let Some(published) = record["pages"]
            .as_array()
            .and_then(|a| a.iter().find(|p| p["sheet"] == sheet.as_str()))
            .and_then(|p| p["objects"].as_array())
        else {
            continue;
        };
        let published: Vec<String> = published
            .iter()
            .filter_map(|u| u.as_str().map(str::to_owned))
            .collect();
        let was: HashSet<&str> = published.iter().map(String::as_str).collect();
        let is: HashSet<&str> = listed.iter().map(String::as_str).collect();
        let mut c = SheetSourceChanges {
            sheet: sheets_sources::sheet_digest(book, &sheet).as_deref()
                != record["sheets"][sheet.as_str()].as_str(),
            parts: parts.clone(),
            ..Default::default()
        };
        for u in &listed {
            if !was.contains(u.as_str()) {
                c.added.push(u.clone());
            } else if digests.get(u).map(String::as_str) != was_digest(u).as_deref() {
                c.changed.push(u.clone());
            }
        }
        c.removed = published
            .iter()
            .filter(|u| !is.contains(u.as_str()))
            .cloned()
            .collect();
        out.push((sheet, c));
    }
    out
}
