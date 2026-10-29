//! An institution's own data profile (docs/adr/0558; plan/gis.md CBS-34's remainder): a profile or a mapping file the
//! user brings, kept in the project's `profiller` folder beside the drawing (`cad.profiles.import`, `cad.profiles.list`),
//! a profile's releases and the migration between them (`cad.profiles.migrate`), and an institution's mapping from its
//! own layer, field and code names to a profile's, which `cad.profiles.apply` and `.check` read the profile through.
//!
//! The release is the profile's own number (`DataProfile.release`; `version` stays the format's). A release lists, as
//! data, the steps that move the data of the release before it: rename a layer or a field, change a field's kind (the
//! values go through the new field's rule), recode values (merge, or split by another field's value) and retire a code
//! of a value list. A mapping never renames anything by itself: it is read only when a command is given it, and
//! what it names is said.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{
    AreaSource, DataProfile, DoorSource, FieldDomain, LayerField, LayerFieldKind, ProfileAddress,
    ProfileFileKind, StreetSource, follow_domains,
};

pub const CAD_PROFILES_IMPORT: &str = "cad.profiles.import";
pub const CAD_PROFILES_IMPORT_VERSION: u32 = 1;
pub const CAD_PROFILES_LIST: &str = "cad.profiles.list";
pub const CAD_PROFILES_LIST_VERSION: u32 = 1;
pub const CAD_PROFILES_MIGRATE: &str = "cad.profiles.migrate";
pub const CAD_PROFILES_MIGRATE_VERSION: u32 = 1;
pub const CAD_PROFILES_REMOVE: &str = "cad.profiles.remove";
pub const CAD_PROFILES_REMOVE_VERSION: u32 = 1;

/// The format a mapping file says it is.
pub const MAPPING_FORMAT: &str = "kentos.profile-mapping";
/// The folder beside the drawing the project's profiles and mappings are kept in.
pub const PROFILE_FOLDER: &str = "profiller";
/// A profile's file name ends so (`<name>.profil.json`), a mapping's so (`<name>.eslem.json`).
pub const PROFILE_SUFFIX: &str = ".profil.json";
pub const MAPPING_SUFFIX: &str = ".eslem.json";
/// The most migrations a profile carries, the most steps of one, the most objects a loss names.
pub const MAX_MIGRATIONS: usize = 32;
pub const MAX_MIGRATION_STEPS: usize = 64;
pub const MAX_LOSS_UIDS: usize = 1000;
/// The most classes of a mapping, fields and codes of one.
pub const MAX_MAPPING_ITEMS: usize = 256;

fn plain_name(id: &str) -> bool {
    kentos_contracts::profile_name_ok(id)
}

/// Whether `name` is a name a profile or a mapping file may carry (small letters, digits and `-`, at most 64).
pub fn profile_file_name_ok(name: &str) -> bool {
    plain_name(name)
}

/// What becomes of a value that does not go through a field's new rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ConvertFail {
    /// The value stays as written (`kept` in the losses); `cad.profiles.check` says it does not fit.
    #[default]
    Keep,
    /// The value is emptied (`cleared` in the losses).
    Clear,
}

/// One value recoded by a `mapValues` step: `from` becomes `to`, only where `whenField` holds `whenEquals` when given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ValueMap {
    pub from: String,
    pub to: String,
    /// Another field of the layer whose value decides (a split: one old code becomes two by it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub when_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub when_equals: Option<String>,
}

/// A step of a migration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MigrationStep {
    /// The layer at the path is renamed (its place in the tree stays).
    #[serde(rename_all = "camelCase")]
    RenameLayer { layer: Vec<String>, to: String },
    /// The layer's field is renamed and its objects' values move with it.
    #[serde(rename_all = "camelCase")]
    RenameField {
        layer: Vec<String>,
        from: String,
        to: String,
    },
    /// The field takes a new definition (its name the same): each value goes through the new rule and is
    /// written in its canonical text; one that does not fit is kept or emptied (`on_fail`) and reported.
    #[serde(rename_all = "camelCase")]
    ChangeFieldType {
        layer: Vec<String>,
        field: Box<LayerField>,
        #[serde(default)]
        on_fail: ConvertFail,
    },
    /// Values recoded: several old values may become one (a merge), one old value two (a split by `whenField`).
    #[serde(rename_all = "camelCase")]
    MapValues {
        layer: Vec<String>,
        field: String,
        map: Vec<ValueMap>,
    },
    /// A code of a project value list is retired; with `replaceWith` every value of a field linked to the list that
    /// holds it (each of a multiple field's) becomes the other.
    #[serde(rename_all = "camelCase")]
    RetireCode {
        domain: String,
        code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        replace_with: Option<String>,
    },
    /// A project value list takes a new id; every field linked to it follows (docs/adr/0647 §3).
    #[serde(rename_all = "camelCase")]
    RenameDomain { domain: String, to: String },
}

