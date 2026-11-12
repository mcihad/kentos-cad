//! A file's rows meeting the objects of the layer they go onto (docs/adr/0628 §5, §6): matched by a key column with
//! Anahtarla birleştir's rule (`ops::statistics::join_key`) or by their lasting ids; a matched object takes its row's
//! attributes as `matched` says (and its geometry when asked), the others are added or left; a key the source
//! repeats is handled as `duplicates` says. A row that cannot go in is refused with its reason — its key empty or
//! repeated, its key on more than one object, a value its field does not take, a geometry its layer or its object does
//! not take — and the import goes on with the rest; the rows refused are written to a CSV to mend and load again.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::identity::uuid_v5;
use kentos_commands::{
    Entity, ImportMerge, LayerField, MergeDuplicates, MergeMatched, MergeReport, MergeUnmatched,
    RejectsWritten, check_value,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::statistics::join_key;

use crate::checks::{Stop, error};
use crate::codes;

/// A row refused.
#[derive(Clone, Debug)]
pub struct Rejected {
    pub table: String,
    pub row: i64,
    pub key: String,
    pub reason: &'static str,
    /// The reason in words, with what was wrong.
    pub detail: String,
    pub entity: Entity,
}

/// What the merge makes of one table's rows on one layer.
#[derive(Debug, Default)]
pub struct Merged {
    /// The rows to add, with their lasting ids (a fresh one where the row's is taken).
    pub add: Vec<(Uuid, Entity)>,
    /// The objects to change, each as it is to become.
    pub update: Vec<(Slot, Entity)>,
    pub report: Option<MergeReport>,
    pub rejected: Vec<Rejected>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// Which family of geometry a kind is: an object takes a row's geometry of its own family only.
fn family(kind: &str) -> &str {
    match kind {
        "line" | "polyline" | "arc" | "spline" | "nurbs" => "line",
        "polygon" | "circle" | "ellipse" => "area",
        k => k,
    }
}

/// An attribute of `attrs` by name, case aside.
fn attr<'a>(attrs: &'a BTreeMap<String, String>, name: &str) -> Option<&'a String> {
    attrs.get(name).or_else(|| {
        attrs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    })
}

