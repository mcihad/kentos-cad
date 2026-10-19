//! Schema 33's networks (docs/specs/kcad-v2.md §6.4.8, docs/adr/0209 §2): the
//! settings' `networks`, read field by field and then checked whole by the
//! contract's rules (`networks_problem`), as the writers check them; an
//! empty list is refused (a writer leaves it out). Schema 87 (docs/adr/0521)
//! adds a network's `turns` and `levels` and a speed cost's `periods`; an
//! older file holding them is refused.

use kentos_contracts::{
    JunctionLayer, JunctionRole, NetworkConnect, NetworkCost, NetworkCostKind, NetworkDef,
    NetworkDirection, NetworkKind, NetworkLayer, NetworkLevels, NetworkPeriod, NetworkTurns,
    NetworkUTurns, TurnLayer, networks_problem,
};

use super::{list, map, named, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// A list of texts that is not empty.
fn texts(r: &mut Reader<'_>, words: &str) -> Result<Vec<String>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| text(r))?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, words));
    }
    Ok(all)
}

fn edges(r: &mut Reader<'_>) -> Result<Vec<NetworkLayer>, KcadError> {
    list(r, |r, _| {
        let (mut layer, mut filter) = (None, None);
        map(r, |r, key| {
            match key {
                "layer" => layer = Some(text(r)?),
                "filter" => filter = Some(text(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(NetworkLayer {
            layer: required(r, layer, "layer")?,
            filter,
        })
    })
}

fn junctions(r: &mut Reader<'_>) -> Result<Vec<JunctionLayer>, KcadError> {
    let at = r.position();
    let roles = [
        ("junction", JunctionRole::Junction),
        ("source", JunctionRole::Source),
        ("valve", JunctionRole::Valve),
    ];
    let all = list(r, |r, _| {
        let (mut layer, mut role, mut filter, mut closed) = (None, None, None, None);
        map(r, |r, key| {
            match key {
                "layer" => layer = Some(text(r)?),
                "role" => role = Some(named(r, &roles)?),
                "filter" => filter = Some(text(r)?),
                "closed" => closed = Some(text(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(JunctionLayer {
            layer: required(r, layer, "layer")?,
            role: required(r, role, "role")?,
            filter,
            closed,
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş düğüm katmanı listesi yazılmaz"));
    }
    Ok(all)
}

fn direction(r: &mut Reader<'_>) -> Result<NetworkDirection, KcadError> {
    let at = r.position();
    let kinds = [("both", 0u8), ("digitized", 1), ("field", 2)];
    let (mut kind, mut field) = (None, None);
    let (mut forward, mut backward, mut closed) = (Vec::new(), Vec::new(), Vec::new());
    map(r, |r, key| {
        match key {
            "kind" => kind = Some(named(r, &kinds)?),
            "field" => field = Some(text(r)?),
            "forward" => forward = texts(r, "boş yön listesi yazılmaz")?,
            "backward" => backward = texts(r, "boş yön listesi yazılmaz")?,
            "closed" => closed = texts(r, "boş yön listesi yazılmaz")?,
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let kind = required(r, kind, "kind")?;
    let any = field.is_some() || !forward.is_empty() || !backward.is_empty() || !closed.is_empty();
    match kind {
        2 => Ok(NetworkDirection::Field {
            field: required(r, field, "field")?,
            forward,
            backward,
            closed,
        }),
        _ if any => Err(r.fail_at(
            Code::BadValue,
            at,
            "yönün alanı ve değerleri yalnız alanla yönde yazılır",
        )),
        0 => Ok(NetworkDirection::Both),
        _ => Ok(NetworkDirection::Digitized),
    }
}

/// A speed cost's periods (schema 87), not empty; their order, the day's bounds and values are checked whole with the network.
fn periods(r: &mut Reader<'_>) -> Result<Vec<NetworkPeriod>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (mut from, mut to, mut factor, mut field) = (None, None, None, None);
        map(r, |r, key| {
            match key {
                "from" => from = Some(r.uint(u64::from(u32::MAX))? as u32),
                "to" => to = Some(r.uint(u64::from(u32::MAX))? as u32),
                "factor" => factor = Some(r.float()?),
                "field" => field = Some(text(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(NetworkPeriod {
            from: required(r, from, "from")?,
            to: required(r, to, "to")?,
            factor,
            field,
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş dönem listesi yazılmaz"));
    }
    Ok(all)
}

/// A network's turns (schema 87): a delay is written only when it is not zero, the layers only when there are some.
fn turns(r: &mut Reader<'_>) -> Result<NetworkTurns, KcadError> {
    let kinds = [
        ("allowed", NetworkUTurns::Allowed),
        ("deadEnds", NetworkUTurns::DeadEnds),
        ("never", NetworkUTurns::Never),
    ];
    let mut u_turns = None;
    let mut delays = [0.0f64; 4];
    let mut layers = Vec::new();
    map(r, |r, key| {
        let slot = match key {
            "uTurns" => {
                u_turns = Some(named(r, &kinds)?);
                return Ok(());
            }
            "layers" => {
                let at = r.position();
                layers = list(r, |r, _| {
                    let (mut layer, mut filter, mut field) = (None, None, None);
                    map(r, |r, key| {
                        match key {
                            "layer" => layer = Some(text(r)?),
                            "filter" => filter = Some(text(r)?),
                            "field" => field = Some(text(r)?),
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(TurnLayer {
                        layer: required(r, layer, "layer")?,
                        filter,
                        field,
                    })
                })?;
                if layers.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş dönüş katmanı listesi yazılmaz",
                    ));
                }
                return Ok(());
            }
            "straight" => 0,
            "right" => 1,
            "left" => 2,
            "back" => 3,
            _ => return Err(unknown(r)),
        };
        let at = r.position();
        let d = r.float()?;
        if d == 0.0 {
            return Err(r.fail_at(Code::BadValue, at, "sıfır gecikme yazılmaz"));
        }
        delays[slot] = d;
        Ok(())
    })?;
    Ok(NetworkTurns {
        u_turns: required(r, u_turns, "uTurns")?,
        straight: delays[0],
        right: delays[1],
        left: delays[2],
        back: delays[3],
        layers,
    })
}

/// A network's level fields (schema 87).
fn levels(r: &mut Reader<'_>) -> Result<NetworkLevels, KcadError> {
    let (mut start, mut end, mut inside) = (None, None, None);
    map(r, |r, key| {
        match key {
            "start" => start = Some(text(r)?),
            "end" => end = Some(text(r)?),
            "inside" => inside = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(NetworkLevels {
        start: required(r, start, "start")?,
        end: required(r, end, "end")?,
        inside,
    })
}

fn costs(r: &mut Reader<'_>, turned: bool) -> Result<Vec<NetworkCost>, KcadError> {
    let at = r.position();
    let kinds = [
        ("speed", NetworkCostKind::Speed),
        ("field", NetworkCostKind::Field),
    ];
    let all = list(r, |r, _| {
        let (mut name, mut kind, mut field, mut unit, mut speed) = (None, None, None, None, None);
        let mut times = Vec::new();
        map(r, |r, key| {
            match key {
                "name" => name = Some(text(r)?),
                "kind" => kind = Some(named(r, &kinds)?),
                "field" => field = Some(text(r)?),
                "unit" => {
                    let at = r.position();
                    let u = text(r)?;
                    if u.is_empty() {
                        return Err(r.fail_at(Code::BadValue, at, "boş birim yazılmaz"));
                    }
                    unit = Some(u);
                }
                "speed" => speed = Some(r.float()?),
                "periods" if turned => times = periods(r)?,
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(NetworkCost {
            name: required(r, name, "name")?,
            kind: required(r, kind, "kind")?,
            field: required(r, field, "field")?,
            unit: unit.unwrap_or_default(),
            speed,
            periods: times,
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş maliyet listesi yazılmaz"));
    }
    Ok(all)
}

/// The project's networks, checked whole; an empty list is not written.
pub(super) fn networks(r: &mut Reader<'_>, turned: bool) -> Result<Vec<NetworkDef>, KcadError> {
    let at = r.position();
    let kinds = [
        ("road", NetworkKind::Road),
        ("utility", NetworkKind::Utility),
    ];
    let connects = [
        ("ends", NetworkConnect::Ends),
        ("vertices", NetworkConnect::Vertices),
    ];
    let all = list(r, |r, _| {
        let (mut id, mut name, mut kind, mut edge_layers) = (None, None, None, None);
        let (mut connect, mut tolerance, mut dir) = (None, None, None);
        let (mut junction_layers, mut cost_list, mut closed) = (Vec::new(), Vec::new(), None);
        let (mut turn_rules, mut level_fields) = (None, None);
        map(r, |r, key| {
            match key {
                "id" => id = Some(text(r)?),
                "name" => name = Some(text(r)?),
                "kind" => kind = Some(named(r, &kinds)?),
                "edges" => edge_layers = Some(edges(r)?),
                "junctions" => junction_layers = junctions(r)?,
                "connect" => connect = Some(named(r, &connects)?),
                "tolerance" => tolerance = Some(r.float()?),
                "direction" => dir = Some(direction(r)?),
                "costs" => cost_list = costs(r, turned)?,
                "closed" => closed = Some(text(r)?),
                "turns" if turned => turn_rules = Some(turns(r)?),
                "levels" if turned => level_fields = Some(levels(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(NetworkDef {
            id: required(r, id, "id")?,
            name: required(r, name, "name")?,
            kind: required(r, kind, "kind")?,
            edges: required(r, edge_layers, "edges")?,
            junctions: junction_layers,
            connect: required(r, connect, "connect")?,
            tolerance: required(r, tolerance, "tolerance")?,
            direction: required(r, dir, "direction")?,
            costs: cost_list,
            closed,
            turns: turn_rules,
            levels: level_fields,
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş ağ listesi yazılmaz"));
    }
    match networks_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}