impl MigrationStep {
    /// The step's kind, as its file writes it.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::RenameLayer { .. } => "renameLayer",
            Self::RenameField { .. } => "renameField",
            Self::ChangeFieldType { .. } => "changeFieldType",
            Self::MapValues { .. } => "mapValues",
            Self::RetireCode { .. } => "retireCode",
            Self::RenameDomain { .. } => "renameDomain",
        }
    }
}

/// The steps that move the data of release `from` to release `to` (`from + 1`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileMigration {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub from: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub to: u32,
    pub title: String,
    pub steps: Vec<MigrationStep>,
}

fn blank(s: &str) -> bool {
    s.trim().is_empty()
}

fn path_problem(path: &[String]) -> bool {
    path.is_empty() || path.iter().any(|n| blank(n))
}

/// What is wrong with a profile's release and migrations; none when they hold: the release at least 1, at most
/// 32 migrations each from one release to the next, below the profile's release and none twice, each with 1 to 64 steps
/// that hold (a layer path of names, names that differ, a field that holds, recodes with a `from` and a `to`, a
/// retired code that is not its own replacement).
pub fn migrations_problem(p: &DataProfile) -> Option<String> {
    if p.release == 0 {
        return Some("Profilin sürümü (release) en az 1 olmalı.".into());
    }
    if p.migrations.len() > MAX_MIGRATIONS {
        return Some(format!("Profilde en çok {MAX_MIGRATIONS} göç olur."));
    }
    for (i, m) in p.migrations.iter().enumerate() {
        if m.from == 0 || m.to != m.from + 1 {
            return Some(format!(
                "{}. göç {} sürümünden {} sürümüne gidiyor; bir göç bir sürümden sonrakine gider.",
                i + 1,
                m.from,
                m.to
            ));
        }
        if m.to > p.release {
            return Some(format!(
                "{}. göç {} sürümüne gidiyor ama profilin sürümü {}.",
                i + 1,
                m.to,
                p.release
            ));
        }
        if p.migrations[..i].iter().any(|o| o.from == m.from) {
            return Some(format!("{} sürümünden iki göç var.", m.from));
        }
        if m.steps.is_empty() || m.steps.len() > MAX_MIGRATION_STEPS {
            return Some(format!(
                "{}. göçte 1 ile {MAX_MIGRATION_STEPS} arası adım olur.",
                i + 1
            ));
        }
        for (j, s) in m.steps.iter().enumerate() {
            if let Some(why) = step_problem(s) {
                return Some(format!("{}. göcün {}. adımı: {why}", i + 1, j + 1));
            }
        }
    }
    None
}

