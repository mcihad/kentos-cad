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

/// The most turn layers a network names (docs/adr/0521 §2).
pub const NETWORK_TURN_LAYER_LIMIT: usize = 4;

/// The longest turn delay (seconds).
pub const NETWORK_DELAY_LIMIT: f64 = 3600.0;

/// The most periods a speed cost's time profile holds (docs/adr/0521 §4).
pub const NETWORK_PERIOD_LIMIT: usize = 24;

/// The greatest factor a period may multiply the speeds by.
pub const NETWORK_FACTOR_LIMIT: f64 = 10.0;

/// A time profile's day, minutes.
pub const DAY_MINUTES: u32 = 1440;

/// The most devices (junction layers with terminals) a network names (docs/adr/0557 §1).
pub const NETWORK_DEVICE_LIMIT: usize = 16;

/// The most terminals a device has, and the most paths through it.
pub const NETWORK_TERMINAL_LIMIT: usize = 8;
pub const NETWORK_PATH_LIMIT: usize = 16;

/// The most configurations a device has besides its own terminals (docs/adr/0562 §2).
pub const NETWORK_CONFIG_LIMIT: usize = 16;

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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
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
    /// Dönüşler (docs/adr/0521 §2): forbidden turns, U-turns and turn delays; none: every turn free.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub turns: Option<NetworkTurns>,
    /// Seviyeler (docs/adr/0521 §3): which ends and insides of the edges connect, by their level fields; none: every
    /// one at one level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub levels: Option<NetworkLevels>,
    /// Terminaller (docs/adr/0557 §1): the ends of a device (a valve's in and out, a transformer's two sides) and which
    /// of them connect through it; a trace goes through a device only along those paths. None: a device is a point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub terminals: Option<NetworkTerminals>,
    /// Alt ağlar (docs/adr/0557 §2): the parts the sources feed, found by `cad.subnetworks.find` and written into an
    /// attribute by `cad.subnetworks.update`. None: the network has no subnetworks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub subnetworks: Option<NetworkSubnetworks>,
}

/// The terminals of a network's devices (docs/adr/0557 §1): the edge fields naming the terminal an edge's start and
/// its end are attached to, and the devices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkTerminals {
    /// The edge attribute whose value (a terminal's id or name) is the terminal its start is attached to.
    pub start: String,
    /// The same for its end.
    pub end: String,
    /// The devices: 1–16, each a junction layer of the network.
    pub devices: Vec<TerminalDevice>,
}

/// A device: every point of the junction layer `layer` has these terminals. An edge attached to one of its terminals
/// goes on through the device only to a terminal on a path with it; an edge that names none (or one the device does
/// not have) passes to every terminal and the trace is said to be unverified.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerminalDevice {
    /// A junction layer of the network (its id).
    pub layer: String,
    /// 2–8 terminals.
    pub terminals: Vec<DeviceTerminal>,
    /// 1–16 paths: each the ids of two or more terminals connected to one another through the device.
    pub paths: Vec<Vec<String>>,
    /// The attribute of the device's points that names their configuration (docs/adr/0562 §2): a point whose value is
    /// a configuration's has that configuration's terminals and paths; one with none, or an unknown value, has the
    /// device's own (an unknown value is said). None: every point has the device's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<String>,
    /// The configurations besides the device's own: 1–16 when `by` is named, none otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerminalConfig>>", optional))]
    pub configs: Vec<TerminalConfig>,
}

/// A configuration of a device (docs/adr/0562 §2): the terminals and paths of the points whose `by` attribute holds
/// `value` (caseless).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerminalConfig {
    pub value: String,
    /// 2–8 terminals.
    pub terminals: Vec<DeviceTerminal>,
    /// 1–16 paths, as the device's.
    pub paths: Vec<Vec<String>>,
}

/// A terminal: its id (lower-case letters, digits and hyphens) and its name (what the tools say).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DeviceTerminal {
    pub id: String,
    pub name: String,
}

/// Which way a subnetwork reaches from its controller (docs/adr/0557 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SubnetworkReach {
    /// Akış aşağı: the way the edges' directions lead.
    Downstream,
    /// Bağlı: along every connection, any way.
    Connected,
}

