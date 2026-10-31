//! Design options (docs/adr/0788, ADV-05c): a bounded set of alternatives for one part of the plan (a flat's kitchen
//! A and B). An option set holds its options, one of them primary; each option holds its objects by their persistent
//! ids. An object in no option is the main model. The drawing, the sheets, the quantities and the schedules show the
//! main model and each set's primary option; a view's recipe (docs/adr/0774) may name another option of a set. The
//! options of one set do not see each other: their walls do not join, their openings do not clash, a space finds its
//! boundary among the main model's walls and its own option's. Accepting an option keeps it as the main model and
//! deletes the others. Phases (docs/adr/0346) are not used for this: an option is not a construction phase.
//!
//! The project keeps its sets in `settings.arch.options` (KCAD 149; docs/adr/0788 §6).

#[cfg(feature = "ts")]
use ts_rs::TS;

use serde::{Deserialize, Serialize};

/// The most option sets a project keeps, the most options in one set and the most objects all options hold.
pub const MAX_OPTION_SETS: usize = 50;
pub const MAX_OPTIONS: usize = 12;
pub const MAX_OPTION_MEMBERS: usize = 20_000;

/// One alternative of a set: its id, its name and its objects by their persistent ids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DesignOption {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub members: Vec<String>,
}

/// A set of alternatives for one part of the plan, one of them primary (the one the drawing shows).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionSet {
    pub id: String,
    pub name: String,
    /// The id of the option the drawing shows.
    pub primary: String,
    pub options: Vec<DesignOption>,
}

/// A view's choice of one set's option (docs/adr/0788 §3): the set's id and the option's.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionChoice {
    pub set: String,
    pub option: String,
}

impl OptionSet {
    /// The option of this id, or else of this name (case aside).
    pub fn option(&self, key: &str) -> Option<&DesignOption> {
        let folded = key.trim().to_lowercase();
        self.options
            .iter()
            .find(|o| o.id == key)
            .or_else(|| self.options.iter().find(|o| o.name.to_lowercase() == folded))
    }

    /// The option `choices` name for this set, else the primary.
    pub fn shown<'a>(&'a self, choices: &'a [OptionChoice]) -> &'a str {
        choices
            .iter()
            .find(|c| c.set == self.id && self.options.iter().any(|o| o.id == c.option))
            .map_or(self.primary.as_str(), |c| c.option.as_str())
    }
}

/// Whether `id` is a set's or an option's id: 1–64 lower-case ASCII letters, digits and hyphens.
pub fn option_id_ok(id: &str) -> bool {
    crate::module_id_ok(id)
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

/// The set of this id, or else of this name (case aside).
pub fn option_set<'a>(sets: &'a [OptionSet], key: &str) -> Option<&'a OptionSet> {
    let folded = key.trim().to_lowercase();
    sets.iter()
        .find(|s| s.id == key)
        .or_else(|| sets.iter().find(|s| s.name.to_lowercase() == folded))
}

/// The set and option an object (by its persistent id) is in; none: the main model.
pub fn option_of<'a>(sets: &'a [OptionSet], uid: &str) -> Option<(&'a OptionSet, &'a DesignOption)> {
    sets.iter().find_map(|s| {
        s.options
            .iter()
            .find(|o| o.members.iter().any(|m| m == uid))
            .map(|o| (s, o))
    })
}

/// The objects a view with `choices` leaves out: each set's options but the one it shows (the primary unless chosen).
pub fn option_hidden_uids<'a>(sets: &'a [OptionSet], choices: &[OptionChoice]) -> Vec<&'a str> {
    let mut out = Vec::new();
    for s in sets {
        let shown = s.shown(choices);
        for o in s.options.iter().filter(|o| o.id != shown) {
            out.extend(o.members.iter().map(String::as_str));
        }
    }
    out
}