fn step_problem(s: &MigrationStep) -> Option<String> {
    match s {
        MigrationStep::RenameLayer { layer, to } => {
            if path_problem(layer) {
                return Some("katman yolu boş ya da boş adlı.".into());
            }
            if blank(to) {
                return Some("yeni katman adı boş.".into());
            }
            None
        }
        MigrationStep::RenameField { layer, from, to } => {
            if path_problem(layer) {
                return Some("katman yolu boş ya da boş adlı.".into());
            }
            if blank(from) || blank(to) || from == to {
                return Some("alanın eski ve yeni adı dolu ve farklı olmalı.".into());
            }
            None
        }
        MigrationStep::ChangeFieldType { layer, field, .. } => {
            if path_problem(layer) {
                return Some("katman yolu boş ya da boş adlı.".into());
            }
            crate::fields_problem(std::slice::from_ref(field))
                .map(|why| format!("alanın yeni tanımı: {why}"))
        }
        MigrationStep::MapValues { layer, field, map } => {
            if path_problem(layer) {
                return Some("katman yolu boş ya da boş adlı.".into());
            }
            if blank(field) {
                return Some("alan adı boş.".into());
            }
            if map.is_empty() || map.len() > MAX_MAPPING_ITEMS {
                return Some(format!(
                    "1 ile {MAX_MAPPING_ITEMS} arası değer eşlemesi olur."
                ));
            }
            for (i, v) in map.iter().enumerate() {
                if blank(&v.from) || blank(&v.to) {
                    return Some("değer eşlemesinin eski ve yeni değeri dolu olmalı.".into());
                }
                if v.when_field.is_some() != v.when_equals.is_some() {
                    return Some(
                        "koşul için hem koşul alanı (whenField) hem değeri (whenEquals) verilir."
                            .into(),
                    );
                }
                if map[..i].iter().any(|o| {
                    o.from == v.from
                        && o.when_field == v.when_field
                        && o.when_equals == v.when_equals
                }) {
                    return Some(format!(
                        "“{}” değeri aynı koşulla iki kez eşlenmiş.",
                        v.from
                    ));
                }
            }
            None
        }
        MigrationStep::RetireCode {
            domain,
            code,
            replace_with,
        } => {
            if blank(domain) || blank(code) {
                return Some("değer listesinin ve kodun adı dolu olmalı.".into());
            }
            if replace_with.as_deref() == Some(code.as_str()) {
                return Some("kod kendisiyle değiştirilemez.".into());
            }
            None
        }
        MigrationStep::RenameDomain { domain, to } => {
            if blank(domain) || !plain_name(to) || domain.trim() == to {
                return Some(
                    "listenin eski kimliği dolu, yeni kimliği küçük harf, rakam ve “-” olmalı ve eskisinden farklı."
                        .into(),
                );
            }
            None
        }
    }
}

// ── The mapping ─────────────────────────────────────────────────────────────────────────────────────────────────

/// A profile's code and what the institution calls it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MappingCode {
    pub profile: String,
    pub name: String,
}

/// A profile field's coded values and the institution's codes for them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MappingCodes {
    /// The profile's field name.
    pub field: String,
    pub codes: Vec<MappingCode>,
}

/// A profile's field and what the institution calls it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MappingField {
    pub profile: String,
    pub name: String,
}

/// A profile class as the institution keeps it: its layer's path, its fields' names, its codes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MappingClass {
    /// The profile's class id.
    pub class: String,
    /// The institution's layer path; none: the profile's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layer: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MappingField>>", optional))]
    pub fields: Vec<MappingField>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MappingCodes>>", optional))]
    pub codes: Vec<MappingCodes>,
}

/// An institution's mapping file: its own layer, field and code names for a profile's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileMapping {
    /// `kentos.profile-mapping`.
    pub format: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    /// Small letters, digits and `-`; the file's name.
    pub name: String,
    pub title: String,
    /// The name of the profile it is for.
    pub profile: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub description: String,
    pub classes: Vec<MappingClass>,
}

/// The profile's field with its list's values (a linked field takes its list's).
fn field_with_values(c: &crate::ProfileClass, p: &DataProfile, name: &str) -> Option<LayerField> {
    let f = c.fields.iter().find(|f| f.name == name)?;
    let (fields, _) = follow_domains(std::slice::from_ref(f), &p.domains);
    fields.into_iter().next()
}