/// A network's subnetworks (docs/adr/0557 §2): its sources are the subnetwork controllers; a subnetwork is what a
/// controller reaches (not past another controller, a closed junction or a closed edge).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkSubnetworks {
    /// The controller's attribute holding its subnetwork's name; none: its label, else its `Ad`, else `Kaynak <no>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// The attribute `cad.subnetworks.update` writes on every object of the network: its subnetworks' names, joined
    /// with `; ` where it is in more than one; taken away where it is in none.
    pub attribute: String,
    pub reach: SubnetworkReach,
    /// Güncel tut (docs/adr/0562 §3): a write that touches the network's layers also rewrites the attribute, in its
    /// own step; when it cannot, the command says `subnetwork_dirty`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub keep_current: bool,
}

/// Where U-turns are made (docs/adr/0521 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NetworkUTurns {
    /// Her yerde: at every node.
    Allowed,
    /// Yalnız çıkmazlarda: where one piece ends (a dead end).
    DeadEnds,
    /// Hiçbir yerde.
    Never,
}

/// A network's turns (docs/adr/0521 §2): where a way may turn back, what a turn at a crossing of three or more
/// pieces costs by its angle (seconds, added to the speed costs), and the turn layers' lines, each a turn from the
/// edge at its start to the edge at its end through their common node: forbidden, or its own delay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkTurns {
    pub u_turns: NetworkUTurns,
    /// Going straight on (within 30° of the way one came), seconds.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub straight: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub right: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub left: f64,
    /// A U-turn, wherever it is made, seconds.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub back: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TurnLayer>>", optional))]
    pub layers: Vec<TurnLayer>,
}

fn is_zero(x: &f64) -> bool {
    *x == 0.0
}

/// A turn layer: its lines are turns; `field`'s number is a turn's delay (seconds), its empty value a forbidden turn;
/// without a field every turn of the layer is forbidden.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TurnLayer {
    pub layer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
}

/// The level fields (docs/adr/0521 §3): an edge's level at its start, at its end and, when given, inside it (else
/// its ends' when they are one, none when they differ); a junction point's from `start` (empty: every level).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkLevels {
    pub start: String,
    pub end: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub inside: Option<String>,
}

