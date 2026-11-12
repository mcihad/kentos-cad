//! `cad.joins.query` v1 (docs/adr/0527): a join view's rows read from the drawing as it is, a page at a time,
//! writing nothing. The shared cases are `fixtures/commands/v1/cad.joins.query.json`.
//!
//! The rule: the main layer's records in document order, each with the joined layer's records whose key meets
//! its own by Anahtarla birleştir's rule (`ops::statistics::join_key`: the spaces round it dropped, `007` and `7`
//! alike), in document order; a multiple text field's codes each a key. A record with several matches is several
//! rows, each `left/right` of their persistent ids; one without a match is one row `left/` (dropped when the view
//! keeps only the matched). A record without a key never matches. A view whose layer or key is gone is broken: no
//! rows, said why. The sums count each record once however many rows it is in.
//!
//! The joined records may be a source's table (docs/adr/0632 §1): read whole through its provider whenever the view
//! is (`sources_query::stage`, at most a million records), with their lasting ids; their cells are read only, and a
//! table that cannot be read breaks the view: no copy of a former reading. Summing columns (§2) give each record the
//! count, exact sum, least, greatest and mean of its joined records' values, after the joined fields.

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    CommandResult, Entity, JOIN_ROWS, JoinColumn, JoinHow, JoinKeep, JoinReport, JoinRow, JoinSide,
    JoinTotal, JoinView, JoinsAnswer, JoinsQuery, LayerField, LayerNode, LayerNodeType,
    MOST_JOIN_ROWS, UID_KEY, join_view_problem,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::ops::statistics::{figures, join_key};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A view's key on a layer: the persistent id, or an attribute (a typed field's name) and whether it holds
/// several codes.
#[derive(Clone)]
enum Key {
    Uid,
    Attr { name: String, many: bool },
}

/// The layer `id` names, if it is a layer.
fn layer<'d>(doc: &'d Document, id: &str) -> Option<&'d LayerNode> {
    doc.layers()
        .get(id)
        .filter(|n| n.kind == LayerNodeType::Layer)
}

/// `key` on `node`: `$uid`, a typed field by its key, or (a layer without typed fields) an attribute's name;
/// none when the layer has typed fields and none of them is it.
fn key_on(node: &LayerNode, key: &str) -> Option<Key> {
    if key == UID_KEY {
        return Some(Key::Uid);
    }
    if node.fields.is_empty() {
        return Some(Key::Attr {
            name: key.to_owned(),
            many: false,
        });
    }
    node.fields
        .iter()
        .find(|f| f.key() == key)
        .map(|f| Key::Attr {
            name: f.name.clone(),
            many: f.holds_many(),
        })
}

/// The keys a record holds by `key`, canonical.
fn keys_of(key: &Key, uid: Uuid, e: &Entity) -> Vec<String> {
    match key {
        Key::Uid => join_key(&uid.to_string()).into_iter().collect(),
        Key::Attr { name, many } => {
            let Some(v) = e.base().attrs.get(name) else {
                return Vec::new();
            };
            if *many {
                let mut out: Vec<String> = Vec::new();
                for code in v.split(';') {
                    if let Some(k) = join_key(code)
                        && !out.contains(&k)
                    {
                        out.push(k);
                    }
                }
                out
            } else {
                join_key(v).into_iter().collect()
            }
        }
    }
}

/// A layer's columns: its typed fields in order (heading the label), else its records' attributes by name.
fn columns_of<'d>(
    doc: &'d Document,
    node: &'d LayerNode,
    only: &[String],
) -> Vec<(String, String, Option<&'d LayerField>)> {
    if !node.fields.is_empty() {
        let pick: Vec<&LayerField> = if only.is_empty() {
            node.fields.iter().collect()
        } else {
            only.iter()
                .filter_map(|k| node.fields.iter().find(|f| f.key() == k))
                .collect()
        };
        return pick
            .into_iter()
            .map(|f| (f.label().to_owned(), f.name.clone(), Some(f)))
            .collect();
    }
    let mut names: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for e in doc.by_layer(&node.id) {
        for k in e.base().attrs.keys() {
            if seen.insert(k.as_str()) {
                names.push(k.clone());
            }
        }
    }
    names.sort();
    if !only.is_empty() {
        names.retain(|n| only.contains(n));
    }
    names.into_iter().map(|n| (n.clone(), n, None)).collect()
}

