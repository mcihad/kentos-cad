//! What a form reads of other layers (docs/adr/0404, docs/adr/0439): a field's list of values from
//! another layer and the objects of another layer that refer to a record.
//!
//! - A list is made once, when the form is prepared: the list layer's objects that the filter takes,
//!   each key once (the first object's label), in the labels' natural order or by a field of its
//!   layer (`sort`). A record narrows it by the chain (`matches`): an object is offered when each of
//!   its matched fields equals the record's (numbers by value, text exactly or folded); an empty
//!   record field offers nothing. A spatial list narrows it further by the record's geometry.
//! - `$uid` as the key writes the object's persistent id: the record refers to it.
//! - A code more than one object holds is a duplicate: the first object's is taken, and the record
//!   holding it is warned or refused as the list says.
//! - The chosen object's fields may be copied into the record's (`copy`), with a stamp of where they
//!   came from: the layer, the code, a fingerprint of the copied values and the day.
//! - A related item lists the other layer's objects whose field holds the record's persistent id.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{
    FieldChoice, FormLookup, FormPreviewValue, LOOKUP_UID, LayerField, LayerFieldKind,
    LookupDuplicates, LookupRelation, compare_decimals, display_value, js_trim,
};
use kentos_expression::js::text::fold_turkish;
use kentos_expression::{Unit, Variable, compile_with};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::spatial_query::{self, Geometry};

use crate::record::{Attrs, Object, holds, schema_in};

/// Another layer a form reads: its name, its fields and its objects (persistent id, attributes) and,
/// for a spatial list, their shapes in the same order (docs/adr/0439 §5; empty otherwise).
#[derive(Clone, Debug, Default)]
pub struct OtherLayer {
    pub name: String,
    pub fields: Vec<LayerField>,
    pub rows: Vec<(String, Attrs)>,
    pub shapes: Vec<Option<Arc<Shape>>>,
}

/// The other layers a form reads, by their ids.
pub type World = BTreeMap<String, OtherLayer>;

/// Why an object picked on the map gives a list no value (docs/adr/0759 §1), in the order asked.
#[derive(Clone, Debug, PartialEq)]
pub enum PickMiss {
    /// The object's key field is empty.
    NoKey,
    /// The list's filter leaves the object out.
    Filtered,
    /// A chained field of the record is empty: the record's fields named.
    ChainEmpty(String),
    /// The object does not match the record's chained fields, named.
    Chain(String),
    /// The object was not in its layer when the list was made.
    NotListed,
    /// A spatial list offers nothing to a record without geometry.
    NoShape,
    /// The object does not stand as the relation asks to the record; for a nearest list, the label
    /// of the value nearest instead.
    Apart {
        relation: LookupRelation,
        nearest: Option<String>,
    },
    /// The code's first object, the one the list holds, does not fit the record.
    Duplicate,
}

impl PickMiss {
    /// The reason's key, as the shared cases write it.
    pub fn key(&self) -> &'static str {
        match self {
            PickMiss::NoKey => "noKey",
            PickMiss::Filtered => "filtered",
            PickMiss::ChainEmpty(_) => "chainEmpty",
            PickMiss::Chain(_) => "chain",
            PickMiss::NotListed => "notListed",
            PickMiss::NoShape => "noShape",
            PickMiss::Apart { .. } => "apart",
            PickMiss::Duplicate => "duplicate",
        }
    }

    /// What it says, the reason and what to do.
    pub fn message(&self) -> String {
        match self {
            PickMiss::NoKey => {
                "Seçilen nesnenin anahtar alanı boş; listeye değer vermez. Başka bir nesne seçin."
                    .to_owned()
            }
            PickMiss::Filtered => {
                "Seçilen nesne listenin süzgecinden geçmiyor; listede olan bir nesne seçin."
                    .to_owned()
            }
            PickMiss::ChainEmpty(fields) => {
                format!("Önce {fields} doldurun: liste kaydın bu değerine göre daralır.")
            }
            PickMiss::Chain(fields) => {
                format!("Seçilen nesne kaydın {fields} değerine uymuyor; uyan bir nesne seçin.")
            }
            PickMiss::NotListed => {
                "Seçilen nesne form açıldığında katmanda yoktu; formu kapatıp yeniden açın."
                    .to_owned()
            }
            PickMiss::NoShape => {
                "Kaydın geometrisi yok; konuma göre daralan listeden seçilemez. Önce kaydı çizin."
                    .to_owned()
            }
            PickMiss::Apart { relation, nearest } => {
                let why = match relation {
                    LookupRelation::Contains => "kaydı içermiyor",
                    LookupRelation::Within => "kaydın içinde kalmıyor",
                    LookupRelation::Intersects => "kayıtla kesişmiyor",
                    LookupRelation::Nearest => "kayda en yakın değil",
                };
                match nearest {
                    Some(label) => format!("Seçilen nesne {why}; en yakını “{label}”."),
                    None => format!("Seçilen nesne {why}; listenin konumuna uyan bir nesne seçin."),
                }
            }
            PickMiss::Duplicate => {
                "Bu değeri taşıyan ilk nesne kayda uymuyor (yinelenen değer ilk nesnenindir); \
                                    katmandaki yinelenen değeri düzeltin."
                    .to_owned()
            }
        }
    }
}