/// What is wrong with a project's option sets: its path and why; none when nothing is.
pub fn option_sets_problem(sets: &[OptionSet]) -> Option<(String, String)> {
    if sets.len() > MAX_OPTION_SETS {
        return Some((
            "options".into(),
            format!("Bir projede en çok {MAX_OPTION_SETS} tasarım seçeneği kümesi olur."),
        ));
    }
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    let mut uids = std::collections::HashSet::new();
    for (i, s) in sets.iter().enumerate() {
        let at = format!("options[{i}]");
        if !option_id_ok(&s.id) || !ids.insert(s.id.as_str()) {
            return Some((
                format!("{at}.id"),
                format!(
                    "“{}” seçenek kümesi kimliği olamaz ya da yineleniyor: küçük harf, rakam ve tire, en çok 64 karakter.",
                    s.id
                ),
            ));
        }
        let name = s.name.trim();
        if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
            return Some((
                format!("{at}.name"),
                "Seçenek kümesinin adı boş olamaz; en çok 120 karakter.".into(),
            ));
        }
        if !names.insert(name.to_lowercase()) {
            return Some((
                format!("{at}.name"),
                format!("“{name}” adlı iki seçenek kümesi var."),
            ));
        }
        if s.options.is_empty() || s.options.len() > MAX_OPTIONS {
            return Some((
                format!("{at}.options"),
                format!("Bir kümede 1 ile {MAX_OPTIONS} arası seçenek olur."),
            ));
        }
        let mut option_ids = std::collections::HashSet::new();
        let mut option_names = std::collections::HashSet::new();
        for (j, o) in s.options.iter().enumerate() {
            let oa = format!("{at}.options[{j}]");
            if !option_id_ok(&o.id) || !option_ids.insert(o.id.as_str()) {
                return Some((
                    format!("{oa}.id"),
                    "Seçeneğin kimliği geçersiz ya da kümede yineleniyor.".into(),
                ));
            }
            let name = o.name.trim();
            if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
                return Some((
                    format!("{oa}.name"),
                    "Seçeneğin adı boş olamaz; en çok 120 karakter.".into(),
                ));
            }
            if !option_names.insert(name.to_lowercase()) {
                return Some((
                    format!("{oa}.name"),
                    format!("Kümede “{name}” adlı iki seçenek var."),
                ));
            }
            for (k, m) in o.members.iter().enumerate() {
                if !uuid_text_ok(m) {
                    return Some((
                        format!("{oa}.members[{k}]"),
                        "Seçeneğin nesnesi küçük harfli UUID metniyle verilir.".into(),
                    ));
                }
                if !uids.insert(m.as_str()) {
                    return Some((
                        format!("{oa}.members[{k}]"),
                        "Bir nesne yalnız bir seçenekte olur.".into(),
                    ));
                }
            }
        }
        if !s.options.iter().any(|o| o.id == s.primary) {
            return Some((
                format!("{at}.primary"),
                "Kümenin birincil seçeneği kümenin seçeneklerinden biri olur.".into(),
            ));
        }
    }
    if uids.len() > MAX_OPTION_MEMBERS {
        return Some((
            "options".into(),
            format!("Seçeneklerde en çok {MAX_OPTION_MEMBERS} nesne olur."),
        ));
    }
    None
}

/// What is wrong with a view recipe's option choices on their own: an id, a set twice; none when they hold.
pub fn option_choices_problem(choices: &[OptionChoice]) -> Option<String> {
    for (i, c) in choices.iter().enumerate() {
        if !option_id_ok(&c.set) || !option_id_ok(&c.option) {
            return Some("Görünüş tarifinin seçeneği bir kümenin ve seçeneğin kimliğiyle verilir.".into());
        }
        if choices[..i].iter().any(|d| d.set == c.set) {
            return Some(format!("“{}” kümesi görünüş tarifinde iki kez seçilmiş.", c.set));
        }
    }
    if choices.len() > MAX_OPTION_SETS {
        return Some(format!("Görünüş tarifinde en çok {MAX_OPTION_SETS} seçenek seçilir."));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> OptionSet {
        OptionSet {
            id: "mutfak".into(),
            name: "Mutfak".into(),
            primary: "a".into(),
            options: vec![
                DesignOption {
                    id: "a".into(),
                    name: "A".into(),
                    members: vec!["00000000-0000-4000-8000-000000000001".into()],
                },
                DesignOption {
                    id: "b".into(),
                    name: "B".into(),
                    members: vec!["00000000-0000-4000-8000-000000000002".into()],
                },
            ],
        }
    }

    #[test]
    fn the_primary_is_shown_unless_a_view_chooses() {
        let sets = vec![set()];
        assert_eq!(option_hidden_uids(&sets, &[]), vec!["00000000-0000-4000-8000-000000000002"]);
        let b = [OptionChoice {
            set: "mutfak".into(),
            option: "b".into(),
        }];
        assert_eq!(option_hidden_uids(&sets, &b), vec!["00000000-0000-4000-8000-000000000001"]);
        assert!(option_sets_problem(&sets).is_none());
    }

    #[test]
    fn an_object_is_in_one_option_only() {
        let mut s = set();
        s.options[1].members = s.options[0].members.clone();
        assert!(option_sets_problem(&[s]).is_some());
    }
}