/// A view read whole: its columns, every row, how the records met and the sums; or why it is broken.
pub struct JoinRead {
    pub broken: Option<String>,
    /// The code of the refusal of a source's table that could not be read (docs/adr/0740 §1: what helps it is
    /// `sources::trouble::Trouble::of`'s); none when the view is read or broken for another reason.
    pub refused: Option<String>,
    pub columns: Vec<JoinColumn>,
    pub rows: Vec<JoinRow>,
    pub report: JoinReport,
    pub totals: Vec<JoinTotal>,
}

fn broken(why: String) -> JoinRead {
    JoinRead {
        broken: Some(why),
        refused: None,
        columns: Vec::new(),
        rows: Vec::new(),
        report: JoinReport::default(),
        totals: Vec::new(),
    }
}

/// Why the view cannot be read in `doc` now; none when it can (a source's table is read with the rows).
pub fn broken_reason(doc: &Document, v: &JoinView) -> Option<String> {
    let mut sides = vec![(&v.layer, &v.key, "katmanı")];
    if v.source.is_none() {
        sides.push((&v.with, &v.with_key, "birleşen katmanı"));
    }
    for (id, key, what) in sides {
        let Some(node) = layer(doc, id) else {
            return Some(format!(
                "Görünümün {what} “{id}” çizimde yok ya da bir grup; görünüm okunamıyor."
            ));
        };
        if key_on(node, key).is_none() {
            return Some(format!(
                "“{}” katmanında görünümün anahtarı “{key}” alanı yok; görünüm okunamıyor.",
                node.name
            ));
        }
    }
    None
}

/// A source's table read for a view (docs/adr/0632 §1): its records with their lasting ids, its columns, its version.
struct Staged {
    objects: Vec<(Uuid, Entity)>,
    fields: Vec<LayerField>,
    stamp: Option<String>,
}

/// The source's table read whole, or why it cannot be.
/// The source's table read, or why not: the refusal's code (none when stopped) and the words.
fn stage_source(
    doc: &Document,
    src: &kentos_commands::JoinSource,
) -> Result<Staged, (Option<String>, String)> {
    let from = crate::sources_query::StageFrom::Table {
        address: &src.path,
        table: &src.table,
    };
    match crate::sources_query::stage(doc.settings(), &from, None, None, None, "source", &|| false)
    {
        Ok((staging, answer)) => Ok(Staged {
            objects: staging.objects,
            fields: staging.fields.unwrap_or_default(),
            stamp: answer.stamp,
        }),
        Err(Some(e)) => Err((
            Some(e.code),
            format!(
                "Görünümün kaynak tablosu “{}” okunamadı: {} Görünüm satır vermez; eski bir okuma gösterilmez.",
                src.table, e.message
            ),
        )),
        Err(None) => Err((None, "Kaynak tablosunun okunuşu durduruldu.".into())),
    }
}