/// What a picked object is judged by (docs/adr/0759 §1): the list's key, filter and matched fields.
#[derive(Clone, Debug)]
pub(crate) struct PickRule {
    by_uid: bool,
    key: Option<String>,
    filter: Option<kentos_expression::Expr>,
    matched: Vec<Option<String>>,
}

/// A value of a list: its code, its label, its matched fields' values, its columns shown, its copied
/// fields' values and its geometry for a spatial list.
#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub code: String,
    pub label: String,
    keys: Vec<Option<String>>,
    columns: Vec<String>,
    copies: Vec<Option<String>>,
    place: Option<Arc<Geometry>>,
}

/// A link of the chain: the record's field's name and how its values compare.
#[derive(Clone, Debug)]
struct Link {
    name: String,
    numeric: bool,
    fold: bool,
}

/// A field's list from another layer, made once.
#[derive(Clone, Debug)]
pub(crate) struct List {
    pub rows: Vec<Row>,
    chain: Vec<Link>,
    /// The codes more than one object holds, with how many hold each.
    duplicates: HashMap<String, usize>,
    pub policy: LookupDuplicates,
    /// The page's length.
    pub limit: Option<usize>,
    /// The columns' labels.
    columns: Vec<String>,
    /// The record's fields' names the copies write, in the copies' order, and the stamp's.
    copy_to: Vec<String>,
    stamp: Option<String>,
    /// The list layer's name, for the stamp.
    layer: String,
    /// The relation and the tolerance in metres.
    spatial: Option<(LookupRelation, f64)>,
    /// What a picked object is judged by (docs/adr/0759 §1).
    pick: PickRule,
}

impl List {
    /// The values the record is offered.
    pub(crate) fn offered<'a>(&'a self, attrs: &'a Attrs, object: &Object) -> Vec<&'a Row> {
        let wanted: Vec<Option<&str>> = self
            .chain
            .iter()
            .map(|l| {
                attrs
                    .get(&l.name)
                    .map(|v| js_trim(v))
                    .filter(|v| !v.is_empty())
            })
            .collect();
        if wanted.iter().any(Option::is_none) {
            return Vec::new();
        }
        let chained = self.rows.iter().filter(|r| {
            r.keys
                .iter()
                .zip(&wanted)
                .zip(&self.chain)
                .all(|((k, w), link)| k.as_deref().is_some_and(|k| same(link, k, w.unwrap_or(""))))
        });
        let Some((relation, tolerance)) = self.spatial else {
            return chained.collect();
        };
        let Some(record) = object.shape.as_deref().and_then(spatial_query::geometry_of) else {
            return Vec::new();
        };
        if relation == LookupRelation::Nearest {
            let mut best: Option<(f64, &Row)> = None;
            for r in chained {
                let Some(place) = &r.place else { continue };
                let d = spatial_query::distance(place, &record);
                if (tolerance > 0.0 && d > tolerance) || best.is_some_and(|(b, _)| d >= b) {
                    continue;
                }
                best = Some((d, r));
            }
            return best.map(|(_, r)| r).into_iter().collect();
        }
        chained
            .filter(|r| {
                r.place
                    .as_deref()
                    .is_some_and(|place| stands(relation, tolerance, place, &record))
            })
            .collect()
    }

