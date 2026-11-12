//! What a zoning plan's project keeps beside its drawing (plan/map-cad.md MAP-09, docs/adr/5040; `.kcad` schema 250):
//! the user's symbol choices for a plan level and a use, which Sembolü karardan ver and the zoning check read before
//! KentOS's table, and plan change drafts. A draft names the plan it changes, the drawing's revision it was made from,
//! its reason (the plan report's justification) and, for each decision area it changes, a field's value before and
//! after; it never writes the decision areas in force, so the plan in force stays as it is until the change is made
//! elsewhere. A draft is never an approved change: approval, the plan's number and its announcement are outside KentOS.
//! The file reader checks the structure here; the commands check the drawing.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most symbol choices a project keeps.
pub const MAX_KEPT_SYMBOLS: usize = 1000;
/// The most drafts a project keeps.
pub const MAX_PLAN_DRAFTS: usize = 100;
/// The most changes a draft holds.
pub const MAX_DRAFT_CHANGES: usize = 10_000;
/// The most characters of a draft's id.
pub const MAX_DRAFT_ID: usize = 64;
/// The most characters of a level, a use, a symbol id, a plan id and a value.
pub const MAX_ZONING_LABEL: usize = 200;
/// The most characters of a draft's reason.
pub const MAX_DRAFT_REASON: usize = 4000;

/// The decision area fields a draft may change (the profile `imar-plani`'s `karar-alani`): the use, the building order
/// and the measured values.
pub const DRAFT_FIELDS: [&str; 9] = ["Kullanim", "Nizam", "TAKS", "KAKS", "KatAdedi", "Hmax", "OnCekme", "YanCekme", "ArkaCekme"];

/// The user's symbol for a use at a level: a library id among the level's choices.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningSymbolPick {
    pub level: String,
    #[serde(rename = "use")]
    #[cfg_attr(feature = "ts", ts(rename = "use"))]
    pub usage: String,
    pub symbol: String,
}

/// A change a draft makes to a decision area: the field and its value before (empty: the area had none) and after.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlanDraftChange {
    /// The decision area's persistent id.
    pub area: String,
    pub field: String,
    #[serde(default)]
    pub before: String,
    pub after: String,
}

/// A plan change draft: its id, the plan (`PlanKimlik`) it changes, the drawing's revision it was made from, its
/// reason and its changes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlanDraft {
    pub id: String,
    pub plan: String,
    pub revision: String,
    pub reason: String,
    pub changes: Vec<PlanDraftChange>,
}

/// The project's zoning plan data (schema 250): the kept symbol choices and the plan change drafts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningPlanData {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ZoningSymbolPick>>"))]
    pub symbols: Vec<ZoningSymbolPick>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<PlanDraft>>"))]
    pub drafts: Vec<PlanDraft>,
}

impl ZoningPlanData {
    /// Neither a choice nor a draft: a project keeps none then.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty() && self.drafts.is_empty()
    }
}

/// A short text: not empty, no space at its ends, no control character, at most `most` characters.
fn label_ok(t: &str, most: usize) -> bool {
    !t.is_empty() && t.trim() == t && t.chars().count() <= most && !t.chars().any(char::is_control)
}

/// A persistent id as the drawing writes it: a lowercase, hyphenated UUID.
fn uid_ok(t: &str) -> bool {
    t.len() == 36
        && t.char_indices().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_digit() || ('a'..='f').contains(&c) })
}