/// The row's attributes checked against the layer's fields: each value as its field writes it, or why not.
fn checked(
    attrs: &BTreeMap<String, String>,
    fields: &[LayerField],
) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for (k, v) in attrs {
        match fields.iter().find(|f| f.name.eq_ignore_ascii_case(k)) {
            Some(f) => {
                let text = check_value(f, v).map_err(|r| r.message)?;
                out.insert(f.name.clone(), text);
            }
            None => {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    Ok(out)
}

/// A lasting id the drawing does not hold yet, made from `uid` and the row's key when `uid` is taken.
fn free_uid(doc: &Document, uid: Uuid, key: &str, taken: &BTreeSet<Uuid>) -> Uuid {
    let mut u = uid;
    let mut n = 0u32;
    while doc.slot_of(u).is_some() || taken.contains(&u) {
        n += 1;
        u = Uuid::from_bytes(uuid_v5(
            uid.as_bytes(),
            format!("kentos-merge:{key}:{n}").as_bytes(),
        ));
    }
    u
}

/// The rows of `table` (each with its lasting id and its row number) met with the objects of `layer`. `columns` are
/// the source's columns: a matched object's attribute a row leaves empty is emptied by `update`. `ids` is whether the
/// rows' lasting ids name them (else a merge without a key is refused).
#[allow(clippy::too_many_arguments)]
pub(crate) fn merge(
    doc: &Document,
    layer: &str,
    table: &str,
    rows: &[(Uuid, Entity)],
    numbers: &[i64],
    columns: &[String],
    m: &ImportMerge,
    ids: bool,
) -> Result<Merged, Stop> {
    let node = doc.layers().get(layer);
    let fields: Vec<LayerField> = node.map(|n| n.fields.clone()).unwrap_or_default();
    let layer_type = node.and_then(|n| n.geometry);
    let target_key = m.target_key.clone().or_else(|| m.key.clone());
    if m.key.is_none() && !ids {
        return Err(refuse(
            codes::MERGE_KEY_REQUIRED,
            format!(
                "“{table}” satırlarının kalıcı kimliği yok; birleştirmek için merge.key ile anahtar sütunu verin."
            ),
            "merge.key",
        ));
    }
    if let Some(k) = &m.key
        && !columns.iter().any(|c| c.eq_ignore_ascii_case(k))
    {
        return Err(refuse(
            codes::MERGE_KEY_NOT_FOUND,
            format!(
                "“{k}” sütunu “{table}” tablosunda yok; sütunlardan birini verin ({}).",
                columns
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            "merge.key",
        ));
    }
    // The layer's objects by their key.
    let mut index: HashMap<String, Vec<Slot>> = HashMap::new();
    if let Some(tk) = &target_key {
        for (_, slot) in doc.layer_slots(layer) {
            if let Some(e) = doc.get(slot)
                && let Some(k) = attr(&e.base().attrs, tk).and_then(|v| join_key(v))
            {
                index.entry(k).or_default().push(slot);
            }
        }
    }
    // The rows' keys, and how many rows give each.
    let keys: Vec<Option<String>> = rows
        .iter()
        .map(|(uid, e)| match &m.key {
            Some(k) => attr(&e.base().attrs, k).and_then(|v| join_key(v)),
            None => Some(uid.to_string()),
        })
        .collect();
    let mut count: HashMap<&str, u32> = HashMap::new();
    for k in keys.iter().flatten() {
        *count.entry(k.as_str()).or_default() += 1;
    }
    if m.duplicates == MergeDuplicates::Stop
        && let Some((i, k)) = keys
            .iter()
            .enumerate()
            .find_map(|(i, k)| k.as_deref().filter(|k| count[k] > 1).map(|k| (i, k)))
    {
        let shown = attr(&rows[i].1.base().attrs, m.key.as_deref().unwrap_or(""))
            .cloned()
            .unwrap_or_else(|| k.to_owned());
        return Err(refuse(
            codes::DUPLICATE_KEY,
            format!(
                "“{table}” tablosunda “{shown}” anahtarı {} satırda; ilkini almak için duplicates: first, hiçbirini almamak için duplicates: none verin.",
                count[k]
            ),
            "merge.duplicates",
        ));
    }
    let mut out = Merged::default();
    let mut report = MergeReport {
        table: table.to_owned(),
        layer: layer.to_owned(),
        added: 0,
        updated: 0,
        unchanged: 0,
        skipped: 0,
        rejected: 0,
        reasons: BTreeMap::new(),
    };
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut taken: BTreeSet<Uuid> = BTreeSet::new();
    let reject = |out: &mut Merged,
                  report: &mut MergeReport,
                  i: usize,
                  reason: &'static str,
                  detail: String| {
        report.rejected += 1;
        *report.reasons.entry(reason.to_owned()).or_default() += 1;
        out.rejected.push(Rejected {
            table: table.to_owned(),
            row: numbers.get(i).copied().unwrap_or(i as i64 + 1),
            key: match &m.key {
                Some(k) => attr(&rows[i].1.base().attrs, k)
                    .cloned()
                    .unwrap_or_default(),
                None => rows[i].0.to_string(),
            },
            reason,
            detail,
            entity: rows[i].1.clone(),
        });
    };
    for (i, (uid, src)) in rows.iter().enumerate() {
        let Some(key) = keys[i].clone() else {
            reject(
                &mut out,
                &mut report,
                i,
                kentos_commands::REJECT_KEY_EMPTY,
                "anahtarı boş".into(),
            );
            continue;
        };
        if count[key.as_str()] > 1 {
            let first = seen.insert(key.clone());
            if m.duplicates == MergeDuplicates::None || !first {
                reject(
                    &mut out,
                    &mut report,
                    i,
                    kentos_commands::REJECT_KEY_DUPLICATE,
                    format!("anahtarı kaynakta {} satırda", count[key.as_str()]),
                );
                continue;
            }
        }
        if let Some(t) = layer_type
            && src.kind() != "record"
            && !t.admits(src.kind())
        {
            reject(
                &mut out,
                &mut report,
                i,
                kentos_commands::REJECT_LAYER_TYPE,
                format!(
                    "{}, {}na konamaz",
                    kentos_commands::kind_words(src.kind()),
                    t.layer_words()
                ),
            );
            continue;
        }
        let attrs = match checked(&src.base().attrs, &fields) {
            Ok(a) => a,
            Err(why) => {
                reject(
                    &mut out,
                    &mut report,
                    i,
                    kentos_commands::REJECT_FIELD_VALUE,
                    why,
                );
                continue;
            }
        };
        // The object the row meets.
        let matched: Option<Slot> = if m.key.is_some() {
            match index.get(&key).map(Vec::as_slice) {
                None | Some([]) => None,
                Some([one]) => Some(*one),
                Some(many) => {
                    reject(
                        &mut out,
                        &mut report,
                        i,
                        kentos_commands::REJECT_TARGET_AMBIGUOUS,
                        format!("anahtarı katmanda {} nesnede", many.len()),
                    );
                    continue;
                }
            }
        } else {
            match doc.slot_of(*uid) {
                Some(slot) if doc.get(slot).is_some_and(|e| e.base().layer_id == layer) => {
                    Some(slot)
                }
                Some(_) => {
                    reject(
                        &mut out,
                        &mut report,
                        i,
                        kentos_commands::REJECT_UID_ELSEWHERE,
                        "kalıcı kimliği başka bir katmanın nesnesinde".into(),
                    );
                    continue;
                }
                None => None,
            }
        };
        let Some(slot) = matched else {
            if m.unmatched == MergeUnmatched::Skip {
                report.skipped += 1;
                continue;
            }
            let mut e = src.clone();
            e.base_mut().attrs = attrs;
            let u = free_uid(doc, *uid, &key, &taken);
            taken.insert(u);
            out.add.push((u, e));
            report.added += 1;
            continue;
        };
        let Some(existing) = doc.get(slot) else {
            continue;
        };
        if m.matched == MergeMatched::Keep {
            report.unchanged += 1;
            continue;
        }
        let mut next = if m.geometry && src.kind() != "record" {
            if family(src.kind()) != family(existing.kind()) {
                reject(
                    &mut out,
                    &mut report,
                    i,
                    kentos_commands::REJECT_GEOMETRY_KIND,
                    format!(
                        "{}, eşleşen {} nesnesinin yerine geçemez",
                        kentos_commands::kind_words(src.kind()),
                        kentos_commands::kind_words(existing.kind())
                    ),
                );
                continue;
            }
            let mut e = src.clone();
            *e.base_mut() = existing.base().clone();
            e
        } else {
            existing.clone()
        };
        let target = &mut next.base_mut().attrs;
        match m.matched {
            MergeMatched::Update => {
                for c in columns {
                    let name = fields
                        .iter()
                        .find(|f| f.name.eq_ignore_ascii_case(c))
                        .map_or_else(|| c.clone(), |f| f.name.clone());
                    match attr(&attrs, c) {
                        Some(v) => {
                            target.insert(name, v.clone());
                        }
                        None => {
                            target.remove(&name);
                        }
                    }
                }
            }
            MergeMatched::FillEmpty => {
                for (k, v) in &attrs {
                    if v.is_empty() {
                        continue;
                    }
                    if attr(target, k).is_none_or(|t| t.trim().is_empty()) {
                        target.insert(k.clone(), v.clone());
                    }
                }
            }
            MergeMatched::Keep => {}
        }
        if &next == existing {
            report.unchanged += 1;
        } else {
            out.update.push((slot, next));
            report.updated += 1;
        }
    }
    out.report = Some(report);
    Ok(out)
}

/// RFC 4180: a field holding a comma, a quote, a line break or spaces at its ends is quoted.
fn quoted(t: &str) -> String {
    if t.contains(',')
        || t.contains('"')
        || t.contains('\n')
        || t.contains('\r')
        || t.starts_with(' ')
        || t.ends_with(' ')
    {
        format!("\"{}\"", t.replace('"', "\"\""))
    } else {
        t.to_owned()
    }
}

/// The rejects file's bytes: `kaynak`, `satir`, `anahtar`, `neden`, the source's columns, `WKT`.
pub fn rejects_csv(rows: &[Rejected], columns: &[String]) -> Vec<u8> {
    let mut out = String::new();
    let head: Vec<String> = ["kaynak", "satir", "anahtar", "neden"]
        .iter()
        .map(|s| (*s).to_owned())
        .chain(columns.iter().cloned())
        .chain(std::iter::once("WKT".to_owned()))
        .collect();
    out.push_str(&head.iter().map(|h| quoted(h)).collect::<Vec<_>>().join(","));
    out.push_str("\r\n");
    for r in rows {
        let attrs = &r.entity.base().attrs;
        let wkt = crate::geopackage::geometry_of(&r.entity)
            .ok()
            .flatten()
            .map(|(g, _)| kentos_geopackage::wkt::write_wkt(&g))
            .unwrap_or_default();
        let cells: Vec<String> = [
            r.table.clone(),
            r.row.to_string(),
            r.key.clone(),
            r.detail.clone(),
        ]
        .into_iter()
        .chain(
            columns
                .iter()
                .map(|c| attr(attrs, c).cloned().unwrap_or_default()),
        )
        .chain(std::iter::once(wkt))
        .collect();
        out.push_str(
            &cells
                .iter()
                .map(|c| quoted(c))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out.into_bytes()
}

/// Checks a rejects path before anything is done: an absolute `.csv` path in a folder that is there.
pub(crate) fn check_rejects_path(path: &Option<String>) -> Result<(), Stop> {
    let Some(p) = path else {
        return Ok(());
    };
    let pp = Path::new(p);
    if p.trim().is_empty() || !pp.is_absolute() || pp.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{p}” reddedilen satırların yazılacağı bir dosyanın mutlak yolu değil; /klasör/reddedilenler.csv gibi verin."
            ),
            "rejects",
        ));
    }
    if !pp
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
    {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” bir .csv dosyası değil; reddedilen satırlar CSV olarak yazılır.",
                pp.file_name()
                    .map_or_else(|| p.clone(), |n| n.to_string_lossy().into_owned())
            ),
            "rejects",
        ));
    }
    if !pp.parent().is_some_and(Path::is_dir) {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” klasörü yok; reddedilen satırlar var olan bir klasöre yazılır.",
                pp.parent().unwrap_or(Path::new("")).display()
            ),
            "rejects",
        ));
    }
    Ok(())
}

