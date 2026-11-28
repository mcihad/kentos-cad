//! Eleman tipleri (docs/adr/8106; plan/bim-plan.md BIM-12): the project's types of columns, beams, slabs, roofs,
//! railings and stairs, in the order of wall styles and door and window types. A type holds the values of its kind's
//! type fields; an element that follows it takes them, but those it keeps as its own (its exceptions), and carries the
//! version it took. The values are the element's own fields by their names on the wire, so one rule serves every kind:
//! a field the type does not give is that field's absence (its default). Where an element is placed (its points, its
//! level, its turn) and what it is called (its tag) stay its own.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Entity;

/// The most types a project keeps.
pub const MAX_ELEMENT_TYPES: usize = 1_000;
/// The longest name a type takes, in letters.
pub const MAX_ELEMENT_TYPE_NAME: usize = 100;

/// What an element type is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ElementTypeKind {
    Column,
    Beam,
    Slab,
    Roof,
    Railing,
    Stair,
}

/// One of a kind's type fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeField {
    /// Its name on the wire, the element's own field's (`width`, `riserHeight`).
    pub key: &'static str,
    /// Its name in the interface, as Öznitelikler writes the element's row.
    pub label: &'static str,
    /// A type must give it: the element has none without it.
    pub required: bool,
    /// Its value is the type's name, not one of its values (a column's `type`, its type's name).
    pub named: bool,
}

const fn field(key: &'static str, label: &'static str) -> TypeField {
    TypeField { key, label, required: false, named: false }
}

const fn must(key: &'static str, label: &'static str) -> TypeField {
    TypeField { key, label, required: true, named: false }
}

const fn named(key: &'static str, label: &'static str) -> TypeField {
    TypeField { key, label, required: false, named: true }
}

/// A column's: what it is (a post), its outline and size, its own height, its fill and material; its name is its
/// type's. Its point, turn, anchor, grids and tag are its own.
const COLUMN: &[TypeField] = &[
    field("member", "Tür"),
    field("shape", "Biçim"),
    must("width", "En"),
    field("depth", "Boy"),
    field("legs", "Kol kalınlıkları"),
    field("height", "Yükseklik"),
    field("fill", "Dolgu"),
    field("material", "Malzeme"),
    named("type", "Kolon türü"),
];

/// A beam's: what it is (a lintel, a bond beam, a rail), its section and material. Its axis, justification, heights
/// over its storey and tag are its own.
const BEAM: &[TypeField] = &[
    field("role", "Tür"),
    field("member", "Kayıt"),
    must("width", "Genişlik"),
    must("height", "Yükseklik"),
    field("material", "Malzeme"),
];

/// A slab's: what it is (a covering's type too), its build-up and edge. Its outline, top, slope, source and whether
/// the quantities count it are its own.
const SLAB: &[TypeField] = &[
    field("role", "Döşeme türü"),
    field("covering", "Kaplama tipi"),
    must("thickness", "Kalınlık"),
    field("materialId", "Proje malzemesi"),
    field("material", "Malzeme"),
    field("edge", "Kenar"),
    field("layers", "Katmanlar"),
];

/// A roof's: its build-up, covering, gutter and snow guard. Its eaves line, slopes, height, overhang and parts are
/// its own.
const ROOF: &[TypeField] = &[
    field("thickness", "Kalınlık"),
    field("material", "Kaplama"),
    field("layers", "Katmanlar"),
    field("gutter", "Oluk"),
    field("snowGuard", "Kar siperliği"),
];

/// A railing's: its height, infill, gap and top rail. Its line and its floor's height are its own.
const RAILING: &[TypeField] = &[
    must("height", "Yükseklik"),
    field("infill", "Dolgu"),
    field("gap", "Çubuk aralığı"),
    must("width", "Küpeşte eni"),
];