    /// The value the object `uid` (its attributes `values`) picked on the map gives the record, as
    /// the list would offer it (docs/adr/0759 §1): its code and label, else why not.
    pub(crate) fn picked(
        &self,
        uid: &str,
        values: &Attrs,
        attrs: &Attrs,
        object: &Object,
    ) -> Result<(String, String), PickMiss> {
        let rule = &self.pick;
        let Some(code) = value(uid, values, rule.key.as_deref(), rule.by_uid) else {
            return Err(PickMiss::NoKey);
        };
        if rule
            .filter
            .as_ref()
            .is_some_and(|e| !holds(e, values, &Object::default()))
        {
            return Err(PickMiss::Filtered);
        }
        let named = |links: &[&Link]| {
            links
                .iter()
                .map(|l| format!("“{}”", l.name))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let empty: Vec<&Link> = self
            .chain
            .iter()
            .filter(|l| attrs.get(&l.name).is_none_or(|v| js_trim(v).is_empty()))
            .collect();
        if !empty.is_empty() {
            return Err(PickMiss::ChainEmpty(named(&empty)));
        }
        let apart: Vec<&Link> = self
            .chain
            .iter()
            .zip(&rule.matched)
            .filter(|(l, name)| {
                let wanted = attrs.get(&l.name).map_or("", |v| js_trim(v));
                !value(uid, values, name.as_deref(), false).is_some_and(|k| same(l, k, wanted))
            })
            .map(|(l, _)| l)
            .collect();
        if !apart.is_empty() {
            return Err(PickMiss::Chain(named(&apart)));
        }
        if self.row(code).is_none() {
            return Err(PickMiss::NotListed);
        }
        let offered = self.offered(attrs, object);
        if let Some(r) = offered.iter().find(|r| r.code == code) {
            return Ok((r.code.clone(), r.label.clone()));
        }
        if let Some((relation, _)) = self.spatial {
            if object.shape.is_none() {
                return Err(PickMiss::NoShape);
            }
            let nearest = (relation == LookupRelation::Nearest)
                .then(|| offered.first().map(|r| r.label.clone()))
                .flatten();
            return Err(PickMiss::Apart { relation, nearest });
        }
        Err(PickMiss::Duplicate)
    }

    /// The record's offered values as choices, a value's columns its description.
    pub(crate) fn choices(&self, attrs: &Attrs, object: &Object) -> Vec<FieldChoice> {
        self.offered(attrs, object)
            .into_iter()
            .map(|r| FieldChoice {
                code: r.code.clone(),
                label: r.label.clone(),
                retired: false,
                description: described(&r.columns),
                from: None,
                until: None,
            })
            .collect()
    }

    /// The first object holding `code`.
    pub(crate) fn row(&self, code: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.code == code)
    }

    /// The chosen object's columns, each with its label (docs/adr/0439 §2); none without columns or
    /// a value in the list.
    pub(crate) fn preview(&self, code: &str) -> Option<Vec<FormPreviewValue>> {
        if self.columns.is_empty() {
            return None;
        }
        let row = self.row(code)?;
        Some(
            self.columns
                .iter()
                .zip(&row.columns)
                .map(|(label, value)| FormPreviewValue {
                    label: label.clone(),
                    value: value.clone(),
                })
                .collect(),
        )
    }

    /// How many objects hold `code`, when more than one does.
    pub(crate) fn duplicated(&self, code: &str) -> Option<usize> {
        self.duplicates.get(code).copied()
    }

    /// The code a typed text names: the text, or the code of the one value labelled so.
    pub(crate) fn code_of<'a>(&'a self, text: &'a str) -> &'a str {
        let t = js_trim(text);
        if self.rows.iter().any(|r| r.code == t) {
            return t;
        }
        self.rows
            .iter()
            .find(|r| r.label == t)
            .map_or(t, |r| r.code.as_str())
    }

