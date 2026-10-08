//! The atlas plan (design §7a): from the coverage layer's objects the host
//! gives, the pages in order — filtered, sorted, named (a name that repeats
//! gets “ (2)”, “ (3)”), each atlas map's view: the object's centre and the
//! scale its policy gives, the largest standard scale it fits by default.
//! Each page is what `RenderInputs.atlas` takes to draw it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::display::{AtlasFeature, AtlasMapView, AtlasPageInput, map};
use crate::error::{Result, SheetError};
use crate::expr::{self, Eval, Scope};
use crate::kinds::*;
use crate::model::*;
use crate::paper;
use crate::units::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AtlasPlan {
    pub pages: Vec<AtlasPageInput>,
    /// Turkish: objects left out, scales that did not fit.
    pub warnings: Vec<String>,
}

fn scope_of(book: &SheetBook, sheet: &Sheet, f: &AtlasFeature, index: u32) -> Scope {
    let mut s = Scope {
        sheet: sheet.variables.clone(),
        project: book.variables.clone(),
        ..Scope::default()
    };
    s.fields = f
        .attributes
        .iter()
        .map(|a| (a.name.clone(), a.value.clone()))
        .collect();
    s.atlas = s.fields.clone();
    s.builtin("atlas_kimlik", VarValue::Text(f.id.clone()));
    s.builtin("atlas_sayfa", VarValue::Number(f64::from(index)));
    s.builtin("pafta_adi", VarValue::Text(sheet.name.clone()));
    s
}

fn truthy(e: &Eval) -> bool {
    match e {
        Eval::Value(VarValue::Bool(b)) => *b,
        Eval::Value(VarValue::Number(x)) => *x != 0.0,
        Eval::Value(VarValue::Text(t)) => !t.is_empty(),
        _ => false,
    }
}

/// The denominator a map's frame needs to show `w` × `h` metres of ground.
fn needed(w: f64, h: f64, frame: &RectUm) -> f64 {
    let fw = f64::from(frame.width.max(1)) / 1_000_000.0;
    let fh = f64::from(frame.height.max(1)) / 1_000_000.0;
    (w / fw).max(h / fh)
}

/// The scale a fixed map's `frame` needs to show `w` × `h` metres of ground
/// (the drawing area's view) with its content turned `rotation` in it
/// (Görünüme sığdır, docs/adr/0206 §1): the view's box turned grows to
/// `w·|cos θ| + h·|sin θ|` by `w·|sin θ| + h·|cos θ|`; the largest standard
/// scale that holds it (the smallest standard denominator at least the
/// one needed), else the smallest whole one; none for no ground.
pub fn fit_view_scale(w: f64, h: f64, rotation: Mdeg, frame: &RectUm) -> Option<u32> {
    if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
        return None;
    }
    let t = f64::from(rotation) / 1000.0 * core::f64::consts::PI / 180.0;
    let (s, c) = (libm::fabs(libm::sin(t)), libm::fabs(libm::cos(t)));
    let need = needed(w * c + h * s, w * s + h * c, frame);
    Some(
        paper::standard_scales()
            .iter()
            .copied()
            .find(|s| f64::from(*s) >= need)
            .unwrap_or_else(|| libm::ceil(need).clamp(1.0, 100_000_000.0) as u32),
    )
}

/// The scale a policy picks for an object `w` × `h` metres in a frame.
fn pick(
    policy: &AtlasScale,
    w: f64,
    h: f64,
    frame: &RectUm,
    warnings: &mut Vec<String>,
    id: &str,
) -> u32 {
    match policy {
        AtlasScale::Fixed(f) => f.scale.max(1),
        AtlasScale::Fit(f) => {
            let m = 1.0 + 2.0 * f64::from(f.margin_pct) / 100.0;
            let need = needed(w * m, h * m, frame);
            if f.standard_scales {
                match paper::standard_scales()
                    .iter()
                    .copied()
                    .find(|s| f64::from(*s) >= need)
                {
                    Some(s) => s,
                    None => {
                        warnings.push(format!(
                            "“{id}” nesnesi standart ölçeklerin hiçbirine sığmıyor; 1/{} kullanıldı.",
                            libm::ceil(need) as u64
                        ));
                        libm::ceil(need).clamp(1.0, 100_000_000.0) as u32
                    }
                }
            } else {
                libm::ceil(need).clamp(1.0, 100_000_000.0) as u32
            }
        }
        AtlasScale::Predefined(p) => {
            let mut scales = p.scales.clone();
            scales.sort_unstable();
            let need = needed(w, h, frame);
            match scales.iter().copied().find(|s| f64::from(*s) >= need) {
                Some(s) => s,
                None => {
                    let last = scales.last().copied().unwrap_or(1000);
                    warnings.push(format!(
                        "“{id}” nesnesi önceden tanımlı ölçeklere sığmıyor; 1/{last} kullanıldı."
                    ));
                    last
                }
            }
        }
    }
}