/// A stair's: its width, tread, landings, winders, centre post, railings and numbering. Its walking line, risers and
/// their height (its storey's) and cut line are its own.
const STAIR: &[TypeField] = &[
    must("width", "Genişlik"),
    must("tread", "Basamak"),
    field("landing", "Sahanlık"),
    field("winders", "Balansman basamağı"),
    field("post", "Merkez direği"),
    field("railings", "Korkuluk"),
    field("numbering", "Basamak numaraları"),
];

impl ElementTypeKind {
    pub const ALL: [ElementTypeKind; 6] = [
        ElementTypeKind::Column,
        ElementTypeKind::Beam,
        ElementTypeKind::Slab,
        ElementTypeKind::Roof,
        ElementTypeKind::Railing,
        ElementTypeKind::Stair,
    ];

    /// Its name on the wire.
    pub fn name(self) -> &'static str {
        match self {
            ElementTypeKind::Column => "column",
            ElementTypeKind::Beam => "beam",
            ElementTypeKind::Slab => "slab",
            ElementTypeKind::Roof => "roof",
            ElementTypeKind::Railing => "railing",
            ElementTypeKind::Stair => "stair",
        }
    }

    /// The element's name, as a list heads it.
    pub fn label(self) -> &'static str {
        match self {
            ElementTypeKind::Column => "Kolon",
            ElementTypeKind::Beam => "Kiriş",
            ElementTypeKind::Slab => "Döşeme",
            ElementTypeKind::Roof => "Çatı",
            ElementTypeKind::Railing => "Korkuluk",
            ElementTypeKind::Stair => "Merdiven",
        }
    }

    /// The element's name in a sentence.
    pub fn words(self) -> &'static str {
        match self {
            ElementTypeKind::Column => "kolon",
            ElementTypeKind::Beam => "kiriş",
            ElementTypeKind::Slab => "döşeme",
            ElementTypeKind::Roof => "çatı",
            ElementTypeKind::Railing => "korkuluk",
            ElementTypeKind::Stair => "merdiven",
        }
    }

    /// The kind of `e`: none for an element no type serves.
    pub fn of(e: &Entity) -> Option<Self> {
        match e {
            Entity::Column(_) => Some(ElementTypeKind::Column),
            Entity::Beam(_) => Some(ElementTypeKind::Beam),
            Entity::Slab(_) => Some(ElementTypeKind::Slab),
            Entity::Roof(_) => Some(ElementTypeKind::Roof),
            Entity::Railing(_) => Some(ElementTypeKind::Railing),
            Entity::Stair(_) => Some(ElementTypeKind::Stair),
            _ => None,
        }
    }

    /// Its type fields, in the order Öznitelikler and the type window show them.
    pub fn fields(self) -> &'static [TypeField] {
        match self {
            ElementTypeKind::Column => COLUMN,
            ElementTypeKind::Beam => BEAM,
            ElementTypeKind::Slab => SLAB,
            ElementTypeKind::Roof => ROOF,
            ElementTypeKind::Railing => RAILING,
            ElementTypeKind::Stair => STAIR,
        }
    }

    /// Its type field `key`.
    pub fn field(self, key: &str) -> Option<&'static TypeField> {
        self.fields().iter().find(|f| f.key == key)
    }

    /// A plain element of the kind, the ground a type's values are tried on.
    fn probe(self) -> Value {
        let p = |x: f64, y: f64| serde_json::json!({ "x": x, "y": y });
        match self {
            ElementTypeKind::Column => serde_json::json!({ "at": p(0.0, 0.0), "width": 0.3 }),
            ElementTypeKind::Beam => {
                serde_json::json!({ "pts": [p(0.0, 0.0), p(5.0, 0.0)], "width": 0.25, "height": 0.5 })
            }
            ElementTypeKind::Slab => serde_json::json!({
                "parts": [{ "outer": { "pts": [p(0.0, 0.0), p(5.0, 0.0), p(5.0, 5.0), p(0.0, 5.0)] } }],
                "thickness": 0.15
            }),
            ElementTypeKind::Roof => serde_json::json!({
                "pts": [p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)],
                "slopes": [30.0, null, 30.0, null]
            }),
            ElementTypeKind::Railing => {
                serde_json::json!({ "pts": [p(0.0, 0.0), p(3.0, 0.0)], "height": 1.1, "width": 0.05 })
            }
            ElementTypeKind::Stair => serde_json::json!({
                "pts": [p(0.0, 0.0), p(3.75, 0.0)],
                "width": 1.2,
                "risers": 16,
                "riserHeight": 0.175,
                "tread": 0.25
            }),
        }
    }
}

