//! Data profiles (docs/adr/0541; plan/gis.md CBS-34): a versioned, named description of a kind of GIS project's data —
//! its classes (each a layer by its path, its geometry, its fields, its identifier, its registered area), the project's
//! value lists, the relations between the classes and how its address data is read — kept as data
//! (`catalog/v1/profiles/`), so no application code names a column. `cad.profiles.apply` adds what the drawing lacks,
//! keeping what it has; `cad.profiles.check` says, without changing anything, where the drawing's data departs from the
//! profile. A profile is a working model; holding to it is not a statement of legal conformity.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{AreaSource, DoorSource, FieldDomain, LayerField, StreetSource};

pub const CAD_PROFILES_APPLY: &str = "cad.profiles.apply";
pub const CAD_PROFILES_APPLY_VERSION: u32 = 1;
pub const CAD_PROFILES_CHECK: &str = "cad.profiles.check";
pub const CAD_PROFILES_CHECK_VERSION: u32 = 1;

/// The format a profile file says it is.
pub const PROFILE_FORMAT: &str = "kentos.data-profile";

/// The most classes, relations and fields of a class a profile holds.
pub const MAX_PROFILE_CLASSES: usize = 64;
pub const MAX_PROFILE_RELATIONS: usize = 64;
/// The most object ids one finding names; its `count` says how many there are.
pub const MAX_FINDING_UIDS: usize = 1000;

/// A class's objects' kind of geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProfileGeometry {
    /// Alan: a polygon (holes and parts allowed).
    Area,
    /// Çizgi: a line or a polyline.
    Line,
    /// Nokta.
    Point,
    /// Geometrisiz kayıt (docs/adr/0410).
    Record,
}

impl ProfileGeometry {
    /// Whether an object of this entity kind (`Entity`'s tag) belongs to the class's geometry.
    pub fn holds(self, kind: &str) -> bool {
        match self {
            Self::Area => kind == "polygon",
            Self::Line => matches!(kind, "line" | "polyline"),
            Self::Point => kind == "point",
            Self::Record => kind == "record",
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Self::Area => "alan",
            Self::Line => "çizgi",
            Self::Point => "nokta",
            Self::Record => "kayıt",
        }
    }
}

/// A class's registered area (tapu ya da kayıt alanı): a field kept apart from the geometric area. Neither overwrites the
/// other; `cad.profiles.check` says where they differ by more than `tolerance`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RegisteredArea {
    /// The field that holds it, in square metres.
    pub field: String,
    /// The difference the profile lets pass, in square metres. It is the profile's, not the law's error limit.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub tolerance: f64,
}

/// A class of the profile: a layer by its path, its objects' geometry, its fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileClass {
    /// Letters, digits, `-`; unique in the profile.
    pub id: String,
    pub title: String,
    /// The theme it belongs to, as the profile names it (“Kadastro”).
    pub theme: String,
    /// The layer's path: its groups' names, then its own; found by the names folded (Turkish case aside).
    pub layer: Vec<String>,
    /// The layer's colour when the profile adds it (a palette name or `#RRGGBB`).
    pub color: String,
    pub geometry: ProfileGeometry,
    /// The identifier field: unique among the class's objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    pub fields: Vec<LayerField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub registered_area: Option<RegisteredArea>,
}

/// A relation between two classes: the child class's field holds a parent object's persistent id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileRelation {
    pub id: String,
    pub name: String,
    /// The parent class's id.
    pub parent: String,
    /// The child class's id.
    pub child: String,
    /// The child class's field.
    pub field: String,
}

/// How the profile's address data is read (docs/adr/0229): the sources' `layer` names a class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileAddress {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub doors: Option<DoorSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub streets: Option<StreetSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AreaSource>>", optional))]
    pub areas: Vec<AreaSource>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub min_score: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub reach: f64,
}

/// A data profile as `catalog/v1/profiles/*.json` keeps it, or as a command gives it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DataProfile {
    /// `kentos.data-profile`.
    pub format: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    pub name: String,
    pub title: String,
    pub description: String,
    /// What it rests on and what it is not (a working profile, not an official application schema).
    pub basis: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<FieldDomain>>", optional))]
    pub domains: Vec<FieldDomain>,
    pub classes: Vec<ProfileClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProfileRelation>>", optional))]
    pub relations: Vec<ProfileRelation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub address: Option<ProfileAddress>,
}

/// KentOS's profiles, by name.
pub const BUILTIN_PROFILES: &[(&str, &str)] = &[(
    "tucbs-temel",
    include_str!("../../../../catalog/v1/profiles/tucbs-temel.json"),
)];