/// What is wrong with a mapping for `p`; none when it holds: its format and version, a plain name, the profile it is for,
/// classes of the profile (none twice) each with a layer path of names, fields of the class (none twice) given names that
/// are plain and do not meet another field's name in the class, codes of a field that has a list of values (each a code of
/// it, none twice, the institution's codes filled and distinct).
pub fn mapping_problem(m: &ProfileMapping, p: &DataProfile) -> Option<String> {
    if m.format != MAPPING_FORMAT || m.version != 1 {
        return Some(format!(
            "Eşlemenin biçimi “{MAPPING_FORMAT}”, sürümü 1 olmalı."
        ));
    }
    if !plain_name(&m.name) {
        return Some(format!(
            "“{}” eşleme adı olamaz: küçük harf, rakam ve tire, en çok 64.",
            m.name
        ));
    }
    if blank(&m.title) {
        return Some("Eşlemenin başlığı boş olamaz.".into());
    }
    if m.profile != p.name {
        return Some(format!(
            "Eşleme “{}” profili için yazılmış, verilen profil “{}”.",
            m.profile, p.name
        ));
    }
    if m.classes.is_empty() || m.classes.len() > MAX_MAPPING_ITEMS {
        return Some(format!(
            "Eşlemede 1 ile {MAX_MAPPING_ITEMS} arası sınıf olur."
        ));
    }
    for (i, mc) in m.classes.iter().enumerate() {
        let Some(c) = p.classes.iter().find(|c| c.id == mc.class) else {
            return Some(format!("“{}” sınıfı profilde yok.", mc.class));
        };
        if m.classes[..i].iter().any(|o| o.class == mc.class) {
            return Some(format!("“{}” sınıfı eşlemede iki kez var.", c.title));
        }
        if !mc.layer.is_empty() && path_problem(&mc.layer) {
            return Some(format!(
                "“{}” sınıfının katman yolunda boş ad var.",
                c.title
            ));
        }
        if mc.fields.len() > MAX_MAPPING_ITEMS || mc.codes.len() > MAX_MAPPING_ITEMS {
            return Some(format!("“{}” sınıfının eşlemesi çok uzun.", c.title));
        }
        let mut finals: Vec<String> = c
            .fields
            .iter()
            .filter(|f| !mc.fields.iter().any(|x| x.profile == f.name))
            .map(|f| f.name.clone())
            .collect();
        for (j, f) in mc.fields.iter().enumerate() {
            if !c.fields.iter().any(|x| x.name == f.profile) {
                return Some(format!(
                    "“{}” alanı “{}” sınıfında yok.",
                    f.profile, c.title
                ));
            }
            if mc.fields[..j].iter().any(|o| o.profile == f.profile) {
                return Some(format!("“{}” alanı iki kez eşlenmiş.", f.profile));
            }
            if blank(&f.name) || f.name.trim() != f.name {
                return Some(format!(
                    "“{}” alanının kurumdaki adı boş ya da baş ve sonunda boşluklu.",
                    f.profile
                ));
            }
            if finals.contains(&f.name) {
                return Some(format!(
                    "“{}” adı “{}” sınıfında başka bir alanın adı olurdu.",
                    f.name, c.title
                ));
            }
            finals.push(f.name.clone());
        }
        for (j, mcs) in mc.codes.iter().enumerate() {
            let Some(f) = field_with_values(c, p, &mcs.field) else {
                return Some(format!(
                    "“{}” alanı “{}” sınıfında yok.",
                    mcs.field, c.title
                ));
            };
            if mc.codes[..j].iter().any(|o| o.field == mcs.field) {
                return Some(format!(
                    "“{}” alanının kodları iki kez eşlenmiş.",
                    mcs.field
                ));
            }
            let Some(values) = &f.values else {
                return Some(format!(
                    "“{}” alanının değer listesi yok; kodları eşlenemez.",
                    mcs.field
                ));
            };
            let mut named: Vec<&str> = values
                .iter()
                .filter(|v| !mcs.codes.iter().any(|c| c.profile == v.code))
                .map(|v| v.code.as_str())
                .collect();
            for (k, code) in mcs.codes.iter().enumerate() {
                if !values.iter().any(|v| v.code == code.profile) {
                    return Some(format!(
                        "“{}” kodu “{}” alanının değer listesinde yok.",
                        code.profile, mcs.field
                    ));
                }
                if mcs.codes[..k].iter().any(|o| o.profile == code.profile) {
                    return Some(format!("“{}” kodu iki kez eşlenmiş.", code.profile));
                }
                if blank(&code.name) || code.name.trim() != code.name || code.name.contains(';') {
                    return Some(format!(
                        "“{}” kodunun kurumdaki adı boş, boşluklu ya da “;” içeriyor.",
                        code.profile
                    ));
                }
                if named.contains(&code.name.as_str()) {
                    return Some(format!(
                        "“{}” kodu “{}” alanında başka bir kodun adı olurdu.",
                        code.name, mcs.field
                    ));
                }
                named.push(&code.name);
            }
        }
    }
    None
}