/// A project's element type (docs/adr/8106 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElementType {
    /// Its lasting key: lower-case letters, digits and `-`, at most 64, not starting with `kentos-`.
    pub id: String,
    /// What it is called (S1, Döşeme 15): unique among its kind's types, case aside.
    pub name: String,
    /// The elements it serves.
    pub kind: ElementTypeKind,
    /// Each change gives it the next; an element that took an older one is behind (eski sürüm).
    #[cfg_attr(feature = "schema", schemars(range(min = 1), extend("x-unit" = "count")))]
    pub version: u32,
    /// Its values: its kind's type fields by their names on the wire (`width`, `material` …), each as the element
    /// writes it; a field it does not give is that field's absence (a column's rectangle, a slab's lone structure).
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    pub values: Map<String, Value>,
}

/// The type an element follows (docs/adr/8106 §1): its id, the version it took and the fields it keeps its own value
/// of (its exceptions), in its kind's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TypeLink {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(range(min = 1), extend("x-unit" = "count")))]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub overrides: Vec<String>,
}

type Problem = Option<(String, String)>;

/// Whether `name` may be a type's name: not blank, no spaces at its ends, no control letters, at most
/// [`MAX_ELEMENT_TYPE_NAME`].
pub fn element_type_name_ok(name: &str) -> bool {
    !name.trim().is_empty()
        && name.trim() == name
        && name.chars().count() <= MAX_ELEMENT_TYPE_NAME
        && !name.chars().any(char::is_control)
}

impl ElementType {
    /// What is wrong with it: its id, name or version, a value of a field its kind has not or names, a required field
    /// not given, or values that make no element of its kind (its kind's own check on a plain one).
    pub fn problem(&self) -> Problem {
        if !crate::opening_type_id_ok(&self.id) {
            return Some((
                "id".into(),
                format!(
                    "“{}” tip kimliği olamaz: küçük harf, rakam ve tire; en çok 64 harf; “kentos-” ile başlamaz.",
                    self.id
                ),
            ));
        }
        if !element_type_name_ok(&self.name) {
            return Some((
                "name".into(),
                format!(
                    "“{}” tip adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok \
                     {MAX_ELEMENT_TYPE_NAME} harf.",
                    self.name
                ),
            ));
        }
        if self.version == 0 {
            return Some(("version".into(), "Tipin sürümü 1'den başlar.".into()));
        }
        let kind = self.kind;
        for (key, value) in &self.values {
            match kind.field(key) {
                None => {
                    let known: Vec<&str> = kind.fields().iter().filter(|f| !f.named).map(|f| f.key).collect();
                    return Some((
                        format!("values.{key}"),
                        format!(
                            "“{key}” {} tipinin alanı değil; tip yalnız şunları verir: {}.",
                            kind.words(),
                            known.join(", ")
                        ),
                    ));
                }
                Some(f) if f.named => {
                    return Some((
                        format!("values.{key}"),
                        format!("{} tipin adıdır; değerlerde verilmez, tipin adı değişir.", f.label),
                    ));
                }
                Some(f) if value.is_null() => {
                    return Some((
                        format!("values.{key}"),
                        format!("“{}” değeri boş olamaz; tip bu alanı vermiyorsa alan hiç yazılmaz.", f.label),
                    ));
                }
                Some(_) => {}
            }
        }
        if let Some(f) = kind.fields().iter().find(|f| f.required && !self.values.contains_key(f.key)) {
            return Some((
                format!("values.{}", f.key),
                format!("{} tipi {} ({}) değerini vermeli.", kind.label(), f.label, f.key),
            ));
        }
        let Value::Object(mut probe) = kind.probe() else {
            return None;
        };
        for f in kind.fields().iter().filter(|f| !f.named) {
            match self.values.get(f.key) {
                Some(v) => {
                    probe.insert(f.key.into(), v.clone());
                }
                None => {
                    probe.remove(f.key);
                }
            }
        }
        match fields_problem(kind, probe) {
            Ok(None) => None,
            Ok(Some((at, why))) => Some((format!("values.{at}"), why)),
            Err(why) => Some(("values".into(), format!("{} tipinin değerleri okunmadı: {why}", kind.label()))),
        }
    }