/// The joined side read: its name, key, columns, records, why its cells are not written, its layer.
struct Side<'d> {
    name: String,
    key: Key,
    columns: Vec<(String, String, Option<&'d LayerField>)>,
    records: Vec<(Uuid, &'d Entity)>,
    fixed: Option<String>,
    node: Option<&'d LayerNode>,
}

/// The view `v` read from `doc` as it is now.
pub fn read(doc: &Document, v: &JoinView) -> JoinRead {
    if let Some(why) = broken_reason(doc, v) {
        return broken(why);
    }
    let Some(left) = layer(doc, &v.layer) else {
        return broken("Görünümün katmanı yok.".into());
    };
    let Some(lk) = key_on(left, &v.key) else {
        return broken("Görünümün anahtarı yok.".into());
    };
    let staged = match &v.source {
        Some(src) => match stage_source(doc, src) {
            Ok(s) => Some(s),
            Err((code, why)) => {
                return JoinRead {
                    refused: code,
                    ..broken(why)
                };
            }
        },
        None => None,
    };
    // The joined side: a layer's records, or the source's.
    let Side {
        name: right_name,
        key: rk,
        columns: right_cols,
        records: rights,
        fixed,
        node: right_node,
    } = match &staged {
        Some(st) => {
            let src = v.source.as_ref().map_or("", |s| s.table.as_str());
            let key = if v.with_key == UID_KEY {
                Key::Uid
            } else if st.fields.is_empty() || st.fields.iter().any(|f| f.name == v.with_key) {
                Key::Attr {
                    name: v.with_key.clone(),
                    many: false,
                }
            } else {
                return broken(format!(
                    "Kaynak tablosu “{src}” içinde görünümün anahtarı “{}” sütunu yok; görünüm okunamıyor.",
                    v.with_key
                ));
            };
            let cols: Vec<(String, String, Option<&LayerField>)> = if v.fields.is_empty() {
                st.fields
                    .iter()
                    .map(|f| (f.label().to_owned(), f.name.clone(), Some(f)))
                    .collect()
            } else {
                v.fields
                    .iter()
                    .filter_map(|k| st.fields.iter().find(|f| f.key() == k || f.name == *k))
                    .map(|f| (f.label().to_owned(), f.name.clone(), Some(f)))
                    .collect()
            };
            Side {
                name: src.to_owned(),
                key,
                columns: cols,
                records: st.objects.iter().map(|(u, e)| (*u, e)).collect(),
                fixed: Some(format!(
                    "Kaynağın kaydı (“{src}”): görünümden yazılmaz; tabloyu katman olarak bağlayıp orada düzenleyin."
                )),
                node: None,
            }
        }
        None => {
            let Some(right) = layer(doc, &v.with) else {
                return broken("Görünümün katmanı yok.".into());
            };
            let Some(rk) = key_on(right, &v.with_key) else {
                return broken("Görünümün anahtarı yok.".into());
            };
            Side {
                name: right.name.clone(),
                key: rk,
                columns: columns_of(doc, right, &v.fields),
                records: doc.by_layer_with_uids(&right.id).collect(),
                fixed: None,
                node: Some(right),
            }
        }
    };
    // Columns: the main layer's, then the joined layer's (a heading the main one has too with its layer's name).
    let left_cols = columns_of(doc, left, &[]);
    let key_attr = |k: &Key| match k {
        Key::Uid => None,
        Key::Attr { name, .. } => Some(name.clone()),
    };
    let (lkey, rkey) = (key_attr(&lk), key_attr(&rk));
    let locked = |node: &LayerNode| doc.layers().is_locked(&node.id);
    let column = |name: String,
                  side,
                  field: String,
                  key: &Option<String>,
                  node: Option<&LayerNode>,
                  fixed: &Option<String>| {
        let reason = if let Some(why) = fixed {
            Some(why.clone())
        } else if key.as_deref() == Some(field.as_str()) {
            Some("Birleştirmenin anahtarı; değişince satır başka kayda bağlanır.".to_owned())
        } else {
            node.filter(|n| locked(n))
                .map(|node| format!("“{}” katmanı kilitli.", node.name))
        };
        JoinColumn {
            name,
            side,
            field,
            editable: reason.is_none(),
            reason,
        }
    };
    let mut columns: Vec<JoinColumn> = left_cols
        .iter()
        .map(|(n, f, _)| {
            column(
                n.clone(),
                JoinSide::Left,
                f.clone(),
                &lkey,
                Some(left),
                &None,
            )
        })
        .collect();
    let left_names: HashSet<String> = left_cols.iter().map(|(n, ..)| n.clone()).collect();
    for (n, f, _) in &right_cols {
        let name = if left_names.contains(n) {
            format!("{n} ({right_name})")
        } else {
            n.clone()
        };
        columns.push(column(
            name,
            JoinSide::Right,
            f.clone(),
            &rkey,
            right_node,
            &fixed,
        ));
    }
    // The summing columns (docs/adr/0632 §2), after the joined fields: read only.
    for a in &v.aggregates {
        columns.push(JoinColumn {
            name: a.name.clone(),
            side: JoinSide::Aggregate,
            field: a.name.clone(),
            editable: false,
            reason: Some(
                "Toplama sütunu: birleşen kayıtlardan her okuyuşta hesaplanır.".to_owned(),
            ),
        });
    }
    let aggregate_fields: Vec<Option<String>> = v
        .aggregates
        .iter()
        .map(|a| {
            a.field.as_ref().map(|k| {
                right_cols
                    .iter()
                    .find(|(_, name, f)| f.is_some_and(|f| f.key() == k) || name == k)
                    .map_or_else(
                        || {
                            let fields: &[LayerField] = match (&staged, right_node) {
                                (Some(st), _) => &st.fields,
                                (None, Some(n)) => &n.fields,
                                (None, None) => &[],
                            };
                            fields
                                .iter()
                                .find(|f| f.key() == k)
                                .map_or_else(|| k.clone(), |f| f.name.clone())
                        },
                        |(_, name, _)| name.clone(),
                    )
            })
        })
        .collect();
    let summed = |found: &[usize]| -> Vec<Option<String>> {
        v.aggregates
            .iter()
            .zip(&aggregate_fields)
            .map(|(a, field)| {
                let values: Vec<Option<&str>> = match field {
                    Some(f) => found
                        .iter()
                        .map(|i| rights[*i].1.base().attrs.get(f).map(String::as_str))
                        .collect(),
                    None => Vec::new(),
                };
                match (a.how, field) {
                    (JoinHow::Count, None) => Some(found.len().to_string()),
                    (JoinHow::Count, Some(_)) => Some(
                        values
                            .iter()
                            .flatten()
                            .filter(|v| !v.trim().is_empty())
                            .count()
                            .to_string(),
                    ),
                    (how, _) => figures(&values, None).ok().and_then(|f| {
                        match how {
                            JoinHow::Sum => f.sum,
                            JoinHow::Min => f.min,
                            JoinHow::Max => f.max,
                            JoinHow::Mean => f.mean,
                            JoinHow::Count => None,
                        }
                        .map(|d| d.text())
                    }),
                }
            })
            .collect()
    };
    // The joined records by key, in document order.
    let mut by_key: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, (uid, e)) in rights.iter().enumerate() {
        for k in keys_of(&rk, *uid, e) {
            by_key.entry(k).or_default().push(i);
        }
    }
    let cells = |e: Option<&Entity>, cols: &[(String, String, Option<&LayerField>)]| {
        cols.iter()
            .map(|(_, f, _)| e.and_then(|e| e.base().attrs.get(f).cloned()))
            .collect::<Vec<_>>()
    };
    let mut report = JoinReport {
        with_records: rights.len() as u32,
        source_stamp: staged.as_ref().and_then(|s| s.stamp.clone()),
        ..JoinReport::default()
    };
    let mut rows = Vec::new();
    // How many records met each joined record (an index, not a search: a view has hundreds of thousands).
    let mut used: Vec<u32> = vec![0; rights.len()];
    let mut fan = false;
    let mut lefts_in: Vec<&Entity> = Vec::new();
    for (uid, e) in doc.by_layer_with_uids(&left.id) {
        report.records += 1;
        let keys = keys_of(&lk, uid, e);
        if keys.is_empty() {
            report.null_keys += 1;
        }
        let mut found: Vec<usize> = keys
            .iter()
            .filter_map(|k| by_key.get(k))
            .flatten()
            .copied()
            .collect();
        found.sort_unstable();
        found.dedup();
        if found.is_empty() {
            report.unmatched += 1;
            if v.keep == JoinKeep::Matched {
                continue;
            }
            let mut c = cells(Some(e), &left_cols);
            c.extend(cells(None, &right_cols));
            c.extend(summed(&[]));
            rows.push(JoinRow {
                id: format!("{uid}/"),
                left: uid.to_string(),
                right: None,
                cells: c,
            });
            lefts_in.push(e);
            continue;
        }
        report.matched += 1;
        fan |= found.len() > 1;
        lefts_in.push(e);
        let sums = summed(&found);
        for i in found {
            used[i] += 1;
            let (ruid, re) = rights[i];
            let mut c = cells(Some(e), &left_cols);
            c.extend(cells(Some(re), &right_cols));
            c.extend(sums.iter().cloned());
            rows.push(JoinRow {
                id: format!("{uid}/{ruid}"),
                left: uid.to_string(),
                right: Some(ruid.to_string()),
                cells: c,
            });
        }
    }
    let rights_in: Vec<usize> = (0..rights.len()).filter(|&i| used[i] > 0).collect();
    report.with_used = rights_in.len() as u32;
    report.rows = rows.len() as u32;
    let shared = used.iter().any(|n| *n > 1);
    report.cardinality = if rights_in.is_empty() {
        String::new()
    } else {
        match (fan, shared) {
            (false, false) => "1:1",
            (true, false) => "1:N",
            (false, true) => "N:1",
            (true, true) => "N:N",
        }
        .to_owned()
    };
    // The sums: each record of the rows once.
    let mut totals = Vec::new();
    for (k, c) in columns.iter().enumerate() {
        let values: Vec<Option<&str>> = match c.side {
            JoinSide::Left => lefts_in
                .iter()
                .map(|e| e.base().attrs.get(&c.field).map(String::as_str))
                .collect(),
            JoinSide::Right => rights_in
                .iter()
                .map(|i| rights[*i].1.base().attrs.get(&c.field).map(String::as_str))
                .collect(),
            // A summing column: each record's value once (its first row's).
            JoinSide::Aggregate => {
                let mut seen: HashSet<&str> = HashSet::new();
                rows.iter()
                    .filter(|r| seen.insert(r.left.as_str()))
                    .map(|r| r.cells.get(k).and_then(|c| c.as_deref()))
                    .collect()
            }
        };
        match figures(&values, None) {
            Ok(f) if f.read > 0 => totals.push(JoinTotal {
                column: k as u32,
                sum: f.sum.map(|d| d.text()),
                count: f.read as u32,
                skipped: f.skipped as u32,
            }),
            Ok(_) => {}
            Err(_) => totals.push(JoinTotal {
                column: k as u32,
                sum: None,
                count: values.iter().flatten().count() as u32,
                skipped: 0,
            }),
        }
    }
    JoinRead {
        broken: None,
        refused: None,
        columns,
        rows,
        report,
        totals,
    }
}

