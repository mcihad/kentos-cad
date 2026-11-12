//! A building's common areas shared out to its independent units (docs/adr/0784 §2), by rule `kentos.share/1`.
//!
//! - Each common area (a group of kind `common`) is its own pool: its rooms (its voids and its outdoor spaces apart),
//!   summed in the drawing's order and taken to whole hundredths of a square metre (a step), the exact binary value
//!   rounded half to even. It is shared among the independent units of its scope: its wing's when it names a wing,
//!   else its building's, else the whole program's.
//! - A unit's weight: one each (`equal`), its own rooms taken to steps the same way (`net`), or its `shareWeight`, a
//!   decimal read exactly (`weight`).
//! - A pool of `T` steps and weights `w` summing to `W`: each unit's exact share is `T·w/W`; it gets its floor, and the
//!   steps left (fewer than the units) go one each to the units with the largest remainders, the earlier in the
//!   program first on a tie. The shares add up to `T` exactly; nothing is dropped and nothing goes to the last unit
//!   for being last.
//! - A pool without units, with a unit lacking its weight, or whose weights add up to nothing is not shared, and says
//!   why; it is never shared in part.
//! - A unit's gross: its own rooms in steps plus its shares. Amounts are written as decimals (“12.35”), never as
//!   floats; an exact share as its fraction of a square metre (“3083/240”).
//!
//! The registered (official) area a unit keeps (`recordedArea`) is neither read nor written here. The independent
//! reference is `scripts/fixtures/program_share_cases.py`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{BuildingProgram, UnitKind};

/// The rule's name and version.
pub const SHARE_RULE: &str = "kentos.share/1";
/// Steps in a square metre: amounts are whole hundredths of a square metre.
pub const SHARE_STEPS: i128 = 100;
/// A weight's scale: its six decimals.
pub const WEIGHT_SCALE: i128 = 1_000_000;

/// How the common areas are shared out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ShareRule {
    /// The same for each unit of the scope.
    #[default]
    Equal,
    /// By each unit's own rooms' area.
    Net,
    /// By each unit's `shareWeight`.
    Weight,
}

impl ShareRule {
    pub const ALL: [ShareRule; 3] = [ShareRule::Equal, ShareRule::Net, ShareRule::Weight];

    pub fn label(self) -> &'static str {
        match self {
            ShareRule::Equal => "Eşit",
            ShareRule::Net => "Net alana göre",
            ShareRule::Weight => "Ağırlığa göre",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ShareRule::Equal => "equal",
            ShareRule::Net => "net",
            ShareRule::Weight => "weight",
        }
    }
}

/// A unit's part of the common areas: its own rooms, its shares and their sum, m² as decimals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramUnitShare {
    pub net: String,
    pub share: String,
    pub gross: String,
}

/// One unit's share of one pool.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramPoolShare {
    /// The unit's id.
    pub unit: String,
    /// Its weight as read (“1”, “48.73”, “1.5”).
    pub weight: String,
    /// Its share rounded to the step, m².
    pub share: String,
    /// Its exact share, m², as a fraction in lowest terms (“3083/240”) or a whole number.
    pub exact: String,
}

/// A common area shared out.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramPool {
    /// The common area's id and number.
    pub unit: String,
    pub number: String,
    /// Its scope: its building, its wing; neither: the whole program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub building: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wing: Option<String>,
    /// Its rooms taken to the step, m².
    pub area: String,
    /// The units' shares, in the program's order; empty when it is not shared.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProgramPoolShare>>", optional))]
    pub shares: Vec<ProgramPoolShare>,
    /// Why it is not shared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unshared: Option<String>,
}

/// The common areas shared out (docs/adr/0784 §2).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramShares {
    /// The rule and its weights (`kentos.share/1 · Net alana göre`).
    pub rule: String,
    /// Each common area, in the program's order.
    pub pools: Vec<ProgramPool>,
    /// The independent units' own rooms, the common areas shared, those not shared and the units' gross, m²; the
    /// gross is the net and the shared exactly.
    pub net: String,
    pub shared: String,
    pub unshared: String,
    pub gross: String,
}

/// An area in whole steps: its exact binary value times 100, rounded half to even.
pub fn steps_of(x: f64) -> i128 {
    if !x.is_finite() || x == 0.0 {
        return 0;
    }
    let bits = x.abs().to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let frac = (bits & ((1u64 << 52) - 1)) as i128;
    let (m, e) = if exp == 0 { (frac, -1074) } else { (frac | (1i128 << 52), exp - 1075) };
    let num = m * SHARE_STEPS;
    let q = if e >= 0 {
        num << e.min(60)
    } else {
        let k = (-e) as u32;
        if k > 64 {
            0
        } else {
            let q = num >> k;
            let r = num - (q << k);
            let half = 1i128 << (k - 1);
            if r > half || (r == half && q & 1 == 1) { q + 1 } else { q }
        }
    };
    if x < 0.0 { -q } else { q }
}

