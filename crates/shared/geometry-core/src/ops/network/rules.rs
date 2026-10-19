//! What a network's definition says about its edges (docs/adr/0209 §2):
//! where they connect and within what tolerance, which way each goes by its
//! direction field's value, and what travelling it costs by its cost
//! fields' values. The host evaluates the definition's expressions and
//! gives the raw values; how they read is here, the same on every platform.

use crate::api::json::Json;
use crate::ops::statistics::read_number;

/// Where edges connect (docs/adr/0209 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connect {
    /// At their ends, and an end on another edge splits it there.
    Ends,
    /// At every vertex too.
    Vertices,
}

/// Which way an edge may be travelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Both,
    /// The way it is drawn only.
    Forward,
    /// Against the way it is drawn only.
    Backward,
    /// Neither way.
    Closed,
}

impl Dir {
    /// Whether the edge may be travelled the way it is drawn (`forward`) or against it.
    pub fn allows(self, forward: bool) -> bool {
        match self {
            Dir::Both => true,
            Dir::Forward => forward,
            Dir::Backward => !forward,
            Dir::Closed => false,
        }
    }
}

/// The definition's direction, its field's values folded as the rules compare them.
#[derive(Clone, Debug, PartialEq)]
pub enum DirectionRule {
    Both,
    Digitized,
    Field {
        forward: Vec<String>,
        backward: Vec<String>,
        closed: Vec<String>,
    },
}

/// A cost besides the length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CostRule {
    /// Minutes: the length over the speed (km/h); `kmh` where the field gives none it can read.
    Speed { kmh: f64 },
    /// The field's value for the whole edge, shared by its pieces by their lengths.
    Field,
}

/// Where U-turns are made (docs/adr/0521 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UTurns {
    Allowed,
    DeadEnds,
    Never,
}

/// The definition's turns (docs/adr/0521 §2): the U-turn policy and the delays of a turn at a crossing of three or
/// more pieces by its angle, minutes: straight on, right, left, and a U-turn's wherever it is made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurnRule {
    pub u_turns: UTurns,
    pub straight: f64,
    pub right: f64,
    pub left: f64,
    pub back: f64,
}

/// A period of a speed cost's day (docs/adr/0521 §4): minutes of the day from and to, and the factor of the edges'
/// speeds (none: each edge's own field gives its speed).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeriodRule {
    pub from: f64,
    pub to: f64,
    pub factor: Option<f64>,
}

/// What the analyses read of a network's definition.
#[derive(Clone, Debug, PartialEq)]
pub struct Rules {
    pub connect: Connect,
    /// Metres.
    pub tolerance: f64,
    pub direction: DirectionRule,
    /// The costs besides the length, in the definition's order (cost 1, 2 …; cost 0 is the length).
    pub costs: Vec<CostRule>,
    /// Each cost's time profile besides the length's (none for most): `periods[c - 1]`.
    pub periods: Vec<Vec<PeriodRule>>,
    /// The turns; none: every turn free and the search by nodes (docs/adr/0209 §4).
    pub turns: Option<TurnRule>,
    /// The level fields: none when the definition has none; `Some(true)` when it names an inside field.
    pub levels: Option<bool>,
}

