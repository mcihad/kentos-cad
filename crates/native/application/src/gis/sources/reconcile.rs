//! `cad.sources.reconcile` (docs/adr/0538, 0553; plan/gis.md CBS-37): a bound layer and its table, both changed since
//! they last matched, put together object by object against the base (`base`): what only the layer changed is written
//! to the table, what only the table changed is taken into the layer, what both changed alike agrees. An object both
//! changed differently is merged field by field when the base keeps its fields (`merge`): an attribute or the geometry
//! only one side changed takes that side's, and only what both changed differently is a conflict (with both values).
//! One side removing what the other changed is a conflict too. Changes that belong together (neighbours whose shared
//! vertex both sides moved, a parent one side removed and a child the other added or changed) are one group,
//! resolved together to one side. A conflict is resolved by the side the input takes (`take`, `rest`); the last writer
//! does not win. Several bound layers of one GeoPackage file go in one call (`with`). The table's side goes into the
//! layers, then the layers are written as `cad.sources.sync` writes them, against the file as it now is; one undo
//! step, one transaction.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use kentos_commands::{
    BaseDetail, CommandResult, CommandWarning, ConflictField, ConflictKind, Entity, GroupReason,
    ReconcileConflict, ReconcileCounts, ReconcileGroup, ReconcileLayer, ReconcileMerge,
    ReconcileSide, SourceLink, SourcesReconcile, SourcesReconciled, SourcesSync, SyncDirection,
};
use kentos_domain::{Document, Slot, Uuid};

use super::base::{self, ABSENT, BaseObj};
use super::merge;
use super::relations::ids_of;
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

/// An object's conflict of its own.
struct Own {
    kind: ConflictKind,
    geometry: bool,
    fields: Vec<ConflictField>,
}

/// What reconciling does with an object, before groups and the sides taken.
enum Way {
    Skip,
    Agreed,
    /// Only the layer changed it: it is written to the table.
    ToTable,
    /// Only the table changed it: it is taken into the layer.
    FromTable,
    /// Both changed it, in ways that do not clash: the merge (neither side's object as it is).
    Merged(Box<merge::Merge>),
    Conflict(Own),
}

/// One object of the reconcile.
struct Obj {
    /// Its layer's index among the reconciled layers.
    li: usize,
    uid: Uuid,
    layer: Option<Entity>,
    table: Option<Entity>,
    ld: Did,
    td: Did,
    detail: Option<BaseDetail>,
    way: Way,
}

/// What an object becomes in the layer.
enum Target {
    /// The layer's object stays (and is written to the table).
    Layer,
    /// The table's state goes into the layer.
    Table,
    /// This object (a merge) goes into the layer and is written to the table.
    Made(Box<Entity>),
}

/// One bound layer of the reconcile, read.
struct Lay {
    link: SourceLink,
    layer: BTreeMap<Uuid, Entity>,
    table: BTreeMap<Uuid, Entity>,
    base: HashMap<Uuid, BaseObj>,
    sha: String,
}

/// The undo step of a reconcile: “Uzlaştır: tablo”.
pub fn step(table: &str) -> String {
    format!("Uzlaştır: {table}")
}

/// Where a refusal points: `layer` for the first layer, `with` for the others.
fn at(i: usize) -> &'static str {
    if i == 0 { "layer" } else { "with" }
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

fn same_essence(a: &Entity, b: &Entity) -> bool {
    gp::essence(a) == gp::essence(b)
}

