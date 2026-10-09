//! Ağ analizi (docs/adr/0209 §2): a project's networks as the project keeps
//! them (`ProjectSettings::networks`): which line layers are its edges and
//! which point layers its junctions, how they connect, which way each edge
//! goes and what travelling it costs. A network is the analysis graph the
//! drawing's objects make each time it is asked (the shared core's
//! `ops::network`), never a copy of them; a road's design alignment is
//! another kind of data (docs/adr/0209 Bağlam). Here are the kinds, the
//! limits and the rules a file, a command and the server check alike.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The tolerance a new network takes (metres).
pub const NETWORK_TOLERANCE: f64 = 0.01;

/// The least and the greatest tolerance a network may name (metres).
pub const NETWORK_TOLERANCES: (f64, f64) = (0.0001, 10.0);

/// The most networks a project holds.
pub const NETWORK_LIMIT: usize = 32;

/// The most edge layers, and the most junction layers, a network names.
pub const NETWORK_LAYER_LIMIT: usize = 16;

/// The most costs a network names besides its length.
pub const NETWORK_COST_LIMIT: usize = 8;

/// The most values each of a direction field's lists holds.
pub const NETWORK_VALUE_LIMIT: usize = 16;

/// The cost every network has, its edges' own lengths in metres; no other cost takes its name.
pub const LENGTH_COST: &str = "Uzunluk";

/// The greatest speed a speed cost's default may be (km/h).
pub const NETWORK_SPEED_LIMIT: f64 = 1000.0;

/// The longest id, name, value, field name, unit and expression (characters).
const ID_LENGTH: usize = 40;
const NAME_LENGTH: usize = 80;
const VALUE_LENGTH: usize = 40;
const FIELD_LENGTH: usize = 64;
const UNIT_LENGTH: usize = 12;
const EXPRESSION_LENGTH: usize = 1000;

/// A network (docs/adr/0209 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkDef {
    /// Lower-case letters, digits and hyphens; one of its kind in the project.
    pub id: String,
    /// Trimmed; one of its kind in the project.
    pub name: String,
    pub kind: NetworkKind,
    /// The line layers whose lines, paths and arcs are the edges, in this order.
    pub edges: Vec<NetworkLayer>,
    /// The point layers whose points are junctions, sources or valves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<JunctionLayer>>", optional))]
    pub junctions: Vec<JunctionLayer>,
    pub connect: NetworkConnect,
    /// Metres, within [`NETWORK_TOLERANCES`].
    pub tolerance: f64,
    pub direction: NetworkDirection,
    /// The costs besides the length, in the order the tools offer them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<NetworkCost>>", optional))]
    pub costs: Vec<NetworkCost>,
    /// An expression: the edges it holds for are out of service, travelled by nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub closed: Option<String>,
}

/// Yol ağı or Şebeke: what the tools offer first; the graph is the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NetworkKind {
    Road,
    Utility,
}

/// An edge layer and, optionally, which of its objects are edges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkLayer {
    /// The layer's id. A layer the project no longer has is said when the network is built.
    pub layer: String,
    /// An expression: only the objects it holds for; absent: all of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<String>,
}

/// A junction layer: its points' role and, optionally, which of them it takes and which are closed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionLayer {
    pub layer: String,
    pub role: JunctionRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<String>,
    /// An expression: the junctions it holds for (a closed valve) are passed by no analysis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub closed: Option<String>,
}

/// What a junction is (docs/adr/0209 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JunctionRole {
    /// Bağlantı: splits the edges it lies on.
    Junction,
    /// Kaynak: where the supply comes from (Yalıtım's beslemesiz kalan).
    Source,
    /// Vana: where Yalıtım stops.
    Valve,
}

/// Where edges connect (docs/adr/0209 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NetworkConnect {
    /// Uçlarda: at their ends, and an end on another edge splits it there.
    Ends,
    /// Köşelerde: at every vertex too.
    Vertices,
}

/// Which way the edges go (docs/adr/0209 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NetworkDirection {
    /// Every edge both ways.
    Both,
    /// Every edge the way it is drawn (pipes drawn with the flow).
    Digitized,
    /// By an attribute's value: `forward` the way the edge is drawn only, `backward` against it only, `closed`
    /// neither; any other value, and none, both ways.
    Field {
        field: String,
        #[serde(default)]
        forward: Vec<String>,
        #[serde(default)]
        backward: Vec<String>,
        #[serde(default)]
        closed: Vec<String>,
    },
}

