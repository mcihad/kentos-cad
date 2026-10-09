//! The project's networks (document schema 33, docs/specs/kcad-v2.md §6.4.8,
//! docs/adr/0209 §2): checked whole by the contract's rules
//! (`networks_problem`), as the readers check them, each map's keys sorted
//! as RFC 8949 sorts them. An optional field is written only when it has a
//! value, a list only when it is not empty.

use kentos_contracts::{
    JunctionRole, NetworkConnect, NetworkCostKind, NetworkDef, NetworkDirection, NetworkKind,
    networks_problem,
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
                e.network_map(f)
            })?;
        }
        self.close();
        Ok(())
    }
}