/// A bound layer read for a reconcile: its link and the checks every layer passes, its two sides and its base.
fn load(doc: &Document, id: &str, at: &'static str) -> Result<Lay, Stop> {
    let Some(link) = doc
        .settings()
        .sources
        .iter()
        .find(|s| s.layer == id)
        .cloned()
    else {
        return Err(refuse(
            codes::SOURCE_NOT_FOUND,
            format!("“{id}” katmanı bir kaynağa bağlı değil; önce cad.sources.edit ile bağlayın."),
            at,
        ));
    };
    if doc.layers().get(&link.layer).is_none() {
        return Err(refuse(
            codes::LAYER_NOT_FOUND,
            format!(
                "Bağlı “{}” katmanı çizimde yok; bağı çözün (cad.sources.edit unlink).",
                link.layer
            ),
            at,
        ));
    }
    if doc.layers().is_locked(&link.layer) {
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; kilidi Katmanlar panelinden açın.",
                link.layer
            ),
            at,
        ));
    }
    if let Origin::File(path) = origin_from(link.kind, &link.path) {
        if !path.is_file() {
            return Err(refuse(
                codes::SOURCE_GONE,
                format!(
                    "{} yerinde yok; dosyanın yeni yerini cad.sources.edit relink ile verin.",
                    link.path
                ),
                at,
            ));
        }
        if let Some(rec) = crate::gis_derive::derived_table(path, &link.table) {
            return Err(refuse(
                codes::DERIVED_DATASET,
                derived_message(&link.table, &rec),
                at,
            ));
        }
    }
    let mut loaded = super::together::load_link(doc, &link, at)?;
    if !link.writable || !loaded.stored.writable {
        return Err(refuse(
            codes::SOURCE_READONLY,
            format!(
                "{} dosyasının “{}” tablosuna yazılamaz (salt okunur); uzlaştırmak katmanın yanını yazar.",
                link.path, link.table
            ),
            at,
        ));
    }
    let Some(base) = base::of(doc, &link) else {
        return Err(refuse(
            codes::NO_SOURCE_BASE,
            format!(
                "“{}” katmanının dosyayla son eşleşmesinden tabanı yok (bu sürümden önce bağlanmış ya da {} nesneden büyük); iki yanın değişiklikleri ayrılamaz. Önce yazın (cad.sources.sync write) ya da tablodan okuyun (read).",
                link.layer,
                kentos_commands::MAX_BASE_ROWS
            ),
            at,
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
    Ok(Lay {
        link,
        layer,
        table,
        base,
        sha,
    })
}

/// How each object of every layer stands: what each side did and what that makes of it.
fn classify(lays: &[Lay]) -> (Vec<Obj>, u32, u32) {
    let mut objs: Vec<Obj> = Vec::new();
    let (mut agreed, mut missing) = (0u32, 0u32);
    for (li, lay) in lays.iter().enumerate() {
        let ids: BTreeSet<Uuid> = lay
            .base
            .keys()
            .chain(lay.layer.keys())
            .chain(lay.table.keys())
            .copied()
            .collect();
        for uid in ids {
            let b = lay.base.get(&uid);
            let (l, t) = (lay.layer.get(&uid), lay.table.get(&uid));
            let ld = did(b.map(|b| b.layer), l.map(base::hash_of));
            let td = did(
                b.map(|b| b.table).filter(|h| *h != ABSENT),
                t.map(base::hash_of),
            );
            let detail = b.and_then(|b| b.detail.clone());
            let way = match (ld, td) {
                (Did::Nothing, Did::Nothing) => Way::Skip,
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
                    Way::Conflict(Own {
                        kind,
                        geometry: true,
                        fields,
                    })
                }
                _ => match (l, t) {
                    (Some(l), Some(t)) if same_essence(l, t) => Way::Agreed,
                    (Some(l), Some(t)) => {
                        let three = detail.as_ref().and_then(|d| merge::merge(l, t, d, None));
                        match three {
                            Some(m) if m.conflicts.is_empty() && !m.geometry_conflict => {
                                if same_essence(&m.entity, l) {
                                    Way::ToTable
                                } else if same_essence(&m.entity, t) {
                                    Way::FromTable
                                } else {
                                    Way::Merged(Box::new(m))
                                }
                            }
                            Some(m) => Way::Conflict(Own {
                                kind: ConflictKind::Changed,
                                geometry: m.geometry_conflict,
                                fields: m.conflicts,
                            }),
                            None => {
                                if b.is_some() {
                                    missing += 1;
                                }
                                Way::Conflict(Own {
                                    kind: ConflictKind::Changed,
                                    geometry: merge::shapes_differ(l, t),
                                    fields: fields_between(l, t),
                                })
                            }
                        }
                    }
                    _ => Way::Skip,
                },
            };
            if matches!(way, Way::Agreed) {
                agreed += 1;
            }
            objs.push(Obj {
                li,
                uid,
                layer: l.cloned(),
                table: t.cloned(),
                ld,
                td,
                detail,
                way,
            });
        }
    }
    objs.sort_by_key(|o| o.uid);
    (objs, agreed, missing)
}

/// A set of objects tied together, by their indexes.
struct Union(Vec<usize>);

impl Union {
    fn new(n: usize) -> Self {
        Union((0..n).collect())
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.0[x] != x {
            self.0[x] = self.0[self.0[x]];
            x = self.0[x];
        }
        x
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a.max(b)] = a.min(b);
        }
    }
}