/// A cost besides the length (docs/adr/0209 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkCost {
    pub name: String,
    pub kind: NetworkCostKind,
    /// The attribute it reads: a speed in km/h, or the whole edge's cost.
    pub field: String,
    /// A field cost's unit (“TL”, “dk”); a speed cost's is minutes and names none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub unit: String,
    /// A speed cost's speed (km/h) where the field gives none it can read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub speed: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NetworkCostKind {
    /// Süre, minutes: the length over the speed.
    Speed,
    /// The field's value for the whole edge, shared by its pieces by their lengths.
    Field,
}

/// A text as the rules compare it: trimmed, upper and lower case the same in Turkish (I is ı's, İ is i's).
pub fn network_caseless(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match c {
            'I' => 'ı',
            'İ' => 'i',
            _ => {
                let mut low = c.to_lowercase();
                match (low.next(), low.next()) {
                    (Some(l), None) => l,
                    _ => c,
                }
            }
        })
        .collect()
}

/// Whether `id` is a network id: 1–40 of `a`–`z`, `0`–`9` and `-`.
pub fn network_id_holds(id: &str) -> bool {
    (1..=ID_LENGTH).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Whether `t` is a tolerance a network may name.
pub fn network_tolerance_holds(t: f64) -> bool {
    t.is_finite() && (NETWORK_TOLERANCES.0..=NETWORK_TOLERANCES.1).contains(&t)
}

fn chars(s: &str) -> usize {
    s.chars().count()
}

/// What is wrong with an expression a network names (`what`: its place), if anything.
fn expression_problem(e: &Option<String>, what: &str) -> Option<String> {
    let e = e.as_ref()?;
    if e.trim().is_empty() {
        return Some(format!("{what} boş; ifadesi olmayan alan yazılmaz"));
    }
    (chars(e) > EXPRESSION_LENGTH).then(|| format!("{what} {EXPRESSION_LENGTH} karakterden uzun"))
}

impl NetworkDef {
    /// What is wrong with the network, in the words a file, a command and the server say it; none when it holds.
    pub fn problem(&self) -> Option<String> {
        if !network_id_holds(&self.id) {
            return Some(format!(
                "ağın kimliği “{}”; 1–{ID_LENGTH} küçük harf, rakam ya da tire olmalı",
                self.id
            ));
        }
        let name = self.name.trim();
        if name.is_empty() || name != self.name || chars(name) > NAME_LENGTH {
            return Some(format!(
                "“{}” ağının adı boş, kırpılmamış ya da {NAME_LENGTH} karakterden uzun",
                self.id
            ));
        }
        let at = |what: &str| format!("“{name}” ağının {what}");
        if self.edges.is_empty() {
            return Some(at("kenar katmanı yok; en az bir çizgi katmanı olmalı"));
        }
        if self.edges.len() > NETWORK_LAYER_LIMIT || self.junctions.len() > NETWORK_LAYER_LIMIT {
            return Some(at(&format!(
                "katmanları çok; en çok {NETWORK_LAYER_LIMIT} kenar ve {NETWORK_LAYER_LIMIT} düğüm katmanı"
            )));
        }
        for (i, e) in self.edges.iter().enumerate() {
            if e.layer.is_empty() {
                return Some(at("kenar katmanının kimliği boş"));
            }
            if let Some(p) = expression_problem(&e.filter, &at("kenar süzgeci")) {
                return Some(p);
            }
            if self.edges[..i]
                .iter()
                .any(|o| o.layer == e.layer && o.filter == e.filter)
            {
                return Some(at("aynı kenar katmanı aynı süzgeçle iki kez var"));
            }
        }
        for (i, j) in self.junctions.iter().enumerate() {
            if j.layer.is_empty() {
                return Some(at("düğüm katmanının kimliği boş"));
            }
            if let Some(p) = expression_problem(&j.filter, &at("düğüm süzgeci"))
                .or_else(|| expression_problem(&j.closed, &at("düğümlerin kapalı ifadesi")))
            {
                return Some(p);
            }
            if self.junctions[..i]
                .iter()
                .any(|o| o.layer == j.layer && o.filter == j.filter)
            {
                return Some(at("aynı düğüm katmanı aynı süzgeçle iki kez var"));
            }
        }
        if !network_tolerance_holds(self.tolerance) {
            return Some(at(&format!(
                "toleransı {} m; 0,0001 ile 10 m arasında olmalı",
                self.tolerance
            )));
        }
        if let NetworkDirection::Field {
            field,
            forward,
            backward,
            closed,
        } = &self.direction
        {
            if field.trim().is_empty() || chars(field) > FIELD_LENGTH {
                return Some(at("yön alanı boş ya da çok uzun"));
            }
            if forward.is_empty() && backward.is_empty() && closed.is_empty() {
                return Some(at(
                    "yön alanının değeri yok; ileri, geri ya da kapalı en az bir değer olmalı",
                ));
            }
            let lists = [forward, backward, closed];
            let mut seen: Vec<String> = Vec::new();
            for list in lists {
                if list.len() > NETWORK_VALUE_LIMIT {
                    return Some(at(&format!(
                        "yön değerleri çok; listede en çok {NETWORK_VALUE_LIMIT} değer"
                    )));
                }
                for v in list {
                    if v.trim().is_empty() || chars(v) > VALUE_LENGTH {
                        return Some(at(&format!(
                            "yön değeri boş ya da {VALUE_LENGTH} karakterden uzun"
                        )));
                    }
                    let key = network_caseless(v);
                    if seen.contains(&key) {
                        return Some(at(&format!("yön değeri “{}” iki kez var", v.trim())));
                    }
                    seen.push(key);
                }
            }
        }
        if self.costs.len() > NETWORK_COST_LIMIT {
            return Some(at(&format!(
                "maliyetleri çok; en çok {NETWORK_COST_LIMIT} ek maliyet"
            )));
        }
        for (i, c) in self.costs.iter().enumerate() {
            let cost = c.name.trim();
            if cost.is_empty() || cost != c.name || chars(cost) > ID_LENGTH {
                return Some(at(&format!(
                    "maliyetinin adı boş, kırpılmamış ya da {ID_LENGTH} karakterden uzun"
                )));
            }
            let key = network_caseless(cost);
            if key == network_caseless(LENGTH_COST) {
                return Some(at(&format!("maliyeti “{cost}”: bu ad ağın uzunluğunundur")));
            }
            if self.costs[..i]
                .iter()
                .any(|o| network_caseless(&o.name) == key)
            {
                return Some(at(&format!("maliyeti “{cost}” iki kez var")));
            }
            if c.field.trim().is_empty() || chars(&c.field) > FIELD_LENGTH {
                return Some(at(&format!(
                    "“{cost}” maliyetinin alanı boş ya da çok uzun"
                )));
            }
            match c.kind {
                NetworkCostKind::Speed => {
                    if !c.unit.is_empty() {
                        return Some(at(&format!(
                            "“{cost}” maliyeti süredir (dakika); birim yazılmaz"
                        )));
                    }
                    if !c
                        .speed
                        .is_some_and(|s| s.is_finite() && s > 0.0 && s <= NETWORK_SPEED_LIMIT)
                    {
                        return Some(at(&format!(
                            "“{cost}” maliyetinin varsayılan hızı sıfırdan büyük, en çok {NETWORK_SPEED_LIMIT} km/sa olmalı"
                        )));
                    }
                }
                NetworkCostKind::Field => {
                    if c.speed.is_some() {
                        return Some(at(&format!("“{cost}” maliyeti alandan okunur; hız almaz")));
                    }
                    if chars(&c.unit) > UNIT_LENGTH || c.unit.trim() != c.unit {
                        return Some(at(&format!(
                            "“{cost}” maliyetinin birimi kırpılmamış ya da {UNIT_LENGTH} karakterden uzun"
                        )));
                    }
                }
            }
        }
        expression_problem(&self.closed, &at("kapalı kenarlar ifadesi"))
    }

    /// The names of its costs in the tools' order: the length first.
    pub fn cost_names(&self) -> Vec<&str> {
        std::iter::once(LENGTH_COST)
            .chain(self.costs.iter().map(|c| c.name.as_str()))
            .collect()
    }

    /// The layers it reads (edges, then junctions), each once in its first place.
    pub fn layers(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for l in self
            .edges
            .iter()
            .map(|e| e.layer.as_str())
            .chain(self.junctions.iter().map(|j| j.layer.as_str()))
        {
            if !out.contains(&l) {
                out.push(l);
            }
        }
        out
    }
}

/// What is wrong with a project's networks: too many, an id or a name twice, or one that does not hold.
pub fn networks_problem(networks: &[NetworkDef]) -> Option<String> {
    if networks.len() > NETWORK_LIMIT {
        return Some(format!(
            "projede {} ağ var; en çok {NETWORK_LIMIT}",
            networks.len()
        ));
    }
    for (i, n) in networks.iter().enumerate() {
        if let Some(p) = n.problem() {
            return Some(p);
        }
        if networks[..i].iter().any(|o| o.id == n.id) {
            return Some(format!("“{}” kimlikli ağ iki kez var", n.id));
        }
        let key = network_caseless(&n.name);
        if networks[..i]
            .iter()
            .any(|o| network_caseless(&o.name) == key)
        {
            return Some(format!("“{}” adlı ağ iki kez var", n.name));
        }
    }
    None
}

/// The networks as a project keeps them: those that hold, the first of an id or a name, at most [`NETWORK_LIMIT`].
pub fn sanitized_networks(networks: Vec<NetworkDef>) -> Vec<NetworkDef> {
    let mut kept: Vec<NetworkDef> = Vec::with_capacity(networks.len().min(NETWORK_LIMIT));
    for n in networks {
        if kept.len() == NETWORK_LIMIT {
            break;
        }
        let key = network_caseless(&n.name);
        if n.problem().is_none()
            && !kept
                .iter()
                .any(|o| o.id == n.id || network_caseless(&o.name) == key)
        {
            kept.push(n);
        }
    }
    kept
}

/// The id a new network takes: the first free `ag-N`.
pub fn next_network_id(networks: &[NetworkDef]) -> String {
    (1..)
        .map(|n| format!("ag-{n}"))
        .find(|id| !networks.iter().any(|o| o.id == *id))
        .unwrap_or_else(|| "ag".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn road() -> NetworkDef {
        NetworkDef {
            id: "ag-1".into(),
            name: "Yollar".into(),
            kind: NetworkKind::Road,
            edges: vec![NetworkLayer {
                layer: "yol".into(),
                filter: None,
            }],
            junctions: Vec::new(),
            connect: NetworkConnect::Ends,
            tolerance: NETWORK_TOLERANCE,
            direction: NetworkDirection::Field {
                field: "yon".into(),
                forward: vec!["FT".into()],
                backward: vec!["TF".into()],
                closed: vec!["N".into()],
            },
            costs: vec![NetworkCost {
                name: "Süre".into(),
                kind: NetworkCostKind::Speed,
                field: "hiz".into(),
                unit: String::new(),
                speed: Some(50.0),
            }],
            closed: None,
        }
    }

    #[test]
    fn a_road_network_holds_and_its_costs_start_with_the_length() {
        let n = road();
        assert_eq!(n.problem(), None);
        assert_eq!(n.cost_names(), vec!["Uzunluk", "Süre"]);
        assert_eq!(networks_problem(std::slice::from_ref(&n)), None);
        assert_eq!(next_network_id(&[n]), "ag-2");
    }

    #[test]
    fn the_rules_say_what_is_wrong() {
        let mut n = road();
        n.id = "Ag 1".into();
        assert!(n.problem().unwrap().contains("kimliği"));
        let mut n = road();
        n.costs[0].name = "uzunluk".into();
        assert!(n.problem().unwrap().contains("uzunluğunundur"));
        let mut n = road();
        if let NetworkDirection::Field { closed, .. } = &mut n.direction {
            closed.push("ft".into());
        }
        assert!(n.problem().unwrap().contains("iki kez"));
        let mut n = road();
        n.tolerance = 20.0;
        assert!(n.problem().unwrap().contains("toleransı"));
        let mut two = road();
        two.id = "ag-2".into();
        two.name = "YOLLAR".into();
        assert!(
            networks_problem(&[road(), two.clone()])
                .unwrap()
                .contains("iki kez")
        );
        assert_eq!(sanitized_networks(vec![road(), two]).len(), 1);
    }

    #[test]
    fn turkish_i_folds_as_turkish() {
        assert_eq!(network_caseless(" IŞIK "), "ışık");
        assert_eq!(network_caseless("İleri"), "ileri");
    }
}