    /// Its value of `field`: the type's name for the named field, else its own (none: the field's absence).
    pub fn value(&self, field: &TypeField) -> Option<Value> {
        if field.named {
            return Some(Value::String(self.name.clone()));
        }
        self.values.get(field.key).cloned()
    }

    /// Its values as an element writes them (docs/adr/8106 §1): each read into its kind's field and written back, so a
    /// field given its default (a column's `fill: false`) is left out and a number is written as the element writes
    /// it (`1` as `1.0`). None when they make no element.
    pub fn canonical_values(&self) -> Option<Map<String, Value>> {
        let Value::Object(mut probe) = self.kind.probe() else {
            return None;
        };
        for f in self.kind.fields().iter().filter(|f| !f.named) {
            match self.values.get(f.key) {
                Some(v) => {
                    probe.insert(f.key.into(), v.clone());
                }
                None => {
                    probe.remove(f.key);
                }
            }
        }
        let written = written_fields(self.kind, probe)?;
        Some(
            self.kind
                .fields()
                .iter()
                .filter(|f| !f.named)
                .filter_map(|f| written.get(f.key).map(|v| (f.key.to_owned(), v.clone())))
                .collect(),
        )
    }

    /// Its value of each of its kind's fields as an element following it writes it, by the fields' order.
    fn written(&self) -> Vec<Option<Value>> {
        let canon = self.canonical_values().unwrap_or_else(|| self.values.clone());
        self.kind
            .fields()
            .iter()
            .map(|f| if f.named { Some(Value::String(self.name.clone())) } else { canon.get(f.key).cloned() })
            .collect()
    }
}

impl TypeLink {
    /// What is wrong with it on an element of `kind`: its id, its version, an exception its kind has not or given
    /// twice.
    pub fn problem(&self, kind: ElementTypeKind) -> Problem {
        if !crate::opening_type_id_ok(&self.id) {
            return Some(("elementType.id".into(), format!("“{}” tip kimliği olamaz.", self.id)));
        }
        if self.version == 0 {
            return Some(("elementType.version".into(), "Tipin sürümü 1'den başlar.".into()));
        }
        for (k, key) in self.overrides.iter().enumerate() {
            if kind.field(key).is_none() {
                return Some((
                    format!("elementType.overrides[{k}]"),
                    format!("“{key}” {} tipinin alanı değil.", kind.words()),
                ));
            }
            if self.overrides[..k].contains(key) {
                return Some((format!("elementType.overrides[{k}]"), format!("“{key}” iki kez yazılmış.")));
            }
        }
        None
    }
}