/// Steps written as m² (“12.35”).
pub fn steps_text(n: i128) -> String {
    let sign = if n < 0 { "-" } else { "" };
    let a = n.unsigned_abs();
    format!("{sign}{}.{:02}", a / 100, a % 100)
}

/// A weight's text read exactly, in millionths; the words when it does not read.
pub fn weight_units(t: &str) -> Result<i128, String> {
    let bad = || {
        format!("Pay ağırlığı “{t}” okunamadı: noktadan önce en çok 12, sonra en çok 6 basamaklı bir sayı yazın (120, 1.5).")
    };
    let (whole, frac) = match t.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (t, None),
    };
    let digits = |s: &str, most: usize| !s.is_empty() && s.len() <= most && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole, 12) || frac.is_some_and(|f| !digits(f, 6)) {
        return Err(bad());
    }
    let w: i128 = whole.parse().map_err(|_| bad())?;
    let f: i128 = match frac {
        Some(f) => format!("{f:0<6}").parse().map_err(|_| bad())?,
        None => 0,
    };
    Ok(w * WEIGHT_SCALE + f)
}

/// Millionths written as the shortest decimal (“1.5”, “120”).
fn weight_text(n: i128) -> String {
    let (w, f) = (n / WEIGHT_SCALE, n % WEIGHT_SCALE);
    if f == 0 {
        return w.to_string();
    }
    let f = format!("{f:06}");
    format!("{w}.{}", f.trim_end_matches('0'))
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.abs()
}

/// `num / den` m² in lowest terms.
fn fraction(num: i128, den: i128) -> String {
    let g = gcd(num, den).max(1);
    let (n, d) = (num / g, den / g);
    if d == 1 { n.to_string() } else { format!("{n}/{d}") }
}

fn quoted(numbers: &[&str]) -> String {
    numbers.iter().map(|n| format!("“{n}”")).collect::<Vec<_>>().join(", ")
}

