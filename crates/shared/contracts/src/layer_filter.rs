//! Katman süzgeci in the layer tree (docs/adr/0211 §2): a layer shows, picks
//! and gives its tools only the objects that pass its filter: a condition in
//! İfadeyle seç's language and/or a list of objects' persistent ids
//! (Seçimden süzgeç). The rules here are the readers', the commands' and the
//! server's; whether the condition compiles is the commands' (a file's
//! reader does not know the language).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::identity::EntityId;
use crate::layer::{LayerNode, LayerNodeType};

/// The longest condition, in characters.
pub const FILTER_EXPRESSION_MAX: usize = 10_000;
/// The most objects a list names.
pub const FILTER_OBJECTS_MAX: usize = 100_000;

/// A layer's filter (docs/adr/0211 §2): an object passes when the condition
/// holds for it and, when there is a list, the list names it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerFilter {
    /// The condition (İfadeyle seç's language, docs/adr/0100): an object
    /// passes when it is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expression: Option<String>,
    /// Only these objects, by their persistent ids (Seçimden süzgeç); an id
    /// the drawing does not hold is not wrong.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<EntityId>>", optional))]
    pub objects: Vec<EntityId>,
}

impl LayerFilter {
    /// What is wrong with it, when anything is.
    pub fn problem(&self) -> Option<String> {
        if self.expression.is_none() && self.objects.is_empty() {
            return Some("süzgeçte ne ifade ne nesne listesi var".to_owned());
        }
        if let Some(e) = &self.expression {
            if e.trim() != e || e.is_empty() {
                return Some(
                    "süzgecin ifadesi boş ya da başında veya sonunda boşluk var".to_owned(),
                );
            }
            if e.chars().count() > FILTER_EXPRESSION_MAX {
                return Some(format!(
                    "süzgecin ifadesi {FILTER_EXPRESSION_MAX} karakterden uzun"
                ));
            }
        }
        if self.objects.len() > FILTER_OBJECTS_MAX {
            return Some(format!(
                "süzgecin listesinde {} nesne var; en çok {FILTER_OBJECTS_MAX}",
                self.objects.len()
            ));
        }
        if self.objects.iter().any(EntityId::is_nil) {
            return Some("süzgecin listesinde boş (sıfır) kimlik var".to_owned());
        }
        let mut seen = std::collections::HashSet::with_capacity(self.objects.len());
        if let Some(twice) = self.objects.iter().find(|id| !seen.insert(**id)) {
            return Some(format!("süzgecin listesinde {twice} iki kez var"));
        }
        None
    }
}

/// What is wrong with the layer tree's filters, when anything is
/// (docs/adr/0211 §2): a filter on a group or on a layer drawn from a
/// service, or a broken filter.
pub fn filters_problem(tree: &[LayerNode]) -> Option<String> {
    for n in tree {
        if let Some(filter) = &n.filter {
            if n.kind == LayerNodeType::Group {
                return Some(format!(
                    "“{}” bir grup; grubun süzgeci olmaz, süzgeç katmanındır",
                    n.name
                ));
            }
            if n.service.is_some() {
                return Some(format!(
                    "“{}” servisten çizilir; nesnesi olmayan katmanın süzgeci olmaz",
                    n.name
                ));
            }
            if let Some(p) = filter.problem() {
                return Some(format!("“{}” katmanının süzgeci: {p}", n.name));
            }
        }
        if let Some(p) = filters_problem(&n.children) {
            return Some(p);
        }
    }
    None
}
