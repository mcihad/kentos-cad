//! Many-to-many references in a bound GeoPackage (docs/adr/0532 §2). A project relation whose child field holds
//! several parents' ids (docs/adr/0409) is, in a file, either a text column of ids joined with `; ` or a GeoPackage
//! related-tables relation (OGC 18-000) KentOS wrote (`kentos_references`) with a mapping table of the child's fid and
//! the parent's. This module finds which of the two a pair of bound tables holds, builds the mapping's rows from the
//! layers' references, and reads them back into the field. A relation of another representation is neither
//! rewritten nor read as text: it is reported.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use kentos_contracts::{LayerField, ProjectRelation, SourceKind, SourceLink};
use kentos_domain::{Document, Uuid};
use kentos_geopackage::relations::{KENTOS_RELATION, Key, MapEdit, Relation};
use kentos_geopackage::{GeoPackage, Layer};

use super::{Stop, Stored, codes, refuse};

/// How the file holds a many-to-many reference between two of its tables.
pub(super) enum Kept {
    /// A text column of ids joined with `; `.
    Text,
    /// KentOS's relation.
    Mapped(Relation),
    /// A well-formed relation of another kind (why it is not KentOS's).
    Foreign(String),
    /// A relation the profile does not allow (why).
    Broken(String),
}

/// A project relation whose child field is a many-to-many reference and whose two layers are bound to tables of one
/// GeoPackage file.
pub(super) struct Tie {
    pub relation: String,
    pub child_layer: String,
    pub parent_layer: String,
    /// The child's field, as the attribute key the objects hold.
    pub field: String,
    pub child: SourceLink,
    pub parent: SourceLink,
    pub kept: Kept,
}

/// The ids a multi-valued reference holds, in order, once each.
pub(super) fn ids_of(value: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    value
        .split(';')
        .map(str::trim)
        .filter(|h| !h.is_empty() && seen.insert(h.to_owned()))
        .map(str::to_owned)
        .collect()
}

/// A relation's child field when it is a many-to-many reference: the field.
pub(super) fn multiple_field<'a>(doc: &'a Document, r: &ProjectRelation) -> Option<&'a LayerField> {
    doc.layers()
        .get(&r.child)
        .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
        .filter(|f| f.multiple)
}

/// How the file holds the reference between the child's table and the parent's.
pub(super) fn kept_between(
    file: &GeoPackage,
    layers: &[Layer],
    child: &str,
    parent: &str,
) -> Result<Kept, String> {
    let relations = file.relations().map_err(|e| e.0)?;
    let mut found: Option<Kept> = None;
    for r in &relations {
        let same = r.base_table == child && r.related_table == parent;
        let reverse = r.base_table == parent && r.related_table == child;
        if !same && !reverse {
            continue;
        }
        let kept = if reverse {
            Kept::Foreign(format!(
                "“{}” ilişkisi ters yönde (taban {parent}, ilgili {child}); KentOS tabanı alt katman yazar",
                r.name
            ))
        } else if let Some(why) = file.relation_problem(r, layers).map_err(|e| e.0)? {
            Kept::Broken(why)
        } else if r.name == KENTOS_RELATION {
            Kept::Mapped(r.clone())
        } else {
            Kept::Foreign(format!(
                "“{}” ilişkisi KentOS'un “{KENTOS_RELATION}” temsili değil",
                r.name
            ))
        };
        // The first KentOS relation wins; otherwise the first one found is told.
        match (&found, &kept) {
            (None, _) | (Some(Kept::Foreign(_) | Kept::Broken(_)), Kept::Mapped(_)) => {
                found = Some(kept);
            }
            _ => {}
        }
    }
    Ok(found.unwrap_or(Kept::Text))
}

/// The relations of the file `table` is a side of, as `cad.sources.describe` says them.
pub(super) fn listed(
    file: &GeoPackage,
    layers: &[Layer],
    table: &str,
) -> Vec<kentos_contracts::SourceRelation> {
    use kentos_contracts::{SourceRelation, SourceRelationKind};
    let Ok(all) = file.relations() else {
        return Vec::new();
    };
    all.iter()
        .filter(|r| r.base_table == table || r.related_table == table)
        .map(|r| {
            let problem = file.relation_problem(r, layers).ok().flatten();
            let rows = if problem.is_none() {
                file.mapping_rows(&r.mapping).map_or(0, |m| m.len() as u64)
            } else {
                0
            };
            let kind = match (&problem, r.name == KENTOS_RELATION) {
                (Some(_), _) => SourceRelationKind::Broken,
                (None, true) => SourceRelationKind::Kentos,
                (None, false) => SourceRelationKind::Foreign,
            };
            SourceRelation {
                name: r.name.clone(),
                base: r.base_table.clone(),
                related: r.related_table.clone(),
                mapping: r.mapping.clone(),
                rows,
                kind,
                why: problem,
            }
        })
        .collect()
}

