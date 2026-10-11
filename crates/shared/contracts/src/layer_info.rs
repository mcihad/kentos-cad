//! A layer's actions and information card (docs/adr/0225 §4, §5): the
//! addresses and files its objects open (Eylemler) and what the Bilgi tool
//! shows of them (Bilgi kartı). Their texts are templates (docs/adr/0225 §6:
//! `{…}` an expression evaluated on the object). The rules here are the
//! readers', the commands' and the server's; whether a template splits and
//! its expressions compile is the commands' (a file's reader does not know
//! the language).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::layer::{LayerNode, LayerNodeType};

/// The most actions a layer has.
pub const MAX_ACTIONS: usize = 32;
/// The longest action name and card title, in characters.
pub const MAX_ACTION_NAME: usize = 100;
/// The longest action target, in characters.
pub const MAX_ACTION_TARGET: usize = 4_096;
/// The longest card title, in characters.
pub const MAX_CARD_TITLE: usize = 1_000;
/// The longest card text, in characters.
pub const MAX_CARD_TEXT: usize = 10_000;
/// The most attributes a card names.
pub const MAX_CARD_FIELDS: usize = 200;

/// An action of a layer's objects (docs/adr/0225 §4): its name and the
/// address or file it opens, a template.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerAction {
    /// The name in the menus and on the card.
    pub name: String,
    /// An `http`, `https` or `mailto` address or a file's path, a template.
    pub target: String,
}

fn yes() -> bool {
    true
}

fn is_yes(b: &bool) -> bool {
    *b
}

/// What the Bilgi tool shows of a layer's objects (docs/adr/0225 §5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InfoCard {
    /// The card's title, a template; absent: the object's kind and label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub title: Option<String>,
    /// The attributes shown, in order; empty: all the object has.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
    /// Lines under the attributes, a template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    /// Whether the object's attachments show (absent: they do).
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub attachments: bool,
    /// Whether the layer's actions show (absent: they do).
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub actions: bool,
}

impl Default for InfoCard {
    fn default() -> Self {
        Self {
            title: None,
            fields: Vec::new(),
            text: None,
            attachments: true,
            actions: true,
        }
    }
}

/// A control character other than those `allowed`.
fn has_control(text: &str, allowed: &[char]) -> bool {
    text.chars()
        .any(|c| c.is_control() && !allowed.contains(&c))
}

/// What is wrong with a layer's actions, when anything is.
pub fn actions_problem(actions: &[LayerAction]) -> Option<String> {
    if actions.len() > MAX_ACTIONS {
        return Some(format!(
            "katmanın {} eylemi var; en çok {MAX_ACTIONS}",
            actions.len()
        ));
    }
    let mut seen = std::collections::HashSet::with_capacity(actions.len());
    for a in actions {
        let name = a.name.trim();
        if name.is_empty() || name != a.name {
            return Some("eylemin adı boş ya da başında veya sonunda boşluk var".to_owned());
        }
        if a.name.chars().count() > MAX_ACTION_NAME || has_control(&a.name, &[]) {
            return Some(format!(
                "“{name}” eyleminin adı {MAX_ACTION_NAME} karakterden uzun ya da denetim karakteri içeriyor"
            ));
        }
        if !seen.insert(name.to_lowercase()) {
            return Some(format!("“{name}” adlı iki eylem var"));
        }
        if a.target.trim().is_empty() {
            return Some(format!("“{name}” eyleminin hedefi boş"));
        }
        if a.target.chars().count() > MAX_ACTION_TARGET || has_control(&a.target, &[]) {
            return Some(format!(
                "“{name}” eyleminin hedefi {MAX_ACTION_TARGET} karakterden uzun ya da denetim karakteri içeriyor"
            ));
        }
    }
    None
}