/// The common areas shared out by the program's rule, each group's rooms (m², summed in the drawing's order) given by
/// `rooms`; none when the program shares nothing. With the report, each independent unit's part, in the program's
/// order.
pub fn shares(
    program: &BuildingProgram,
    rooms: &dyn Fn(&str) -> f64,
) -> Option<(ProgramShares, Vec<(String, ProgramUnitShare)>)> {
    let rule = program.share_rule?;
    let units: Vec<(usize, &crate::ProgramUnit)> =
        program.units.iter().enumerate().filter(|(_, u)| u.kind == UnitKind::Unit).collect();
    let mut got: Vec<i128> = vec![0; program.units.len()];
    let mut pools = Vec::new();
    let (mut shared, mut unshared) = (0i128, 0i128);
    for c in program.units.iter().filter(|u| u.kind == UnitKind::Common) {
        let total = steps_of(rooms(&c.id));
        let scope: Vec<&(usize, &crate::ProgramUnit)> = units
            .iter()
            .filter(|(_, u)| match (&c.wing, &c.building) {
                (Some(w), _) => u.wing.as_ref() == Some(w),
                (None, Some(b)) => u.building.as_ref() == Some(b),
                (None, None) => true,
            })
            .collect();
        let mut pool = ProgramPool {
            unit: c.id.clone(),
            number: c.number.clone(),
            building: c.building.clone(),
            wing: c.wing.clone(),
            area: steps_text(total),
            shares: Vec::new(),
            unshared: None,
        };
        let refused = |why: String| format!("{why}; ortak alan paylaştırılmadı.");
        let weights: Result<Vec<i128>, String> = if scope.is_empty() {
            let place = match (&c.wing, &c.building) {
                (Some(w), Some(b)) => {
                    let name = program
                        .building(b)
                        .and_then(|b| b.wings.iter().find(|x| &x.id == w))
                        .map_or(w.as_str(), |x| x.name.as_str());
                    format!("“{name}” kanadında")
                }
                (None, Some(b)) => format!("“{}” binasında", program.building(b).map_or(b.as_str(), |x| x.name.as_str())),
                _ => "Programda".into(),
            };
            Err(refused(format!("{place} bağımsız bölüm yok")))
        } else {
            match rule {
                ShareRule::Equal => Ok(scope.iter().map(|_| 1).collect()),
                ShareRule::Net => Ok(scope.iter().map(|(_, u)| steps_of(rooms(&u.id))).collect()),
                ShareRule::Weight => {
                    let missing: Vec<&str> =
                        scope.iter().filter(|(_, u)| u.share_weight.is_none()).map(|(_, u)| u.number.as_str()).collect();
                    if missing.is_empty() {
                        Ok(scope
                            .iter()
                            .map(|(_, u)| u.share_weight.as_deref().and_then(|w| weight_units(w).ok()).unwrap_or(0))
                            .collect())
                    } else {
                        Err(refused(format!("{} bölümünün pay ağırlığı yok", quoted(&missing))))
                    }
                }
            }
        };
        let weights = weights.and_then(|ws| {
            if ws.iter().sum::<i128>() == 0 {
                Err(refused(match rule {
                    ShareRule::Net => "Kapsamdaki bölümlerin net alanı sıfır".into(),
                    _ => "Kapsamdaki bölümlerin pay ağırlıklarının toplamı sıfır".into(),
                }))
            } else {
                Ok(ws)
            }
        });
        match weights {
            Err(why) => {
                pool.unshared = Some(why);
                unshared += total;
            }
            Ok(ws) => {
                let whole: i128 = ws.iter().sum();
                let mut part: Vec<i128> = ws.iter().map(|w| total * w / whole).collect();
                let left = total - part.iter().sum::<i128>();
                let mut order: Vec<usize> = (0..ws.len()).collect();
                order.sort_by(|&a, &b| (total * ws[b] % whole).cmp(&(total * ws[a] % whole)).then(a.cmp(&b)));
                for &k in order.iter().take(left as usize) {
                    part[k] += 1;
                }
                for (k, (i, u)) in scope.iter().enumerate() {
                    got[*i] += part[k];
                    pool.shares.push(ProgramPoolShare {
                        unit: u.id.clone(),
                        weight: match rule {
                            ShareRule::Equal => "1".into(),
                            ShareRule::Net => steps_text(ws[k]),
                            ShareRule::Weight => weight_text(ws[k]),
                        },
                        share: steps_text(part[k]),
                        exact: fraction(total * ws[k], whole * SHARE_STEPS),
                    });
                }
                shared += total;
            }
        }
        pools.push(pool);
    }
    let mut net = 0i128;
    let mut parts = Vec::with_capacity(units.len());
    for (i, u) in &units {
        let own = steps_of(rooms(&u.id));
        net += own;
        parts.push((
            u.id.clone(),
            ProgramUnitShare { net: steps_text(own), share: steps_text(got[*i]), gross: steps_text(own + got[*i]) },
        ));
    }
    Some((
        ProgramShares {
            rule: format!("{SHARE_RULE} · {}", rule.label()),
            pools,
            net: steps_text(net),
            shared: steps_text(shared),
            unshared: steps_text(unshared),
            gross: steps_text(net + shared),
        },
        parts,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProgramBuilding, ProgramUnit, ProgramWing};

    #[test]
    fn steps_round_the_exact_value_half_to_even() {
        assert_eq!(steps_of(12.345), 1235, "12.345 is a little more in binary");
        assert_eq!(steps_of(2.675), 267, "2.675 is a little less");
        assert_eq!(steps_of(1.005), 100);
        assert_eq!(steps_of(0.125), 12, "an exact half goes to the even step");
        assert_eq!(steps_of(0.375), 38);
        assert_eq!(steps_of(48.0), 4800);
        assert_eq!(steps_of(1.0e-9), 0);
        assert_eq!(steps_text(4873), "48.73");
        assert_eq!(steps_text(5), "0.05");
    }

    #[test]
    fn weights_read_exactly() {
        assert_eq!(weight_units("1.5"), Ok(1_500_000));
        assert_eq!(weight_units("120"), Ok(120_000_000));
        assert_eq!(weight_text(1_500_000), "1.5");
        assert!(weight_units("1,5").is_err());
        assert!(weight_units("-1").is_err());
        assert!(weight_units("1.1234567").is_err());
        assert!(weight_units(".5").is_err());
    }

    fn unit(id: &str, number: &str, kind: UnitKind, building: Option<&str>, wing: Option<&str>) -> ProgramUnit {
        ProgramUnit {
            id: id.into(),
            number: number.into(),
            kind,
            building: building.map(Into::into),
            wing: wing.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn the_steps_left_go_to_the_largest_remainders_and_add_up() {
        let program = BuildingProgram {
            buildings: vec![ProgramBuilding {
                id: "a".into(),
                name: "A".into(),
                wings: vec![ProgramWing { id: "d".into(), name: "Doğu".into(), levels: vec![] }],
                levels: vec![],
            }],
            units: vec![
                unit("1", "1", UnitKind::Unit, Some("a"), None),
                unit("2", "2", UnitKind::Unit, Some("a"), Some("d")),
                unit("3", "3", UnitKind::Unit, Some("a"), Some("d")),
                unit("o", "O", UnitKind::Common, Some("a"), None),
                unit("k", "K", UnitKind::Common, Some("a"), Some("d")),
            ],
            share_rule: Some(ShareRule::Equal),
            ..Default::default()
        };
        let rooms = |id: &str| match id {
            "o" => 1.0,
            "k" => 0.05,
            _ => 10.0,
        };
        let (report, parts) = shares(&program, &rooms).expect("shared");
        let o = &report.pools[0];
        assert_eq!(o.shares.iter().map(|s| s.share.as_str()).collect::<Vec<_>>(), ["0.34", "0.33", "0.33"]);
        assert_eq!(o.shares[0].exact, "1/3");
        let k = &report.pools[1];
        assert_eq!(k.shares.iter().map(|s| s.share.as_str()).collect::<Vec<_>>(), ["0.03", "0.02"]);
        assert_eq!(parts[1].1.gross, "10.36");
        assert_eq!((report.shared.as_str(), report.gross.as_str()), ("1.05", "31.05"));
    }
}