/// A period of a speed cost's day (docs/adr/0521 §4): from `from` to `to` (minutes of the day), the edges go at their
/// speed times `factor`, or at the speed `field` gives (km/h; 0 closed, empty or unreadable the edge's own speed).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkPeriod {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "day-min")))]
    pub from: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "day-min")))]
    pub to: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub speed: Option<f64>,
    /// A speed cost's time profile (docs/adr/0521 §4): its periods in the day's order; the rest of the day the
    /// edges' own speeds. Read when a question gives a time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<NetworkPeriod>>", optional))]
    pub periods: Vec<NetworkPeriod>,
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
                    if let Some(p) = periods_problem(&c.periods) {
                        return Some(at(&format!("“{cost}” maliyetinin zaman profili: {p}")));
                    }
                }
                NetworkCostKind::Field => {
                    if !c.periods.is_empty() {
                        return Some(at(&format!(
                            "“{cost}” maliyeti alandan okunur; zaman profili yalnız hızla maliyette olur"
                        )));
                    }
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
        if let Some(p) = expression_problem(&self.closed, &at("kapalı kenarlar ifadesi")) {
            return Some(p);
        }
        if let Some(t) = &self.turns {
            for (what, d) in [
                ("düz geçiş", t.straight),
                ("sağa dönüş", t.right),
                ("sola dönüş", t.left),
                ("geri dönüş", t.back),
            ] {
                if !(d.is_finite() && (0.0..=NETWORK_DELAY_LIMIT).contains(&d)) {
                    return Some(at(&format!(
                        "{what} gecikmesi {d} sn; 0 ile {NETWORK_DELAY_LIMIT} sn arasında olmalı"
                    )));
                }
            }
            if t.layers.len() > NETWORK_TURN_LAYER_LIMIT {
                return Some(at(&format!(
                    "dönüş katmanı çok; en çok {NETWORK_TURN_LAYER_LIMIT}"
                )));
            }
            for (i, l) in t.layers.iter().enumerate() {
                if l.layer.is_empty() {
                    return Some(at("dönüş katmanının kimliği boş"));
                }
                if let Some(p) = expression_problem(&l.filter, &at("dönüş süzgeci")) {
                    return Some(p);
                }
                if l.field.as_ref().is_some_and(|f| field_bad(f)) {
                    return Some(at(
                        "dönüş katmanının gecikme alanı boş, kırpılmamış ya da çok uzun",
                    ));
                }
                if t.layers[..i]
                    .iter()
                    .any(|o| o.layer == l.layer && o.filter == l.filter)
                {
                    return Some(at("aynı dönüş katmanı aynı süzgeçle iki kez var"));
                }
            }
        }
        if let Some(l) = &self.levels
            && (field_bad(&l.start)
                || field_bad(&l.end)
                || l.inside.as_ref().is_some_and(|f| field_bad(f)))
        {
            return Some(at("seviye alanı boş, kırpılmamış ya da çok uzun"));
        }
        if let Some(t) = &self.terminals
            && let Some(p) = self.terminals_problem(t)
        {
            return Some(at(&p));
        }
        if let Some(s) = &self.subnetworks
            && let Some(p) = self.subnetworks_problem(s)
        {
            return Some(at(&p));
        }
        None
    }

    /// What is wrong with the terminals (docs/adr/0557 §1), in words that continue “ağının …”.
    fn terminals_problem(&self, t: &NetworkTerminals) -> Option<String> {
        if field_bad(&t.start) || field_bad(&t.end) {
            return Some("terminal alanı boş, kırpılmamış ya da çok uzun".into());
        }
        if t.start == t.end {
            return Some(
                "başlangıç ve bitiş terminal alanları aynı; her ucun kendi alanı olmalı".into(),
            );
        }
        if t.devices.is_empty() || t.devices.len() > NETWORK_DEVICE_LIMIT {
            return Some(format!(
                "terminalli cihazı yok ya da çok; 1–{NETWORK_DEVICE_LIMIT} cihaz olmalı"
            ));
        }
        for (i, d) in t.devices.iter().enumerate() {
            let n = i + 1;
            if !self.junctions.iter().any(|j| j.layer == d.layer) {
                return Some(format!(
                    "{n}. cihazın katmanı ağın düğüm katmanlarından biri değil"
                ));
            }
            if t.devices[..i].iter().any(|o| o.layer == d.layer) {
                return Some(format!("{n}. cihazın katmanı iki kez var"));
            }
            if let Some(p) = terminal_set_problem(&format!("{n}. cihazın"), &d.terminals, &d.paths)
            {
                return Some(p);
            }
            if let Some(p) = self.configs_problem(n, d) {
                return Some(p);
            }
        }
        None
    }

    /// What is wrong with a device's configurations (docs/adr/0562 §2), in words that continue “ağının …”.
    fn configs_problem(&self, n: usize, d: &TerminalDevice) -> Option<String> {
        match (&d.by, d.configs.is_empty()) {
            (None, true) => return None,
            (None, false) => {
                return Some(format!(
                    "{n}. cihazın yapılandırmaları var ama hangi öznitelikle seçildiği yok"
                ));
            }
            (Some(_), true) => {
                return Some(format!(
                    "{n}. cihazın yapılandırma özniteliği var ama yapılandırması yok"
                ));
            }
            (Some(by), false) => {
                if field_bad(by) {
                    return Some(format!(
                        "{n}. cihazın yapılandırma özniteliği boş, kırpılmamış ya da çok uzun"
                    ));
                }
            }
        }
        if d.configs.len() > NETWORK_CONFIG_LIMIT {
            return Some(format!(
                "{n}. cihazın yapılandırması çok; en çok {NETWORK_CONFIG_LIMIT} olmalı"
            ));
        }
        let mut seen: Vec<String> = Vec::new();
        for c in &d.configs {
            let value = c.value.trim();
            if value.is_empty() || value != c.value || chars(value) > VALUE_LENGTH {
                return Some(format!(
                    "{n}. cihazın yapılandırma değeri boş, kırpılmamış ya da {VALUE_LENGTH} karakterden uzun"
                ));
            }
            let key = network_caseless(value);
            if seen.contains(&key) {
                return Some(format!("{n}. cihazın “{value}” yapılandırması iki kez var"));
            }
            seen.push(key);
            if let Some(p) = terminal_set_problem(
                &format!("{n}. cihazın “{value}” yapılandırmasının"),
                &c.terminals,
                &c.paths,
            ) {
                return Some(p);
            }
        }
        None
    }

    /// What is wrong with the subnetworks (docs/adr/0557 §2), in words that continue “ağının …”.
    fn subnetworks_problem(&self, s: &NetworkSubnetworks) -> Option<String> {
        if !self.junctions.iter().any(|j| j.role == JunctionRole::Source) {
            return Some(
                "alt ağlarının denetleyicisi yok; en az bir Kaynak düğüm katmanı olmalı".into(),
            );
        }
        if field_bad(&s.attribute) {
            return Some("alt ağ özniteliği boş, kırpılmamış ya da çok uzun".into());
        }
        if s.name.as_ref().is_some_and(|n| field_bad(n)) {
            return Some("alt ağ adı alanı boş, kırpılmamış ya da çok uzun".into());
        }
        let reads: Vec<&str> = s
            .name
            .iter()
            .map(String::as_str)
            .chain(self.terminals.iter().flat_map(|t| [t.start.as_str(), t.end.as_str()]))
            .chain(
                self.terminals
                    .iter()
                    .flat_map(|t| t.devices.iter().filter_map(|d| d.by.as_deref())),
            )
            .chain(match &self.direction {
                NetworkDirection::Field { field, .. } => Some(field.as_str()),
                _ => None,
            })
            .collect();
        reads
            .contains(&s.attribute.as_str())
            .then(|| "alt ağ özniteliği ağın okuduğu bir alanın üstüne yazar; başka bir ad seçin".into())
    }

    /// Whether a question about the network may give a time: one of its speed costs has a time profile.
    pub fn timed(&self) -> bool {
        self.costs.iter().any(|c| !c.periods.is_empty())
    }

    /// The names of its costs in the tools' order: the length first.
    pub fn cost_names(&self) -> Vec<&str> {
        std::iter::once(LENGTH_COST)
            .chain(self.costs.iter().map(|c| c.name.as_str()))
            .collect()
    }

    /// The layers it reads (edges, then junctions, then turns), each once in its first place.
    pub fn layers(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for l in self
            .edges
            .iter()
            .map(|e| e.layer.as_str())
            .chain(self.junctions.iter().map(|j| j.layer.as_str()))
            .chain(
                self.turns
                    .iter()
                    .flat_map(|t| t.layers.iter().map(|l| l.layer.as_str())),
            )
        {
            if !out.contains(&l) {
                out.push(l);
            }
        }
        out
    }
}