impl InfoCard {
    /// What is wrong with it, when anything is.
    pub fn problem(&self) -> Option<String> {
        if let Some(title) = &self.title
            && (title.trim().is_empty()
                || title.chars().count() > MAX_CARD_TITLE
                || has_control(title, &[]))
        {
            return Some(format!(
                "kartın başlığı boş, {MAX_CARD_TITLE} karakterden uzun ya da satır sonu içeriyor"
            ));
        }
        if let Some(text) = &self.text
            && (text.trim().is_empty()
                || text.chars().count() > MAX_CARD_TEXT
                || has_control(text, &['\n']))
        {
            return Some(format!(
                "kartın metni boş, {MAX_CARD_TEXT} karakterden uzun ya da denetim karakteri içeriyor"
            ));
        }
        if self.fields.len() > MAX_CARD_FIELDS {
            return Some(format!(
                "kart {} alan gösteriyor; en çok {MAX_CARD_FIELDS}",
                self.fields.len()
            ));
        }
        let mut seen = std::collections::HashSet::with_capacity(self.fields.len());
        for f in &self.fields {
            if f.trim().is_empty() {
                return Some("kartın alanlarında boş ad var".to_owned());
            }
            if !seen.insert(f.as_str()) {
                return Some(format!("kart “{f}” alanını iki kez gösteriyor"));
            }
        }
        None
    }
}

/// What is wrong with the layer tree's actions and cards, when anything is
/// (docs/adr/0225 §4, §5): on a group or on a layer drawn from a service
/// (it holds no objects), or broken.
pub fn infos_problem(tree: &[LayerNode]) -> Option<String> {
    for n in tree {
        let has = !n.actions.is_empty() || n.card.is_some();
        if has && (n.kind != LayerNodeType::Layer || n.service.is_some()) {
            return Some(format!(
                "“{}”: eylemler ve bilgi kartı yalnız nesneleri olan katmanda olur",
                n.name
            ));
        }
        if let Some(problem) = actions_problem(&n.actions) {
            return Some(format!("“{}”: {problem}", n.name));
        }
        if let Some(problem) = n.card.as_ref().and_then(InfoCard::problem) {
            return Some(format!("“{}”: {problem}", n.name));
        }
        if let Some(problem) = infos_problem(&n.children) {
            return Some(problem);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(name: &str, target: &str) -> LayerAction {
        LayerAction {
            name: name.into(),
            target: target.into(),
        }
    }

    #[test]
    fn actions_are_named_once_with_a_target() {
        assert_eq!(
            actions_problem(&[action(
                "Haritada aç",
                "https://maps.example/?q={$enlem},{$boylam}"
            )]),
            None
        );
        assert!(actions_problem(&[action("", "https://a.org")]).is_some());
        assert!(actions_problem(&[action(" A", "https://a.org")]).is_some());
        assert!(actions_problem(&[action("A", " ")]).is_some());
        assert!(actions_problem(&[action("A", "x"), action("a", "y")]).is_some());
    }

    #[test]
    fn a_card_keeps_its_rules_and_writes_its_defaults_away() {
        let card = InfoCard {
            title: Some("Parsel {Ada}/{Parsel}".into()),
            fields: vec!["Malik".into(), "Nitelik".into()],
            text: Some("Alan: {yuvarla($alan, 2)} m²\nTapu: {[Tapu alanı]}".into()),
            ..InfoCard::default()
        };
        assert_eq!(card.problem(), None);
        let json = serde_json::to_string(&card).expect("JSON");
        assert!(
            !json.contains("attachments") && !json.contains("actions"),
            "{json}"
        );
        let back: InfoCard = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, card);
        let hidden: InfoCard = serde_json::from_str(r#"{"attachments":false}"#).expect("reads");
        assert!(!hidden.attachments && hidden.actions);
        assert!(
            InfoCard {
                title: Some("a\nb".into()),
                ..InfoCard::default()
            }
            .problem()
            .is_some()
        );
        assert!(
            InfoCard {
                fields: vec!["A".into(), "A".into()],
                ..InfoCard::default()
            }
            .problem()
            .is_some()
        );
    }
}
