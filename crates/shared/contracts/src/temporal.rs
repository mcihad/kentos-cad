//! Zamansal katmanlar ve senaryolar in the layer tree (docs/adr/0210 §2): a
//! layer's time setting (which attributes hold its objects' start and end, a
//! key for comparing versions, Birikimli), a group made a scenario, and a
//! scenario layer standing for a base layer (`replaces`). The rules here are
//! the readers', the commands' and the server's.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::layer::{LayerNode, LayerNodeType};

/// The longest field name a time setting names.
pub const TIME_FIELD_MAX: usize = 64;
/// The longest scenario note.
pub const SCENARIO_NOTE_MAX: usize = 500;
/// The longest scenario name (Senaryo oluştur).
pub const SCENARIO_NAME_MAX: usize = 80;

/// A layer's time setting (docs/adr/0210 §2): its objects' start (or
/// moment) and end are the values of these attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerTime {
    /// The attribute holding the start (an instant layer's moment).
    pub start: String,
    /// The attribute holding the end; absent, the objects are moments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end: Option<String>,
    /// The attribute naming an object across its versions (Zamanı karşılaştır's key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    /// Birikimli: an object shows from its start on, whatever its end; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub cumulative: bool,
}

fn field_problem(what: &str, name: &str) -> Option<String> {
    if name.trim() != name {
        return Some(format!(
            "{what} alanının adının başında ya da sonunda boşluk var"
        ));
    }
    if name.is_empty() {
        return Some(format!("{what} alanının adı boş"));
    }
    (name.chars().count() > TIME_FIELD_MAX)
        .then(|| format!("{what} alanının adı {TIME_FIELD_MAX} karakterden uzun"))
}

impl LayerTime {
    /// Whether its objects have an end (aralıklı) rather than a moment (anlık).
    pub fn ranged(&self) -> bool {
        self.end.is_some()
    }

    /// What is wrong with it, when anything is.
    pub fn problem(&self) -> Option<String> {
        field_problem("başlangıç", &self.start)
            .or_else(|| self.end.as_deref().and_then(|e| field_problem("bitiş", e)))
            .or_else(|| self.key.as_deref().and_then(|k| field_problem("kimlik", k)))
            .or_else(|| {
                (self.end.as_deref() == Some(self.start.as_str()))
                    .then(|| "başlangıç ve bitiş aynı alan olamaz".to_owned())
            })
    }
}

/// A group made a scenario (docs/adr/0210 §9): its layers are an alternative
/// kept apart from the field state (Mevcut durum).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenarioInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

impl ScenarioInfo {
    pub fn problem(&self) -> Option<String> {
        let note = self.note.as_deref()?;
        if note.trim() != note || note.is_empty() {
            return Some("senaryonun notu boş ya da başında veya sonunda boşluk var".to_owned());
        }
        (note.chars().count() > SCENARIO_NOTE_MAX)
            .then(|| format!("senaryonun notu {SCENARIO_NOTE_MAX} karakterden uzun"))
    }
}

/// What is wrong with the layer tree's time settings and scenarios, when
/// anything is (docs/adr/0210 §2): a time on a group or a service layer, a
/// broken setting; a scenario on a layer or inside another scenario; a
/// `replaces` outside a scenario, on a group, empty, naming itself, naming a
/// node that is not a base layer, or a base layer replaced twice in one
/// scenario. A `replaces` naming no node of the tree is not wrong: it is left out.
pub fn scenarios_problem(tree: &[LayerNode]) -> Option<String> {
    fn find<'a>(
        nodes: &'a [LayerNode],
        id: &str,
        in_scenario: bool,
    ) -> Option<(&'a LayerNode, bool)> {
        for n in nodes {
            if n.id == id {
                return Some((n, in_scenario));
            }
            if let Some(found) = find(&n.children, id, in_scenario || n.scenario.is_some()) {
                return Some(found);
            }
        }
        None
    }
    fn walk(
        tree: &[LayerNode],
        nodes: &[LayerNode],
        scenario: Option<&LayerNode>,
        replaced: &mut Vec<(String, String)>,
    ) -> Option<String> {
        for n in nodes {
            if let Some(time) = &n.time {
                if n.kind == LayerNodeType::Group {
                    return Some(format!(
                        "“{}” bir grup; grubun zamanı olmaz, zaman katmanındır",
                        n.name
                    ));
                }
                if n.service.is_some() {
                    return Some(format!(
                        "“{}” servisten çizilir; nesnesi olmayan katmanın zamanı olmaz",
                        n.name
                    ));
                }
                if let Some(p) = time.problem() {
                    return Some(format!("“{}” katmanının zamanı: {p}", n.name));
                }
            }
            if let Some(info) = &n.scenario {
                if n.kind == LayerNodeType::Layer {
                    return Some(format!("“{}” bir katman; yalnız grup senaryo olur", n.name));
                }
                if let Some(outer) = scenario {
                    return Some(format!(
                        "“{}” senaryosu “{}” senaryosunun içinde; senaryo iç içe olmaz",
                        n.name, outer.name
                    ));
                }
                if let Some(p) = info.problem() {
                    return Some(format!("“{}” senaryosu: {p}", n.name));
                }
            }
            if let Some(target) = &n.replaces {
                let Some(owner) = scenario else {
                    return Some(format!(
                        "“{}” bir senaryonun içinde değil; yalnız senaryo katmanı bir katmanın yerine geçer",
                        n.name
                    ));
                };
                if n.kind == LayerNodeType::Group {
                    return Some(format!(
                        "“{}” bir grup; yalnız katman bir katmanın yerine geçer",
                        n.name
                    ));
                }
                if target.is_empty() {
                    return Some(format!("“{}” katmanının yerine geçtiği katman boş", n.name));
                }
                if *target == n.id {
                    return Some(format!("“{}” katmanı kendi yerine geçemez", n.name));
                }
                if let Some((base, in_scenario)) = find(tree, target, false) {
                    if base.kind != LayerNodeType::Layer || in_scenario || base.scenario.is_some() {
                        return Some(format!(
                            "“{}” katmanı “{}” düğümünün yerine geçiyor; yalnız bir ana katmanın (senaryo dışındaki katmanın) yerine geçilir",
                            n.name, base.name
                        ));
                    }
                    if replaced.iter().any(|(s, t)| *s == owner.id && t == target) {
                        return Some(format!(
                            "“{}” senaryosunda “{}” katmanının yerine iki katman geçiyor",
                            owner.name, base.name
                        ));
                    }
                    replaced.push((owner.id.clone(), target.clone()));
                }
            }
            let inner = if n.scenario.is_some() {
                Some(n)
            } else {
                scenario
            };
            if let Some(p) = walk(tree, &n.children, inner, replaced) {
                return Some(p);
            }
        }
        None
    }
    walk(tree, tree, None, &mut Vec::new())
}

