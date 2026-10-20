//! `cad.records.page` v1 (docs/adr/0532 §3): a layer's records, or the records a project relation ties to one record,
//! a page at a time, after the project's layer filters, with the version of the source the layer is bound to. The
//! records are in lasting-id order (a relation's parents, in the order of the reference), the cursor is the id the
//! previous page ended with, `total` counts what matches over every page; `expectedRevision` and `expectedSource`
//! make the pages after the first the first's own data or a conflict. The shared cases are
//! `fixtures/commands/v1/cad.records.page.json`.

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    CommandResult, Entity, MOST_PAGED, PAGED, PagedRecord, PagedSource, ProjectRelation,
    RecordsPage, RecordsPaged, RelationSide,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The ids a reference field holds: several joined by `;` in a many-to-many one, one otherwise.
fn ids(value: &str, many: bool) -> Vec<String> {
    if many {
        let mut seen = HashSet::new();
        value
            .split(';')
            .map(str::trim)
            .filter(|h| !h.is_empty() && seen.insert(h.to_owned()))
            .map(str::to_owned)
            .collect()
    } else {
        let v = value.trim();
        if v.is_empty() {
            Vec::new()
        } else {
            vec![v.to_owned()]
        }
    }
}

fn relation<'a>(doc: &'a Document, id: &str) -> Result<&'a ProjectRelation, Stop> {
    doc.settings()
        .relations
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| {
            fail(
                codes::UNKNOWN_RELATION,
                format!("“{id}” kimlikli bir ilişki yok; cad.schema.describe ilişkileri listeler."),
                "relation",
            )
        })
}

fn layer_of<'a>(
    doc: &'a Document,
    id: &str,
    path: &str,
) -> Result<&'a kentos_commands::LayerNode, Stop> {
    doc.layers()
        .get(id)
        .filter(|n| n.kind == kentos_commands::LayerNodeType::Layer)
        .ok_or_else(|| {
            fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                path,
            )
        })
}

