//! A general NURBS curve (docs/adr/0259): its degree, control points,
//! weights and knots as the source, exactly as given; what AutoCAD keeps
//! with it (whether it is closed or periodic, the fit points it was drawn
//! through, their end tangents and tolerance) kept beside them. KentOS's own
//! Catmull–Rom curve (`spline`) is another kind and stays as it is.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{EntityBase, Vec2};

/// The highest degree a NURBS object may have: AutoCAD writes at most 11.
pub const MAX_NURBS_DEGREE: u32 = 11;
/// The most control points one object may have.
pub const MAX_NURBS_POINTS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NurbsEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    /// The polynomial degree, 1 to 11.
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 11)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub degree: u32,
    /// The control points, at least `degree + 1`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ctrl: Vec<Vec2>,
    /// The knot vector: `ctrl.len() + degree + 1` numbers, never decreasing;
    /// the curve runs over `knots[degree] .. knots[ctrl.len()]`, which is not empty.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub knots: Vec<f64>,
    /// Each control point's weight, above 0; absent: all 1 (not rational).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub weights: Option<Vec<f64>>,
    /// Its ends meet (AutoCAD's flag 1); the source's word, kept as written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub closed: bool,
    /// Periodic (AutoCAD's flag 2): kept as written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub periodic: bool,
    /// The points it was drawn through, kept with it; the control points are the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub fit: Option<Vec<Vec2>>,
    /// The fit points' start and end tangent directions; null where none was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub tangents: Option<[Option<Vec2>; 2]>,
    /// How far the curve may pass from its fit points, ≥ 0 (AutoCAD's 44).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub fit_tolerance: Option<f64>,
}

/// What is wrong with a NURBS curve's numbers, said so a reader can fix it; none: it is a curve.
pub fn nurbs_problem(
    degree: u32,
    ctrl: &[Vec2],
    knots: &[f64],
    weights: Option<&[f64]>,
    fit: Option<&[Vec2]>,
    fit_tolerance: Option<f64>,
) -> Option<String> {
    if !(1..=MAX_NURBS_DEGREE).contains(&degree) {
        return Some(format!(
            "Derece {degree}; 1 ile {MAX_NURBS_DEGREE} arasında olmalı."
        ));
    }
    let p = degree as usize;
    if ctrl.len() < p + 1 {
        return Some(format!(
            "{p}. dereceden eğrinin en az {} denetim noktası olur; {} verildi.",
            p + 1,
            ctrl.len()
        ));
    }
    if ctrl.len() > MAX_NURBS_POINTS {
        return Some(format!(
            "Denetim noktası en çok {MAX_NURBS_POINTS}; {} verildi.",
            ctrl.len()
        ));
    }
    if ctrl.iter().any(|q| !q.x.is_finite() || !q.y.is_finite()) {
        return Some("Denetim noktalarının koordinatları sonlu sayı olmalı.".into());
    }
    let want = ctrl.len() + p + 1;
    if knots.len() != want {
        return Some(format!(
            "Düğüm sayısı denetim noktası sayısı + derece + 1 = {want} olmalı; {} verildi.",
            knots.len()
        ));
    }
    if knots.iter().any(|k| !k.is_finite()) {
        return Some("Düğümler sonlu sayı olmalı.".into());
    }
    if knots.windows(2).any(|w| w[1] < w[0]) {
        return Some("Düğümler azalmamalı: her düğüm bir öncekinden küçük olmaz.".into());
    }
    if knots[ctrl.len()] <= knots[p] {
        return Some(
            "Eğrinin aralığı boş: düğümlerin ortası (derece. ile denetim noktası sayısı. arası) artmalı.".into(),
        );
    }
    // A knot repeated more than the degree breaks the curve in two.
    let mut run = 1;
    for w in knots[1..knots.len() - 1].windows(2) {
        run = if w[1] == w[0] { run + 1 } else { 1 };
        if run > p {
            return Some(format!(
                "İç düğüm {} kez yinelenmiş; en çok derece ({p}) kez yinelenir.",
                run
            ));
        }
    }
    if let Some(w) = weights {
        if w.len() != ctrl.len() {
            return Some(format!(
                "Ağırlık sayısı ({}) denetim noktası sayısından ({}) farklı.",
                w.len(),
                ctrl.len()
            ));
        }
        if w.iter().any(|x| !x.is_finite() || *x <= 0.0) {
            return Some("Ağırlıklar sonlu ve sıfırdan büyük olmalı.".into());
        }
    }
    if let Some(f) = fit
        && f.iter().any(|q| !q.x.is_finite() || !q.y.is_finite())
    {
        return Some("Geçiş noktalarının koordinatları sonlu sayı olmalı.".into());
    }
    if let Some(t) = fit_tolerance
        && (!t.is_finite() || t < 0.0)
    {
        return Some("Geçiş toleransı sıfır ya da büyük bir sayı olmalı.".into());
    }
    None
}