/// The view the input names: a kept one, or its definition.
/// The view's rows as a sheet to write (Dışa aktar, docs/adr/0632 §3): every column with its heading made unique,
/// the summing columns numbers, the others their words; empty cells empty.
pub fn sheet(r: &JoinRead, name: &str) -> kentos_commands::WorkbookSheet {
    let mut taken: HashSet<String> = HashSet::new();
    let columns = r
        .columns
        .iter()
        .map(|c| {
            let mut heading = c.name.clone();
            let mut n = 2;
            while !taken.insert(heading.clone()) {
                heading = format!("{} {n}", c.name);
                n += 1;
            }
            kentos_commands::WorkbookColumn {
                name: heading,
                kind: if c.side == JoinSide::Aggregate {
                    kentos_commands::CellKind::Number
                } else {
                    kentos_commands::CellKind::Text
                },
                decimals: None,
            }
        })
        .collect();
    let rows = r
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|c| c.clone().unwrap_or_default())
                .collect()
        })
        .collect();
    kentos_commands::WorkbookSheet {
        name: name.to_owned(),
        columns,
        rows,
    }
}

fn view_of(doc: &Document, input: &JoinsQuery) -> Result<JoinView, Stop> {
    match (&input.view, &input.definition) {
        (Some(id), None) => doc
            .settings()
            .join_views
            .iter()
            .find(|v| v.id == *id)
            .cloned()
            .ok_or_else(|| {
                fail(
                    codes::UNKNOWN_JOIN_VIEW,
                    format!("“{id}” kimlikli bir birleşik görünüm yok. Var olan bir görünümün kimliğini verin."),
                    "view",
                )
            }),
        (None, Some(v)) => match join_view_problem(v) {
            Some(p) => Err(fail(
                codes::INVALID_JOIN_VIEW,
                format!("Görünüm kurallarına uymuyor: {p}"),
                "definition",
            )),
            None => Ok(v.clone()),
        },
        _ => Err(fail(
            codes::INVALID_JOIN_VIEW,
            "Ya kayıtlı bir görünümün kimliğini (view) ya da bir görünüm tanımını (definition) verin; ikisini birden değil."
                .into(),
            "view",
        )),
    }
}