/// The profile read through the mapping: the mapped classes' layers at the institution's paths, the fields under the
/// institution's names (the profile's name kept as the field's stable id), the codes under the institution's codes (a
/// field whose codes are mapped no longer links the profile's list, its values are the mapped copy), and the key, the
/// registered area, the relations and the address data naming the fields by the institution's names. What the mapping
/// does not name stays as the profile has it. The mapping must hold for the profile (`mapping_problem`).
pub fn through(p: &DataProfile, m: &ProfileMapping) -> DataProfile {
    let mut out = p.clone();
    let field_name = |class: &str, name: &str| -> String {
        m.classes
            .iter()
            .find(|c| c.class == class)
            .and_then(|c| c.fields.iter().find(|f| f.profile == name))
            .map_or_else(|| name.to_owned(), |f| f.name.clone())
    };
    for c in &mut out.classes {
        let Some(mc) = m.classes.iter().find(|x| x.class == c.id) else {
            continue;
        };
        if !mc.layer.is_empty() {
            c.layer = mc.layer.clone();
        }
        let (followed, _) = follow_domains(&c.fields, &p.domains);
        let mut fields = Vec::with_capacity(c.fields.len());
        for (f, own) in followed.into_iter().zip(&c.fields) {
            let codes = mc.codes.iter().find(|x| x.field == own.name);
            let mut f = match codes {
                // The field's codes are the institution's: the list's copy, mapped, no longer linked.
                Some(codes) => {
                    let mut f = f;
                    f.domain = None;
                    if let Some(values) = &mut f.values {
                        for v in values.iter_mut() {
                            if let Some(c) = codes.codes.iter().find(|c| c.profile == v.code) {
                                v.code = c.name.clone();
                            }
                        }
                    }
                    if let Some(d) = &f.default
                        && let Some(c) = codes.codes.iter().find(|c| c.profile == *d)
                    {
                        f.default = Some(c.name.clone());
                    }
                    f
                }
                None => own.clone(),
            };
            if let Some(x) = mc.fields.iter().find(|x| x.profile == own.name) {
                f.id = Some(own.name.clone());
                f.name = x.name.clone();
            }
            fields.push(f);
        }
        c.fields = fields;
        if let Some(k) = &c.key {
            c.key = Some(field_name(&c.id, k));
        }
        if let Some(r) = &mut c.registered_area {
            r.field = field_name(&c.id, &r.field);
        }
    }
    for r in &mut out.relations {
        r.field = field_name(&r.child, &r.field);
    }
    if let Some(a) = &mut out.address {
        map_address(a, &field_name);
    }
    out
}

fn map_address(a: &mut ProfileAddress, field_name: &dyn Fn(&str, &str) -> String) {
    if let Some(DoorSource {
        layer,
        street,
        door,
        neighbourhood,
        district,
        province,
        postcode,
        building,
        code,
    }) = &mut a.doors
    {
        let class = layer.clone();
        let map = |n: &mut String| *n = field_name(&class, n);
        map(street);
        map(door);
        for o in [neighbourhood, district, province, postcode, building, code]
            .into_iter()
            .flatten()
        {
            *o = field_name(&class, o);
        }
    }
    if let Some(StreetSource {
        layer,
        name,
        neighbourhood,
        district,
    }) = &mut a.streets
    {
        let class = layer.clone();
        *name = field_name(&class, name);
        for o in [neighbourhood, district].into_iter().flatten() {
            *o = field_name(&class, o);
        }
    }
    for AreaSource {
        layer,
        name,
        district,
        ..
    } in &mut a.areas
    {
        let class = layer.clone();
        *name = field_name(&class, name);
        if let Some(d) = district {
            *d = field_name(&class, d);
        }
    }
}

/// A coded list as the profile's domains hold them, with `code` retired (the project's lists are changed the same way).
pub fn retire_in(d: &mut FieldDomain, code: &str) -> bool {
    let Some(values) = &mut d.values else {
        return false;
    };
    match values.iter_mut().find(|v| v.code == code) {
        Some(v) if !v.retired => {
            v.retired = true;
            true
        }
        _ => false,
    }
}

