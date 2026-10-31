//! Room modules (docs/adr/0775, ADV-05): a bathroom or a room drawn once and placed again as linked instances. A
//! module's definition is its members (walls, openings, spaces, fixtures, dimensions, tags, texts) in its own frame,
//! its base at the origin, each under a key; each instance says where the definition stands (point, turn, mirror,
//! level, phase, numbering), which version it took, which of its objects stands for which key, and which keys it
//! keeps as its own (local exceptions). The objects are ordinary drawing objects: nothing hides in a block.
//!
//! The project keeps its modules in `settings.arch.modules` (KCAD 138).

#[cfg(feature = "ts")]
use ts_rs::TS;

use serde::{Deserialize, Serialize};

use crate::{ArchPhase, Entity, Vec2};

/// The most modules a project keeps, and the most members and instances one has.
pub const MAX_MODULES: usize = 200;
pub const MAX_MODULE_MEMBERS: usize = 2_000;
pub const MAX_MODULE_INSTANCES: usize = 1_000;

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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    pub members: Vec<ModuleObject>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ModuleInstance>>", optional))]
    pub instances: Vec<ModuleInstance>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub removed: bool,
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
    matches!(
        e,
        Entity::Wall(_)
            | Entity::Opening(_)
            | Entity::Space(_)
            | Entity::Insert(_)
            | Entity::Dimension(_)
            | Entity::Tag(_)
            | Entity::Text(_)
    )
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
        if m.removed && !m.members.is_empty() {
            return Some((
                format!("{at}.members"),
                "Kaldırılmış modülün tanımı boş olur.".into(),
            ));
        }
        if !m.removed && m.members.is_empty() {
            return Some((
                format!("{at}.members"),
                "Modülün tanımında en az bir nesne olur.".into(),
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
                    "Modülde duvar, açıklık, mahal, donatı, ölçü, etiket ve yazı olur.".into(),
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
            for (k, e) in inst.exceptions.iter().enumerate() {
                if !own.contains(e.as_str()) || inst.exceptions[..k].contains(e) {
                    return Some((
                        format!("{ia}.exceptions[{k}]"),
                        "Örnek istisnası örneğin bir nesnesinin anahtarıdır ve bir kez geçer."
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