impl NurbsEntity {
    /// What is wrong with it ([`nurbs_problem`]); none: it is a curve.
    pub fn problem(&self) -> Option<String> {
        if let Some([a, b]) = &self.tangents
            && [a, b]
                .into_iter()
                .flatten()
                .any(|t| !t.x.is_finite() || !t.y.is_finite())
        {
            return Some("Teğetler sonlu sayı olmalı.".into());
        }
        nurbs_problem(
            self.degree,
            &self.ctrl,
            &self.knots,
            self.weights.as_deref(),
            self.fit.as_deref(),
            self.fit_tolerance,
        )
    }
}

/// The clamped knots of a curve of `degree` over `n` control points with the
/// spans equal (0, …, 0, 1, …, k, …, k): what a curve drawn by its control
/// points gets (AutoCAD's SPLINE by control vertices, uniform knots).
pub fn clamped_uniform_knots(degree: u32, n: usize) -> Vec<f64> {
    let p = degree as usize;
    let spans = n.saturating_sub(p);
    let mut knots = Vec::with_capacity(n + p + 1);
    knots.extend(std::iter::repeat_n(0.0, p + 1));
    knots.extend((1..spans).map(|i| i as f64));
    knots.extend(std::iter::repeat_n(spans as f64, p + 1));
    knots
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    #[test]
    fn clamped_knots_have_the_right_count_and_ends() {
        let k = clamped_uniform_knots(3, 6);
        assert_eq!(k, vec![0., 0., 0., 0., 1., 2., 3., 3., 3., 3.]);
        let k = clamped_uniform_knots(2, 3);
        assert_eq!(k, vec![0., 0., 0., 1., 1., 1.]);
    }

    #[test]
    fn problems_are_found() {
        let c = [v(0., 0.), v(1., 1.), v(2., 0.)];
        let k = clamped_uniform_knots(2, 3);
        assert_eq!(nurbs_problem(2, &c, &k, None, None, None), None);
        assert!(nurbs_problem(0, &c, &k, None, None, None).is_some());
        assert!(nurbs_problem(12, &c, &k, None, None, None).is_some());
        assert!(
            nurbs_problem(3, &c, &k, None, None, None)
                .unwrap()
                .contains("en az 4")
        );
        assert!(
            nurbs_problem(2, &c, &k[..5], None, None, None)
                .unwrap()
                .contains("Düğüm sayısı")
        );
        let down = [0., 0., 0., 1., 0.5, 1.];
        assert!(
            nurbs_problem(2, &c, &down, None, None, None)
                .unwrap()
                .contains("azalmamalı")
        );
        let empty = [0., 0., 0., 0., 0., 0.];
        assert!(
            nurbs_problem(2, &c, &empty, None, None, None)
                .unwrap()
                .contains("boş")
        );
        assert!(
            nurbs_problem(2, &c, &k, Some(&[1., 0., 1.]), None, None)
                .unwrap()
                .contains("Ağırlıklar")
        );
        assert!(
            nurbs_problem(2, &c, &k, Some(&[1., 1.]), None, None)
                .unwrap()
                .contains("Ağırlık sayısı")
        );
        assert!(
            nurbs_problem(2, &c, &k, None, None, Some(-1.))
                .unwrap()
                .contains("tolerans")
        );
        let c4 = [v(0., 0.), v(1., 1.), v(2., 0.), v(3., 1.), v(4., 0.)];
        let triple = [0., 0., 0., 0.5, 0.5, 0.5, 1., 1.];
        assert!(
            nurbs_problem(2, &c4, &triple, None, None, None)
                .unwrap()
                .contains("yinelenmiş")
        );
    }
}