/// The element's own fields as an object, with its kind.
fn own_object(e: &Entity) -> Option<(ElementTypeKind, Map<String, Value>)> {
    let (kind, value) = match e {
        Entity::Column(x) => (ElementTypeKind::Column, serde_json::to_value(&x.column)),
        Entity::Beam(x) => (ElementTypeKind::Beam, serde_json::to_value(&x.beam)),
        Entity::Slab(x) => (ElementTypeKind::Slab, serde_json::to_value(&x.slab)),
        Entity::Roof(x) => (ElementTypeKind::Roof, serde_json::to_value(&x.roof)),
        Entity::Railing(x) => (ElementTypeKind::Railing, serde_json::to_value(&x.railing)),
        Entity::Stair(x) => (ElementTypeKind::Stair, serde_json::to_value(&x.stair)),
        _ => return None,
    };
    match value {
        Ok(Value::Object(m)) => Some((kind, m)),
        _ => None,
    }
}

/// `e` with its own fields `m`.
fn with_object(e: &Entity, m: Map<String, Value>) -> Result<Entity, String> {
    let v = Value::Object(m);
    let mut out = e.clone();
    let done = match &mut out {
        Entity::Column(x) => serde_json::from_value(v).map(|f| x.column = f),
        Entity::Beam(x) => serde_json::from_value(v).map(|f| x.beam = f),
        Entity::Slab(x) => serde_json::from_value(v).map(|f| x.slab = f),
        Entity::Roof(x) => serde_json::from_value(v).map(|f| x.roof = f),
        Entity::Railing(x) => serde_json::from_value(v).map(|f| x.railing = f),
        Entity::Stair(x) => serde_json::from_value(v).map(|f| x.stair = f),
        _ => return Err("bu nesnenin tipi olmaz".into()),
    };
    done.map_err(|e| e.to_string())?;
    Ok(out)
}

/// The kind's own check of fields `m`: Ok(None) when they make an element, its problem, or why they do not read.
fn fields_problem(kind: ElementTypeKind, m: Map<String, Value>) -> Result<Problem, String> {
    let v = Value::Object(m);
    let read = |e: serde_json::Error| e.to_string();
    Ok(match kind {
        ElementTypeKind::Column => serde_json::from_value::<crate::ColumnFields>(v).map_err(read)?.problem(),
        ElementTypeKind::Beam => serde_json::from_value::<crate::BeamFields>(v).map_err(read)?.problem(),
        ElementTypeKind::Slab => serde_json::from_value::<crate::SlabFields>(v).map_err(read)?.problem(),
        ElementTypeKind::Roof => serde_json::from_value::<crate::RoofFields>(v).map_err(read)?.problem(),
        ElementTypeKind::Railing => serde_json::from_value::<crate::RailingFields>(v).map_err(read)?.problem(),
        ElementTypeKind::Stair => serde_json::from_value::<crate::StairFields>(v).map_err(read)?.problem(),
    })
}