/// A built-in profile by its name (case aside); none for a name KentOS has not.
pub fn builtin_profile(name: &str) -> Option<DataProfile> {
    let key = name.trim().to_lowercase();
    BUILTIN_PROFILES
        .iter()
        .find(|(n, _)| *n == key)
        .and_then(|(_, text)| serde_json::from_str(text).ok())
}

fn plain_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// What is wrong with a profile by its shape; none when it holds: its format and version, at most 64 classes with
/// unique plain ids, a layer path of names, fields that hold (`fields_problem`), the key and the registered area naming
/// one of the class's fields (the area a number), the value lists that hold and that the fields name, relations between
/// the profile's classes naming a child's text field, address sources naming classes.
pub fn profile_problem(p: &DataProfile) -> Option<String> {
    if p.format != PROFILE_FORMAT || p.version != 1 {
        return Some(format!(
            "Profilin biçimi “{PROFILE_FORMAT}”, sürümü 1 olmalı."
        ));
    }
    if p.name.trim().is_empty() || p.title.trim().is_empty() {
        return Some("Profilin adı ve başlığı boş olamaz.".into());
    }
    if p.classes.is_empty() || p.classes.len() > MAX_PROFILE_CLASSES {
        return Some(format!(
            "Profilde 1 ile {MAX_PROFILE_CLASSES} arası sınıf olur."
        ));
    }
    if let Some((_, why)) = crate::domains_problem(&p.domains) {
        return Some(why);
    }
    let mut ids: Vec<&str> = Vec::new();
    for c in &p.classes {
        if !plain_id(&c.id) {
            return Some(format!(
                "“{}” sınıf kimliği olamaz: küçük harf, rakam ve tire, en çok 64.",
                c.id
            ));
        }
        if ids.contains(&c.id.as_str()) {
            return Some(format!("“{}” sınıfı iki kez var.", c.id));
        }
        ids.push(&c.id);
        if c.layer.is_empty() || c.layer.iter().any(|n| n.trim().is_empty()) {
            return Some(format!("“{}” sınıfının katman yolu boş ya da boş adlı.", c.title));
        }
        if let Some(why) = crate::fields_problem(&c.fields) {
            return Some(format!("“{}” sınıfının alanları: {why}", c.title));
        }
        let field = |name: &str| c.fields.iter().find(|f| f.name == name);
        if let Some(k) = &c.key
            && field(k).is_none()
        {
            return Some(format!("“{}” sınıfının kimlik alanı “{k}” alanlarında yok.", c.title));
        }
        if let Some(r) = &c.registered_area {
            match field(&r.field) {
                None => {
                    return Some(format!(
                        "“{}” sınıfının kayıtlı alanı “{}” alanlarında yok.",
                        c.title, r.field
                    ));
                }
                Some(f) if !matches!(f.kind, crate::LayerFieldKind::Decimal | crate::LayerFieldKind::Integer) => {
                    return Some(format!(
                        "“{}” sınıfının kayıtlı alanı “{}” sayı alanı olmalı.",
                        c.title, r.field
                    ));
                }
                _ => {}
            }
            if !(r.tolerance.is_finite() && r.tolerance >= 0.0) {
                return Some(format!(
                    "“{}” sınıfının alan toleransı sıfır ya da artı bir sayı olmalı.",
                    c.title
                ));
            }
            if c.geometry != ProfileGeometry::Area {
                return Some(format!("“{}” sınıfı alan değil; kayıtlı alanı olamaz.", c.title));
            }
        }
        for f in &c.fields {
            if let Some(d) = &f.domain
                && !p.domains.iter().any(|x| &x.id == d)
            {
                return Some(format!(
                    "“{}” sınıfının “{}” alanı profilde olmayan “{d}” değer listesini anıyor.",
                    c.title, f.name
                ));
            }
        }
    }
    if p.relations.len() > MAX_PROFILE_RELATIONS {
        return Some(format!("Profilde en çok {MAX_PROFILE_RELATIONS} ilişki olur."));
    }
    let class = |id: &str| p.classes.iter().find(|c| c.id == id);
    let mut rel_ids: Vec<&str> = Vec::new();
    for r in &p.relations {
        if !plain_id(&r.id) || rel_ids.contains(&r.id.as_str()) {
            return Some(format!("“{}” ilişki kimliği olamaz ya da iki kez var.", r.id));
        }
        rel_ids.push(&r.id);
        let (Some(_), Some(child)) = (class(&r.parent), class(&r.child)) else {
            return Some(format!("“{}” ilişkisi profilde olmayan bir sınıfı anıyor.", r.name));
        };
        match child.fields.iter().find(|f| f.name == r.field) {
            Some(f) if f.kind == crate::LayerFieldKind::Text => {}
            _ => {
                return Some(format!(
                    "“{}” ilişkisinin alanı “{}”, “{}” sınıfının metin alanı olmalı.",
                    r.name, r.field, child.title
                ));
            }
        }
    }
    if let Some(a) = &p.address {
        let named = a
            .doors
            .iter()
            .map(|d| d.layer.as_str())
            .chain(a.streets.iter().map(|s| s.layer.as_str()))
            .chain(a.areas.iter().map(|s| s.layer.as_str()));
        for l in named {
            if class(l).is_none() {
                return Some(format!("Adres verisi profilde olmayan “{l}” sınıfını anıyor."));
            }
        }
    }
    None
}