// ── Commands ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Where a profile or a mapping is from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProfileSource {
    /// KentOS's own (`catalog/v1/profiles/`).
    Builtin,
    /// A file in the project's `profiller` folder.
    Project,
    /// Carried inside the drawing's own file (the settings' `profileFiles`, docs/adr/0735).
    Drawing,
}

/// Where `cad.profiles.import` keeps a profile or a mapping (docs/adr/0735).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProfileStore {
    /// A file of the `profiller` folder beside the drawing's file (docs/adr/0558).
    #[default]
    Folder,
    /// Inside the drawing's own file: it goes wherever the drawing goes, a cloud project's too.
    Drawing,
}

/// Input of `cad.profiles.list` v1 (“Veri profilleri”): the built-in profiles, those the drawing carries inside its file
/// (docs/adr/0735) and the files of the project's `profiller` folder, profiles and mappings. It changes nothing. The
/// folder is the drawing's own (`drawing`): without it the folder is not read. Refusals: none; a file that does not read
/// is listed with its problem; a folder file whose name and kind the drawing carries too is listed `shadowed`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesList {
    /// The open drawing's own file, an absolute path, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A profile or a mapping of the list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileListed {
    pub kind: ProfileFileKind,
    pub source: ProfileSource,
    /// The name it is given to the commands by: a built-in's, or the file's up to its suffix.
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub title: String,
    /// A profile's release; a mapping's: 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub release: u32,
    /// A profile's classes, a mapping's classes it names.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub classes: u32,
    /// A profile's migrations.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub migrations: u32,
    /// The profile a mapping is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    /// A project file's path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// Why a file is not usable: it does not read, or does not hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
    /// A folder file the drawing's copy of the same name and kind stands for: the commands read the drawing's.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub shadowed: bool,
}

/// Output (and plan) of `cad.profiles.list` v1: the built-in profiles in their order, then the drawing's profiles and
/// mappings by name, then the folder's by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesListed {
    pub items: Vec<ProfileListed>,
    /// The project's folder, when the drawing has a file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub folder: Option<String>,
}

/// Input of `cad.profiles.import` v1 (“Veri profili içe aktar”): a profile or a mapping the user brings, checked and
/// kept where `store` says: a file of the project's `profiller` folder (`<name>.profil.json`, `<name>.eslem.json`, the
/// default), written whole or not at all, or inside the drawing's own file (the settings' `profileFiles`, docs/adr/0735;
/// a settings edit, not an undo step). The source is a JSON file (`source`) or the profile (`profile`) or mapping
/// (`mapping`) itself; one only.
///
/// Refusals, in this order: `profile_source_missing`, `invalid_revision`, `revision_conflict`, `invalid_file_path` (the
/// source, the drawing), `file_not_found`, `invalid_profile` (JSON that does not read, a profile that does not hold, a
/// text too long for the drawing, or more than the drawing carries), `unknown_profile_format`,
/// `unsupported_profile_version`, `invalid_profile_mapping`, `profile_name_taken` (a built-in's name), `profile_downgrade`
/// (a profile of an earlier release than the one kept, without `overwrite`), `file_exists` (the folder),
/// `profile_exists` (the drawing), `file_unwritable`. Warnings: `mapping_unchecked` (the profile it is for was not
/// found), `profile_replaced`, `profile_shadowed` (the drawing and the folder both have one of the name and kind).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesImport {
    /// A JSON file, an absolute path or from the drawing's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<DataProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping: Option<ProfileMapping>,
    /// The open drawing's own file, an absolute path: the folder is beside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    /// A file of the same name is written over (a profile of the same or an earlier release too).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub overwrite: Option<bool>,
    /// Where it is kept: the folder (the default) or the drawing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub store: Option<ProfileStore>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output (and plan) of `cad.profiles.import` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileImported {
    pub kind: ProfileFileKind,
    pub name: String,
    pub title: String,
    /// A profile's release; a mapping's: 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub release: u32,
    /// The release of the file it replaced, when it replaced a profile's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub previous_release: Option<u32>,
    /// A profile's classes, a mapping's classes it names.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub classes: u32,
    /// A profile's migrations; the releases they go from, in order.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub migrations: u32,
    /// Where it is kept.
    pub store: ProfileStore,
    /// The file written, when it is kept in the folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// Whether a file (or the drawing's copy) was replaced.
    pub replaced: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
}

