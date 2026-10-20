//! `cad.sources.reconcile` (docs/adr/0538, plan/gis.md CBS-37): a bound layer and its table, both changed since they
//! last matched, put together object by object against the base (`base`): what only the layer changed is written to
//! the table, what only the table changed is taken into the layer, what both changed alike agrees, and what both
//! changed otherwise (or one removed and the other changed) is a conflict resolved by the side the input takes. The
//! table's side goes into the layer, then the layer is written as `cad.sources.sync` writes it, against the file as it
//! now is; one undo step, one transaction.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_contracts::{
    CommandResult, CommandWarning, ConflictField, ConflictKind, Entity, ReconcileConflict,
    ReconcileCounts, ReconcileSide, SourcesReconcile, SourcesReconciled, SourcesSync,
    SyncDirection,
};
use kentos_domain::{Document, Slot, Uuid};

use super::base::{self, ABSENT};
use super::{Origin, codes, conflict, derived_message, origin_from, refuse, set_link, sync_run};
use crate::ExecutionContext;
use crate::checks::Stop;
use crate::geopackage as gp;

/// What a side did to an object since the base.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Did {
    Nothing,
    Added,
    Changed,
    Removed,
}

fn did(base: Option<u64>, now: Option<u64>) -> Did {
    match (base, now) {
        (None, None) => Did::Nothing,
        (None, Some(_)) => Did::Added,
        (Some(_), None) => Did::Removed,
        (Some(b), Some(n)) if b == n => Did::Nothing,
        (Some(_), Some(_)) => Did::Changed,
    }
}

/// What reconciling does with an object.
enum Way {
    /// The table's state goes into the layer.
    FromTable,
    /// The layer's goes to the table (by the write).
    ToTable,
    /// Both alike.
    Agreed,
    /// A conflict, its index in the answer's list.
    Conflict(usize),
}

/// The undo step of a reconcile: “Uzlaştır: tablo”.
pub fn step(table: &str) -> String {
    format!("Uzlaştır: {table}")
}

/// The attributes two objects hold differently, by name.
fn fields_between(l: &Entity, t: &Entity) -> Vec<ConflictField> {
    let (la, ta) = (&l.base().attrs, &t.base().attrs);
    let names: BTreeSet<&String> = la.keys().chain(ta.keys()).collect();
    names
        .into_iter()
        .filter(|n| la.get(*n) != ta.get(*n))
        .map(|n| ConflictField {
            name: n.clone(),
            layer: la.get(n).cloned(),
            table: ta.get(n).cloned(),
        })
        .collect()
}

/// The attributes of an object one side removed, as the side holding it has them.
fn fields_of_one(e: &Entity, side: ReconcileSide) -> Vec<ConflictField> {
    e.base()
        .attrs
        .iter()
        .map(|(n, v)| ConflictField {
            name: n.clone(),
            layer: (side == ReconcileSide::Layer).then(|| v.clone()),
            table: (side == ReconcileSide::Table).then(|| v.clone()),
        })
        .collect()
}

/// Whether two objects' kinds or geometries differ (their essences without the attributes).
fn shapes_differ(l: &Entity, t: &Entity) -> bool {
    let strip = |e: &Entity| {
        let mut v = gp::essence(e);
        if let Some(m) = v.as_object_mut() {
            m.remove("attrs");
        }
        v
    };
    strip(l) != strip(t)
}