/// Why the data cannot be kept: the field and what is wrong, or none.
pub fn zoning_plan_problem(d: &ZoningPlanData) -> Option<String> {
    if d.symbols.len() > MAX_KEPT_SYMBOLS {
        return Some(format!("symbols: en çok {MAX_KEPT_SYMBOLS} sembol seçimi saklanır"));
    }
    for (i, s) in d.symbols.iter().enumerate() {
        if ![&s.level, &s.usage, &s.symbol].iter().all(|t| label_ok(t, MAX_ZONING_LABEL)) {
            return Some(format!(
                "symbols[{i}]: kademe, kullanım ve sembol boşluksuz başlayıp biten 1–{MAX_ZONING_LABEL} karakterdir"
            ));
        }
        if d.symbols[..i].iter().any(|o| o.level == s.level && o.usage == s.usage) {
            return Some(format!("symbols[{i}]: aynı kademe ve kullanımın sembolü iki kez yazılmış"));
        }
    }
    if d.drafts.len() > MAX_PLAN_DRAFTS {
        return Some(format!("drafts: en çok {MAX_PLAN_DRAFTS} plan değişikliği taslağı saklanır"));
    }
    for (i, t) in d.drafts.iter().enumerate() {
        let at = |f: &str| format!("drafts[{i}].{f}");
        if !label_ok(&t.id, MAX_DRAFT_ID) {
            return Some(format!("{}: taslağın kimliği boşluksuz başlayıp biten 1–{MAX_DRAFT_ID} karakterdir", at("id")));
        }
        if d.drafts[..i].iter().any(|o| o.id == t.id) {
            return Some(format!("{}: “{}” kimlikli iki taslak var", at("id"), t.id));
        }
        if !label_ok(&t.plan, MAX_ZONING_LABEL) {
            return Some(format!("{}: planın kimliği boşluksuz başlayıp biten 1–{MAX_ZONING_LABEL} karakterdir", at("plan")));
        }
        if t.revision.is_empty() || !t.revision.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("{}: taslağın yapıldığı çizim sürümü bir tamsayı yazısıdır", at("revision")));
        }
        if t.reason.trim() != t.reason || t.reason.is_empty() || t.reason.chars().count() > MAX_DRAFT_REASON {
            return Some(format!("{}: gerekçe boşluksuz başlayıp biten 1–{MAX_DRAFT_REASON} karakterdir", at("reason")));
        }
        if t.changes.is_empty() || t.changes.len() > MAX_DRAFT_CHANGES {
            return Some(format!("{}: taslakta 1–{MAX_DRAFT_CHANGES} değişiklik olur", at("changes")));
        }
        for (k, c) in t.changes.iter().enumerate() {
            let at = format!("drafts[{i}].changes[{k}]");
            if !uid_ok(&c.area) {
                return Some(format!("{at}.area: karar alanının kimliği küçük harfli, tireli bir UUID'dir"));
            }
            if !DRAFT_FIELDS.contains(&c.field.as_str()) {
                return Some(format!("{at}.field: taslak yalnız {} alanlarını değiştirir", DRAFT_FIELDS.join(", ")));
            }
            if t.changes[..k].iter().any(|o| o.area == c.area && o.field == c.field) {
                return Some(format!("{at}: aynı karar alanının aynı alanı iki kez değişiyor"));
            }
            if !(c.before.is_empty() || label_ok(&c.before, MAX_ZONING_LABEL)) || !label_ok(&c.after, MAX_ZONING_LABEL) {
                return Some(format!(
                    "{at}: önceki ve sonraki değer boşluksuz başlayıp biten en çok {MAX_ZONING_LABEL} karakterdir; sonraki boş olmaz"
                ));
            }
            if c.before == c.after {
                return Some(format!("{at}: önceki ve sonraki değer aynı"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> ZoningPlanData {
        ZoningPlanData {
            symbols: vec![ZoningSymbolPick { level: "uip".into(), usage: "konut".into(), symbol: "a".into() }],
            drafts: vec![PlanDraft {
                id: "T1".into(),
                plan: "UİP-1".into(),
                revision: "0".into(),
                reason: "Gerekçe".into(),
                changes: vec![PlanDraftChange {
                    area: "0192f5c0-5040-7000-8000-000000000001".into(),
                    field: "Kullanim".into(),
                    before: "park".into(),
                    after: "ticaret".into(),
                }],
            }],
        }
    }

    #[test]
    fn a_draft_and_a_choice_are_kept_and_each_rule_refuses() {
        assert_eq!(zoning_plan_problem(&draft()), None);
        let refused = |change: fn(&mut ZoningPlanData)| {
            let mut d = draft();
            change(&mut d);
            zoning_plan_problem(&d).is_some()
        };
        assert!(refused(|d| d.symbols.push(d.symbols[0].clone())));
        assert!(refused(|d| d.drafts[0].reason = String::new()));
        assert!(refused(|d| d.drafts[0].revision = "x".into()));
        assert!(refused(|d| d.drafts[0].changes.clear()));
        assert!(refused(|d| d.drafts[0].changes[0].field = "Plan".into()));
        assert!(refused(|d| d.drafts[0].changes[0].after = "park".into()));
        assert!(refused(|d| d.drafts[0].changes[0].area = "X".into()));
        assert!(refused(|d| d.drafts.push(d.drafts[0].clone())));
    }
}