    /// The record's fields' names the list copies into or stamps.
    pub(crate) fn targets(&self) -> impl Iterator<Item = &str> {
        self.copy_to.iter().chain(&self.stamp).map(String::as_str)
    }

    /// Whether the list copies or stamps.
    pub(crate) fn copies(&self) -> bool {
        !self.copy_to.is_empty() || self.stamp.is_some()
    }

    /// The record `attrs` with the chosen object's copied fields and the stamp written, the field
    /// named `field` holding `code` (docs/adr/0439 §3); a stamp that already names the code and the
    /// copied values is kept (its day with it).
    pub(crate) fn copy_into(&self, attrs: &mut Attrs, code: &str, today: Option<&str>) {
        let Some(row) = self.row(code) else { return };
        for (to, value) in self.copy_to.iter().zip(&row.copies) {
            match value {
                Some(v) => {
                    attrs.insert(to.clone(), v.clone());
                }
                None => {
                    attrs.remove(to);
                }
            }
        }
        if let Some(stamp) = &self.stamp {
            let print = fingerprint(&row.copies);
            let kept = attrs
                .get(stamp)
                .is_some_and(|s| stamp_names(s, code, &print));
            if !kept {
                let mut text = format!("{} · {code} · #{print}", self.layer);
                if let Some(day) = today {
                    text.push_str(" · ");
                    text.push_str(day);
                }
                attrs.insert(stamp.clone(), text);
            }
        }
    }

    /// Whether the list stamps and the record's stamp does not name `code`.
    pub(crate) fn stale_stamp(&self, attrs: &Attrs, code: &str) -> bool {
        self.stamp.as_ref().is_some_and(|s| {
            attrs.get(s).is_none_or(|text| {
                let mut parts = text.split(" · ");
                let _ = parts.next();
                parts.next() != Some(code)
            })
        })
    }

    /// Whether the record's stamp names `code` with copied values the source no longer has: the
    /// source changed after the copy (docs/adr/0439 §3).
    pub(crate) fn stale(&self, attrs: &Attrs, code: &str) -> bool {
        let (Some(stamp), Some(row)) = (&self.stamp, self.row(code)) else {
            return false;
        };
        let Some(text) = attrs.get(stamp) else {
            return false;
        };
        let mut parts = text.split(" · ");
        let (_, held, print) = (parts.next(), parts.next(), parts.next());
        held == Some(code)
            && print.is_some_and(|p| p.strip_prefix('#') != Some(fingerprint(&row.copies).as_str()))
    }
}

/// Whether the stamp `text` names `code` and the fingerprint `print`.
fn stamp_names(text: &str, code: &str, print: &str) -> bool {
    let mut parts = text.split(" · ");
    let _ = parts.next();
    parts.next() == Some(code) && parts.next().and_then(|p| p.strip_prefix('#')) == Some(print)
}

/// The copied values' fingerprint: FNV-1a (32 bits) of their texts, an empty one as nothing, each
/// ended by U+001F, in eight lower-case hexadecimal digits.
pub(crate) fn fingerprint(values: &[Option<String>]) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for v in values {
        for b in v.as_deref().unwrap_or("").bytes().chain([0x1f]) {
            h ^= u32::from(b);
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    format!("{h:08x}")
}

/// A value's columns as one description: the filled ones joined by “ · ”; none when all are empty.
fn described(columns: &[String]) -> Option<String> {
    let shown: Vec<&str> = columns
        .iter()
        .map(String::as_str)
        .filter(|c| !c.is_empty())
        .collect();
    (!shown.is_empty()).then(|| shown.join(" · "))
}

/// Whether a list's value `k` equals the record's `w` by the link's rule.
fn same(link: &Link, k: &str, w: &str) -> bool {
    if link.numeric {
        return compare_decimals(k, w) == Some(Ordering::Equal);
    }
    if link.fold {
        return folded(k) == folded(w);
    }
    k == w
}

/// A text folded: trimmed, Turkish letters folded, lower case.
fn folded(text: &str) -> String {
    fold_turkish(js_trim(text)).to_lowercase()
}

/// Whether the list's object `place` stands in `relation` to the record's geometry.
fn stands(relation: LookupRelation, tolerance: f64, place: &Geometry, record: &Geometry) -> bool {
    let (a, b) = (&place.bounds, &record.bounds);
    let apart = a.min_x - tolerance > b.max_x
        || b.min_x - tolerance > a.max_x
        || a.min_y - tolerance > b.max_y
        || b.min_y - tolerance > a.max_y;
    if apart {
        return false;
    }
    match relation {
        LookupRelation::Contains => spatial_query::contains(place, record),
        LookupRelation::Within => spatial_query::contains(record, place),
        LookupRelation::Intersects if tolerance > 0.0 => {
            spatial_query::distance(place, record) <= tolerance
        }
        LookupRelation::Intersects => spatial_query::intersects(place, record),
        LookupRelation::Nearest => true,
    }
}

/// A layer field's name by its key.
fn name_of<'a>(fields: &'a [LayerField], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|f| f.key() == key)
        .map(|f| f.name.as_str())
}