/// A text as the rules compare it: trimmed, upper and lower case the same in
/// Turkish (I is ı's, İ is i's) — `kentos_contracts::network_caseless`.
pub fn caseless(s: &str) -> String {
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

/// The number an attribute's text is by `kentos.statistics/1` (a point or a comma), as a double.
pub fn number(text: &str) -> Option<f64> {
    read_number(text)
        .map(|d| d.to_f64())
        .filter(|x| x.is_finite())
}

fn texts(v: &Json, what: &str) -> Result<Vec<String>, String> {
    match v {
        Json::Null => Ok(Vec::new()),
        Json::Arr(list) => list
            .iter()
            .map(|x| match x {
                Json::Str(s) => Ok(caseless(s)),
                _ => Err(format!("ağın {what} değerleri metin olmalı")),
            })
            .collect(),
        _ => Err(format!("ağın {what} değerleri liste olmalı")),
    }
}

impl Rules {
    /// The rules of a definition in the contract's JSON form (`NetworkDef`); its other fields are not read.
    pub fn from_json(def: &Json) -> Result<Rules, String> {
        let connect = match def.get("connect") {
            Json::Str(s) if s == "ends" => Connect::Ends,
            Json::Str(s) if s == "vertices" => Connect::Vertices,
            _ => return Err("ağın bağlanma kuralı ends ya da vertices olmalı".into()),
        };
        let tolerance = match def.get("tolerance") {
            Json::Num(t) if t.is_finite() && *t > 0.0 => *t,
            _ => return Err("ağın toleransı sıfırdan büyük bir sayı olmalı".into()),
        };
        let d = def.get("direction");
        let direction = match d.get("kind") {
            Json::Str(k) if k == "both" => DirectionRule::Both,
            Json::Str(k) if k == "digitized" => DirectionRule::Digitized,
            Json::Str(k) if k == "field" => DirectionRule::Field {
                forward: texts(d.get("forward"), "ileri")?,
                backward: texts(d.get("backward"), "geri")?,
                closed: texts(d.get("closed"), "kapalı")?,
            },
            _ => return Err("ağın yönü both, digitized ya da field olmalı".into()),
        };
        let costs = match def.get("costs") {
            Json::Null => Vec::new(),
            Json::Arr(list) => list
                .iter()
                .map(|c| match (c.get("kind"), c.get("speed")) {
                    (Json::Str(k), Json::Num(s)) if k == "speed" && s.is_finite() && *s > 0.0 => {
                        Ok(CostRule::Speed { kmh: *s })
                    }
                    (Json::Str(k), _) if k == "field" => Ok(CostRule::Field),
                    _ => Err("ağın maliyeti speed (hızıyla) ya da field olmalı".to_string()),
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err("ağın maliyetleri liste olmalı".into()),
        };
        let mut periods = Vec::with_capacity(costs.len());
        for (k, rule) in costs.iter().enumerate() {
            let list = match def.get("costs") {
                Json::Arr(l) => l[k].get("periods"),
                _ => &Json::Null,
            };
            let read = match list {
                Json::Null => Vec::new(),
                Json::Arr(ps) if matches!(rule, CostRule::Speed { .. }) => ps
                    .iter()
                    .map(|p| match (p.get("from"), p.get("to"), p.get("factor")) {
                        (Json::Num(f), Json::Num(t), factor) if f.is_finite() && t > f => {
                            Ok(PeriodRule {
                                from: *f,
                                to: *t,
                                factor: match factor {
                                    Json::Num(x) if x.is_finite() && *x >= 0.0 => Some(*x),
                                    _ => None,
                                },
                            })
                        }
                        _ => Err("ağın zaman profilinin dönemi from < to dakika olmalı".to_string()),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err("ağın zaman profili yalnız hızla maliyette bir liste olur".into()),
            };
            periods.push(read);
        }
        let turns = match def.get("turns") {
            Json::Null => None,
            t => {
                let u_turns = match t.get("uTurns") {
                    Json::Str(s) if s == "allowed" => UTurns::Allowed,
                    Json::Str(s) if s == "deadEnds" => UTurns::DeadEnds,
                    Json::Str(s) if s == "never" => UTurns::Never,
                    _ => return Err("ağın geri dönüşü allowed, deadEnds ya da never olmalı".into()),
                };
                // Seconds in the definition, minutes in the costs.
                let delay = |key: &str| match t.get(key) {
                    Json::Num(x) if x.is_finite() && *x >= 0.0 => Ok(*x / 60.0),
                    Json::Null => Ok(0.0),
                    _ => Err(format!("ağın {key} gecikmesi sıfır ya da artı saniye olmalı")),
                };
                Some(TurnRule {
                    u_turns,
                    straight: delay("straight")?,
                    right: delay("right")?,
                    left: delay("left")?,
                    back: delay("back")?,
                })
            }
        };
        let levels = match def.get("levels") {
            Json::Null => None,
            l => Some(!matches!(l.get("inside"), Json::Null)),
        };
        Ok(Rules {
            connect,
            tolerance,
            direction,
            costs,
            periods,
            turns,
            levels,
        })
    }

    /// Whether cost `c` is a time (a speed cost, minutes): turn delays are added to it.
    pub fn is_time(&self, c: usize) -> bool {
        c >= 1 && matches!(self.costs.get(c - 1), Some(CostRule::Speed { .. }))
    }

    /// The speed (km/h) an edge goes at with speed cost `c` by its field's value, and whether the value was not read;
    /// none for another cost.
    pub fn speed_of(&self, c: usize, value: Option<&str>) -> Option<(f64, bool)> {
        match self.costs.get(c.checked_sub(1)?)? {
            CostRule::Speed { kmh } => {
                let read = value.and_then(number).filter(|s| *s > 0.0);
                Some((read.unwrap_or(*kmh), read.is_none()))
            }
            CostRule::Field => None,
        }
    }

    /// The way an edge goes by its direction field's value (none: the field is not set).
    pub fn direction_of(&self, value: Option<&str>) -> Dir {
        match &self.direction {
            DirectionRule::Both => Dir::Both,
            DirectionRule::Digitized => Dir::Forward,
            DirectionRule::Field {
                forward,
                backward,
                closed,
            } => {
                let Some(v) = value else { return Dir::Both };
                let k = caseless(v);
                if forward.contains(&k) {
                    Dir::Forward
                } else if backward.contains(&k) {
                    Dir::Backward
                } else if closed.contains(&k) {
                    Dir::Closed
                } else {
                    Dir::Both
                }
            }
        }
    }

    /// What one metre of an edge `length` long costs with cost `c` (≥ 1) by its field's value: none
    /// when the edge cannot be travelled with it; and whether the value was not read (a speed's
    /// default taken, or a field cost missing).
    pub fn rate(&self, c: usize, value: Option<&str>, length: f64) -> (Option<f64>, bool) {
        match self.costs[c - 1] {
            CostRule::Speed { kmh } => {
                let read = value.and_then(number).filter(|s| *s > 0.0);
                let speed = read.unwrap_or(kmh);
                // Minutes a metre: 60 minutes over the metres an hour.
                (Some(60.0 / (speed * 1000.0)), read.is_none())
            }
            CostRule::Field => match value.and_then(number).filter(|v| *v >= 0.0) {
                Some(v) if length > 0.0 => (Some(v / length), false),
                Some(_) => (Some(0.0), false),
                None => (None, true),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roads() -> Rules {
        Rules::from_json(
            &Json::parse(
                r#"{"connect":"ends","tolerance":0.01,
                "direction":{"kind":"field","field":"yon","forward":["FT","İleri"],"backward":["TF"],"closed":["N"]},
                "costs":[{"name":"Süre","kind":"speed","field":"hiz","speed":50},{"name":"Ücret","kind":"field","field":"u"}]}"#,
            )
            .expect("JSON"),
        )
        .expect("rules")
    }

    #[test]
    fn a_field_direction_reads_its_values_caseless_and_other_values_both_ways() {
        let r = roads();
        assert_eq!(r.direction_of(Some(" ft ")), Dir::Forward);
        assert_eq!(r.direction_of(Some("ileri")), Dir::Forward);
        assert_eq!(r.direction_of(Some("ILERI")), Dir::Both, "I is ı's");
        assert_eq!(r.direction_of(Some("tf")), Dir::Backward);
        assert_eq!(r.direction_of(Some("n")), Dir::Closed);
        assert_eq!(r.direction_of(None), Dir::Both);
        assert!(Dir::Forward.allows(true) && !Dir::Forward.allows(false));
    }

    #[test]
    fn costs_read_speeds_and_values_by_the_statistics_rule() {
        let r = roads();
        // 50 km/h: 0.0012 minutes a metre; a comma reads.
        assert_eq!(r.rate(1, Some("50"), 100.0), (Some(60.0 / 50_000.0), false));
        assert_eq!(
            r.rate(1, Some("36,5"), 100.0),
            (Some(60.0 / 36_500.0), false)
        );
        assert_eq!(
            r.rate(1, Some("hızlı"), 100.0),
            (Some(60.0 / 50_000.0), true)
        );
        assert_eq!(r.rate(1, Some("0"), 100.0), (Some(60.0 / 50_000.0), true));
        assert_eq!(r.rate(1, None, 100.0), (Some(60.0 / 50_000.0), true));
        assert_eq!(r.rate(2, Some("2,5"), 100.0), (Some(0.025), false));
        assert_eq!(r.rate(2, Some("-1"), 100.0), (None, true));
        assert_eq!(r.rate(2, None, 100.0), (None, true));
    }
}