/// The page asked for: its records and where the listing stands.
fn run(doc: &Document, input: &RecordsPage) -> Result<RecordsPaged, Stop> {
    let limit = input.limit.unwrap_or(PAGED);
    if limit == 0 || limit > MOST_PAGED {
        return Err(fail(
            codes::INVALID_COUNT,
            format!("Bir sayfada 1 ile {MOST_PAGED} arasında kayıt okunur; {limit} verildi."),
            "limit",
        ));
    }
    let cursor = match &input.cursor {
        None => None,
        Some(c) => Some(Uuid::parse_str(c).ok().filter(|u| u.to_string() == *c).ok_or_else(|| {
            fail(
                codes::INVALID_CURSOR,
                format!("“{c}” bu listenin sayfa imleci değil; bir önceki cevabın next'ini verin."),
                "cursor",
            )
        })?),
    };
    if input.layer.is_some() == input.relation.is_some() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Katmanı (layer) ya da bir ilişkiyi (relation ve of) verin; ikisi birden ya da hiçbiri olmaz.".into(),
            "layer",
        ));
    }
    if input.relation.is_none() && (input.of.is_some() || input.side.is_some()) {
        return Err(fail(
            codes::INVALID_INPUT,
            "of ve side ilişkiyle (relation) birlikte verilir.".into(),
            "of",
        ));
    }
    // The ordered list of the records the page is cut from.
    let mut list: Vec<(Uuid, &Entity)>;
    let layer_id: String;
    if let Some(id) = &input.layer {
        layer_of(doc, id, "layer")?;
        layer_id = id.clone();
        list = doc.by_layer_with_uids(id).collect();
        list.sort_by_key(|(u, _)| *u);
    } else {
        let r = relation(doc, input.relation.as_deref().unwrap_or_default())?;
        let Some(of_text) = &input.of else {
            return Err(fail(
                codes::INVALID_INPUT,
                "İlişki okunurken of (kaydın kimliği) verilmeli.".into(),
                "of",
            ));
        };
        let of = Uuid::parse_str(of_text)
            .ok()
            .filter(|u| u.to_string() == *of_text)
            .ok_or_else(|| {
                fail(
                    codes::INVALID_INPUT,
                    format!("“{of_text}” bir kalıcı kimlik değil (küçük harfli, tireli UUID)."),
                    "of",
                )
            })?;
        let side = input.side.unwrap_or_default();
        let (here, there) = match side {
            RelationSide::Children => (&r.parent, &r.child),
            RelationSide::Parents => (&r.child, &r.parent),
        };
        let Some(slot) = doc.slot_of(of) else {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!("{of} kimlikli nesne çizimde yok."),
                "of",
            ));
        };
        let object = doc.get(slot).ok_or_else(|| {
            fail(
                codes::ENTITY_NOT_FOUND,
                format!("{of} kimlikli nesne çizimde yok."),
                "of",
            )
        })?;
        if object.base().layer_id != *here {
            return Err(fail(
                codes::FORM_LAYER_MISMATCH,
                format!(
                    "{of} nesnesi “{}” katmanında; “{}” ilişkisinde {} tarafı “{here}” katmanıdır.",
                    object.base().layer_id,
                    r.name,
                    match side {
                        RelationSide::Children => "üst",
                        RelationSide::Parents => "alt",
                    }
                ),
                "of",
            ));
        }
        layer_id = there.clone();
        let field = layer_of(doc, &r.child, "relation")?
            .fields
            .iter()
            .find(|f| f.key() == r.field);
        let (name, many) = field.map_or((r.field.clone(), false), |f| (f.name.clone(), f.multiple));
        match side {
            RelationSide::Children => {
                list =
                    doc.by_layer_with_uids(&r.child)
                        .filter(|(_, e)| {
                            e.base().attrs.get(&name).is_some_and(|v| {
                                ids(v, many).iter().any(|h| *h == of_text.as_str())
                            })
                        })
                        .collect();
                list.sort_by_key(|(u, _)| *u);
            }
            RelationSide::Parents => {
                list = object
                    .base()
                    .attrs
                    .get(&name)
                    .map(|v| ids(v, many))
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|h| Uuid::parse_str(h).ok())
                    .filter_map(|u| {
                        let slot = doc.slot_of(u)?;
                        let e = doc.get(slot)?;
                        (e.base().layer_id == r.parent).then_some((u, e))
                    })
                    .collect();
            }
        }
    }
    // The fields to return exist.
    let layer = layer_of(doc, &layer_id, "layer")?;
    for (i, f) in input.fields.iter().enumerate() {
        if !layer.fields.is_empty() && !layer.fields.iter().any(|lf| lf.name == *f) {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{f}” alanı “{}” katmanında yok.", layer.name),
                &format!("fields.{i}"),
            ));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let source = doc
        .settings()
        .sources
        .iter()
        .find(|s| s.layer == layer_id)
        .map(|s| PagedSource {
            id: s.id.clone(),
            kind: s.kind,
            table: s.table.clone(),
            revision: s.revision.clone(),
            edited: crate::sources::layer_digest(doc, &s.layer) != s.digest,
        });
    if let (Some(expected), Some(now)) = (&input.expected_source, &source)
        && *expected != now.revision
    {
        return Err(Stop::conflict(error(
            codes::SOURCE_CHANGED,
            format!(
                "“{}” katmanının kaynağı ilk sayfadan sonra değişti ({}…); sonraki sayfa aynı verinin sayfası olmaz. Listeyi baştan okuyun.",
                layer_id,
                now.revision.chars().take(12).collect::<String>()
            ),
            Some("expectedSource".to_owned()),
        )));
    }
    // What the layer filters leave out is neither a record nor in the total.
    let out: HashSet<Slot> = crate::layer_filter::left_out(doc);
    let list: Vec<(Uuid, &Entity)> = list
        .into_iter()
        .filter(|(u, _)| doc.slot_of(*u).is_some_and(|s| !out.contains(&s)))
        .collect();
    let words: Vec<String> = input
        .query
        .as_deref()
        .map(|q| {
            kentos_commands::fields::fold(q)
                .split_whitespace()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let list: Vec<(Uuid, &Entity)> = list
        .into_iter()
        .filter(|(_, e)| {
            words.is_empty() || {
                let text = kentos_commands::fields::fold(
                    &e.base()
                        .attrs
                        .values()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                words.iter().all(|w| text.contains(w.as_str()))
            }
        })
        .collect();
    let total = list.len() as u32;
    let start = match cursor {
        None => 0,
        Some(c) => list
            .iter()
            .position(|(u, _)| *u == c)
            .map(|p| p + 1)
            .ok_or_else(|| {
                fail(
                    codes::INVALID_CURSOR,
                    format!("“{c}” bu listede yok; bir önceki cevabın next'ini verin."),
                    "cursor",
                )
            })?,
    };
    let page: Vec<&(Uuid, &Entity)> = list.iter().skip(start).take(limit as usize).collect();
    let next = (start + page.len() < list.len())
        .then(|| page.last().map(|(u, _)| u.to_string()))
        .flatten();
    let records = page
        .iter()
        .map(|(u, e)| {
            let attrs: BTreeMap<String, String> = e
                .base()
                .attrs
                .iter()
                .filter(|(k, _)| input.fields.is_empty() || input.fields.contains(k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            PagedRecord {
                uid: u.to_string(),
                label: e.base().label.clone(),
                attrs,
            }
        })
        .collect();
    Ok(RecordsPaged {
        layer: layer_id,
        total,
        records,
        next,
        source,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &RecordsPage) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &RecordsPage) -> CommandResult<RecordsPaged> {
    match run(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: RecordsPage) -> CommandResult<RecordsPaged> {
    plan(cx, &input)
}
