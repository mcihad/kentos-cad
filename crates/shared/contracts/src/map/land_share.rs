//! Arsa payı (docs/adr/8031; Kat Mülkiyeti Kanunu 2/d, 3, 12/a; rule `kentos.land/1`): the land's co-ownership shares
//! given to the independent units "in proportion to their values worked out by their location and size".
//!
//! - **A unit's value:** its total gross (PAİY 4/ğ-4: its gross and its annexes') taken to whole hundredths of a square
//!   metre, times its value factor (`landFactor`, a decimal read exactly; 1 when not given) for its location.
//! - **The whole (payda):** the program's `landTotal`; else the parcel's area in whole m² (the deeds' common form, a
//!   450 m² parcel's flats 45/450 …); else the units' values in whole m² summed.
//! - **The shares:** each unit's exact part of the whole is `T·v/V`; it gets its floor, and the parts left (fewer than
//!   the units) go one each to the largest remainders, the earlier unit in the program first on a tie. They add up to
//!   the whole exactly; no unit is left without a share (Madde 5: no land share may stay unattached).
//! - A unit without area, a whole smaller than the units or one that leaves a unit no share is refused, and says why.

use crate::program_share::{SHARE_STEPS, WEIGHT_SCALE, steps_of, weight_units};
use crate::{BuildingProgram, UnitKind};

/// The rule's name and version.
pub const LAND_RULE: &str = "kentos.land/1";

/// The land shares: the whole and each independent unit's part of it, in the program's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LandShares {
    pub total: u64,
    pub shares: Vec<(String, u64)>,
}

impl LandShares {
    /// The part of the unit of the id given, written as the deed writes it (“225/450”).
    pub fn of(&self, id: &str) -> Option<String> {
        self.shares
            .iter()
            .find(|(u, _)| u == id)
            .map(|(_, n)| format!("{n}/{}", self.total))
    }
}