/// A group of related changes: its members (indexes into the objects, in the ids' order) and why.
struct Grp {
    members: Vec<usize>,
    reasons: BTreeSet<u8>,
}

/// An object's vertex hashes: at the base, in the layer, in the table.
type Sets = (Vec<u32>, Vec<u32>, Vec<u32>);

const VERTEX: u8 = 0;
const RELATION: u8 = 1;

/// The most objects sharing one vertex that are paired up (a corner of a thousand objects is no neighbourhood).
const MOST_AT_A_VERTEX: usize = 64;

/// The groups of related changes among the objects that changed (docs/adr/0553 §2).
fn groups_of(doc: &Document, lays: &[Lay], objs: &[Obj]) -> Vec<Grp> {
    let active = |o: &Obj| !matches!(o.way, Way::Skip | Way::Agreed);
    let n = objs.len();
    let mut union = Union::new(n);
    let mut reasons: HashMap<usize, BTreeSet<u8>> = HashMap::new();
    let mut tied: Vec<(usize, usize, u8)> = Vec::new();

    // Neighbours that shared a vertex at the base, which both sides moved.
    let sets: Vec<Option<Sets>> = objs
        .iter()
        .map(|o| {
            if !active(o) {
                return None;
            }
            let base = o.detail.as_ref()?.vertices.clone()?;
            // The sides' vertices however many: the base keeps a many-cornered object's shared ones (docs/adr/0648 §4).
            let of = |e: &Option<Entity>| match e {
                None => Vec::new(),
                Some(e) => merge::vertex_set_all(e),
            };
            Some((base, of(&o.layer), of(&o.table)))
        })
        .collect();
    let mut at_vertex: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, s) in sets.iter().enumerate() {
        if let Some((base, _, _)) = s {
            for q in base {
                at_vertex.entry(*q).or_default().push(i);
            }
        }
    }
    for (q, members) in &at_vertex {
        if members.len() < 2 || members.len() > MOST_AT_A_VERTEX {
            continue;
        }
        for (k, &i) in members.iter().enumerate() {
            for &j in &members[k + 1..] {
                let (Some((_, li, ti)), Some((_, lj, tj))) = (&sets[i], &sets[j]) else {
                    continue;
                };
                let held =
                    |a: &[u32], b: &[u32]| a.binary_search(q).is_ok() && b.binary_search(q).is_ok();
                if !held(li, lj) && !held(ti, tj) {
                    tied.push((i, j, VERTEX));
                }
            }
        }
    }

    // A parent one side removed and a child the other side added or changed.
    let by_uid: HashMap<Uuid, usize> = objs.iter().enumerate().map(|(i, o)| (o.uid, i)).collect();
    let names: Vec<&str> = lays.iter().map(|l| l.link.layer.as_str()).collect();
    for r in &doc.settings().relations {
        let (Some(_), Some(_)) = (
            names.iter().position(|l| *l == r.child),
            names.iter().position(|l| *l == r.parent),
        ) else {
            continue;
        };
        let Some(field) = doc
            .layers()
            .get(&r.child)
            .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
            .map(|f| f.name.clone())
        else {
            continue;
        };
        for (ci, c) in objs.iter().enumerate() {
            if !active(c) || names[c.li] != r.child {
                continue;
            }
            let refs = |e: &Option<Entity>| -> HashSet<String> {
                e.as_ref()
                    .and_then(|e| e.base().attrs.get(&field))
                    .map(|v| ids_of(v).into_iter().collect())
                    .unwrap_or_default()
            };
            let (rl, rt) = (refs(&c.layer), refs(&c.table));
            for text in rl.union(&rt) {
                let Some(&pi) = Uuid::parse_str(text).ok().and_then(|u| by_uid.get(&u)) else {
                    continue;
                };
                let p = &objs[pi];
                if !active(p) || names[p.li] != r.parent {
                    continue;
                }
                let new = |d: Did| matches!(d, Did::Added | Did::Changed);
                let clash = (p.ld == Did::Removed
                    && p.td != Did::Removed
                    && new(c.td)
                    && rt.contains(text))
                    || (p.td == Did::Removed
                        && p.ld != Did::Removed
                        && new(c.ld)
                        && rl.contains(text));
                if clash {
                    tied.push((ci, pi, RELATION));
                }
            }
        }
    }
    for &(a, b, why) in &tied {
        union.join(a, b);
        reasons.entry(a).or_default().insert(why);
        reasons.entry(b).or_default().insert(why);
    }
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &(a, b, _) in &tied {
        for x in [a, b] {
            let root = union.find(x);
            let members = by_root.entry(root).or_default();
            if !members.contains(&x) {
                members.push(x);
            }
        }
    }
    by_root
        .into_values()
        .map(|mut members| {
            members.sort_unstable();
            let reasons = members
                .iter()
                .filter_map(|m| reasons.get(m))
                .flatten()
                .copied()
                .collect();
            Grp { members, reasons }
        })
        .collect()
}