fn run(
    doc: &mut Document,
    input: &SourcesReconcile,
    write: bool,
    resolved: bool,
) -> Result<(SourcesReconciled, Vec<CommandWarning>), Stop> {
    let Some(mut link) = doc
        .settings()
        .sources
        .iter()
        .find(|s| s.layer == input.layer)
        .cloned()
    else {
        return Err(refuse(
            codes::SOURCE_NOT_FOUND,
            format!(
                "“{}” katmanı bir kaynağa bağlı değil; önce cad.sources.edit ile bağlayın.",
                input.layer
            ),
            "layer",
        ));
    };
    if doc.layers().get(&link.layer).is_none() {
        return Err(refuse(
            codes::LAYER_NOT_FOUND,
            format!(
                "Bağlı “{}” katmanı çizimde yok; bağı çözün (cad.sources.edit unlink).",
                link.layer
            ),
            "layer",
        ));
    }
    if doc.layers().is_locked(&link.layer) {
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; kilidi Katmanlar panelinden açın.",
                link.layer
            ),
            "layer",
        ));
    }
    let source = origin_from(link.kind, &link.path);
    if let Origin::File(path) = source {
        if !path.is_file() {
            return Err(refuse(
                codes::SOURCE_GONE,
                format!(
                    "{} yerinde yok; dosyanın yeni yerini cad.sources.edit relink ile verin.",
                    link.path
                ),
                "layer",
            ));
        }
        if let Some(rec) = crate::gis_derive::derived_table(path, &link.table) {
            return Err(refuse(
                codes::DERIVED_DATASET,
                derived_message(&link.table, &rec),
                "layer",
            ));
        }
    }
    let mut loaded = super::together::load_link(doc, &link, "layer")?;
    if !link.writable || !loaded.stored.writable {
        return Err(refuse(
            codes::SOURCE_READONLY,
            format!(
                "{} dosyasının “{}” tablosuna yazılamaz (salt okunur); uzlaştırmak katmanın yanını yazar.",
                link.path, link.table
            ),
            "layer",
        ));
    }
    let Some(base) = base::of(doc, &link) else {
        return Err(refuse(
            codes::NO_SOURCE_BASE,
            format!(
                "“{}” katmanının dosyayla son eşleşmesinden tabanı yok (bu sürümden önce bağlanmış ya da {} nesneden büyük); iki yanın değişiklikleri ayrılamaz. Önce yazın (cad.sources.sync write) ya da tablodan okuyun (read).",
                link.layer,
                kentos_contracts::MAX_BASE_ROWS
            ),
            "layer",
        ));
    };
    super::together::patch_read(doc, &link, &mut loaded.stored)?;
    let sha = loaded.stored.sha256.clone();
    let table: BTreeMap<Uuid, Entity> = loaded
        .stored
        .in_project(loaded.mover.as_ref())
        .into_iter()
        .collect();
    let layer: BTreeMap<Uuid, Entity> = doc
        .by_layer_with_uids(&link.layer)
        .map(|(u, e)| (u, e.clone()))
        .collect();
    let ids: BTreeSet<Uuid> = base
        .keys()
        .chain(layer.keys())
        .chain(table.keys())
        .copied()
        .collect();

    // Each object's way, in the lasting ids' order.
    let mut conflicts: Vec<ReconcileConflict> = Vec::new();
    let mut ways: Vec<(Uuid, Way)> = Vec::new();
    let mut agreed = 0u32;
    for uid in ids {
        let b = base.get(&uid).copied();
        let (l, t) = (layer.get(&uid), table.get(&uid));
        let ld = did(b.map(|b| b.0), l.map(base::hash_of));
        let td = did(
            b.map(|b| b.1).filter(|h| *h != ABSENT),
            t.map(base::hash_of),
        );
        let way = match (ld, td) {
            (Did::Nothing, Did::Nothing) => continue,
            (_, Did::Nothing) => Way::ToTable,
            (Did::Nothing, _) => Way::FromTable,
            (Did::Removed, Did::Removed) => Way::Agreed,
            (Did::Removed, _) | (_, Did::Removed) => {
                let kind = if ld == Did::Removed {
                    ConflictKind::RemovedInLayer
                } else {
                    ConflictKind::RemovedInTable
                };
                // The side that holds it gives every attribute; the other has none.
                let fields = match (l, t) {
                    (Some(l), None) => fields_of_one(l, ReconcileSide::Layer),
                    (None, Some(t)) => fields_of_one(t, ReconcileSide::Table),
                    _ => Vec::new(),
                };
                conflicts.push(ReconcileConflict {
                    object: uid.to_string(),
                    kind,
                    geometry: true,
                    fields,
                    side: None,
                });
                Way::Conflict(conflicts.len() - 1)
            }
            _ => match (l, t) {
                (Some(l), Some(t)) if gp::essence(l) == gp::essence(t) => Way::Agreed,
                (Some(l), Some(t)) => {
                    conflicts.push(ReconcileConflict {
                        object: uid.to_string(),
                        kind: ConflictKind::Changed,
                        geometry: shapes_differ(l, t),
                        fields: fields_between(l, t),
                        side: None,
                    });
                    Way::Conflict(conflicts.len() - 1)
                }
                _ => continue,
            },
        };
        if matches!(way, Way::Agreed) {
            agreed += 1;
        }
        ways.push((uid, way));
    }

    // The sides taken.
    let at: HashMap<String, usize> = conflicts
        .iter()
        .enumerate()
        .map(|(i, c)| (c.object.clone(), i))
        .collect();
    let mut taken: HashMap<usize, ReconcileSide> = HashMap::new();
    for (k, t) in input.take.iter().enumerate() {
        let path = format!("take.{k}.object");
        let Some(&i) = Uuid::parse_str(&t.object)
            .ok()
            .and_then(|u| at.get(&u.to_string()))
        else {
            return Err(refuse(
                codes::NOT_A_CONFLICT,
                format!(
                    "{} bir çatışma değil; yan yalnız plan'ın conflicts'indeki nesneler için seçilir.",
                    t.object
                ),
                &path,
            ));
        };
        if taken.insert(i, t.side).is_some() {
            return Err(refuse(
                codes::INVALID_INPUT,
                format!("{} için iki kez yan seçilmiş; her çatışma bir kez.", t.object),
                &path,
            ));
        }
    }
    for (i, c) in conflicts.iter_mut().enumerate() {
        c.side = taken.get(&i).copied().or(input.rest);
    }
    let unresolved = conflicts.iter().filter(|c| c.side.is_none()).count() as u32;
    if resolved && unresolved > 0 {
        return Err(conflict(
            codes::UNRESOLVED_CONFLICTS,
            format!(
                "{unresolved} çatışma çözülmedi; hiçbir şey yazılmadı. Her biri için take ile ya da kalanlar için rest ile bir yan seçin (layer: katmanınki, table: tablonunki)."
            ),
            "take",
        ));
    }

    // What goes into the layer: the table's state of what only the table changed and of the conflicts taken to it.
    // An unresolved conflict (a plan) is left as the table holds it, so the write below leaves it alone too.
    let mut from = ReconcileCounts::default();
    let mut updates: Vec<(Slot, Entity)> = Vec::new();
    let mut adds: Vec<(Uuid, Entity)> = Vec::new();
    let mut gone: Vec<Slot> = Vec::new();
    for (uid, way) in &ways {
        let into_layer = match way {
            Way::FromTable => true,
            Way::Conflict(i) => conflicts[*i].side != Some(ReconcileSide::Layer),
            Way::ToTable | Way::Agreed => false,
        };
        if !into_layer {
            continue;
        }
        let counted = !matches!(way, Way::Conflict(i) if conflicts[*i].side.is_none());
        match (layer.get(uid), table.get(uid)) {
            (Some(_), Some(t)) => {
                let Some(slot) = doc.slot_of(*uid) else { continue };
                let Some(next) = doc.get(slot).and_then(|old| gp::with_essence(old, t)) else {
                    continue;
                };
                updates.push((slot, next));
                if counted {
                    from.modified += 1;
                }
            }
            (None, Some(t)) => {
                let mut e = t.clone();
                e.base_mut().layer_id = link.layer.clone();
                adds.push((*uid, e));
                if counted {
                    from.added += 1;
                }
            }
            (Some(_), None) => {
                if let Some(slot) = doc.slot_of(*uid) {
                    gone.push(slot);
                    if counted {
                        from.removed += 1;
                    }
                }
            }
            (None, None) => {}
        }
    }

    let label = step(&link.table);
    let sync = SourcesSync {
        layer: link.layer.clone(),
        direction: SyncDirection::Write,
        discard: false,
        with: Vec::new(),
        topology: None,
    };
    let body = |doc: &mut Document| {
        if !updates.is_empty() {
            doc.update_many(updates, &label);
        }
        if !adds.is_empty() {
            doc.add_many_with_uids(adds, &label).map_err(|e| {
                refuse(codes::IMPORT_UNREADABLE, e.0, "layer")
            })?;
        }
        if !gone.is_empty() {
            doc.remove(&gone);
        }
        // The layer now holds the table's side: it is written against the file as it is.
        link.revision = sha.clone();
        set_link(doc, Some(link.clone()), None);
        sync_run(doc, &sync, write)
    };
    let (synced, warnings) = if write {
        doc.transact(&label, body)?
    } else {
        let mut copy = doc.clone();
        body(&mut copy)?
    };
    Ok((
        SourcesReconciled {
            from_table: from,
            to_table: ReconcileCounts {
                added: synced.added,
                modified: synced.modified,
                removed: synced.removed,
            },
            agreed,
            conflicts,
            unresolved,
            source: if write { synced.source } else { sha },
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

fn completed<T>(r: Result<(T, Vec<CommandWarning>), Stop>) -> CommandResult<T> {
    match r {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// Whether executing would go: unresolved conflicts are refused as the execution refuses them.
pub fn validate(cx: &ExecutionContext<'_>, input: &SourcesReconcile) -> CommandResult<()> {
    let mut copy = cx.doc.clone();
    match run(&mut copy, input, false, true) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &SourcesReconcile,
) -> CommandResult<SourcesReconciled> {
    let mut copy = cx.doc.clone();
    completed(run(&mut copy, input, false, false))
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SourcesReconcile,
) -> CommandResult<SourcesReconciled> {
    // A plan through the protocol runs this on a reading copy: it writes no file then (docs/adr/0532 §4).
    let write = !cx.dry();
    completed(run(cx.doc, &input, write, true))
}
