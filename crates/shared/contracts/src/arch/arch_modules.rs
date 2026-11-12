//! Room modules (docs/adr/0775, ADV-05): a bathroom or a room drawn once and placed again as linked instances. A
//! module's definition is its members (walls, openings, spaces, fixtures, dimensions, tags, texts) in its own frame,
//! its base at the origin, each under a key; each instance says where the definition stands (point, turn, mirror,
//! level, phase, numbering), which version it took, which of its objects stands for which key, and which keys it
//! keeps as its own (local exceptions). The objects are ordinary drawing objects: nothing hides in a block.
//!
//! The project keeps its modules in `settings.arch.modules` (KCAD 142); an instance's building-program group (`unit`)
//! came after its first slice (docs/adr/0779, KCAD 145): a flat placed again takes its own flat. A member's storey
//! (docs/adr/0796, KCAD 152): a module may span several levels (a duplex, a stair core), each member on its storey above
//! the instance's base level.

#[cfg(feature = "ts")]
use ts_rs::TS;

use serde::{Deserialize, Serialize};

use crate::{ArchPhase, Entity, Vec2};

/// The most modules a project keeps, and the most members and instances one has.
pub const MAX_MODULES: usize = 200;
pub const MAX_MODULE_MEMBERS: usize = 2_000;
pub const MAX_MODULE_INSTANCES: usize = 1_000;
/// The most storeys a module spans (docs/adr/0796 §1): a member's storey is below it.
pub const MAX_MODULE_STOREYS: u32 = 16;

/// One object of a module's definition: its key (the id of the object it was taken from) and the object in the
/// definition's frame, its level and phase left to the instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleMember {
    pub key: String,
    pub entity: Entity,
    /// Its storey above the instance's base level (docs/adr/0796 §1): 0 the base level, 1 the project's next level up
    /// (the levels by elevation). Only an object's with a level of its own ([`holds_own_storey`]); an opening, a tag, a
    /// dimension or a text follows what it is on.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    pub storey: u32,
}

/// One object of an instance: the definition's key and the drawing object that stands for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleObject {
    pub key: String,
    pub uid: String,
}

/// A kitchen run a module holds (docs/adr/8050 §5, KCAD 245): its recipe about the definition's base
/// (`cad.casework.run`'s input without an expected revision), laid again by each instance with its matrix (a mirror
/// turns its side and its single doors' hands over) among the instance's walls: each instance's run is a run of its own,
/// following its own walls; its objects are the run's, not the module's members.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleRun {
    /// Its key: the id of the run it was taken from.
    pub key: String,
    pub recipe: crate::CaseworkRun,
    /// Its storey above the instance's base level (docs/adr/0796 §1), as a member's.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    pub storey: u32,
}

/// How an instance numbers its spaces, openings and tags' marks: the definition's number with a prefix and a suffix.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleNumbering {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub suffix: Option<String>,
}

impl ModuleNumbering {
    pub fn is_empty(&self) -> bool {
        self.prefix.as_deref().is_none_or(str::is_empty)
            && self.suffix.as_deref().is_none_or(str::is_empty)
    }

    /// `number` as this instance writes it.
    pub fn apply(&self, number: &str) -> String {
        format!(
            "{}{number}{}",
            self.prefix.as_deref().unwrap_or(""),
            self.suffix.as_deref().unwrap_or("")
        )
    }
}

/// Why an instance's link broke: its module's definition was removed; its objects stay as they last were.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ModuleBreak {
    SourceGone,
}

/// Where a module stands once: its base point, its turn (radians, counter-clockwise), whether it is mirrored (across
/// its own y axis, before the turn), its level and phase, its numbering, the definition's version it took, its
/// objects and the keys it keeps as its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleInstance {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mirror: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub phase: Option<ArchPhase>,
    #[serde(default, skip_serializing_if = "ModuleNumbering::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ModuleNumbering>", optional))]
    pub numbering: ModuleNumbering,
    /// The building program's group (docs/adr/0773) this instance's spaces are in (docs/adr/0779 §4): a flat module
    /// placed again is another flat. None: its spaces keep the definition's group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
    /// One storey of a module of several (docs/adr/0811, Archicad's Single Story): the instance holds only the
    /// definition's members on that storey, standing on the instance's `level`. None: every storey, the base on `level`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub storey: Option<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    pub members: Vec<ModuleObject>,
    /// Its kitchen runs (docs/adr/8050 §5): the definition's run key and the id of the instance's run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ModuleObject>>", optional))]
    pub runs: Vec<ModuleObject>,
    /// The keys it keeps as its own: its objects' and its runs'.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub exceptions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<ModuleBreak>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