/// The atlas pages of a sheet for the coverage layer's objects.
pub fn atlas_plan(
    book: &SheetBook,
    sheet_id: &str,
    features: &[AtlasFeature],
) -> Result<AtlasPlan> {
    let sheet = book.sheet(sheet_id).ok_or_else(|| {
        SheetError::new(
            "unknown_sheet",
            format!("“{sheet_id}” paftası kitapta yok."),
        )
    })?;
    let atlas = sheet.atlas.as_ref().ok_or_else(|| {
        SheetError::new(
            "no_atlas",
            "Bu paftanın atlası yok: Pafta → Atlas'tan açın.",
        )
    })?;
    let mut warnings = Vec::new();
    // Filter.
    let mut kept: Vec<&AtlasFeature> = Vec::new();
    for (i, f) in features.iter().enumerate() {
        if atlas.filter.trim().is_empty() {
            kept.push(f);
            continue;
        }
        let e = expr::evaluate(&atlas.filter, &scope_of(book, sheet, f, i as u32 + 1));
        if let Eval::Error(msg) = &e {
            return Err(SheetError::new(
                "expression_error",
                format!("Atlas süzgeci çalışmıyor: {msg}"),
            ));
        }
        if truthy(&e) {
            kept.push(f);
        }
    }
    // Sort (a stable sort: equal keys keep the layer's order).
    if !atlas.sort.is_empty() {
        let keys: Vec<Vec<VarValue>> = kept
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let s = scope_of(book, sheet, f, i as u32 + 1);
                atlas
                    .sort
                    .iter()
                    .map(|k| match expr::evaluate(&k.expression, &s) {
                        Eval::Value(v) => v,
                        _ => VarValue::Null,
                    })
                    .collect()
            })
            .collect();
        let mut order: Vec<usize> = (0..kept.len()).collect();
        order.sort_by(|&a, &b| {
            for (i, k) in atlas.sort.iter().enumerate() {
                let o = compare(&keys[a][i], &keys[b][i]);
                let o = if k.descending { o.reverse() } else { o };
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            a.cmp(&b)
        });
        kept = order.into_iter().map(|i| kept[i]).collect();
    }
    let count = kept.len() as u32;
    let mut names: Vec<String> = Vec::new();
    let mut pages = Vec::with_capacity(kept.len());
    for (i, f) in kept.into_iter().enumerate() {
        let index = i as u32 + 1;
        let scope = scope_of(book, sheet, f, index);
        let base = match expr::evaluate_text(&atlas.page_name, &scope) {
            Eval::Value(v) => expr::value_text(&v),
            Eval::Missing(n) => expr::missing_mark(&n),
            _ => f.id.clone(),
        };
        let mut name = base.clone();
        let mut k = 2;
        while names.contains(&name) {
            name = format!("{base} ({k})");
            k += 1;
        }
        names.push(name.clone());
        let mut maps = Vec::new();
        for it in &sheet.items {
            let ItemKind::Map(m) = &it.kind else { continue };
            let MapView::Atlas(view) = &m.view else {
                continue;
            };
            let frame = map::content_rect(it, m);
            // The object's box as the map turns it.
            let b = f.bbox;
            let c = [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0];
            let corners = [[b[0], b[1]], [b[2], b[1]], [b[2], b[3]], [b[0], b[3]]];
            let (s, k_) = sin_cos(view.rotation);
            let (mut w, mut h) = (0.0f64, 0.0f64);
            for p in corners {
                let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
                // Turned onto the paper's axes: across and up.
                let x = dx * k_ + dy * s;
                let y = -dx * s + dy * k_;
                w = w.max(2.0 * x.abs());
                h = h.max(2.0 * y.abs());
            }
            let scale = pick(&view.policy, w, h, &frame, &mut warnings, &f.id);
            maps.push(AtlasMapView {
                item: it.id.clone(),
                center: GroundPoint { x: c[0], y: c[1] },
                scale,
                rotation: view.rotation,
            });
        }
        pages.push(AtlasPageInput {
            feature: f.clone(),
            index,
            count,
            name,
            maps,
        });
    }
    if features.len() as u32 > count {
        warnings.push(format!(
            "Süzgeç {} nesneyi dışarıda bıraktı.",
            features.len() as u32 - count
        ));
    }
    Ok(AtlasPlan { pages, warnings })
}

fn compare(a: &VarValue, b: &VarValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (VarValue::Null, VarValue::Null) => Ordering::Equal,
        (VarValue::Null, _) => Ordering::Greater,
        (_, VarValue::Null) => Ordering::Less,
        (VarValue::Number(x), VarValue::Number(y)) => x.total_cmp(y),
        _ => kentos_expression::js::collate::compare_tr(&expr::value_text(a), &expr::value_text(b)),
    }
}