/// Input of `cad.profiles.remove` v1 (“Veri profilini çizimden kaldır”): a profile or a mapping the drawing carries inside
/// its file (docs/adr/0735) taken out of it; a settings edit, not an undo step. A file of the `profiller` folder is not
/// touched: when the folder holds one of the same name and kind, the commands read it from then on (`folderPath`).
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `unknown_profile` (a profile the drawing does not
/// carry), `unknown_mapping` (a mapping the drawing does not carry). Warnings: `profile_still_named` (a mapping the
/// project keeps names the profile, and no other copy of it is left).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesRemove {
    pub kind: ProfileFileKind,
    pub name: String,
    /// The open drawing's own file, an absolute path: its folder is looked in for a file of the same name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output (and plan) of `cad.profiles.remove` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileRemoved {
    pub kind: ProfileFileKind,
    pub name: String,
    /// The text taken out.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    /// The folder's file of the same name and kind, read from now on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub folder_path: Option<String>,
}

/// Input of `cad.profiles.migrate` v1 (“Veri profili göçü”): the project's data moved from release `from` of a profile to
/// its release `to` (the profile's own by default) by the steps the profile's migrations list: a layer or a field renamed,
/// a field's kind changed with its values carried through the new rule, values recoded, a code retired. All of it is one
/// undo step “Profil göçü” (the value lists too). A step whose layer or field the drawing has not is left out and said;
/// a value that does not fit is kept or emptied as the step says and reported by object. The profile is `profile`, else
/// the built-in or project `name`. It does not add what the new release brings: `cad.profiles.apply` does, and
/// `cad.profiles.check` says where the data still departs.
///
/// Refusals, in this order: `unknown_profile`, `invalid_profile`, `invalid_migration` (`to` not above `from`, or above
/// the profile's release), `migration_gap` (no unbroken chain of the profile's migrations from `from` to `to`),
/// `invalid_revision`, `revision_conflict`, `layer_refused` (a step the drawing refuses; nothing is written). Warnings:
/// `migration_step_skipped`, `profile_layer_skipped` (a locked layer), `values_unfit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesMigrate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<DataProfile>,
    /// The release the data is at.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub from: u32,
    /// The release it goes to; none: the profile's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub to: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    /// The institution's mapping the data is written with (docs/adr/0647 §4): each step is read through it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping: Option<ProfileMapping>,
    /// The mapping by its name, from the project's `profiller` folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// One step as it was run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MigratedStep {
    /// The release its migration goes from.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub migration: u32,
    /// Its place in the migration, from 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    pub kind: String,
    /// The layer's id, when the step has a layer and the drawing has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// Whether it changed the drawing; a step left out is `false`.
    pub applied: bool,
    /// The objects it wrote.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    pub note: String,
}

/// Why a value was not carried through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LossKind {
    /// The value did not fit the new rule and stays as written.
    Kept,
    /// The value did not fit the new rule and was emptied.
    Cleared,
}

/// Objects whose value did not go through a changed field's new rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MigrationLoss {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub migration: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub step: u32,
    pub layer: String,
    pub field: String,
    pub kind: LossKind,
    /// The value as it was.
    pub value: String,
    /// The objects, in the drawing's order, at most 1000.
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    pub message: String,
}

/// Output (and plan) of `cad.profiles.migrate` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesMigrated {
    /// Whether anything was written.
    pub changed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub from: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub to: u32,
    pub steps: Vec<MigratedStep>,
    /// The values that did not fit, by value and layer.
    pub losses: Vec<MigrationLoss>,
    /// Objects written, once each.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The mapping the steps were read through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping: Option<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// A field kind as the messages say it (small letters).
pub fn kind_word(kind: LayerFieldKind) -> String {
    kind.label().to_lowercase()
}