/// The project's many-to-many references between layers bound to tables of one GeoPackage file, each with how the file
/// holds it. A file that cannot be read leaves its relations out (the layer's own load says why).
pub(super) fn ties(doc: &Document) -> Vec<Tie> {
    let mut out = Vec::new();
    let mut files: HashMap<String, Option<(GeoPackage, Vec<Layer>)>> = HashMap::new();
    for r in &doc.settings().relations {
        let Some(field) = multiple_field(doc, r) else {
            continue;
        };
        let bound = |layer: &str| {
            doc.settings()
                .sources
                .iter()
                .find(|s| s.layer == layer && s.kind == SourceKind::Geopackage)
        };
        let (Some(child), Some(parent)) = (bound(&r.child), bound(&r.parent)) else {
            continue;
        };
        if child.path != parent.path || r.child == r.parent {
            continue;
        }
        let entry = files.entry(child.path.clone()).or_insert_with(|| {
            let file = GeoPackage::open(Path::new(&child.path), false).ok()?;
            let layers = file.layers().ok()?;
            Some((file, layers))
        });
        let Some((file, layers)) = entry else {
            continue;
        };
        let Ok(kept) = kept_between(file, layers, &child.table, &parent.table) else {
            continue;
        };
        out.push(Tie {
            relation: r.id.clone(),
            child_layer: r.child.clone(),
            parent_layer: r.parent.clone(),
            field: field.name.clone(),
            child: child.clone(),
            parent: parent.clone(),
            kept,
        });
    }
    out
}

/// The mapped ties among `ties` that touch one of `layers` (as child or parent).
pub(super) fn mapped_touching<'a>(ties: &'a [Tie], layers: &[String]) -> Vec<&'a Tie> {
    ties.iter()
        .filter(|t| matches!(t.kept, Kept::Mapped(_)))
        .filter(|t| layers.contains(&t.child_layer) || layers.contains(&t.parent_layer))
        .collect()
}

/// The text a relation left unmapped is told with (a warning's message), for the ties that touch `layers`.
pub(super) fn unmapped_notes(ties: &[Tie], layers: &[String]) -> Vec<String> {
    ties.iter()
        .filter(|t| layers.contains(&t.child_layer) || layers.contains(&t.parent_layer))
        .filter_map(|t| match &t.kept {
            Kept::Text => Some(format!(
                "“{}” ilişkisinin ‘{}’ alanı {} tablosunda “; ” ile birleşmiş metin sütunu olarak kaldı (cad.sources.relate map ile ilişki profiline taşınır)",
                t.relation, t.field, t.child.table
            )),
            Kept::Foreign(why) | Kept::Broken(why) => Some(format!(
                "“{}” ilişkisi: {why}; ‘{}’ alanı yalnız metin sütunu olarak yazıldı, dosyadaki ilişkinin eşleme satırlarına dokunulmadı",
                t.relation, t.field
            )),
            Kept::Mapped(_) => None,
        })
        .collect()
}

/// A table's rows as the mapping sees them: a row the file holds, or one the same transaction inserts.
pub(super) struct Side<'a> {
    pub stored: &'a Stored,
    /// The write job of the table, with its inserts' places by lasting id; none: the table is not written.
    pub job: Option<(usize, &'a HashMap<Uuid, usize>)>,
}

impl Side<'_> {
    fn key(&self, uid: Uuid) -> Option<Key> {
        if let Some((fid, ..)) = self.stored.rows.get(&uid) {
            return Some(Key::Fid(*fid));
        }
        if let Some(fid) = self.stored.outside.get(&uid) {
            return Some(Key::Fid(*fid));
        }
        let (job, inserts) = self.job?;
        inserts.get(&uid).map(|i| Key::Inserted(job, *i))
    }

    fn outside_fid(&self, fid: i64) -> bool {
        self.stored.outside.values().any(|f| *f == fid)
    }
}

/// What a write does to a mapping table.
pub(super) struct Planned {
    /// The table's rows after, when they change.
    pub edit: Option<MapEdit>,
    pub added: u32,
    pub removed: u32,
}

fn order(k: &Key) -> (u8, i64) {
    match k {
        Key::Fid(f) => (0, *f),
        Key::Inserted(j, i) => (1, (*j as i64) << 32 | *i as i64),
    }
}

