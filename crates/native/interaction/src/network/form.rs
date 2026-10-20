//! Ağlar's form (docs/adr/0209 §10; the web's `model/networkForm.ts`): a
//! network as its window shows and takes it, everything as typed. The
//! tolerance in the project's length unit; the direction's values a list
//! with commas or semicolons; a speed cost's default speed (km/sa) empty for
//! 50. `def_of` turns it into the contract's network or says, in words, what
//! is wrong (the form's own words, then the contract's). New networks take
//! their kind's defaults. Both are held to fixtures/network/v1/form.json
//! (scripts/fixtures/network_form_cases.py). Turns, levels and a speed cost's
//! time profile (docs/adr/0521): the delays in seconds, a profile as
//! `07:00-09:30 0,5; 17:00-19:00 hiz_aksam` (a factor or a speed field).
//! Terminals and subnetworks (docs/adr/0557): a device's terminals as
//! `yg: Yüksek gerilim; ag: Alçak gerilim` (an id, and its name when it has
//! one), its paths as `yg, ag; yg, c` (the ids of each path), the edge fields
//! naming the terminals, and the subnetworks' name field, attribute and reach.

use kentos_commands::{
    DeviceTerminal, JunctionLayer, JunctionRole, NETWORK_TOLERANCE, NetworkConnect, NetworkCost,
    NetworkCostKind, NetworkDef, NetworkDirection, NetworkKind, NetworkLayer, NetworkLevels,
    NetworkPeriod, NetworkSubnetworks, NetworkTerminals, NetworkTurns, NetworkUTurns,
    SubnetworkReach, TerminalConfig, TerminalDevice, TurnLayer, clock_text,
};
use kentos_geometry_core::tools::point_text::js_trim;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeRow {
    pub layer: String,
    pub filter: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JunctionRow {
    pub layer: String,
    pub role: JunctionRole,
    pub filter: String,
    pub closed: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostRow {
    pub name: String,
    pub kind: NetworkCostKind,
    pub field: String,
    pub unit: String,
    pub speed: String,
    /// A speed cost's time profile as typed (docs/adr/0521 §4): `07:00-09:30 0,5; 17:00-19:00 hiz_aksam`.
    pub periods: String,
}

/// A turn layer's row: its lines are turns; the field's number a delay (seconds), none: every turn forbidden.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnRow {
    pub layer: String,
    pub filter: String,
    pub field: String,
}

/// A device's row (docs/adr/0557 §1): a junction layer, its terminals and the paths through it, as typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceRow {
    pub layer: String,
    /// `yg: Yüksek gerilim; ag: Alçak gerilim`: each terminal's id and, after a colon, its name (none: the id).
    pub terminals: String,
    /// `yg, ag; yg, c`: each path's terminal ids, split at commas, the paths at semicolons.
    pub paths: String,
    /// The attribute of its points that names their configuration (docs/adr/0562 §2); empty: none.
    pub by: String,
    /// `üç yollu = a: A; b: B | a, b || trafo = yg: Yüksek; ag: Alçak | yg, ag`: each configuration's value, after
    /// an equals sign its terminals and after a bar its paths, the configurations split at double bars.
    pub configs: String,
}

/// Yön as the form offers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionKind {
    Both,
    Digitized,
    Field,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkForm {
    pub id: String,
    pub name: String,
    pub kind: NetworkKind,
    pub edges: Vec<EdgeRow>,
    pub junctions: Vec<JunctionRow>,
    pub connect: NetworkConnect,
    pub tolerance: String,
    pub direction: DirectionKind,
    pub field: String,
    pub forward: String,
    pub backward: String,
    pub shut: String,
    pub costs: Vec<CostRow>,
    pub closed: String,
    /// Dönüşler (docs/adr/0521 §2): none, no turn rules; the delays as typed, seconds (empty: 0).
    pub u_turns: Option<NetworkUTurns>,
    pub straight: String,
    pub right: String,
    pub left: String,
    pub back: String,
    pub turn_layers: Vec<TurnRow>,
    /// Seviyeler (docs/adr/0521 §3): the level fields; the start and the end empty, none.
    pub level_start: String,
    pub level_end: String,
    pub level_inside: String,
    /// Terminaller (docs/adr/0557 §1): the edge fields naming the terminals of an edge's start and end (both empty:
    /// none) and the devices.
    pub terminal_start: String,
    pub terminal_end: String,
    pub devices: Vec<DeviceRow>,
    /// Alt ağlar (docs/adr/0557 §2): none, no subnetworks; the controller's name field (empty: its label), the
    /// attribute written.
    pub subnet_reach: Option<SubnetworkReach>,
    pub subnet_name: String,
    pub subnet_attribute: String,
    /// Güncel tut (docs/adr/0562 §3): the attribute is rewritten by every write that touches the network.
    pub subnet_keep: bool,
}