/// The base layers a scenario group stands in for (its layers' `replaces`
/// that name a base layer of `tree`), each with the scenario layer, in the tree's order.
pub fn scenario_pairs<'a>(
    tree: &'a [LayerNode],
    scenario: &'a LayerNode,
) -> Vec<(&'a str, &'a str)> {
    fn base(nodes: &[LayerNode], id: &str, inside: bool) -> bool {
        nodes.iter().any(|n| {
            (n.id == id && !inside && n.kind == LayerNodeType::Layer)
                || base(&n.children, id, inside || n.scenario.is_some())
        })
    }
    fn walk<'a>(tree: &[LayerNode], nodes: &'a [LayerNode], out: &mut Vec<(&'a str, &'a str)>) {
        for n in nodes {
            if let Some(t) = &n.replaces
                && n.kind == LayerNodeType::Layer
                && base(tree, t, false)
            {
                out.push((t.as_str(), n.id.as_str()));
            }
            walk(tree, &n.children, out);
        }
    }
    let mut out = Vec::new();
    walk(tree, &scenario.children, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: LayerNodeType, children: Vec<LayerNode>) -> LayerNode {
        LayerNode {
            id: id.to_owned(),
            name: id.to_owned(),
            kind,
            visible: true,
            locked: false,
            expanded: true,
            style: serde_json::from_value(serde_json::json!({
                "color": "fg", "lineType": "continuous", "lineWeight": 0.25
            }))
            .expect("a style"),
            children,
            snap: None,
            fields: Vec::new(),
            service: None,
            feed: None,
            time: None,
            scenario: None,
            replaces: None,
        }
    }

    #[test]
    fn a_scenario_layer_stands_for_one_base_layer() {
        let mut copy = node("yol-a", LayerNodeType::Layer, Vec::new());
        copy.replaces = Some("yol".into());
        let mut group = node("a", LayerNodeType::Group, vec![copy.clone()]);
        group.scenario = Some(ScenarioInfo::default());
        let tree = vec![group.clone(), node("yol", LayerNodeType::Layer, Vec::new())];
        assert_eq!(scenarios_problem(&tree), None);
        assert_eq!(scenario_pairs(&tree, &tree[0]), [("yol", "yol-a")]);
        let mut twice = group.clone();
        let mut again = copy.clone();
        again.id = "yol-b".into();
        twice.children.push(again);
        let bad = vec![twice, node("yol", LayerNodeType::Layer, Vec::new())];
        assert!(scenarios_problem(&bad).unwrap().contains("iki katman"));
        // A base layer gone is no fault: the scenario layer stands for nothing.
        let gone = vec![group];
        assert_eq!(scenarios_problem(&gone), None);
        assert!(scenario_pairs(&gone, &gone[0]).is_empty());
        let mut outside = node("x", LayerNodeType::Layer, Vec::new());
        outside.replaces = Some("yol".into());
        assert!(
            scenarios_problem(&[outside])
                .unwrap()
                .contains("senaryonun içinde değil")
        );
    }

    #[test]
    fn a_time_setting_names_its_fields() {
        let ok = LayerTime {
            start: "baslangic".into(),
            end: Some("bitis".into()),
            key: None,
            cumulative: false,
        };
        assert_eq!(ok.problem(), None);
        let same = LayerTime {
            end: Some("baslangic".into()),
            ..ok.clone()
        };
        assert!(same.problem().unwrap().contains("aynı alan"));
        let blank = LayerTime {
            start: " ".into(),
            ..ok
        };
        assert!(blank.problem().is_some());
    }
}