/// A room module: its id and name, its definition's version and members, and its instances. A removed module keeps
/// its instances, broken, and no members.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchModule {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ModuleMember>>", optional))]
    pub members: Vec<ModuleMember>,
    /// Its kitchen runs (docs/adr/8050 §5, KCAD 245).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ModuleRun>>", optional))]
    pub runs: Vec<ModuleRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ModuleInstance>>", optional))]
    pub instances: Vec<ModuleInstance>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub removed: bool,
}

/// Whether a member of `modules` stands on a storey above its instance's base level (KCAD 152, docs/adr/0796 §1).
pub fn modules_have_storeys(modules: &[ArchModule]) -> bool {
    modules.iter().any(|m| m.members.iter().any(|x| x.storey > 0))
}

/// Whether an instance of `modules` holds one storey only (KCAD 155, docs/adr/0811).
pub fn modules_have_instance_storeys(modules: &[ArchModule]) -> bool {
    modules
        .iter()
        .any(|m| m.instances.iter().any(|i| i.storey.is_some()))
}

/// Whether a module of `modules` holds a kitchen run (KCAD 245, docs/adr/8050 §5).
pub fn modules_have_runs(modules: &[ArchModule]) -> bool {
    modules
        .iter()
        .any(|m| !m.runs.is_empty() || m.instances.iter().any(|i| !i.runs.is_empty()))
}

/// Whether `modules` hold what came with the module scope (KCAD 245, docs/adr/8050): a column, a stair, a ramp, a slab
/// or a room separator as a member, or a kitchen run.
pub fn modules_have_scope(modules: &[ArchModule]) -> bool {
    modules_have_runs(modules)
        || modules.iter().any(|m| {
            m.members.iter().any(|x| {
                matches!(
                    x.entity,
                    Entity::Column(_)
                        | Entity::Stair(_)
                        | Entity::Ramp(_)
                        | Entity::Slab(_)
                        | Entity::Polyline(_)
                )
            })
        })
}

/// Whether any instance of `modules` names a building-program group (KCAD 145, docs/adr/0779 §4).
pub fn modules_have_units(modules: &[ArchModule]) -> bool {
    modules
        .iter()
        .any(|m| m.instances.iter().any(|i| i.unit.is_some()))
}

impl ArchModule {
    /// The instance of this id.
    pub fn instance(&self, id: &str) -> Option<&ModuleInstance> {
        self.instances.iter().find(|i| i.id == id)
    }

    /// The member of this key.
    pub fn member(&self, key: &str) -> Option<&ModuleMember> {
        self.members.iter().find(|m| m.key == key)
    }

    /// The kitchen run of this key (docs/adr/8050 §5).
    pub fn run(&self, key: &str) -> Option<&ModuleRun> {
        self.runs.iter().find(|r| r.key == key)
    }

    /// How many storeys the definition spans: one past its highest member's or run's (docs/adr/0796 §1).
    pub fn storeys(&self) -> u32 {
        self.members
            .iter()
            .map(|m| m.storey)
            .chain(self.runs.iter().map(|r| r.storey))
            .max()
            .unwrap_or(0)
            + 1
    }
}

/// Whether `id` is a module's id: 1–64 lower-case ASCII letters, digits and hyphens.
pub fn module_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Whether a member may be one: the kinds a first module takes (docs/adr/0775 §1).
pub fn module_kind_ok(e: &Entity) -> bool {
    // docs/adr/8050 §4: a room separator, not a plain polyline.
    if let Entity::Polyline(p) = e {
        return p.space_separator.is_some();
    }
    matches!(
        e,
        Entity::Wall(_)
            | Entity::Opening(_)
            | Entity::Space(_)
            | Entity::Insert(_)
            | Entity::Dimension(_)
            | Entity::Tag(_)
            | Entity::Text(_)
            // docs/adr/8050 §1: a module's columns, stairs, ramps and slabs.
            | Entity::Column(_)
            | Entity::Stair(_)
            | Entity::Ramp(_)
            | Entity::Slab(_)
    )
}

/// Whether a member stands on a storey of its own (docs/adr/0796 §1, 8050 §1): a wall, a space, a fixture, a column, a
/// stair, a ramp, a slab or a room separator; an opening, a tag, a dimension or a text follows what it is on.
pub fn holds_own_storey(e: &Entity) -> bool {
    match e {
        Entity::Polyline(p) => p.space_separator.is_some(),
        _ => matches!(
            e,
            Entity::Wall(_)
                | Entity::Space(_)
                | Entity::Insert(_)
                | Entity::Column(_)
                | Entity::Stair(_)
                | Entity::Ramp(_)
                | Entity::Slab(_)
        ),
    }
}