/// What is wrong with a device's terminals and paths (docs/adr/0557 §1), `who` being the words they belong to
/// (“1. cihazın”, “1. cihazın “x” yapılandırmasının”).
fn terminal_set_problem(
    who: &str,
    terminals: &[DeviceTerminal],
    paths: &[Vec<String>],
) -> Option<String> {
    if terminals.len() < 2 || terminals.len() > NETWORK_TERMINAL_LIMIT {
        return Some(format!("{who} terminalleri 2–{NETWORK_TERMINAL_LIMIT} olmalı"));
    }
    let mut keys: Vec<String> = Vec::new();
    for tm in terminals {
        if !network_id_holds(&tm.id) {
            return Some(format!(
                "{who} terminal kimliği “{}”; 1–{ID_LENGTH} küçük harf, rakam ya da tire olmalı",
                tm.id
            ));
        }
        let name = tm.name.trim();
        if name.is_empty() || name != tm.name || chars(name) > VALUE_LENGTH {
            return Some(format!(
                "{who} “{}” terminalinin adı boş, kırpılmamış ya da {VALUE_LENGTH} karakterden uzun",
                tm.id
            ));
        }
        let (id_key, name_key) = (network_caseless(&tm.id), network_caseless(name));
        let mut mine = vec![id_key.clone()];
        if name_key != id_key {
            mine.push(name_key);
        }
        for k in mine {
            if keys.contains(&k) {
                return Some(format!(
                    "{who} terminalleri karışıyor: “{k}” bir terminalin kimliği ya da adı olarak iki kez var"
                ));
            }
            keys.push(k);
        }
    }
    if paths.is_empty() || paths.len() > NETWORK_PATH_LIMIT {
        return Some(format!(
            "{who} yolu yok ya da çok; 1–{NETWORK_PATH_LIMIT} yol olmalı"
        ));
    }
    for (k, path) in paths.iter().enumerate() {
        if path.len() < 2 {
            return Some(format!("{who} {}. yolu en az iki terminal içermeli", k + 1));
        }
        for (m, id) in path.iter().enumerate() {
            if !terminals.iter().any(|tm| tm.id == *id) {
                return Some(format!(
                    "{who} {}. yolundaki “{id}” cihazın terminali değil",
                    k + 1
                ));
            }
            if path[..m].contains(id) {
                return Some(format!("{who} {}. yolunda “{id}” iki kez var", k + 1));
            }
        }
    }
    None
}