/// A layer field by its key.
fn field_of<'a>(fields: &'a [LayerField], key: &str) -> Option<&'a LayerField> {
    fields.iter().find(|f| f.key() == key)
}

/// The value of an object's field (`$uid`: its persistent id), trimmed; none when empty.
fn value<'a>(uid: &'a str, attrs: &'a Attrs, name: Option<&str>, by_uid: bool) -> Option<&'a str> {
    let v = if by_uid {
        uid
    } else {
        attrs.get(name?).map_or("", String::as_str)
    };
    let v = js_trim(v);
    (!v.is_empty()).then_some(v)
}

/// Whether a field's kind is a number's.
fn numeric(kind: LayerFieldKind) -> bool {
    matches!(kind, LayerFieldKind::Integer | LayerFieldKind::Decimal)
}

/// Two sort values in the list's order: numbers by value, dates as written, text naturally; an empty
/// one after a filled one whichever the direction.
fn sort_order(
    numbers: bool,
    plain: bool,
    descending: bool,
    a: Option<&str>,
    b: Option<&str>,
) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => {
            let o = if numbers {
                compare_decimals(a, b).unwrap_or_else(|| a.cmp(b))
            } else if plain {
                a.cmp(b)
            } else {
                kentos_geometry_core::text::natural::natural_cmp(a, b)
            };
            if descending { o.reverse() } else { o }
        }
    }
}