fn uuid_text_ok(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_digit() || matches!(c, 'a'..='f')
            }
        })
}

/// What is wrong with a module's kitchen run's recipe (docs/adr/8050 §5): its points finite, at most two corners, no
/// expected revision and a layer.
fn run_recipe_problem(r: &crate::CaseworkRun) -> Option<&'static str> {
    if r.layer_id.is_empty() {
        return Some("Mutfak dizisinin katmanı boş.");
    }
    if r.through
        .as_ref()
        .is_some_and(|t| t.len() > crate::CASEWORK_CORNERS_MAX)
    {
        return Some("Mutfak dizisi en çok iki köşelidir.");
    }
    if !r.path().iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
        return Some("Mutfak dizisinin noktaları sonlu sayılar olur.");
    }
    if r.expected_revision.is_some() {
        return Some("Mutfak dizisinin tarifi beklenen sürüm taşımaz.");
    }
    None
}

/// What is wrong with a project's modules: its path and why; none when nothing is.
pub fn arch_modules_problem(modules: &[ArchModule]) -> Option<(String, String)> {
    if modules.len() > MAX_MODULES {
        return Some((
            "modules".into(),
            format!("Bir projede en çok {MAX_MODULES} oda modülü olur."),
        ));
    }
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    let mut uids = std::collections::HashSet::new();
    let mut runs = std::collections::HashSet::new();
    for (i, m) in modules.iter().enumerate() {
        let at = format!("modules[{i}]");
        if !module_id_ok(&m.id) {
            return Some((
                format!("{at}.id"),
                format!(
                    "“{}” modül kimliği olamaz: küçük harf, rakam ve tire, en çok 64 karakter.",
                    m.id
                ),
            ));
        }
        if !ids.insert(m.id.as_str()) {
            return Some((
                format!("{at}.id"),
                format!("“{}” kimlikli iki modül var.", m.id),
            ));
        }
        let name = m.name.trim();
        if name.is_empty() || name.chars().count() > 120 {
            return Some((
                format!("{at}.name"),
                "Modülün adı boş olamaz; en çok 120 karakter.".into(),
            ));
        }
        if !names.insert(name.to_lowercase()) {
            return Some((
                format!("{at}.name"),
                format!("“{name}” adlı iki modül var."),
            ));
        }
        if m.version == 0 {
            return Some((
                format!("{at}.version"),
                "Modülün sürümü 1 ya da daha büyük olmalı.".into(),
            ));
        }
        if m.removed && (!m.members.is_empty() || !m.runs.is_empty()) {
            return Some((
                format!("{at}.members"),
                "Kaldırılmış modülün tanımı boş olur.".into(),
            ));
        }
        if !m.removed && m.members.is_empty() && m.runs.is_empty() {
            return Some((
                format!("{at}.members"),
                "Modülün tanımında en az bir nesne ya da mutfak dizisi olur.".into(),
            ));
        }
        if m.members.len() > MAX_MODULE_MEMBERS || m.instances.len() > MAX_MODULE_INSTANCES {
            return Some((
                at,
                format!(
                    "Bir modülde en çok {MAX_MODULE_MEMBERS} nesne ve {MAX_MODULE_INSTANCES} örnek olur."
                ),
            ));
        }
        let mut keys = std::collections::HashSet::new();
        for (k, member) in m.members.iter().enumerate() {
            if !uuid_text_ok(&member.key) || !keys.insert(member.key.as_str()) {
                return Some((
                    format!("{at}.members[{k}].key"),
                    "Modül nesnesinin anahtarı küçük harfli UUID metni olur ve bir kez geçer."
                        .into(),
                ));
            }
            if !module_kind_ok(&member.entity) {
                return Some((
                    format!("{at}.members[{k}].entity"),
                    "Modülde duvar, açıklık, mahal, mahal ayırıcı, donatı, kolon, merdiven, rampa, döşeme, ölçü, etiket ve yazı olur.".into(),
                ));
            }
            if member.storey >= MAX_MODULE_STOREYS
                || (member.storey > 0 && !holds_own_storey(&member.entity))
            {
                return Some((
                    format!("{at}.members[{k}].storey"),
                    format!(
                        "Modül nesnesinin katı 0 ile {} arasındadır ve yalnız kendi katı olan nesnenin olur (duvar, mahal, mahal ayırıcı, donatı, kolon, merdiven, rampa, döşeme).",
                        MAX_MODULE_STOREYS - 1
                    ),
                ));
            }
        }
        for (k, run) in m.runs.iter().enumerate() {
            if !uuid_text_ok(&run.key) || !keys.insert(run.key.as_str()) {
                return Some((
                    format!("{at}.runs[{k}].key"),
                    "Modülün mutfak dizisinin anahtarı küçük harfli UUID metni olur ve bir kez geçer.".into(),
                ));
            }
            if let Some(why) = run_recipe_problem(&run.recipe) {
                return Some((format!("{at}.runs[{k}].recipe"), why.into()));
            }
            if run.storey >= MAX_MODULE_STOREYS {
                return Some((
                    format!("{at}.runs[{k}].storey"),
                    format!(
                        "Mutfak dizisinin katı 0 ile {} arasındadır.",
                        MAX_MODULE_STOREYS - 1
                    ),
                ));
            }
        }
        let mut instance_ids = std::collections::HashSet::new();
        for (j, inst) in m.instances.iter().enumerate() {
            let ia = format!("{at}.instances[{j}]");
            if !module_id_ok(&inst.id) || !instance_ids.insert(inst.id.as_str()) {
                return Some((
                    format!("{ia}.id"),
                    "Örneğin kimliği geçersiz ya da yineleniyor.".into(),
                ));
            }
            if !(inst.at.x.is_finite() && inst.at.y.is_finite() && inst.rotation.is_finite()) {
                return Some((
                    format!("{ia}.at"),
                    "Örneğin yeri ve dönüşü sonlu sayılar olur.".into(),
                ));
            }
            if inst
                .unit
                .as_deref()
                .is_some_and(|u| u.trim().is_empty() || u.chars().count() > 64)
            {
                return Some((
                    format!("{ia}.unit"),
                    "Örneğin bölümü boş olamaz; en çok 64 karakter.".into(),
                ));
            }
            if inst.storey.is_some_and(|s| s >= MAX_MODULE_STOREYS) {
                return Some((
                    format!("{ia}.storey"),
                    format!("Örneğin katı 0 ile {} arasındadır.", MAX_MODULE_STOREYS - 1),
                ));
            }
            if inst.version == 0 || inst.version > m.version {
                return Some((
                    format!("{ia}.version"),
                    "Örneğin sürümü 1 ile modülün sürümü arasında olur.".into(),
                ));
            }
            if m.removed != (inst.broken == Some(ModuleBreak::SourceGone)) {
                return Some((
                    format!("{ia}.broken"),
                    "Kaynağı kaldırılmış modülün örnekleri kopuk (sourceGone) olur, öbürleri olmaz.".into(),
                ));
            }
            let mut own = std::collections::HashSet::new();
            for (k, o) in inst.members.iter().enumerate() {
                if (!m.removed && m.member(&o.key).is_none()) || !own.insert(o.key.as_str()) {
                    return Some((
                        format!("{ia}.members[{k}].key"),
                        "Örneğin nesnesi tanımın bir anahtarını bir kez anar.".into(),
                    ));
                }
                if !uuid_text_ok(&o.uid) || !uids.insert(o.uid.as_str()) {
                    return Some((
                        format!("{ia}.members[{k}].uid"),
                        "Bir nesne yalnız bir örneğin olur; kimliği küçük harfli UUID metni olur."
                            .into(),
                    ));
                }
            }
            for (k, o) in inst.runs.iter().enumerate() {
                if (!m.removed && m.run(&o.key).is_none()) || !own.insert(o.key.as_str()) {
                    return Some((
                        format!("{ia}.runs[{k}].key"),
                        "Örneğin mutfak dizisi tanımın bir dizisinin anahtarını bir kez anar."
                            .into(),
                    ));
                }
                if !uuid_text_ok(&o.uid) || !runs.insert(o.uid.as_str()) {
                    return Some((
                        format!("{ia}.runs[{k}].uid"),
                        "Bir mutfak dizisi yalnız bir örneğin olur; kimliği küçük harfli UUID metni olur.".into(),
                    ));
                }
            }
            for (k, e) in inst.exceptions.iter().enumerate() {
                if !own.contains(e.as_str()) || inst.exceptions[..k].contains(e) {
                    return Some((
                        format!("{ia}.exceptions[{k}]"),
                        "Örnek istisnası örneğin bir nesnesinin ya da mutfak dizisinin anahtarıdır ve bir kez geçer."
                            .into(),
                    ));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbering_wraps_a_number() {
        let n = ModuleNumbering {
            prefix: Some("D2-".into()),
            suffix: None,
        };
        assert_eq!(n.apply("B1"), "D2-B1");
        assert!(ModuleNumbering::default().is_empty());
    }

    #[test]
    fn ids_are_lower_case() {
        assert!(module_id_ok("banyo-1"));
        assert!(!module_id_ok("Banyo"));
        assert!(!module_id_ok(""));
    }
}