/// The attribute the subnetworks' names are written to when none is typed.
pub const DEFAULT_SUBNET_ATTRIBUTE: &str = "Alt ağ";

/// A speed cost's default speed when none is typed, km/sa.
pub const DEFAULT_SPEED: f64 = 50.0;

/// A field's values when Yön takes them from a field and none are typed yet: the field, forward, backward, closed.
pub const DIRECTION_DEFAULTS: [&str; 4] = ["yon", "FT, ileri", "TF, geri", "N, kapalı"];

/// A number as typed: a dot or a comma for the decimals (`5`, `5,25`, `.5`, `5.`); none when it is not one.
pub fn decimal(text: &str) -> Option<f64> {
    let t = js_trim(text);
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    let ok = match body.find(['.', ',']) {
        None => !body.is_empty() && digits(body),
        Some(i) => {
            let (int, frac) = (&body[..i], &body[i + 1..]);
            digits(int) && digits(frac) && !(int.is_empty() && frac.is_empty())
        }
    };
    ok.then(|| t.replace(',', ".").parse().ok()).flatten()
}

/// A number as the form shows it: twelve significant digits (no noise from a unit's conversion), as JavaScript's
/// `String(Number(v.toPrecision(12)))`.
pub fn shown(v: f64) -> String {
    let n: f64 = format!("{v:.11e}").parse().unwrap_or(v);
    kentos_project::new_project::js_number(n)
}