fn check(doc: &Document, input: &JoinsQuery) -> Result<(JoinView, usize, usize), Stop> {
    let v = view_of(doc, input)?;
    let limit = input.limit.unwrap_or(JOIN_ROWS);
    if limit == 0 || limit > MOST_JOIN_ROWS {
        return Err(fail(
            codes::INVALID_COUNT,
            format!("Bir sayfada 1 ile {MOST_JOIN_ROWS} arasında satır verilir; {limit} verildi."),
            "limit",
        ));
    }
    let offset = match &input.cursor {
        None => 0,
        Some(c) => c
            .parse::<usize>()
            .ok()
            .filter(|n| c == &n.to_string())
            .ok_or_else(|| {
                fail(
                    codes::INVALID_CURSOR,
                    format!(
                        "“{c}” bu görünümün sayfa imleci değil; bir önceki cevabın next'ini verin."
                    ),
                    "cursor",
                )
            })?,
    };
    Ok((v, offset, limit as usize))
}

fn answer(doc: &Document, input: &JoinsQuery) -> CommandResult<JoinsAnswer> {
    let (v, offset, limit) = match check(doc, input) {
        Ok(x) => x,
        Err(stop) => return stop.into(),
    };
    let read = read(doc, &v);
    let total = read.rows.len();
    if offset > total {
        return fail(
            codes::INVALID_CURSOR,
            format!("“{offset}” bu görünümün sayfa imleci değil: görünüm {total} satır; bir önceki cevabın next'ini verin."),
            "cursor",
        )
        .into();
    }
    let end = (offset + limit).min(total);
    let mut rows = read.rows;
    let page: Vec<JoinRow> = rows.drain(offset..end).collect();
    CommandResult::Completed {
        output: JoinsAnswer {
            name: v.name,
            broken: read.broken,
            columns: read.columns,
            rows: page,
            next: (end < total).then(|| end.to_string()),
            report: read.report,
            totals: read.totals,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &JoinsQuery) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The page; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &JoinsQuery) -> CommandResult<JoinsAnswer> {
    answer(cx.doc, input)
}

/// The page. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: JoinsQuery) -> CommandResult<JoinsAnswer> {
    answer(cx.doc, &input)
}