/// Fields `m` read into the kind's and written back: as an element writes them. None when they do not read.
fn written_fields(kind: ElementTypeKind, m: Map<String, Value>) -> Option<Map<String, Value>> {
    fn again<T: serde::de::DeserializeOwned + Serialize>(v: Value) -> Option<Value> {
        serde_json::to_value(serde_json::from_value::<T>(v).ok()?).ok()
    }
    let v = Value::Object(m);
    let out = match kind {
        ElementTypeKind::Column => again::<crate::ColumnFields>(v),
        ElementTypeKind::Beam => again::<crate::BeamFields>(v),
        ElementTypeKind::Slab => again::<crate::SlabFields>(v),
        ElementTypeKind::Roof => again::<crate::RoofFields>(v),
        ElementTypeKind::Railing => again::<crate::RailingFields>(v),
        ElementTypeKind::Stair => again::<crate::StairFields>(v),
    }?;
    match out {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// The kind's own check of `e`.
fn element_problem(e: &Entity) -> Problem {
    match e {
        Entity::Column(x) => x.column.problem(),
        Entity::Beam(x) => x.beam.problem(),
        Entity::Slab(x) => x.slab.problem(),
        Entity::Mass(x) => x.mass.problem(),
        Entity::Roof(x) => x.roof.problem(),
        Entity::Railing(x) => x.railing.problem(),
        Entity::Stair(x) => x.stair.problem(),
        _ => None,
    }
}

/// The values a type made from `e` holds (Tipe çevir): its kind's type fields as `e` writes them, the named one left
/// out; with its kind. None: no type serves it.
pub fn element_type_values(e: &Entity) -> Option<(ElementTypeKind, Map<String, Value>)> {
    let (kind, own) = own_object(e)?;
    let values = kind
        .fields()
        .iter()
        .filter(|f| !f.named)
        .filter_map(|f| own.get(f.key).map(|v| (f.key.to_owned(), v.clone())))
        .collect();
    Some((kind, values))
}

/// `e`'s value of `field`: the element's own (none: the field's absence).
pub fn element_value(e: &Entity, field: &TypeField) -> Option<Value> {
    own_object(e).and_then(|(_, m)| m.get(field.key).cloned())
}

/// The type `e` follows, as it says it (its link).
pub fn type_link(e: &Entity) -> Option<&TypeLink> {
    e.arch_info().and_then(|a| a.element_type.as_ref())
}

/// `e` following `t` (docs/adr/8106 §2): each type field takes the type's value (its absence where the type gives
/// none; a column's type its name), but those in `keep`, which stay its own; it carries the type's id and version,
/// `keep` its exceptions. Refused (field, why): no type serves `e`, `t` is another kind's, or the values make no
/// element (its kind's own check, which the change would break: a leg thicker than the column).
pub fn follow_type(e: &Entity, t: &ElementType, keep: &[String]) -> Result<Entity, (String, String)> {
    let Some((kind, mut own)) = own_object(e) else {
        return Err(("uids".into(), "Bu nesnenin tipi olmaz: kolon, kiriş, döşeme, çatı, korkuluk ya da merdiven değil.".into()));
    };
    if kind != t.kind {
        return Err((
            "uids".into(),
            format!("“{}” bir {} tipi; bu nesne {}.", t.name, t.kind.words(), kind.words()),
        ));
    }
    for f in kind.fields() {
        if keep.iter().any(|k| k == f.key) {
            continue;
        }
        match t.value(f) {
            Some(v) => {
                own.insert(f.key.into(), v);
            }
            None => {
                own.remove(f.key);
            }
        }
    }
    let mut out = with_object(e, own).map_err(|why| ("values".to_owned(), format!("Tipin değerleri bu {} nesnesine yazılmadı: {why}", kind.words())))?;
    if let Some(problem) = element_problem(&out) {
        return Err(problem);
    }
    let overrides = kind.fields().iter().filter(|f| keep.iter().any(|k| k == f.key)).map(|f| f.key.to_owned()).collect();
    if let Some(arch) = out.arch_info_mut() {
        arch.element_type = Some(TypeLink { id: t.id.clone(), version: t.version, overrides });
    }
    Ok(out)
}

/// The type fields in which `e` is not `t`'s, in its kind's order: its exceptions while it follows `t`'s version.
pub fn fields_off_type(e: &Entity, t: &ElementType) -> Vec<String> {
    let Some((kind, own)) = own_object(e) else {
        return Vec::new();
    };
    if kind != t.kind {
        return Vec::new();
    }
    kind.fields()
        .iter()
        .zip(t.written())
        .filter(|(f, v)| own.get(f.key) != v.as_ref())
        .map(|(f, _)| f.key.to_owned())
        .collect()
}

/// After an edit from `before` to `after` of an element following `t`, its exceptions (docs/adr/8106 §2): at the
/// type's version, the fields in which it is not the type's (a field set back to the type's value follows it again);
/// at an older one, those it kept and those the edit changed to another value than the type's latest.
pub fn exceptions_after(before: &Entity, after: &Entity, t: &ElementType) -> Vec<String> {
    let Some(link) = type_link(after) else {
        return Vec::new();
    };
    if link.version >= t.version {
        return fields_off_type(after, t);
    }
    let (Some((kind, was)), Some((_, now))) = (own_object(before), own_object(after)) else {
        return link.overrides.clone();
    };
    kind.fields()
        .iter()
        .zip(t.written())
        .filter(|(f, v)| {
            link.overrides.iter().any(|k| k == f.key) || (was.get(f.key) != now.get(f.key) && now.get(f.key) != v.as_ref())
        })
        .map(|(f, _)| f.key.to_owned())
        .collect()
}

/// What is wrong with a project's types: one's own problem, two of one id, or two of one kind and name (case aside).
pub fn element_types_problem(types: &[ElementType]) -> Problem {
    if types.len() > MAX_ELEMENT_TYPES {
        return Some((
            "elementTypes".into(),
            format!("Bir projede en çok {MAX_ELEMENT_TYPES} eleman tipi olur; {} verildi.", types.len()),
        ));
    }
    for (i, t) in types.iter().enumerate() {
        if let Some((at, why)) = t.problem() {
            return Some((format!("elementTypes[{i}].{at}"), why));
        }
        if let Some(u) = types[..i].iter().find(|u| u.id == t.id) {
            return Some((
                format!("elementTypes[{i}].id"),
                format!("“{}” kimliği iki tipte var (“{}”, “{}”).", t.id, u.name, t.name),
            ));
        }
        let folded = t.name.to_lowercase();
        if types[..i].iter().any(|u| u.kind == t.kind && u.name.to_lowercase() == folded) {
            return Some((
                format!("elementTypes[{i}].name"),
                format!("“{}” adında iki {} tipi var; birine başka ad verin.", t.name, t.kind.words()),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column_type(values: Value) -> ElementType {
        ElementType {
            id: "s1".into(),
            name: "S1".into(),
            kind: ElementTypeKind::Column,
            version: 1,
            values: values.as_object().cloned().unwrap_or_default(),
        }
    }

    #[test]
    fn every_kind_s_plain_element_holds_and_its_required_fields_are_those_it_needs() {
        for kind in ElementTypeKind::ALL {
            let Value::Object(probe) = kind.probe() else { panic!("{kind:?}") };
            assert_eq!(fields_problem(kind, probe.clone()), Ok(None), "{kind:?}");
            for f in kind.fields().iter().filter(|f| !f.named) {
                let mut without = probe.clone();
                without.remove(f.key);
                let reads = fields_problem(kind, without).is_ok();
                assert_eq!(reads, !f.required, "{kind:?}.{}", f.key);
            }
        }
    }

    #[test]
    fn a_type_must_give_its_kind_s_fields_and_only_them() {
        assert_eq!(column_type(serde_json::json!({ "width": 0.3, "depth": 0.5 })).problem(), None);
        let (at, _) = column_type(serde_json::json!({ "depth": 0.5 })).problem().expect("no width");
        assert_eq!(at, "values.width");
        let (at, _) = column_type(serde_json::json!({ "width": 0.3, "tread": 0.25 })).problem().expect("not a column's");
        assert_eq!(at, "values.tread");
        let (at, _) = column_type(serde_json::json!({ "width": 0.3, "type": "S2" })).problem().expect("its name");
        assert_eq!(at, "values.type");
        let (at, _) = column_type(serde_json::json!({ "width": -0.3 })).problem().expect("no column");
        assert!(at.starts_with("values."), "{at}");
    }

    #[test]
    fn a_column_follows_its_type_keeps_its_exception_and_says_where_it_differs() {
        let e: Entity = serde_json::from_value(serde_json::json!({
            "kind": "column", "id": 1, "layerId": "kolon", "attrs": {},
            "at": { "x": 4.0, "y": 2.0 }, "width": 0.4, "rotation": 0.5, "tag": "S101"
        }))
        .expect("a column");
        let t = column_type(serde_json::json!({ "width": 0.3, "depth": 0.5, "material": "C30" }));
        let typed = follow_type(&e, &t, &[]).expect("follows");
        let Entity::Column(c) = &typed else { panic!() };
        assert_eq!((c.column.width, c.column.depth, c.column.material.as_deref()), (0.3, Some(0.5), Some("C30")));
        assert_eq!(c.column.kind.as_deref(), Some("S1"));
        assert_eq!((c.column.rotation, c.column.tag.as_deref()), (0.5, Some("S101")), "its own stay");
        assert_eq!(type_link(&typed).map(|l| (l.version, l.overrides.len())), Some((1, 0)));
        assert!(fields_off_type(&typed, &t).is_empty());
        // Its width its own: an exception that stays when the type changes.
        let kept = follow_type(&e, &t, &["width".into()]).expect("follows");
        assert_eq!(type_link(&kept).map(|l| l.overrides.clone()), Some(vec!["width".to_owned()]));
        assert_eq!(fields_off_type(&kept, &t), ["width"]);
        let mut wider = t.clone();
        wider.version = 2;
        wider.values.insert("width".into(), serde_json::json!(0.35));
        let next = follow_type(&kept, &wider, &["width".into()]).expect("follows");
        let Entity::Column(c) = &next else { panic!() };
        assert_eq!(c.column.width, 0.4);
        // An edit setting its width back to the type's: no exception any more.
        let mut back = next.clone();
        if let Entity::Column(c) = &mut back {
            c.column.width = 0.35;
        }
        assert!(exceptions_after(&next, &back, &wider).is_empty());
    }

    #[test]
    fn a_type_s_values_are_written_as_the_element_writes_them() {
        let t = column_type(serde_json::json!({ "width": 1, "fill": false, "shape": "rect", "depth": 0.5 }));
        let canon = t.canonical_values().expect("they read");
        assert_eq!(Value::Object(canon), serde_json::json!({ "width": 1.0, "depth": 0.5 }));
        // A column following it is not an exception for the default it was given.
        let e: Entity = serde_json::from_value(serde_json::json!({
            "kind": "column", "id": 1, "layerId": "kolon", "attrs": {}, "at": { "x": 0.0, "y": 0.0 }, "width": 0.3
        }))
        .expect("a column");
        let typed = follow_type(&e, &t, &[]).expect("follows");
        assert!(fields_off_type(&typed, &t).is_empty(), "{:?}", fields_off_type(&typed, &t));
    }

    #[test]
    fn a_change_that_makes_no_element_is_refused() {
        let e: Entity = serde_json::from_value(serde_json::json!({
            "kind": "column", "id": 1, "layerId": "kolon", "attrs": {},
            "at": { "x": 0.0, "y": 0.0 }, "width": 0.6, "depth": 0.6, "shape": "lShape", "legs": [0.25, 0.25]
        }))
        .expect("an L column");
        let t = column_type(serde_json::json!({ "width": 0.2 }));
        let (at, _) = follow_type(&e, &t, &["shape".into(), "legs".into()]).expect_err("legs thicker than it");
        assert!(at.starts_with("legs"), "{at}");
    }

    #[test]
    fn two_types_of_one_kind_and_name_are_refused_another_kind_may_share_it() {
        let a = column_type(serde_json::json!({ "width": 0.3 }));
        let mut b = a.clone();
        b.id = "s1-2".into();
        assert_eq!(element_types_problem(&[a.clone(), b.clone()]).map(|p| p.0), Some("elementTypes[1].name".into()));
        let beam = ElementType {
            id: "s1-kiris".into(),
            name: "s1".into(),
            kind: ElementTypeKind::Beam,
            version: 1,
            values: serde_json::json!({ "width": 0.25, "height": 0.5 }).as_object().cloned().unwrap_or_default(),
        };
        assert_eq!(element_types_problem(&[a, beam]), None);
    }
}