/// A list as typed: split at commas and semicolons, trimmed, empty ones dropped.
pub fn values_of(text: &str) -> Vec<String> {
    text.split([',', ';'])
        .map(js_trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

fn joined(list: &[String]) -> String {
    list.join(", ")
}

/// A clock as typed (the contract's rule).
pub use kentos_commands::clock_of;

/// A time profile as typed: periods split at semicolons, each `from-to` then a factor or a speed field.
pub fn periods_of(text: &str) -> Result<Vec<NetworkPeriod>, String> {
    let mut out = Vec::new();
    for (i, part) in text
        .split(';')
        .map(js_trim)
        .filter(|p| !p.is_empty())
        .enumerate()
    {
        let n = i + 1;
        let (span, rest) = match part.find(char::is_whitespace) {
            Some(at) => (&part[..at], js_trim(&part[at..])),
            None => (part, ""),
        };
        let clocks = span
            .split_once(['-', '–'])
            .and_then(|(a, b)| Some((clock_of(a)?, clock_of(b)?)));
        let Some((from, to)) = clocks else {
            return Err(format!("{n}. dönem saatleri “07:00-09:30” gibi yazılır."));
        };
        if rest.is_empty() {
            return Err(format!(
                "{n}. dönemin çarpanı ya da hız alanı yok (ör. “07:00-09:30 0,5”)."
            ));
        }
        let (factor, field) = match decimal(rest) {
            Some(f) => (Some(f), None),
            None => (None, Some(rest.to_owned())),
        };
        out.push(NetworkPeriod {
            from,
            to,
            factor,
            field,
        });
    }
    Ok(out)
}

/// A time profile as the form shows it.
pub fn periods_text(periods: &[NetworkPeriod]) -> String {
    periods
        .iter()
        .map(|p| {
            let what = match (&p.factor, &p.field) {
                (Some(f), _) => shown(*f).replace('.', ","),
                (None, Some(f)) => f.clone(),
                (None, None) => String::new(),
            };
            format!("{}-{} {what}", clock_text(p.from), clock_text(p.to))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// A device's terminals as typed: split at semicolons, each `id: name` (or just the id, which is then its name too).
pub fn terminals_of(text: &str) -> Vec<DeviceTerminal> {
    text.split(';')
        .map(js_trim)
        .filter(|t| !t.is_empty())
        .map(|t| match t.split_once(':') {
            Some((id, name)) => DeviceTerminal {
                id: js_trim(id).to_owned(),
                name: js_trim(name).to_owned(),
            },
            None => DeviceTerminal {
                id: t.to_owned(),
                name: t.to_owned(),
            },
        })
        .collect()
}

/// A device's terminals as the form shows them (`id: name`, just the id where the name is the id).
pub fn terminals_text(terminals: &[DeviceTerminal]) -> String {
    terminals
        .iter()
        .map(|t| {
            if t.name == t.id {
                t.id.clone()
            } else {
                format!("{}: {}", t.id, t.name)
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// A device's paths as typed: the paths split at semicolons, each one's terminals at commas.
pub fn paths_of(text: &str) -> Vec<Vec<String>> {
    text.split(';')
        .map(|p| {
            p.split(',')
                .map(js_trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|p| !p.is_empty())
        .collect()
}

/// A device's paths as the form shows them.
pub fn paths_text(paths: &[Vec<String>]) -> String {
    paths
        .iter()
        .map(|p| p.join(", "))
        .collect::<Vec<_>>()
        .join("; ")
}

/// A device's configurations as typed (docs/adr/0562 §2): `value = terminals | paths`, split at double bars; a
/// configuration with no bar has no paths, one with no equals sign no value (the contract says both).
pub fn configs_of(text: &str) -> Vec<TerminalConfig> {
    text.split("||")
        .map(js_trim)
        .filter(|c| !c.is_empty())
        .map(|c| {
            let (value, rest) = c.split_once('=').unwrap_or(("", c));
            let (terminals, paths) = rest.split_once('|').unwrap_or((rest, ""));
            TerminalConfig {
                value: js_trim(value).to_owned(),
                terminals: terminals_of(terminals),
                paths: paths_of(paths),
            }
        })
        .collect()
}

/// A device's configurations as the form shows them.
pub fn configs_text(configs: &[TerminalConfig]) -> String {
    configs
        .iter()
        .map(|c| {
            format!(
                "{} = {} | {}",
                c.value,
                terminals_text(&c.terminals),
                paths_text(&c.paths)
            )
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

/// A delay as the form shows it: empty for none.
fn delay_text(d: f64) -> String {
    if d == 0.0 { String::new() } else { shown(d) }
}

/// A network as the form shows it; `from_metres` gives the project's length unit.
pub fn form_of(def: &NetworkDef, from_metres: impl Fn(f64) -> f64) -> NetworkForm {
    let (direction, field, forward, backward, shut) = match &def.direction {
        NetworkDirection::Both => (
            DirectionKind::Both,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ),
        NetworkDirection::Digitized => (
            DirectionKind::Digitized,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ),
        NetworkDirection::Field {
            field,
            forward,
            backward,
            closed,
        } => (
            DirectionKind::Field,
            field.clone(),
            joined(forward),
            joined(backward),
            joined(closed),
        ),
    };
    NetworkForm {
        id: def.id.clone(),
        name: def.name.clone(),
        kind: def.kind,
        edges: def
            .edges
            .iter()
            .map(|e| EdgeRow {
                layer: e.layer.clone(),
                filter: e.filter.clone().unwrap_or_default(),
            })
            .collect(),
        junctions: def
            .junctions
            .iter()
            .map(|j| JunctionRow {
                layer: j.layer.clone(),
                role: j.role,
                filter: j.filter.clone().unwrap_or_default(),
                closed: j.closed.clone().unwrap_or_default(),
            })
            .collect(),
        connect: def.connect,
        tolerance: shown(from_metres(def.tolerance)),
        direction,
        field,
        forward,
        backward,
        shut,
        costs: def
            .costs
            .iter()
            .map(|c| CostRow {
                name: c.name.clone(),
                kind: c.kind,
                field: c.field.clone(),
                unit: c.unit.clone(),
                speed: c.speed.map(shown).unwrap_or_default(),
                periods: periods_text(&c.periods),
            })
            .collect(),
        closed: def.closed.clone().unwrap_or_default(),
        u_turns: def.turns.as_ref().map(|t| t.u_turns),
        straight: def
            .turns
            .as_ref()
            .map(|t| delay_text(t.straight))
            .unwrap_or_default(),
        right: def
            .turns
            .as_ref()
            .map(|t| delay_text(t.right))
            .unwrap_or_default(),
        left: def
            .turns
            .as_ref()
            .map(|t| delay_text(t.left))
            .unwrap_or_default(),
        back: def
            .turns
            .as_ref()
            .map(|t| delay_text(t.back))
            .unwrap_or_default(),
        turn_layers: def
            .turns
            .iter()
            .flat_map(|t| &t.layers)
            .map(|l| TurnRow {
                layer: l.layer.clone(),
                filter: l.filter.clone().unwrap_or_default(),
                field: l.field.clone().unwrap_or_default(),
            })
            .collect(),
        level_start: def
            .levels
            .as_ref()
            .map(|l| l.start.clone())
            .unwrap_or_default(),
        level_end: def
            .levels
            .as_ref()
            .map(|l| l.end.clone())
            .unwrap_or_default(),
        level_inside: def
            .levels
            .as_ref()
            .and_then(|l| l.inside.clone())
            .unwrap_or_default(),
        terminal_start: def
            .terminals
            .as_ref()
            .map(|t| t.start.clone())
            .unwrap_or_default(),
        terminal_end: def
            .terminals
            .as_ref()
            .map(|t| t.end.clone())
            .unwrap_or_default(),
        devices: def
            .terminals
            .iter()
            .flat_map(|t| &t.devices)
            .map(|d| DeviceRow {
                layer: d.layer.clone(),
                terminals: terminals_text(&d.terminals),
                paths: paths_text(&d.paths),
                by: d.by.clone().unwrap_or_default(),
                configs: configs_text(&d.configs),
            })
            .collect(),
        subnet_reach: def.subnetworks.as_ref().map(|s| s.reach),
        subnet_name: def
            .subnetworks
            .as_ref()
            .and_then(|s| s.name.clone())
            .unwrap_or_default(),
        subnet_attribute: def
            .subnetworks
            .as_ref()
            .map(|s| s.attribute.clone())
            .unwrap_or_default(),
        subnet_keep: def.subnetworks.as_ref().is_some_and(|s| s.keep_current),
    }
}

/// A road network's first cost: Süre from the speed field `hiz`, 50 km/sa.
fn road_cost() -> CostRow {
    CostRow {
        name: "Süre".into(),
        kind: NetworkCostKind::Speed,
        field: "hiz".into(),
        unit: String::new(),
        speed: "50".into(),
        periods: String::new(),
    }
}

/// A new network of `kind` named `name` with id `id`, its first edge layer `layer`: a road network both ways with
/// Süre from the speed field `hiz` (50 km/sa); a utility network in the direction its pipes were drawn, no costs.
pub fn new_form(
    id: &str,
    name: &str,
    kind: NetworkKind,
    layer: &str,
    from_metres: impl Fn(f64) -> f64,
) -> NetworkForm {
    NetworkForm {
        id: id.to_owned(),
        name: name.to_owned(),
        kind,
        edges: if layer.is_empty() {
            Vec::new()
        } else {
            vec![EdgeRow {
                layer: layer.to_owned(),
                filter: String::new(),
            }]
        },
        junctions: Vec::new(),
        connect: NetworkConnect::Ends,
        tolerance: shown(from_metres(NETWORK_TOLERANCE)),
        direction: if kind == NetworkKind::Road {
            DirectionKind::Both
        } else {
            DirectionKind::Digitized
        },
        field: String::new(),
        forward: String::new(),
        backward: String::new(),
        shut: String::new(),
        costs: if kind == NetworkKind::Road {
            vec![road_cost()]
        } else {
            Vec::new()
        },
        closed: String::new(),
        u_turns: None,
        straight: String::new(),
        right: String::new(),
        left: String::new(),
        back: String::new(),
        turn_layers: Vec::new(),
        level_start: String::new(),
        level_end: String::new(),
        level_inside: String::new(),
        terminal_start: String::new(),
        terminal_end: String::new(),
        devices: Vec::new(),
        subnet_reach: None,
        subnet_name: String::new(),
        subnet_attribute: String::new(),
        subnet_keep: false,
    }
}

/// The first letter upper case, a full stop at the end: the contract's words as a sentence.
fn sentence(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => format!(
            "{}{}.",
            crate::prompt::upper_tr(&c.to_string()),
            chars.as_str()
        ),
        None => String::new(),
    }
}

fn opt(s: &str) -> Option<String> {
    let t = js_trim(s);
    (!t.is_empty()).then(|| t.to_owned())
}

/// The network the form writes, or what is wrong in words: a tolerance or a speed that is not a number (the form's
/// words), then the contract's rules (as a sentence). `to_metres`: the project's length unit to metres.
pub fn def_of(form: &NetworkForm, to_metres: impl Fn(f64) -> f64) -> Result<NetworkDef, String> {
    let name = js_trim(&form.name).to_owned();
    let who = if name.is_empty() {
        "Ağın".to_owned()
    } else {
        format!("“{name}” ağının")
    };
    let Some(t) = decimal(&form.tolerance) else {
        return Err(format!("{who} toleransı bir sayı olmalı (ör. 0,01)."));
    };
    let mut costs = Vec::new();
    for c in &form.costs {
        let cost = js_trim(&c.name).to_owned();
        match c.kind {
            NetworkCostKind::Speed => {
                let typed = js_trim(&c.speed);
                let speed = if typed.is_empty() {
                    Some(DEFAULT_SPEED)
                } else {
                    decimal(typed)
                };
                let Some(speed) = speed else {
                    return Err(format!(
                        "{who} “{cost}” maliyetinin hızı bir sayı olmalı (km/sa)."
                    ));
                };
                let periods = periods_of(&c.periods)
                    .map_err(|e| format!("{who} “{cost}” maliyetinin zaman profili: {e}"))?;
                costs.push(NetworkCost {
                    name: cost,
                    kind: NetworkCostKind::Speed,
                    field: js_trim(&c.field).to_owned(),
                    unit: String::new(),
                    speed: Some(speed),
                    periods,
                });
            }
            NetworkCostKind::Field => costs.push(NetworkCost {
                name: cost,
                kind: NetworkCostKind::Field,
                field: js_trim(&c.field).to_owned(),
                unit: js_trim(&c.unit).to_owned(),
                speed: None,
                periods: Vec::new(),
            }),
        }
    }
    let turns = match form.u_turns {
        None => None,
        Some(u_turns) => {
            let mut delays = [0.0; 4];
            for (slot, (what, typed)) in [
                ("düz geçiş", &form.straight),
                ("sağa dönüş", &form.right),
                ("sola dönüş", &form.left),
                ("geri dönüş", &form.back),
            ]
            .into_iter()
            .enumerate()
            {
                if js_trim(typed).is_empty() {
                    continue;
                }
                let Some(d) = decimal(typed) else {
                    return Err(format!("{who} {what} gecikmesi bir sayı olmalı (sn)."));
                };
                delays[slot] = d;
            }
            Some(NetworkTurns {
                u_turns,
                straight: delays[0],
                right: delays[1],
                left: delays[2],
                back: delays[3],
                layers: form
                    .turn_layers
                    .iter()
                    .map(|l| TurnLayer {
                        layer: l.layer.clone(),
                        filter: opt(&l.filter),
                        field: opt(&l.field),
                    })
                    .collect(),
            })
        }
    };
    let levels = match (opt(&form.level_start), opt(&form.level_end)) {
        (None, None) if opt(&form.level_inside).is_none() => None,
        (Some(start), Some(end)) => Some(NetworkLevels {
            start,
            end,
            inside: opt(&form.level_inside),
        }),
        _ => {
            return Err(format!(
                "{who} seviyesi için başlangıç ve bitiş alanı birlikte yazılır."
            ));
        }
    };
    let terminals = match (opt(&form.terminal_start), opt(&form.terminal_end)) {
        (None, None) if form.devices.is_empty() => None,
        (Some(start), Some(end)) => Some(NetworkTerminals {
            start,
            end,
            devices: form
                .devices
                .iter()
                .map(|d| TerminalDevice {
                    layer: d.layer.clone(),
                    terminals: terminals_of(&d.terminals),
                    paths: paths_of(&d.paths),
                    by: opt(&d.by),
                    configs: configs_of(&d.configs),
                })
                .collect(),
        }),
        _ => {
            return Err(format!(
                "{who} terminalleri için başlangıç ve bitiş alanı birlikte yazılır."
            ));
        }
    };
    let subnetworks = form.subnet_reach.map(|reach| NetworkSubnetworks {
        name: opt(&form.subnet_name),
        attribute: opt(&form.subnet_attribute)
            .unwrap_or_else(|| DEFAULT_SUBNET_ATTRIBUTE.to_owned()),
        reach,
        keep_current: form.subnet_keep,
    });
    let def = NetworkDef {
        id: form.id.clone(),
        name,
        kind: form.kind,
        edges: form
            .edges
            .iter()
            .map(|e| NetworkLayer {
                layer: e.layer.clone(),
                filter: opt(&e.filter),
            })
            .collect(),
        junctions: form
            .junctions
            .iter()
            .map(|j| JunctionLayer {
                layer: j.layer.clone(),
                role: j.role,
                filter: opt(&j.filter),
                closed: opt(&j.closed),
            })
            .collect(),
        connect: form.connect,
        tolerance: to_metres(t),
        direction: match form.direction {
            DirectionKind::Both => NetworkDirection::Both,
            DirectionKind::Digitized => NetworkDirection::Digitized,
            DirectionKind::Field => NetworkDirection::Field {
                field: js_trim(&form.field).to_owned(),
                forward: values_of(&form.forward),
                backward: values_of(&form.backward),
                closed: values_of(&form.shut),
            },
        },
        costs,
        closed: opt(&form.closed),
        turns,
        levels,
        terminals,
        subnetworks,
    };
    match def.problem() {
        Some(wrong) => Err(sentence(&wrong)),
        None => Ok(def),
    }
}

/// The expressions of a form, each with its place in words (the window compiles them before Kaydet).
pub fn form_expressions(form: &NetworkForm) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, e) in form.edges.iter().enumerate() {
        if let Some(t) = opt(&e.filter) {
            out.push((format!("{}. kenar katmanının süzgeci", i + 1), t));
        }
    }
    for (i, j) in form.junctions.iter().enumerate() {
        if let Some(t) = opt(&j.filter) {
            out.push((format!("{}. düğüm katmanının süzgeci", i + 1), t));
        }
        if let Some(t) = opt(&j.closed) {
            out.push((format!("{}. düğüm katmanının kapalı ifadesi", i + 1), t));
        }
    }
    if let Some(t) = opt(&form.closed) {
        out.push(("Kapalı kenarların ifadesi".to_owned(), t));
    }
    if form.u_turns.is_some() {
        for (i, l) in form.turn_layers.iter().enumerate() {
            if let Some(t) = opt(&l.filter) {
                out.push((format!("{}. dönüş katmanının süzgeci", i + 1), t));
            }
        }
    }
    out
}

/// Whether a form's costs are a new road network's (Süre from `hiz`, 50 km/sa).
fn road_costs(f: &NetworkForm) -> bool {
    match &f.costs[..] {
        [c] => {
            let speed = if c.speed.is_empty() { "50" } else { &c.speed };
            c.name == "Süre"
                && c.kind == NetworkCostKind::Speed
                && c.field == "hiz"
                && decimal(speed) == Some(DEFAULT_SPEED)
        }
        _ => false,
    }
}

/// The form with another kind: what is still the old kind's default becomes the new kind's (the direction: both ways
/// or as drawn; the costs: a road network's Süre or none); what was changed stays.
pub fn retyped(f: &NetworkForm, kind: NetworkKind) -> NetworkForm {
    let mut out = f.clone();
    if f.kind == kind {
        return out;
    }
    out.kind = kind;
    if kind == NetworkKind::Utility {
        if f.direction == DirectionKind::Both {
            out.direction = DirectionKind::Digitized;
        }
        if road_costs(f) {
            out.costs.clear();
        }
    } else {
        if f.direction == DirectionKind::Digitized {
            out.direction = DirectionKind::Both;
        }
        if f.costs.is_empty() {
            out.costs = vec![road_cost()];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/network/v1/form.json"
        );
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
    }

    fn text(v: &Value, key: &str) -> String {
        v[key].as_str().unwrap_or_default().to_owned()
    }

    fn enum_of<T: serde::de::DeserializeOwned>(v: &Value) -> T {
        serde_json::from_value(v.clone()).expect("a value of the contract")
    }

    fn form(v: &Value) -> NetworkForm {
        let list = |key: &str| v[key].as_array().cloned().unwrap_or_default();
        NetworkForm {
            id: text(v, "id"),
            name: text(v, "name"),
            kind: enum_of(&v["kind"]),
            edges: list("edges")
                .iter()
                .map(|e| EdgeRow {
                    layer: text(e, "layer"),
                    filter: text(e, "filter"),
                })
                .collect(),
            junctions: list("junctions")
                .iter()
                .map(|j| JunctionRow {
                    layer: text(j, "layer"),
                    role: enum_of(&j["role"]),
                    filter: text(j, "filter"),
                    closed: text(j, "closed"),
                })
                .collect(),
            connect: enum_of(&v["connect"]),
            tolerance: text(v, "tolerance"),
            direction: match v["direction"].as_str() {
                Some("field") => DirectionKind::Field,
                Some("digitized") => DirectionKind::Digitized,
                _ => DirectionKind::Both,
            },
            field: text(v, "field"),
            forward: text(v, "forward"),
            backward: text(v, "backward"),
            shut: text(v, "shut"),
            costs: list("costs")
                .iter()
                .map(|c| CostRow {
                    name: text(c, "name"),
                    kind: enum_of(&c["kind"]),
                    field: text(c, "field"),
                    unit: text(c, "unit"),
                    speed: text(c, "speed"),
                    periods: text(c, "periods"),
                })
                .collect(),
            closed: text(v, "closed"),
            u_turns: (!v["uTurns"].is_null()).then(|| enum_of(&v["uTurns"])),
            straight: text(v, "straight"),
            right: text(v, "right"),
            left: text(v, "left"),
            back: text(v, "back"),
            turn_layers: list("turnLayers")
                .iter()
                .map(|l| TurnRow {
                    layer: text(l, "layer"),
                    filter: text(l, "filter"),
                    field: text(l, "field"),
                })
                .collect(),
            level_start: text(v, "levelStart"),
            level_end: text(v, "levelEnd"),
            level_inside: text(v, "levelInside"),
            terminal_start: text(v, "terminalStart"),
            terminal_end: text(v, "terminalEnd"),
            devices: list("devices")
                .iter()
                .map(|d| DeviceRow {
                    layer: text(d, "layer"),
                    terminals: text(d, "terminals"),
                    paths: text(d, "paths"),
                    by: text(d, "by"),
                    configs: text(d, "configs"),
                })
                .collect(),
            subnet_reach: (!v["subnetReach"].is_null()).then(|| enum_of(&v["subnetReach"])),
            subnet_name: text(v, "subnetName"),
            subnet_attribute: text(v, "subnetAttribute"),
            subnet_keep: v["subnetKeep"].as_bool().unwrap_or(false),
        }
    }

    fn per_metre(unit: &str) -> f64 {
        match unit {
            "cm" => 100.0,
            "mm" => 1000.0,
            _ => 1.0,
        }
    }

    /// What a form writes in a project's unit, the form's own words, the kind's defaults and networks shown and
    /// written back, as fixtures/network/v1/form.json says them.
    #[test]
    fn the_form_writes_what_the_shared_cases_say() {
        let f = fixture();
        assert_eq!(f["format"], "kentos.network-form");
        for c in f["cases"].as_array().expect("cases") {
            let name = text(c, "name");
            let unit = per_metre(c["unit"].as_str().unwrap_or("m"));
            let got = def_of(&form(&c["form"]), |v| v / unit);
            match c.get("problem").and_then(Value::as_str) {
                Some(words) => assert_eq!(got, Err(words.to_owned()), "{name}"),
                None => assert_eq!(got, Ok(enum_of::<NetworkDef>(&c["network"])), "{name}"),
            }
        }
        for c in f["retype"].as_array().expect("retypes") {
            let name = text(c, "name");
            assert_eq!(
                retyped(&form(&c["form"]), enum_of(&c["kind"])),
                form(&c["expect"]),
                "{name}"
            );
        }
        for n in f["roundTrip"].as_array().expect("networks") {
            let def: NetworkDef = enum_of(n);
            assert_eq!(
                def_of(&form_of(&def, |m| m), |v| v),
                Ok(def.clone()),
                "{}",
                def.name
            );
        }
    }
}
