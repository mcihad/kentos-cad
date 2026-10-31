//! A record's related records for `ilişkili_toplam` (docs/adr/0761): the records of a project
//! relation's child layer whose field holds the record's persistent id. They are found once, when the
//! form is prepared, from the world the host gives (the relation's child layer, its objects and their
//! shapes), and indexed by the parent's id: a call reads only the records tied to the record.
//!
//! ```text
//!  bina (B-4)  ◀── malik.bina ──  malik  Hisse  Durum
//!                                  M-1    1/2    Etkin
//!                                  M-2    1/2    Etkin      ilişkili_toplam('Malikler', 'say') → 3
//!                                  M-3    —      Pasif
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use kentos_contracts::{FormBody, js_trim};
use kentos_expression::js::text::fold_turkish;
use kentos_expression::library::Func;
use kentos_expression::scalar::R;
use kentos_expression::world::WorldCall;
use kentos_expression::world::related::related_value;
use kentos_expression::{Schema, compile_with};

use crate::lookup::World;
use crate::object::object_of_shape;
use crate::record::{Attrs, Object, RuleScope};

/// The rule properties a form's items hold, by their JSON names.
const RULES: [&str; 7] = [
    "visibleWhen",
    "requiredWhen",
    "readOnlyWhen",
    "defaultExpr",
    "calculate",
    "choiceFilter",
    "rule",
];

/// Whether `world` holds the relation `named` (by its id or its name).
pub(crate) fn in_world(named: &str, world: &World) -> bool {
    let key = fold_turkish(js_trim(named));
    world.values().any(|o| {
        o.relations
            .iter()
            .any(|r| fold_turkish(&r.id) == key || fold_turkish(&r.name) == key)
    })
}

/// The relations the form's rules name in `ilişkili_toplam`, as written, each once: what the host puts
/// in the world (the relation's child layer and its objects) for the form to read.
pub fn named_relations(body: &FormBody) -> Vec<String> {
    related_needs(body).0
}

/// [`named_relations`], and whether a call's inner expressions read a related record's geometry (`$alan`,
/// `$uzunluk` …): only then the host gives the records' shapes.
pub fn related_needs(body: &FormBody) -> (Vec<String>, bool) {
    let Ok(json) = serde_json::to_value(body) else {
        return (Vec::new(), false);
    };
    let schema = Schema {
        relations: true,
        ..Schema::default()
    };
    let mut out: Vec<String> = Vec::new();
    let mut shapes = false;
    let mut sources = Vec::new();
    rule_sources(&json, &mut sources);
    for src in sources {
        if !src.contains('(') {
            continue;
        }
        let Ok(e) = compile_with(src, &schema) else {
            continue;
        };
        for c in &e.world {
            if c.func != Func::RelatedAggregate {
                continue;
            }
            let Some(name) = c.layer.as_deref().map(js_trim).filter(|n| !n.is_empty()) else {
                continue;
            };
            shapes |= [&c.value, &c.condition]
                .into_iter()
                .flatten()
                .any(|e| reads_geometry(e.needs));
            if !out.iter().any(|o| fold_turkish(o) == fold_turkish(name)) {
                out.push(name.to_owned());
            }
        }
    }
    (out, shapes)
}

/// Whether an expression reads its object's geometry.
fn reads_geometry(n: kentos_expression::Needs) -> bool {
    n.measured || n.vertices || n.centroid || n.bounds
}

/// The rule texts under `RULES` anywhere in a form's JSON.
fn rule_sources<'j>(v: &'j serde_json::Value, out: &mut Vec<&'j str>) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                match v {
                    serde_json::Value::String(s) if RULES.contains(&k.as_str()) => out.push(s),
                    _ => rule_sources(v, out),
                }
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|i| rule_sources(i, out)),
        _ => {}
    }
}

/// One relation's records, indexed by the parent ids their field holds.
#[derive(Debug)]
struct Set {
    /// The relation's id and name, folded: what a call names it by.
    id: String,
    name: String,
    records: Vec<(Attrs, Object)>,
    by_parent: HashMap<String, Vec<usize>>,
}

/// The related records of every relation a form names, found once.
#[derive(Debug, Default)]
pub(crate) struct Relatives {
    sets: Vec<Set>,
}

impl Relatives {
    /// The relations of `world` the form's `names` call for; none when it names none.
    pub(crate) fn of(names: &[String], world: &World) -> Option<Relatives> {
        if names.is_empty() {
            return None;
        }
        let wanted: Vec<String> = names.iter().map(|n| fold_turkish(js_trim(n))).collect();
        let mut sets = Vec::new();
        for other in world.values() {
            for r in &other.relations {
                let (id, name) = (fold_turkish(&r.id), fold_turkish(&r.name));
                if !wanted.iter().any(|w| *w == id || *w == name) {
                    continue;
                }
                let field = other
                    .fields
                    .iter()
                    .find(|f| f.key() == r.field)
                    .map_or(r.field.as_str(), |f| f.name.as_str());
                let mut records = Vec::new();
                let mut by_parent: HashMap<String, Vec<usize>> = HashMap::new();
                for (k, (uid, attrs)) in other.rows.iter().enumerate() {
                    let Some(held) = attrs.get(field).filter(|v| !js_trim(v).is_empty()) else {
                        continue;
                    };
                    let at = records.len();
                    // A many-to-many field holds several ids (docs/adr/0409 §1).
                    for parent in held.split(';').map(js_trim).filter(|p| !p.is_empty()) {
                        let list = by_parent.entry(parent.to_owned()).or_default();
                        if list.last() != Some(&at) {
                            list.push(at);
                        }
                    }
                    let shape = other.shapes.get(k).cloned().flatten();
                    let mut object = object_of_shape(shape, &other.name, k as f64 + 1.0);
                    object.kind = "Kayıt".to_owned();
                    object.uid = Some(uid.clone());
                    records.push((attrs.clone(), object));
                }
                sets.push(Set {
                    id,
                    name,
                    records,
                    by_parent,
                });
            }
        }
        Some(Relatives { sets })
    }

    /// The relation a call names: by its id first, then by its name.
    fn set(&self, named: &str) -> Option<&Set> {
        let key = fold_turkish(js_trim(named));
        self.sets
            .iter()
            .find(|s| s.id == key)
            .or_else(|| self.sets.iter().find(|s| s.name == key))
    }
}

/// The related records an object carries to its rules; one form's, shared.
#[derive(Clone, Debug)]
pub struct RelatedRecords(pub(crate) Arc<Relatives>);

impl PartialEq for RelatedRecords {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl RelatedRecords {
    /// Call `c`'s value for the record `uid` (a new record, none: no related record).
    pub(crate) fn value(&self, c: &WorldCall, uid: Option<&str>, out: &mut String) -> R<'static> {
        let set = c.layer.as_deref().and_then(|n| self.0.set(n));
        let empty: &[usize] = &[];
        let (records, at) = match (set, uid) {
            (Some(s), Some(uid)) => (
                s.records.as_slice(),
                s.by_parent.get(js_trim(uid)).map_or(empty, Vec::as_slice),
            ),
            (Some(s), None) => (s.records.as_slice(), empty),
            // A relation the form does not know: empty, as the preflight warns.
            (None, _) => return R::V(kentos_expression::scalar::V::Null),
        };
        related_value(
            c,
            at,
            |&i, expr| {
                expr.item(&RuleScope {
                    expr,
                    attrs: &records[i].0,
                    object: &records[i].1,
                })
            },
            out,
        )
    }
}