fn completed<T>(r: Result<(T, Vec<CommandWarning>), Stop>) -> CommandResult<T> {
    match r {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

fn run(
    doc: &mut Document,
    input: &SourcesReconcile,
    write: bool,
    resolved: bool,
) -> Result<(SourcesReconciled, Vec<CommandWarning>), Stop> {
    // The layers: each bound, of one GeoPackage file, read.
    let ids: Vec<String> = std::iter::once(input.layer.clone())
        .chain(input.with.iter().cloned())
        .collect();
    let mut seen = HashSet::new();
    for (i, id) in ids.iter().enumerate() {
        if !seen.insert(id.as_str()) {
            return Err(refuse(
                codes::INVALID_INPUT,
                format!("“{id}” katmanı birden çok kez verildi; her katman bir kez uzlaştırılır."),
                at(i),
            ));
        }
    }
    let mut lays: Vec<Lay> = Vec::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
        lays.push(load(doc, id, at(i))?);
    }
    if lays.len() > 1 {
        let first = &lays[0].link;
        for (i, l) in lays.iter().enumerate().skip(1) {
            // One GeoPackage file or one database (docs/adr/0648 §3): a transaction spans no two sources.
            if l.link.kind != first.kind || l.link.path != first.path {
                return Err(refuse(
                    codes::TRANSACTION_SPLIT,
                    format!(
                        "“{}” katmanı {} kaynağında, “{}” katmanı {} dosyasında; birlikte uzlaştırılacak katmanlar tek GeoPackage dosyasında ya da tek veritabanında olmalı. Katmanları ayrı ayrı uzlaştırın.",
                        l.link.layer, l.link.path, first.layer, first.path
                    ),
                    at(i),
                ));
            }
        }
    }
    let multi = lays.len() > 1;
    let (objs, agreed, missing) = classify(&lays);
    let groups = groups_of(doc, &lays, &objs);
    let group_of: HashMap<usize, usize> = groups
        .iter()
        .enumerate()
        .flat_map(|(g, grp)| grp.members.iter().map(move |m| (*m, g)))
        .collect();

    // The conflicts and groups as the answer lists them: an object of its own, or one a group holds.
    let layer_name = |li: usize| multi.then(|| lays[li].link.layer.clone());
    let mut conflicts: Vec<ReconcileConflict> = Vec::new();
    let mut conflict_of: HashMap<usize, usize> = HashMap::new();
    for (i, o) in objs.iter().enumerate() {
        let group = group_of
            .get(&i)
            .map(|g| objs[groups[*g].members[0]].uid.to_string());
        let c = match (&o.way, group_of.contains_key(&i)) {
            (Way::Conflict(own), _) => ReconcileConflict {
                object: o.uid.to_string(),
                layer: layer_name(o.li),
                kind: own.kind,
                group,
                geometry: own.geometry,
                fields: own.fields.clone(),
                side: None,
            },
            (_, true) => {
                let (geometry, fields) = match (&o.layer, &o.table) {
                    (Some(l), Some(t)) => (merge::shapes_differ(l, t), fields_between(l, t)),
                    (Some(l), None) => (true, fields_of_one(l, ReconcileSide::Layer)),
                    (None, Some(t)) => (true, fields_of_one(t, ReconcileSide::Table)),
                    (None, None) => (true, Vec::new()),
                };
                ReconcileConflict {
                    object: o.uid.to_string(),
                    layer: layer_name(o.li),
                    kind: ConflictKind::Related,
                    group,
                    geometry,
                    fields,
                    side: None,
                }
            }
            _ => continue,
        };
        conflict_of.insert(i, conflicts.len());
        conflicts.push(c);
    }

    // The sides taken: one for a conflict, one for a whole group.
    let at_object: HashMap<Uuid, usize> = conflict_of
        .iter()
        .map(|(i, c)| (objs[*i].uid, *c))
        .collect();
    let mut taken: HashMap<usize, ReconcileSide> = HashMap::new();
    let mut named: HashSet<usize> = HashSet::new();
    let mut group_side: HashMap<usize, ReconcileSide> = HashMap::new();
    let index_of: HashMap<usize, usize> = conflict_of.iter().map(|(i, c)| (*c, *i)).collect();
    for (k, t) in input.take.iter().enumerate() {
        let path = format!("take.{k}.object");
        let Some(&c) = Uuid::parse_str(&t.object)
            .ok()
            .and_then(|u| at_object.get(&u))
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
        if !named.insert(c) {
            return Err(refuse(
                codes::INVALID_INPUT,
                format!(
                    "{} için iki kez yan seçilmiş; her çatışma bir kez.",
                    t.object
                ),
                &path,
            ));
        }
        match group_of.get(&index_of[&c]) {
            Some(&g) => {
                if group_side.insert(g, t.side).is_some_and(|s| s != t.side) {
                    return Err(refuse(
                        codes::GROUP_SPLIT,
                        format!(
                            "{} nesnesi birlikte değişen bir grubun üyesi; grubun öbür nesneleri için başka yan seçilmiş. Grubun tamamı için bir yan seçin.",
                            t.object
                        ),
                        &path,
                    ));
                }
            }
            None => {
                taken.insert(c, t.side);
            }
        }
    }
    let side_of = |i: usize| -> Option<ReconcileSide> {
        match group_of.get(&i) {
            Some(g) => group_side.get(g).copied().or(input.rest),
            None => conflict_of
                .get(&i)
                .and_then(|c| taken.get(c))
                .copied()
                .or(input.rest),
        }
    };
    for (i, c) in &conflict_of {
        conflicts[*c].side = side_of(*i);
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

    // What each object becomes. An unresolved one (a plan) is left as the table holds it, uncounted, so the write
    // below leaves it alone too.
    let mut targets: Vec<Option<(Target, bool)>> = Vec::with_capacity(objs.len());
    for (i, o) in objs.iter().enumerate() {
        let counted = !(conflict_of.contains_key(&i) && side_of(i).is_none());
        let pick = |s: Option<ReconcileSide>| match s {
            Some(ReconcileSide::Layer) => Target::Layer,
            _ => Target::Table,
        };
        let target = match (&o.way, group_of.contains_key(&i)) {
            (Way::Skip | Way::Agreed, _) => None,
            (_, true) => Some(pick(side_of(i))),
            (Way::ToTable, _) => Some(Target::Layer),
            (Way::FromTable, _) => Some(Target::Table),
            (Way::Merged(m), _) => Some(Target::Made(Box::new(m.entity.clone()))),
            (Way::Conflict(own), _) => {
                let side = side_of(i);
                let made = match (&o.layer, &o.table, &o.detail, own.kind) {
                    (Some(l), Some(t), Some(d), ConflictKind::Changed) => {
                        merge::merge(l, t, d, side).map(|m| m.entity)
                    }
                    _ => None,
                };
                Some(match (made, side) {
                    (Some(e), Some(_)) => Target::Made(Box::new(e)),
                    _ => pick(side),
                })
            }
        };
        targets.push(target.map(|t| (t, counted)));
    }

    // What goes into the layers: the table's state of what only the table changed, the merges and the conflicts
    // taken to the table.
    let mut from = vec![ReconcileCounts::default(); lays.len()];
    let mut updates: Vec<(Slot, Entity)> = Vec::new();
    let mut adds: Vec<(usize, Uuid, Entity)> = Vec::new();
    let mut gone: Vec<Slot> = Vec::new();
    for (o, target) in objs.iter().zip(&targets) {
        let Some((target, counted)) = target else {
            continue;
        };
        let counts = &mut from[o.li];
        match target {
            Target::Layer => {}
            Target::Made(m) => {
                let (Some(l), Some(slot)) = (&o.layer, doc.slot_of(o.uid)) else {
                    continue;
                };
                if same_essence(l, m) {
                    continue;
                }
                let Some(next) = doc.get(slot).and_then(|old| gp::with_essence(old, m)) else {
                    continue;
                };
                updates.push((slot, next));
                if *counted {
                    counts.modified += 1;
                }
            }
            Target::Table => match (&o.layer, &o.table) {
                (Some(l), Some(t)) => {
                    if same_essence(l, t) {
                        continue;
                    }
                    let Some(slot) = doc.slot_of(o.uid) else {
                        continue;
                    };
                    let Some(next) = doc.get(slot).and_then(|old| gp::with_essence(old, t)) else {
                        continue;
                    };
                    updates.push((slot, next));
                    if *counted {
                        counts.modified += 1;
                    }
                }
                (None, Some(t)) => {
                    let mut e = t.clone();
                    e.base_mut().layer_id = lays[o.li].link.layer.clone();
                    adds.push((o.li, o.uid, e));
                    if *counted {
                        counts.added += 1;
                    }
                }
                (Some(_), None) => {
                    if let Some(slot) = doc.slot_of(o.uid) {
                        gone.push(slot);
                        if *counted {
                            counts.removed += 1;
                        }
                    }
                }
                (None, None) => {}
            },
        }
    }

    let label = step(&lays[0].link.table);
    let sync = SourcesSync {
        layer: lays[0].link.layer.clone(),
        direction: SyncDirection::Write,
        discard: false,
        with: input.with.clone(),
        topology: None,
    };
    // Each layer at its own table's state (a database's tables have each their own revision).
    let links: Vec<(SourceLink, String)> = lays
        .iter()
        .map(|l| (l.link.clone(), l.sha.clone()))
        .collect();
    let body = |doc: &mut Document| {
        if !updates.is_empty() {
            doc.update_many(updates, &label);
        }
        if !adds.is_empty() {
            let adds: Vec<(Uuid, Entity)> = adds.into_iter().map(|(_, u, e)| (u, e)).collect();
            doc.add_many_with_uids(adds, &label)
                .map_err(|e| refuse(codes::IMPORT_UNREADABLE, e.0, "layer"))?;
        }
        if !gone.is_empty() {
            doc.remove(&gone);
        }
        // The layers now hold the table's side: they are written against the file as it is.
        for (mut link, sha) in links {
            link.revision = sha;
            set_link(doc, Some(link), None);
        }
        sync_run(doc, &sync, write)
    };
    let (synced, mut warnings) = if write {
        doc.transact(&label, body)?
    } else {
        let mut copy = doc.clone();
        body(&mut copy)?
    };

    let merged: Vec<ReconcileMerge> = objs
        .iter()
        .enumerate()
        .filter_map(|(i, o)| match &o.way {
            Way::Merged(m) if !group_of.contains_key(&i) => Some(ReconcileMerge {
                object: o.uid.to_string(),
                layer: layer_name(o.li),
                from_layer: m.from_layer.clone(),
                from_table: m.from_table.clone(),
                geometry: m.geometry,
            }),
            _ => None,
        })
        .collect();
    let group_out: Vec<ReconcileGroup> = groups
        .iter()
        .enumerate()
        .map(|(g, grp)| ReconcileGroup {
            id: objs[grp.members[0]].uid.to_string(),
            objects: grp
                .members
                .iter()
                .map(|m| objs[*m].uid.to_string())
                .collect(),
            reasons: grp
                .reasons
                .iter()
                .map(|r| {
                    if *r == VERTEX {
                        GroupReason::SharedVertex
                    } else {
                        GroupReason::Relation
                    }
                })
                .collect(),
            side: group_side.get(&g).copied().or(input.rest),
        })
        .collect();
    let sum = |f: fn(&ReconcileCounts) -> u32| from.iter().map(f).sum::<u32>();
    let layers_out: Vec<ReconcileLayer> = if multi {
        lays.iter()
            .enumerate()
            .map(|(i, lay)| {
                let to = synced
                    .layers
                    .iter()
                    .find(|l| l.layer == lay.link.layer)
                    .map(|l| ReconcileCounts {
                        added: l.added,
                        modified: l.modified,
                        removed: l.removed,
                    })
                    .unwrap_or_default();
                ReconcileLayer {
                    layer: lay.link.layer.clone(),
                    from_table: from[i],
                    to_table: to,
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    if missing > 0 {
        warnings.push(CommandWarning {
            code: codes::FIELD_BASE_MISSING.into(),
            message: format!(
                "{missing} nesneyi iki yan da değiştirdi ve tabanın bunların alan kaydı yok; öznitelikleri tek tek birleştirilemedi, bütün olarak çatışıyor."
            ),
            path: Some("layer".into()),
        });
    }
    Ok((
        SourcesReconciled {
            from_table: ReconcileCounts {
                added: sum(|c| c.added),
                modified: sum(|c| c.modified),
                removed: sum(|c| c.removed),
            },
            to_table: ReconcileCounts {
                added: synced.added,
                modified: synced.modified,
                removed: synced.removed,
            },
            agreed,
            merged,
            groups: group_out,
            layers: layers_out,
            conflicts,
            unresolved,
            source: if write { synced.source } else { sha_of(&lays) },
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

fn sha_of(lays: &[Lay]) -> String {
    lays[0].sha.clone()
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
    // One step of the drawing and its own row in the journal (docs/adr/0253 §3, 0548 §4).
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SOURCES_RECONCILE,
        kentos_commands::CAD_SOURCES_RECONCILE_VERSION,
        input,
        written_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_execute(
    cx: &mut ExecutionContext<'_>,
    input: SourcesReconcile,
) -> CommandResult<SourcesReconciled> {
    // A plan through the protocol runs this on a reading copy: it writes no file then (docs/adr/0532 §4).
    let write = !cx.dry();
    completed(run(cx.doc, &input, write, true))
}