/// Writes the rejects file whole (beside it first, then in its place).
pub(crate) fn write_rejects(path: &str, bytes: &[u8], rows: u32) -> Result<RejectsWritten, Stop> {
    let target = Path::new(path);
    let tmp = target.with_extension("csv.kentos-yaziliyor");
    let fail = |e: std::io::Error| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("{path} yazılamadı: {e}. Klasörün yazılabilir olduğunu denetleyin."),
            "rejects",
        )
    };
    std::fs::write(&tmp, bytes).map_err(fail)?;
    std::fs::rename(&tmp, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail(e)
    })?;
    Ok(RejectsWritten {
        path: path.to_owned(),
        rows,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(bytes),
    })
}

/// The warning of the rows refused.
pub fn warning(reports: &[MergeReport], file: &str) -> Option<kentos_commands::CommandWarning> {
    let total: u32 = reports.iter().map(|r| r.rejected).sum();
    if total == 0 {
        return None;
    }
    let mut reasons: BTreeMap<&str, u32> = BTreeMap::new();
    for r in reports {
        for (k, n) in &r.reasons {
            *reasons.entry(k.as_str()).or_default() += n;
        }
    }
    Some(kentos_commands::CommandWarning {
        code: codes::ROWS_REJECTED.into(),
        message: format!(
            "{file}: {total} satır alınmadı ({}).",
            reasons
                .iter()
                .map(|(k, n)| format!("{n} satırın {}", kentos_commands::reject_words(k)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        path: Some("merge".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rejects file the disk does not take (a folder where its part is written) refuses with file_unwritable and
    /// leaves no file: the import's step around it is undone (the shared cases cannot make such a disk).
    #[test]
    fn a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing() {
        let dir = std::env::temp_dir().join(format!("kentos-reddedilen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("r.csv.kentos-yaziliyor")).expect("a folder in the way");
        let path = dir.join("r.csv");
        let error = write_rejects(&path.to_string_lossy(), b"kaynak\r\n", 0)
            .expect_err("a folder is in the way")
            .into_error();
        assert_eq!(error.code, codes::FILE_UNWRITABLE);
        assert_eq!(error.path.as_deref(), Some("rejects"));
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Fields that hold a comma, a quote or a line break are quoted; the lines end with CRLF.
    #[test]
    fn the_rejects_file_quotes_as_rfc_4180_says() {
        assert_eq!(quoted("a,b"), "\"a,b\"");
        assert_eq!(quoted("“x” \"y\""), "\"“x” \"\"y\"\"\"");
        assert_eq!(quoted(" boşluk"), "\" boşluk\"");
        assert_eq!(quoted("düz"), "düz");
        let bytes = rejects_csv(&[], &["ad".to_owned()]);
        assert_eq!(bytes, b"kaynak,satir,anahtar,neden,ad,WKT\r\n");
    }
}