/// The land shares of `program`'s independent units, each one's total gross (m²) given by `gross`, over the program's
/// whole, else `parcel` (m²) in whole square metres, else their values summed; none without units. The words when
/// they cannot be given.
pub fn land_shares(
    program: &BuildingProgram,
    gross: &dyn Fn(&str) -> f64,
    parcel: Option<f64>,
) -> Result<Option<LandShares>, String> {
    let units: Vec<&crate::ProgramUnit> = program
        .units
        .iter()
        .filter(|u| u.kind == UnitKind::Unit)
        .collect();
    if units.is_empty() {
        return Ok(None);
    }
    let mut values: Vec<i128> = Vec::with_capacity(units.len());
    for u in &units {
        let factor = match &u.land_factor {
            Some(f) => {
                weight_units(f).map_err(|why| why.replacen("Pay ağırlığı", "Değer katsayısı", 1))?
            }
            None => WEIGHT_SCALE,
        };
        let v = steps_of(gross(&u.id)) * factor;
        if v <= 0 {
            return Err(format!(
                "“{}” bölümünün alanı yok: arsa payı verilemez; önce mahallerini bölüme verin.",
                u.number
            ));
        }
        values.push(v);
    }
    let whole_m2 = |v: i128| {
        let unit = SHARE_STEPS * WEIGHT_SCALE;
        let (q, r) = (v / unit, v % unit);
        // Half to even, as the shares round their steps.
        let up = 2 * r > unit || (2 * r == unit && q % 2 == 1);
        (q + i128::from(up)).max(1)
    };
    let total: i128 = match (program.land_total, parcel) {
        (Some(t), _) => i128::from(t),
        (None, Some(p)) if p.is_finite() && p >= 0.5 => whole_m2(steps_of(p) * WEIGHT_SCALE),
        _ => values.iter().map(|&v| whole_m2(v)).sum(),
    };
    let n = units.len() as i128;
    if total < n {
        return Err(format!(
            "Arsa payı paydası {total}, bölüm sayısından ({n}) küçük: her bölüme pay düşmez; paydayı büyütün."
        ));
    }
    let sum: i128 = values.iter().sum();
    let mut part: Vec<i128> = values.iter().map(|v| total * v / sum).collect();
    let left = total - part.iter().sum::<i128>();
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| {
        (total * values[b] % sum)
            .cmp(&(total * values[a] % sum))
            .then(a.cmp(&b))
    });
    for &k in order.iter().take(left as usize) {
        part[k] += 1;
    }
    if let Some(k) = part.iter().position(|&p| p == 0) {
        return Err(format!(
            "Arsa payı paydası {total} küçük: “{}” bölümüne pay düşmüyor; paydayı büyütün.",
            units[k].number
        ));
    }
    Ok(Some(LandShares {
        total: total as u64,
        shares: units
            .iter()
            .zip(&part)
            .map(|(u, &p)| (u.id.clone(), p as u64))
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProgramUnit;

    fn program(units: &[(&str, UnitKind, Option<&str>)]) -> BuildingProgram {
        BuildingProgram {
            units: units
                .iter()
                .map(|(id, kind, factor)| ProgramUnit {
                    id: (*id).into(),
                    number: (*id).into(),
                    kind: *kind,
                    land_factor: factor.map(str::to_owned),
                    annex_of: (*kind == UnitKind::Annex).then(|| "1".to_owned()),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn the_parcel_s_area_is_shared_by_the_units_values_to_the_last_square_metre() {
        // Totals 60, 30, 30 m² over a 450 m² parcel: 225, 112.5, 112.5; the half goes to the earlier unit.
        let p = program(&[
            ("1", UnitKind::Unit, None),
            ("2", UnitKind::Unit, None),
            ("3", UnitKind::Unit, None),
        ]);
        let gross = |id: &str| match id {
            "1" => 60.0,
            _ => 30.0,
        };
        let s = land_shares(&p, &gross, Some(450.0))
            .expect("shares")
            .expect("units");
        assert_eq!(s.total, 450);
        assert_eq!(
            s.shares.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
            [225, 113, 112]
        );
        assert_eq!(s.of("1").as_deref(), Some("225/450"));
        // A value factor of 1.2 on the third: 60, 30, 36 → 214.29, 107.14, 128.57; the one left to the largest
        // remainder, the third's.
        let p = program(&[
            ("1", UnitKind::Unit, None),
            ("2", UnitKind::Unit, None),
            ("3", UnitKind::Unit, Some("1.2")),
        ]);
        let s = land_shares(&p, &gross, Some(450.0))
            .expect("shares")
            .expect("units");
        assert_eq!(
            s.shares.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
            [214, 107, 129]
        );
    }

    #[test]
    fn without_a_parcel_the_whole_is_the_values_in_square_metres_and_a_given_whole_wins() {
        let p = program(&[
            ("1", UnitKind::Unit, None),
            ("2", UnitKind::Unit, None),
            ("e", UnitKind::Annex, None),
        ]);
        let gross = |id: &str| if id == "1" { 45.4 } else { 44.6 };
        let s = land_shares(&p, &gross, None)
            .expect("shares")
            .expect("units");
        assert_eq!(
            (s.total, s.shares.len()),
            (90, 2),
            "annexes take no share of their own"
        );
        let mut q = p.clone();
        q.land_total = Some(1000);
        let s = land_shares(&q, &gross, Some(450.0))
            .expect("shares")
            .expect("units");
        assert_eq!(s.total, 1000);
        assert_eq!(s.shares.iter().map(|(_, n)| *n).sum::<u64>(), 1000);
    }

    #[test]
    fn a_unit_without_area_or_a_whole_too_small_is_refused() {
        let p = program(&[("1", UnitKind::Unit, None), ("2", UnitKind::Unit, None)]);
        let err =
            land_shares(&p, &|id| if id == "1" { 50.0 } else { 0.0 }, None).expect_err("refused");
        assert!(err.starts_with("“2” bölümünün alanı yok"), "{err}");
        let mut q = p.clone();
        q.land_total = Some(1);
        let err = land_shares(&q, &|_| 50.0, None).expect_err("refused");
        assert!(err.contains("bölüm sayısından (2) küçük"), "{err}");
        let mut q = p.clone();
        q.land_total = Some(3);
        let err =
            land_shares(&q, &|id| if id == "1" { 1000.0 } else { 1.0 }, None).expect_err("refused");
        assert!(err.contains("“2” bölümüne pay düşmüyor"), "{err}");
    }
}