/// Whether a field name a network's turns, levels or periods name is not one: empty, untrimmed or too long.
fn field_bad(f: &str) -> bool {
    f.trim().is_empty() || f.trim() != f || chars(f) > FIELD_LENGTH
}

/// What is wrong with a speed cost's periods (docs/adr/0521 §4), if anything: each within the day, later than the one
/// before it, a factor (0–10) or a field, not both; at most [`NETWORK_PERIOD_LIMIT`]; not the whole day closed.
pub fn periods_problem(periods: &[NetworkPeriod]) -> Option<String> {
    if periods.len() > NETWORK_PERIOD_LIMIT {
        return Some(format!("en çok {NETWORK_PERIOD_LIMIT} dönem"));
    }
    let mut last = 0;
    for (i, p) in periods.iter().enumerate() {
        let n = i + 1;
        if p.from >= p.to || p.to > DAY_MINUTES {
            return Some(format!(
                "{n}. dönem {}–{}; başlangıcı bitişinden önce, ikisi de 00:00 ile 24:00 arasında olmalı",
                clock_text(p.from),
                clock_text(p.to)
            ));
        }
        if i > 0 && p.from < last {
            return Some(format!(
                "{n}. dönem bir öncekiyle örtüşüyor ya da ondan önce; dönemler günün sırasıyla, örtüşmeden yazılır"
            ));
        }
        last = p.to;
        match (&p.factor, &p.field) {
            (Some(f), None) => {
                if !(f.is_finite() && (0.0..=NETWORK_FACTOR_LIMIT).contains(f)) {
                    return Some(format!(
                        "{n}. dönemin çarpanı {f}; 0 ile {NETWORK_FACTOR_LIMIT} arasında olmalı"
                    ));
                }
            }
            (None, Some(f)) => {
                if field_bad(f) {
                    return Some(format!(
                        "{n}. dönemin hız alanı boş, kırpılmamış ya da çok uzun"
                    ));
                }
            }
            _ => {
                return Some(format!(
                    "{n}. dönemin ya çarpanı ya hız alanı olur, ikisi birden ya da hiçbiri olmaz"
                ));
            }
        }
    }
    let closed: u32 = periods
        .iter()
        .filter(|p| p.factor == Some(0.0))
        .map(|p| p.to - p.from)
        .sum();
    (closed == DAY_MINUTES).then(|| "bütün gün kapalı; en az bir dakika açık olmalı".to_owned())
}

/// A clock as typed: `7`, `07:30`, `7.30`, `24:00`; minutes of the day, none when it is not one.
pub fn clock_of(text: &str) -> Option<u32> {
    let t = text.trim();
    let (h, m) = match t.find([':', '.']) {
        Some(i) => (&t[..i], &t[i + 1..]),
        None => (t, "0"),
    };
    let digits = |s: &str| !s.is_empty() && s.len() <= 2 && s.chars().all(|c| c.is_ascii_digit());
    if !digits(h) || !digits(m) {
        return None;
    }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (m < 60 && h * 60 + m <= DAY_MINUTES).then_some(h * 60 + m)
}