/// Input of `cad.profiles.apply` v1 (“Veri profilini uygula”): the profile's layers (with their groups), fields, value
/// lists, relations and address data the drawing lacks; what it has is kept and said. The layers and their fields are
/// one undo step “Veri profili”; the value lists, relations and address data are project settings, no undo step.
///
/// The profile is `profile` when given, else the built-in `name`. Refusals, in this order: `unknown_profile` (neither, or
/// a name KentOS has not), `invalid_profile`, `invalid_revision`, `revision_conflict`. Warnings: `profile_field_differs`,
/// `profile_domain_differs`, `profile_layer_skipped`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesApply {
    /// A built-in profile's name: `tucbs-temel`. `profile`, when given, goes first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Or the profile itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<DataProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output (and plan) of `cad.profiles.apply` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesApplied {
    /// Whether anything was written.
    pub changed: bool,
    /// The layers and groups added, by their ids, groups before their layers.
    pub layers: Vec<String>,
    /// The fields added to layers the drawing had, as “layer name.field”.
    pub fields: Vec<String>,
    /// The value lists added, by their ids.
    pub domains: Vec<String>,
    /// The relations added, by their ids.
    pub relations: Vec<String>,
    /// Whether the profile's address data was added.
    pub address: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.profiles.check` v1 (“Veri profilini denetle”): where the drawing's data departs from a profile. It
/// changes nothing; its plan gives the same answer.
///
/// Refusals: `unknown_profile`, `invalid_profile`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<DataProfile>,
}

/// What a finding is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProfileFindingKind {
    /// The class's layer is not in the drawing (`cad.profiles.apply` adds it).
    MissingLayer,
    /// A field of the class the layer has not (apply adds it).
    MissingField,
    /// A field the layer has with another kind (apply keeps it).
    FieldKind,
    /// A value list of the profile the project has not (apply adds it).
    MissingDomain,
    /// A relation of the profile the project has not (apply adds it).
    MissingRelation,
    /// The profile's address data is not defined (apply adds it).
    MissingAddress,
    /// Objects of another kind of geometry on the class's layer.
    WrongGeometry,
    /// A required field left empty.
    RequiredEmpty,
    /// A value its field's rules refuse (kind, length, range, value list).
    BadValue,
    /// An identifier more than one object holds.
    DuplicateKey,
    /// A registered area farther from the geometric area than the tolerance (one that is not a number is a
    /// `badValue`).
    AreaDifference,
    /// A relation's field naming no object of the parent class.
    RelationBroken,
}

/// One finding of `cad.profiles.check`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileFinding {
    pub kind: ProfileFindingKind,
    /// The class's id; none for a value list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class: Option<String>,
    /// The layer's id, when the drawing has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The field, the value list's or the relation's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub subject: Option<String>,
    /// The value: the duplicate identifier, the value refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<String>,
    /// The objects it is about, in the drawing's order, at most 1000.
    pub uids: Vec<String>,
    /// How many objects it is about.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    /// For an area difference: the geometric area, the registered area and their difference, square metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub geometric: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub registered: Option<f64>,
    /// Whether `cad.profiles.apply` fixes it.
    pub fixable: bool,
    pub message: String,
}

/// A class's line of the report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileClassSummary {
    pub class: String,
    pub title: String,
    /// The layer's id; none when the drawing has not the layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub findings: u32,
}

/// Output (and plan) of `cad.profiles.check` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesReport {
    pub profile: String,
    pub title: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    /// The profile's basis, said with the answer.
    pub basis: String,
    pub classes: Vec<ProfileClassSummary>,
    /// The findings: the value lists, then class by class in the profile's order (the layer, its fields, its objects'
    /// geometry, required, values, identifiers, areas), then the relations and the address data.
    pub findings: Vec<ProfileFinding>,
    /// No finding. It says the data holds to this profile, not that it conforms to any regulation.
    pub clean: bool,
}
