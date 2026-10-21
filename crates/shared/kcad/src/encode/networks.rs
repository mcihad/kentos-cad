//! The project's networks (document schema 33, docs/specs/kcad-v2.md §6.4.8,
//! docs/adr/0209 §2): checked whole by the contract's rules
//! (`networks_problem`), as the readers check them, each map's keys sorted
//! as RFC 8949 sorts them. An optional field is written only when it has a
//! value, a list only when it is not empty. Schema 87 (docs/adr/0521) adds a
//! network's `turns` and `levels` and a speed cost's `periods`. Schema 95
//! (docs/adr/0567) a network's vehicle `profiles` and dated `closures`.

use kentos_contracts::{
    DeviceTerminal, JunctionRole, NetworkConnect, NetworkCostKind, NetworkDef, NetworkDirection,
    NetworkClosures, NetworkKind, NetworkLevels, NetworkProfile, NetworkPeriod, NetworkSubnetworks, NetworkTerminals, NetworkTurns,
    NetworkUTurns, SubnetworkReach, TerminalConfig, TerminalDevice, TurnLayer, networks_problem,
};

use super::Encoder;
use crate::cbor::{Seg, key_order};
use crate::error::{Code, KcadError};

/// One field before the fields are sorted by key.
enum V<'d> {
    Text(&'d str),
    Float(f64),
    Texts(&'d [String]),
    Edges(&'d NetworkDef),
    Junctions(&'d NetworkDef),
    Direction(&'d NetworkDirection),
    Costs(&'d NetworkDef),
    Uint(u32),
    Turns(&'d NetworkTurns),
    TurnLayers(&'d [TurnLayer]),
    Levels(&'d NetworkLevels),
    Periods(&'d [NetworkPeriod]),
    Terminals(&'d NetworkTerminals),
    Devices(&'d [TerminalDevice]),
    Paths(&'d [Vec<String>]),
    DeviceTerminals(&'d [DeviceTerminal]),
    Configs(&'d [TerminalConfig]),
    True,
    Subnetworks(&'d NetworkSubnetworks),
    Profiles(&'d [NetworkProfile]),
    Closures(&'d NetworkClosures),
    Limits(&'d NetworkProfile),
    Forbidden(&'d NetworkProfile),
}

fn u_turns_name(u: NetworkUTurns) -> &'static str {
    match u {
        NetworkUTurns::Allowed => "allowed",
        NetworkUTurns::DeadEnds => "deadEnds",
        NetworkUTurns::Never => "never",
    }
}

fn kind_name(k: NetworkKind) -> &'static str {
    match k {
        NetworkKind::Road => "road",
        NetworkKind::Utility => "utility",
    }
}

fn connect_name(c: NetworkConnect) -> &'static str {
    match c {
        NetworkConnect::Ends => "ends",
        NetworkConnect::Vertices => "vertices",
    }
}

fn role_name(r: JunctionRole) -> &'static str {
    match r {
        JunctionRole::Junction => "junction",
        JunctionRole::Source => "source",
        JunctionRole::Valve => "valve",
    }
}

fn cost_kind_name(k: NetworkCostKind) -> &'static str {
    match k {
        NetworkCostKind::Speed => "speed",
        NetworkCostKind::Field => "field",
    }
}

impl<'d> Encoder<'d> {
    /// A map of `fields`, their keys sorted.
    fn network_map(&mut self, mut fields: Vec<(&'static str, V<'d>)>) -> Result<(), KcadError> {
        fields.sort_by(|a, b| key_order(a.0, b.0));
        self.open(fields.len(), true)?;
        for (key, v) in fields {
            self.key(key);
            self.at(Seg::Name(key), |e| e.network_value(v))?;
        }
        self.close();
        Ok(())
    }

    fn network_value(&mut self, v: V<'d>) -> Result<(), KcadError> {
        match v {
            V::Text(t) => self.text(t),
            V::Float(x) => self.float(x),
            V::Uint(n) => {
                self.w.uint(u64::from(n));
                Ok(())
            }
            V::Turns(t) => {
                let mut f = vec![("uTurns", V::Text(u_turns_name(t.u_turns)))];
                for (key, d) in [
                    ("straight", t.straight),
                    ("right", t.right),
                    ("left", t.left),
                    ("back", t.back),
                ] {
                    if d != 0.0 {
                        f.push((key, V::Float(d)));
                    }
                }
                if !t.layers.is_empty() {
                    f.push(("layers", V::TurnLayers(&t.layers)));
                }
                self.network_map(f)
            }
            V::TurnLayers(layers) => {
                self.open(layers.len(), false)?;
                for (i, l) in layers.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        let mut f = vec![("layer", V::Text(&l.layer))];
                        if let Some(x) = &l.filter {
                            f.push(("filter", V::Text(x)));
                        }
                        if let Some(x) = &l.field {
                            f.push(("field", V::Text(x)));
                        }
                        e.network_map(f)
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Terminals(t) => self.network_map(vec![
                ("start", V::Text(&t.start)),
                ("end", V::Text(&t.end)),
                ("devices", V::Devices(&t.devices)),
            ]),
            V::Devices(list) => {
                self.open(list.len(), false)?;
                for (i, d) in list.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        let mut f = vec![
                            ("layer", V::Text(&d.layer)),
                            ("paths", V::Paths(&d.paths)),
                            ("terminals", V::DeviceTerminals(&d.terminals)),
                        ];
                        if let Some(by) = &d.by {
                            f.push(("by", V::Text(by)));
                        }
                        if !d.configs.is_empty() {
                            f.push(("configs", V::Configs(&d.configs)));
                        }
                        e.network_map(f)
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Configs(list) => {
                self.open(list.len(), false)?;
                for (i, c) in list.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        e.network_map(vec![
                            ("value", V::Text(&c.value)),
                            ("paths", V::Paths(&c.paths)),
                            ("terminals", V::DeviceTerminals(&c.terminals)),
                        ])
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Paths(paths) => {
                self.open(paths.len(), false)?;
                for (k, path) in paths.iter().enumerate() {
                    self.at(Seg::Index(k), |e| e.network_value(V::Texts(path)))?;
                }
                self.close();
                Ok(())
            }
            V::DeviceTerminals(list) => {
                self.open(list.len(), false)?;
                for (k, t) in list.iter().enumerate() {
                    self.at(Seg::Index(k), |e| {
                        e.network_map(vec![("id", V::Text(&t.id)), ("name", V::Text(&t.name))])
                    })?;
                }
                self.close();
                Ok(())
            }
            V::True => {
                self.w.bool(true);
                Ok(())
            }
            V::Subnetworks(s) => {
                let mut f = vec![
                    ("attribute", V::Text(&s.attribute)),
                    (
                        "reach",
                        V::Text(match s.reach {
                            SubnetworkReach::Downstream => "downstream",
                            SubnetworkReach::Connected => "connected",
                        }),
                    ),
                ];
                if let Some(x) = &s.name {
                    f.push(("name", V::Text(x)));
                }
                if s.keep_current {
                    f.push(("keepCurrent", V::True));
                }
                self.network_map(f)
            }
            V::Levels(l) => {
                let mut f = vec![("start", V::Text(&l.start)), ("end", V::Text(&l.end))];
                if let Some(x) = &l.inside {
                    f.push(("inside", V::Text(x)));
                }
                self.network_map(f)
            }
            V::Periods(list) => {
                self.open(list.len(), false)?;
                for (i, p) in list.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        let mut f = vec![("from", V::Uint(p.from)), ("to", V::Uint(p.to))];
                        if let Some(x) = p.factor {
                            f.push(("factor", V::Float(x)));
                        }
                        if let Some(x) = &p.field {
                            f.push(("field", V::Text(x)));
                        }
                        e.network_map(f)
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Profiles(list) => {
                self.open(list.len(), false)?;
                for (i, p) in list.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        let mut f = vec![("name", V::Text(&p.name))];
                        if !p.limits.is_empty() {
                            f.push(("limits", V::Limits(p)));
                        }
                        if !p.forbidden.is_empty() {
                            f.push(("forbidden", V::Forbidden(p)));
                        }
                        if let Some(x) = p.top_speed {
                            f.push(("topSpeed", V::Float(x)));
                        }
                        if p.both_ways {
                            f.push(("bothWays", V::True));
                        }
                        e.network_map(f)
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Limits(p) => {
                self.open(p.limits.len(), false)?;
                for (i, l) in p.limits.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        e.network_map(vec![("field", V::Text(&l.field)), ("value", V::Float(l.value))])
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Forbidden(p) => {
                self.open(p.forbidden.len(), false)?;
                for (i, l) in p.forbidden.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        e.network_map(vec![("field", V::Text(&l.field)), ("values", V::Texts(&l.values))])
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Closures(c) => {
                let mut f = vec![("from", V::Text(&c.from)), ("to", V::Text(&c.to))];
                if let Some(d) = &c.days {
                    f.push(("days", V::Text(d)));
                }
                if let Some(h) = &c.hours {
                    f.push(("hours", V::Text(h)));
                }
                self.network_map(f)
            }
            V::Texts(list) => {
                self.open(list.len(), false)?;
                for (i, t) in list.iter().enumerate() {
                    self.at(Seg::Index(i), |e| e.text(t))?;
                }
                self.close();
                Ok(())
            }
            V::Edges(n) => {
                self.open(n.edges.len(), false)?;
                for (i, l) in n.edges.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        let mut f = vec![("layer", V::Text(&l.layer))];
                        if let Some(x) = &l.filter {
                            f.push(("filter", V::Text(x)));
                        }
                        e.network_map(f)
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Junctions(n) => {
                self.open(n.junctions.len(), false)?;
                for (i, j) in n.junctions.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        let mut f = vec![
                            ("layer", V::Text(&j.layer)),
                            ("role", V::Text(role_name(j.role))),
                        ];
                        if let Some(x) = &j.filter {
                            f.push(("filter", V::Text(x)));
                        }
                        if let Some(x) = &j.closed {
                            f.push(("closed", V::Text(x)));
                        }
                        e.network_map(f)
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Direction(d) => match d {
                NetworkDirection::Both => self.network_map(vec![("kind", V::Text("both"))]),
                NetworkDirection::Digitized => {
                    self.network_map(vec![("kind", V::Text("digitized"))])
                }
                NetworkDirection::Field {
                    field,
                    forward,
                    backward,
                    closed,
                } => {
                    let mut f = vec![("kind", V::Text("field")), ("field", V::Text(field))];
                    for (key, list) in [
                        ("forward", forward),
                        ("backward", backward),
                        ("closed", closed),
                    ] {
                        if !list.is_empty() {
                            f.push((key, V::Texts(list)));
                        }
                    }
                    self.network_map(f)
                }
            },
            V::Costs(n) => {
                self.open(n.costs.len(), false)?;
                for (i, c) in n.costs.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        let mut f = vec![
                            ("name", V::Text(&c.name)),
                            ("kind", V::Text(cost_kind_name(c.kind))),
                            ("field", V::Text(&c.field)),
                        ];
                        if !c.unit.is_empty() {
                            f.push(("unit", V::Text(&c.unit)));
                        }
                        if let Some(s) = c.speed {
                            f.push(("speed", V::Float(s)));
                        }
                        if !c.periods.is_empty() {
                            f.push(("periods", V::Periods(&c.periods)));
                        }
                        e.network_map(f)
                    })?;
                }
                self.close();
                Ok(())
            }
        }
    }

    /// The project's networks (docs/adr/0209 §2), checked whole.
    pub(super) fn networks(&mut self, list: &'d [NetworkDef]) -> Result<(), KcadError> {
        if let Some(problem) = networks_problem(list) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(list.len(), false)?;
        for (i, n) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                let mut f = vec![
                    ("id", V::Text(&n.id)),
                    ("name", V::Text(&n.name)),
                    ("kind", V::Text(kind_name(n.kind))),
                    ("edges", V::Edges(n)),
                    ("connect", V::Text(connect_name(n.connect))),
                    ("tolerance", V::Float(n.tolerance)),
                    ("direction", V::Direction(&n.direction)),
                ];
                if !n.junctions.is_empty() {
                    f.push(("junctions", V::Junctions(n)));
                }
                if !n.costs.is_empty() {
                    f.push(("costs", V::Costs(n)));
                }
                if let Some(x) = &n.closed {
                    f.push(("closed", V::Text(x)));
                }
                if let Some(t) = &n.turns {
                    f.push(("turns", V::Turns(t)));
                }
                if let Some(l) = &n.levels {
                    f.push(("levels", V::Levels(l)));
                }
                if let Some(t) = &n.terminals {
                    f.push(("terminals", V::Terminals(t)));
                }
                if let Some(s) = &n.subnetworks {
                    f.push(("subnetworks", V::Subnetworks(s)));
                }
                if !n.profiles.is_empty() {
                    f.push(("profiles", V::Profiles(&n.profiles)));
                }
                if let Some(c) = &n.closures {
                    f.push(("closures", V::Closures(c)));
                }
                e.network_map(f)
            })?;
        }
        self.close();
        Ok(())
    }
}