/// The list of `lookup` over the record's layer `fields` (the preflight said it holds); none when
/// its layer is not in the world.
pub(crate) fn list_of(
    lookup: &FormLookup,
    fields: &[LayerField],
    world: &World,
    variables: &[Variable],
    unit: Unit,
) -> Option<List> {
    let other = world.get(&lookup.layer)?;
    let by_uid = lookup.key == LOOKUP_UID;
    let key = name_of(&other.fields, &lookup.key);
    let label = lookup
        .label
        .as_deref()
        .map(|l| (l == LOOKUP_UID, name_of(&other.fields, l)));
    let matched: Vec<Option<&str>> = lookup
        .matches
        .iter()
        .map(|m| name_of(&other.fields, &m.field))
        .collect();
    // A column or a copy by key: `$uid` is the object's id, a field its value (shown as people read it).
    let columns: Vec<(bool, Option<&LayerField>)> = lookup
        .columns
        .iter()
        .map(|c| (c == LOOKUP_UID, field_of(&other.fields, c)))
        .collect();
    let copies: Vec<(bool, Option<&str>)> = lookup
        .copy
        .iter()
        .map(|c| (c.from == LOOKUP_UID, name_of(&other.fields, &c.from)))
        .collect();
    let sort = lookup.sort.as_ref().map(|s| {
        let f = field_of(&other.fields, &s.field);
        (
            s.field == LOOKUP_UID,
            f.map(|f| f.name.as_str()),
            f.is_some_and(|f| numeric(f.kind)),
            f.is_some_and(|f| f.kind.is_temporal()),
            s.descending,
        )
    });
    let filter = lookup
        .filter
        .as_deref()
        .and_then(|src| compile_with(src, &schema_in(&other.fields, variables, unit)).ok());
    // A world without the layer's shapes (a host that read none) does not narrow by geometry; the server reads
    // them around the written records (docs/adr/0759 §2).
    let spatial = lookup
        .spatial
        .as_ref()
        .filter(|_| other.shapes.len() == other.rows.len())
        .map(|s| {
            (
                s.relation,
                s.tolerance.unwrap_or(0.0).max(0.0) / unit.factor(),
            )
        });
    let none = Object::default();
    let mut seen = HashSet::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut rows: Vec<(Row, Option<String>)> = Vec::new();
    for (i, (uid, attrs)) in other.rows.iter().enumerate() {
        let Some(code) = value(uid, attrs, key, by_uid) else {
            continue;
        };
        if filter.as_ref().is_some_and(|e| !holds(e, attrs, &none)) {
            continue;
        }
        *counts.entry(code.to_owned()).or_default() += 1;
        if !seen.insert(code) {
            continue;
        }
        let shown = match label {
            Some((l_uid, l_name)) => value(uid, attrs, l_name, l_uid).unwrap_or(code),
            None => code,
        };
        let keys = matched
            .iter()
            .map(|name| value(uid, attrs, *name, false).map(str::to_owned))
            .collect();
        let columns = columns
            .iter()
            .map(|(by, f)| match (by, f) {
                (true, _) => uid.clone(),
                (false, Some(f)) => value(uid, attrs, Some(&f.name), false)
                    .map_or_else(String::new, |v| display_value(f, v)),
                (false, None) => String::new(),
            })
            .collect();
        let copied = copies
            .iter()
            .map(|(by, name)| value(uid, attrs, *name, *by).map(str::to_owned))
            .collect();
        let place = spatial.and_then(|_| {
            other
                .shapes
                .get(i)
                .and_then(Option::as_deref)
                .and_then(spatial_query::geometry_of)
                .map(Arc::new)
        });
        let by = sort
            .and_then(|(s_uid, s_name, ..)| value(uid, attrs, s_name, s_uid).map(str::to_owned));
        rows.push((
            Row {
                code: code.to_owned(),
                label: shown.to_owned(),
                keys,
                columns,
                copies: copied,
                place,
            },
            by,
        ));
    }
    let natural = |a: &Row, b: &Row| {
        kentos_geometry_core::text::natural::natural_cmp(&a.label, &b.label)
            .then_with(|| a.code.cmp(&b.code))
    };
    match sort {
        Some((_, _, numbers, plain, descending)) => rows.sort_by(|(ra, a), (rb, b)| {
            sort_order(numbers, plain, descending, a.as_deref(), b.as_deref())
                .then_with(|| natural(ra, rb))
        }),
        None => rows.sort_by(|(a, _), (b, _)| natural(a, b)),
    }
    // The chain's links and, in the same order, the list layer's field each reads.
    let (chain, picked_names): (Vec<Link>, Vec<Option<String>>) = lookup
        .matches
        .iter()
        .filter_map(|m| {
            let ours = field_of(fields, &m.record)?;
            let theirs = field_of(&other.fields, &m.field);
            Some((
                Link {
                    name: ours.name.clone(),
                    numeric: numeric(ours.kind) && theirs.is_some_and(|t| numeric(t.kind)),
                    fold: m.fold,
                },
                theirs.map(|t| t.name.clone()),
            ))
        })
        .unzip();
    counts.retain(|_, n| *n > 1);
    Some(List {
        rows: rows.into_iter().map(|(r, _)| r).collect(),
        chain,
        duplicates: counts,
        policy: lookup.duplicates,
        limit: lookup.limit.map(|n| n as usize),
        columns: lookup
            .columns
            .iter()
            .map(|c| {
                field_of(&other.fields, c)
                    .map_or_else(|| "Kimlik".to_owned(), |f| f.label().to_owned())
            })
            .collect(),
        copy_to: lookup
            .copy
            .iter()
            .filter_map(|c| name_of(fields, &c.to).map(str::to_owned))
            .collect(),
        stamp: lookup
            .stamp
            .as_deref()
            .and_then(|s| name_of(fields, s))
            .map(str::to_owned),
        layer: other.name.clone(),
        spatial,
        pick: PickRule {
            by_uid,
            key: key.map(str::to_owned),
            filter,
            matched: picked_names,
        },
    })
}

