//! The adjustments a project keeps (plan/map-cad.md MAP-18, docs/adr/5200): a horizontal (Yatay ağ) or height (Kot
//! ağı) network's adjustment by `cad.survey.adjust`, kept under a name with its result as it came: each new point's
//! place or height and standard deviation, the degrees of freedom, the a posteriori standard deviation of unit weight
//! and the model test, and the drawing's revision it was made on. BÖHHBÜY m. 98(1) asks the adjustment and calculation
//! results to be delivered; a sheet's quality table reads them from here, never from a new adjustment.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Vec2;
use crate::review_notes::review_label_ok;

/// The most adjustments a project keeps, and the most points one keeps.
pub const MAX_KEPT_ADJUSTMENTS: usize = 1_000;
pub const MAX_KEPT_ADJUSTED_POINTS: usize = 100_000;
/// The most characters of a kept adjustment's name and of a point's name.
pub const MAX_KEPT_ADJUSTMENT_NAME: usize = 120;

/// Which network an adjustment was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum KeptAdjustmentKind {
    Horizontal,
    Height,
}

/// A new point as adjusted: its name, its place (horizontal) or its height (height), its standard deviation (m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KeptAdjustedPoint {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub h: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: f64,
}

/// An adjustment kept under its name (unique in the project): its kind, the drawing's revision it was made on, its new
/// points, the degrees of freedom, m0 and the model test (none without redundancy).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KeptAdjustment {
    pub name: String,
    pub kind: KeptAdjustmentKind,
    pub revision: String,
    pub points: Vec<KeptAdjustedPoint>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub f: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub m0: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed: Option<bool>,
}

/// What is wrong with the kept adjustments, in words; none when they read. Names unique and short labels; a revision a
/// whole number's text; points at most 100 000, their names unique and short labels, a horizontal one's a finite place
/// and no height, a height one's a finite height and no place, a standard deviation finite and not below nought; m0
/// finite and not below nought; m0 and the test only with redundancy (f above nought).
pub fn kept_adjustments_problem(list: &[KeptAdjustment]) -> Option<String> {
    if list.len() > MAX_KEPT_ADJUSTMENTS {
        return Some(format!("Proje en çok {MAX_KEPT_ADJUSTMENTS} dengeleme kaydı tutar"));
    }
    let mut names = HashSet::new();
    for a in list {
        if !review_label_ok(&a.name, MAX_KEPT_ADJUSTMENT_NAME) || !names.insert(a.name.as_str()) {
            return Some(format!("“{}” dengeleme kaydının adı geçersiz ya da iki kez yazılmış", a.name));
        }
        if a.revision.is_empty() || !a.revision.bytes().all(|b| b.is_ascii_digit()) {
            return Some(format!("“{}” dengeleme kaydının sürümü bir tamsayı yazısı değil", a.name));
        }
        if a.points.len() > MAX_KEPT_ADJUSTED_POINTS {
            return Some(format!("“{}” dengeleme kaydı en çok {MAX_KEPT_ADJUSTED_POINTS} nokta tutar", a.name));
        }
        let mut points = HashSet::new();
        for p in &a.points {
            let placed = match a.kind {
                KeptAdjustmentKind::Horizontal => p.at.is_some_and(|v| v.x.is_finite() && v.y.is_finite()) && p.h.is_none(),
                KeptAdjustmentKind::Height => p.h.is_some_and(f64::is_finite) && p.at.is_none(),
            };
            if !review_label_ok(&p.name, MAX_KEPT_ADJUSTMENT_NAME)
                || !points.insert(p.name.as_str())
                || !placed
                || !(p.sigma.is_finite() && p.sigma >= 0.0)
            {
                return Some(format!("“{}” dengeleme kaydının “{}” noktası geçersiz", a.name, p.name));
            }
        }
        if a.m0.is_some_and(|m| !(m.is_finite() && m >= 0.0)) || (a.f == 0 && (a.m0.is_some() || a.passed.is_some())) {
            return Some(format!("“{}” dengeleme kaydının m0'ı ya da testi geçersiz", a.name));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept() -> KeptAdjustment {
        KeptAdjustment {
            name: "Poligon 1".into(),
            kind: KeptAdjustmentKind::Horizontal,
            revision: "12".into(),
            points: vec![KeptAdjustedPoint { name: "P1".into(), at: Some(Vec2 { x: 1.0, y: 2.0 }), h: None, sigma: 0.004 }],
            f: 3,
            m0: Some(1.2),
            passed: Some(true),
        }
    }

    #[test]
    fn a_kept_adjustment_reads_and_what_it_may_not_hold() {
        assert_eq!(kept_adjustments_problem(&[kept()]), None);
        let broken = |f: &dyn Fn(&mut KeptAdjustment)| {
            let mut a = kept();
            f(&mut a);
            kept_adjustments_problem(&[a])
        };
        assert!(broken(&|a| a.points[0].h = Some(1.0)).is_some());
        assert!(broken(&|a| a.points[0].sigma = -1.0).is_some());
        assert!(broken(&|a| a.revision = "x".into()).is_some());
        assert!(broken(&|a| a.f = 0).is_some());
        assert!(kept_adjustments_problem(&[kept(), kept()]).is_some());
    }
}