/// The mapping rows a tie's references make, set against the rows the file holds. A reference to an object the table
/// does not hold (and this write does not insert) is `relation_dangling`: a mapping row cannot name it, and a silent
/// drop would lose the link.
pub(super) fn plan_mapping(
    doc: &Document,
    tie: &Tie,
    mapping: &str,
    file_rows: &[(i64, i64)],
    child: &Side<'_>,
    parent: &Side<'_>,
) -> Result<Planned, Stop> {
    let mut rows: Vec<(Key, Vec<(Key, usize)>)> = Vec::new();
    for (uid, e) in doc.by_layer_with_uids(&tie.child_layer) {
        let Some(ck) = child.key(uid) else { continue };
        let ids = e
            .base()
            .attrs
            .get(&tie.field)
            .map(|v| ids_of(v))
            .unwrap_or_default();
        let mut targets = Vec::with_capacity(ids.len());
        for (n, id) in ids.iter().enumerate() {
            let target = Uuid::parse_str(id)
                .ok()
                .filter(|u| {
                    doc.slot_of(*u)
                        .and_then(|s| doc.get(s))
                        .is_some_and(|p| p.base().layer_id == tie.parent_layer)
                })
                .ok_or_else(|| {
                    refuse(
                        codes::RELATION_DANGLING,
                        format!(
                            "{uid} kayıtlı {} katmanı nesnesinin ‘{}’ alanı {id} kimliğini tutuyor; {} katmanında bu kimlikte nesne yok, eşleme satırı kurulamaz. Başvuruyu düzeltin.",
                            tie.child_layer, tie.field, tie.parent_layer
                        ),
                        "layer",
                    )
                })?;
            let pk = parent.key(target).ok_or_else(|| {
                refuse(
                    codes::RELATION_DANGLING,
                    format!(
                        "{uid} kayıtlı {} katmanı nesnesi {id} kimlikli {} nesnesini tutuyor; o {} tablosunda yok. Üst katmanı da birlikte yazın (with).",
                        tie.child_layer, tie.parent_layer, tie.parent.table
                    ),
                    "layer",
                )
            })?;
            targets.push((pk, n));
        }
        rows.push((ck, targets));
    }
    rows.sort_by_key(|(k, _)| order(k));
    let mut pairs: Vec<(Key, Key)> = Vec::new();
    for (ck, targets) in &rows {
        for (pk, _) in targets {
            pairs.push((*ck, *pk));
        }
    }
    // Rows of the file whose child or parent lies outside a part's area are kept: the layers do not hold them.
    let mut kept_rows: Vec<(Key, Key)> = Vec::new();
    for (b, r) in file_rows {
        if child.outside_fid(*b) || parent.outside_fid(*r) {
            kept_rows.push((Key::Fid(*b), Key::Fid(*r)));
        }
    }
    let wanted: HashSet<(i64, i64)> = pairs
        .iter()
        .chain(&kept_rows)
        .filter_map(|(b, r)| match (b, r) {
            (Key::Fid(b), Key::Fid(r)) => Some((*b, *r)),
            _ => None,
        })
        .collect();
    let inserted = pairs
        .iter()
        .any(|(b, r)| matches!(b, Key::Inserted(..)) || matches!(r, Key::Inserted(..)));
    let had: HashSet<(i64, i64)> = file_rows.iter().copied().collect();
    let added = pairs
        .iter()
        .filter(|(b, r)| match (b, r) {
            (Key::Fid(b), Key::Fid(r)) => !had.contains(&(*b, *r)),
            _ => true,
        })
        .count() as u32;
    let removed = had.iter().filter(|p| !wanted.contains(p)).count() as u32;
    pairs.extend(kept_rows);
    let edit = (inserted || added > 0 || removed > 0).then(|| MapEdit {
        mapping: mapping.to_owned(),
        pairs,
    });
    Ok(Planned {
        edit,
        added,
        removed,
    })
}

/// A child's references as the mapping rows hold them, by the child's fid: the parents' lasting ids in the rows'
/// order. A row naming a parent the table lacks is `relation_dangling`.
pub(super) fn read_references(
    rows: &[(i64, i64)],
    parent_table: &str,
    parent_fids: &HashSet<i64>,
    parent_uids: &HashMap<i64, String>,
    mapping: &str,
) -> Result<HashMap<i64, Vec<String>>, Stop> {
    let mut out: HashMap<i64, Vec<String>> = HashMap::new();
    for (b, r) in rows {
        if !parent_fids.contains(r) {
            return Err(refuse(
                codes::RELATION_DANGLING,
                format!(
                    "“{mapping}” eşleme tablosunun {b} → {r} satırı {parent_table} tablosunda olmayan bir satıra gidiyor; dosyadaki ilişki onarılmadan okunamaz."
                ),
                "layer",
            ));
        }
        let uid = parent_uids
            .get(r)
            .and_then(|u| Uuid::parse_str(u).ok())
            .unwrap_or_else(|| crate::geopackage::row_uid(parent_table, *r));
        let list = out.entry(*b).or_default();
        let text = uid.to_string();
        if !list.contains(&text) {
            list.push(text);
        }
    }
    Ok(out)
}