/// Minutes of the day as a clock says them: `07:30`, `24:00`.
pub fn clock_text(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
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
                periods: Vec::new(),
            }],
            closed: None,
            turns: None,
            levels: None,
            terminals: None,
            subnetworks: None,
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
    fn turns_levels_and_periods_say_what_is_wrong() {
        let mut n = road();
        n.turns = Some(NetworkTurns {
            u_turns: NetworkUTurns::DeadEnds,
            straight: 0.0,
            right: 5.0,
            left: 15.0,
            back: 30.0,
            layers: vec![TurnLayer {
                layer: "donus".into(),
                filter: None,
                field: Some("gecikme".into()),
            }],
        });
        n.levels = Some(NetworkLevels {
            start: "z_bas".into(),
            end: "z_son".into(),
            inside: None,
        });
        let rush = |from: u32, to: u32, factor: f64| NetworkPeriod {
            from,
            to,
            factor: Some(factor),
            field: None,
        };
        n.costs[0].periods = vec![rush(420, 570, 0.5), rush(1020, 1140, 0.6)];
        assert_eq!(n.problem(), None);
        assert!(n.timed());
        assert_eq!(n.layers(), vec!["yol", "donus"]);
        let mut bad = n.clone();
        bad.costs[0].periods.swap(0, 1);
        assert!(bad.problem().unwrap().contains("örtüşüyor"));
        let mut bad = n.clone();
        bad.costs[0].periods = vec![rush(0, 1440, 0.0)];
        assert!(bad.problem().unwrap().contains("bütün gün kapalı"));
        let mut bad = n.clone();
        bad.costs[0].periods[0].field = Some("hiz_sabah".into());
        assert!(bad.problem().unwrap().contains("ikisi birden"));
        let mut bad = n.clone();
        if let Some(t) = &mut bad.turns {
            t.left = -1.0;
        }
        assert!(bad.problem().unwrap().contains("sola dönüş"));
        let mut bad = n.clone();
        bad.levels.as_mut().unwrap().end = " z_son".into();
        assert!(bad.problem().unwrap().contains("seviye alanı"));
        assert_eq!(clock_text(450), "07:30");
    }

    fn utility() -> NetworkDef {
        let mut n = road();
        n.kind = NetworkKind::Utility;
        n.direction = NetworkDirection::Digitized;
        n.costs.clear();
        n.junctions = vec![
            JunctionLayer {
                layer: "trafo".into(),
                role: JunctionRole::Junction,
                filter: None,
                closed: None,
            },
            JunctionLayer {
                layer: "santral".into(),
                role: JunctionRole::Source,
                filter: None,
                closed: None,
            },
        ];
        n.terminals = Some(NetworkTerminals {
            start: "uc_bas".into(),
            end: "uc_son".into(),
            devices: vec![TerminalDevice {
                layer: "trafo".into(),
                terminals: vec![
                    DeviceTerminal {
                        id: "yg".into(),
                        name: "Yüksek gerilim".into(),
                    },
                    DeviceTerminal {
                        id: "ag".into(),
                        name: "Alçak gerilim".into(),
                    },
                ],
                paths: vec![vec!["yg".into(), "ag".into()]],
                by: None,
                configs: Vec::new(),
            }],
        });
        n.subnetworks = Some(NetworkSubnetworks {
            name: Some("Ad".into()),
            attribute: "Alt ağ".into(),
            reach: SubnetworkReach::Downstream,
            keep_current: false,
        });
        n
    }

    #[test]
    fn terminals_and_subnetworks_say_what_is_wrong() {
        let n = utility();
        assert_eq!(n.problem(), None);
        let bad = |edit: &dyn Fn(&mut NetworkDef)| {
            let mut m = n.clone();
            edit(&mut m);
            m.problem().expect("a problem")
        };
        fn dev(m: &mut NetworkDef) -> &mut TerminalDevice {
            &mut m.terminals.as_mut().unwrap().devices[0]
        }
        assert!(bad(&|m| m.terminals.as_mut().unwrap().end = "uc_bas".into()).contains("aynı"));
        assert!(bad(&|m| dev(m).layer = "yok".into()).contains("düğüm katmanlarından"));
        assert!(bad(&|m| dev(m).terminals.truncate(1)).contains("2–8"));
        assert!(bad(&|m| dev(m).terminals[1].id = "YG".into()).contains("kimliği"));
        assert!(bad(&|m| dev(m).terminals[1].name = "YÜKSEK GERİLİM".into()).contains("karışıyor"));
        assert!(bad(&|m| dev(m).terminals[1].name = "yg".into()).contains("karışıyor"));
        assert!(bad(&|m| dev(m).paths.clear()).contains("yolu yok"));
        assert!(bad(&|m| dev(m).paths[0] = vec!["yg".into()]).contains("en az iki"));
        assert!(bad(&|m| dev(m).paths[0][1] = "x".into()).contains("terminali değil"));
        assert!(bad(&|m| dev(m).paths[0][1] = "yg".into()).contains("iki kez"));
        assert!(bad(&|m| m.subnetworks.as_mut().unwrap().attribute = "uc_bas".into()).contains("üstüne"));
        assert!(bad(&|m| m.subnetworks.as_mut().unwrap().attribute = " x".into()).contains("özniteliği"));
        assert!(bad(&|m| m.junctions.retain(|j| j.role != JunctionRole::Source)).contains("Kaynak"));
        // A name the id folds to is the same terminal's own: allowed.
        let mut same = n.clone();
        same.terminals.as_mut().unwrap().devices[0].terminals[0].name = "YG".into();
        assert_eq!(same.problem(), None);
    }

    /// A device's configurations and `keepCurrent` (docs/adr/0562 §2, §3): what holds and what each rule says.
    #[test]
    fn configurations_and_keep_current_say_what_is_wrong() {
        let config = |value: &str| TerminalConfig {
            value: value.into(),
            terminals: vec![
                DeviceTerminal {
                    id: "a".into(),
                    name: "A".into(),
                },
                DeviceTerminal {
                    id: "b".into(),
                    name: "B".into(),
                },
            ],
            paths: vec![vec!["a".into(), "b".into()]],
        };
        let mut n = utility();
        {
            let d = &mut n.terminals.as_mut().unwrap().devices[0];
            d.by = Some("tip".into());
            d.configs = vec![config("trafo"), config("ışık")];
        }
        n.subnetworks.as_mut().unwrap().keep_current = true;
        assert_eq!(n.problem(), None);
        let bad = |edit: &dyn Fn(&mut TerminalDevice)| {
            let mut m = n.clone();
            edit(&mut m.terminals.as_mut().unwrap().devices[0]);
            m.problem().expect("a problem")
        };
        assert!(bad(&|d| d.by = None).contains("hangi öznitelikle"));
        assert!(bad(&|d| d.configs.clear()).contains("yapılandırması yok"));
        assert!(bad(&|d| d.by = Some(" tip".into())).contains("kırpılmamış"));
        assert!(bad(&|d| d.configs[1].value = "TRAFO".into()).contains("iki kez"));
        // Turkish I folds to ı: “IŞIK” is the second configuration's “ışık”.
        assert!(bad(&|d| d.configs[0].value = "IŞIK".into()).contains("iki kez"));
        assert!(bad(&|d| d.configs[0].value.clear()).contains("yapılandırma değeri"));
        assert!(bad(&|d| d.configs[0].terminals.truncate(1)).contains("“trafo” yapılandırmasının"));
        assert!(bad(&|d| d.configs[1].paths[0][1] = "z".into()).contains("“ışık” yapılandırmasının"));
        assert!(bad(&|d| d.configs = (0..17).map(|i| config(&format!("v{i}"))).collect()).contains("en çok 16"));
        // The attribute that chooses a configuration is read, so the subnetworks may not write it.
        let mut over = n.clone();
        over.subnetworks.as_mut().unwrap().attribute = "tip".into();
        assert!(over.problem().expect("a problem").contains("üstüne"));
    }

    #[test]
    fn turkish_i_folds_as_turkish() {
        assert_eq!(network_caseless(" IŞIK "), "ışık");
        assert_eq!(network_caseless("İleri"), "ileri");
    }
}