/// Each code of `lookup`'s layer with its label (docs/adr/0404 §2), for showing a stored value:
/// every object, filter and chain aside, the first object's label for a code; none when the layer
/// is not in the world.
pub fn labels_of(lookup: &FormLookup, world: &World) -> BTreeMap<String, String> {
    let Some(other) = world.get(&lookup.layer) else {
        return BTreeMap::new();
    };
    let by_uid = lookup.key == LOOKUP_UID;
    let key = name_of(&other.fields, &lookup.key);
    let label = lookup
        .label
        .as_deref()
        .map(|l| (l == LOOKUP_UID, name_of(&other.fields, l)));
    let mut out = BTreeMap::new();
    for (uid, attrs) in &other.rows {
        let Some(code) = value(uid, attrs, key, by_uid) else {
            continue;
        };
        if out.contains_key(code) {
            continue;
        }
        let shown = match label {
            Some((l_uid, l_name)) => value(uid, attrs, l_name, l_uid).unwrap_or(code),
            None => code,
        };
        out.insert(code.to_owned(), shown.to_owned());
    }
    out
}

/// A related item's objects, made once: each object's persistent id and the id its field holds.
#[derive(Clone, Debug)]
pub(crate) struct Related {
    pub rows: Vec<(String, String)>,
}

impl Related {
    /// The objects that refer to the record, in the layer's order.
    pub(crate) fn of(&self, object: &Object) -> Vec<String> {
        let Some(uid) = object.uid.as_deref() else {
            return Vec::new();
        };
        // A many-to-many field holds several ids (docs/adr/0409 §1); an id has no `;`.
        self.rows
            .iter()
            .filter(|(_, held)| held.split(';').any(|h| js_trim(h) == uid))
            .map(|(own, _)| own.clone())
            .collect()
    }
}

/// The related item over `layer`'s `field`; none when the layer is not in the world.
pub(crate) fn related_of(layer: &str, field: &str, world: &World) -> Option<Related> {
    let other = world.get(layer)?;
    let name = name_of(&other.fields, field);
    Some(Related {
        rows: other
            .rows
            .iter()
            .filter_map(|(uid, attrs)| {
                value(uid, attrs, name, false).map(|v| (uid.clone(), v.to_owned()))
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let row = |uid: &str, code: &str| {
            (
                uid.to_owned(),
                [("Kod".to_owned(), code.to_owned())].into_iter().collect(),
            )
        };
        let mut w = World::new();
        w.insert(
            "bina".into(),
            OtherLayer {
                name: "Bina".into(),
                fields: vec![
                    serde_json::from_value(serde_json::json!({ "name": "Kod", "kind": "text" }))
                        .expect("a field"),
                ],
                rows: vec![
                    row("u1", "B-1"),
                    row("u2", " "),
                    row("u3", "B-3"),
                    row("u1", "B-9"),
                ],
                shapes: Vec::new(),
            },
        );
        w
    }

    /// docs/adr/0410 §4: a stored code shown by its label, the first object's; an empty label shows
    /// the code; a layer not in the world gives none.
    #[test]
    fn codes_are_labelled_by_the_first_object() {
        let lookup = FormLookup {
            layer: "bina".into(),
            key: LOOKUP_UID.into(),
            label: Some("Kod".into()),
            ..FormLookup::default()
        };
        let labels = labels_of(&lookup, &world());
        assert_eq!(labels.get("u1").map(String::as_str), Some("B-1"));
        assert_eq!(labels.get("u2").map(String::as_str), Some("u2"));
        assert_eq!(labels.get("u3").map(String::as_str), Some("B-3"));
        let elsewhere = FormLookup {
            layer: "yok".into(),
            ..lookup
        };
        assert!(labels_of(&elsewhere, &world()).is_empty());
    }
}
