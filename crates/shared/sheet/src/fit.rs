//! One fit-to-objects calculation for the designer, Python and AI (ADR 0592).
use crate::{
    GroundPoint, Item, ItemKind, MapView, Result, SheetError,
    ops::{Op, SetItemProps},
    units::sin_cos,
};

/// A ground rectangle in canonical metres. The margin is a percentage of its size on each side.
/// Uses the map's content (padding and label band excluded), preserves rotation, and never edits the source.
pub fn map(item: &Item, bounds: [f64; 4], margin_pct: u8, standard_scales: bool) -> Result<Op> {
    if bounds.iter().any(|x| !x.is_finite()) || bounds[2] < bounds[0] || bounds[3] < bounds[1] {
        return Err(SheetError::at(
            "invalid_extent",
            "bounds",
            "Kapsam sonlu ve sıralı dört koordinat olmalı.",
        ));
    }
    if margin_pct > 100 {
        return Err(SheetError::at(
            "out_of_range",
            "marginPct",
            "Kenar payı %0–100 arasında olmalı.",
        ));
    }
    if item.locked {
        return Err(SheetError::new(
            "item_locked",
            "Kilitli görünüm penceresi değiştirilemez.",
        ));
    }
    let ItemKind::Map(m) = &item.kind else {
        return Err(SheetError::new(
            "not_fixed_map",
            "Öğe sabit bir görünüm penceresi olmalı.",
        ));
    };
    if m.view_locked {
        return Err(SheetError::new(
            "item_locked",
            "Görünüm kilitli; önce görünüm kilidini açın.",
        ));
    }
    let MapView::Fixed(view) = &m.view else {
        return Err(SheetError::new(
            "not_fixed_map",
            "Atlas görünümü nesneye sığdırılamaz; sabit görünüm seçin.",
        ));
    };
    let frame = crate::display::map_content_rect(item, m);
    let w = bounds[2] - bounds[0];
    let h = bounds[3] - bounds[1];
    let (s, c) = sin_cos(view.rotation);
    let factor = 1. + 2. * f64::from(margin_pct) / 100.;
    let need = ((w * c.abs() + h * s.abs()) * 1_000_000. / f64::from(frame.width))
        .max((w * s.abs() + h * c.abs()) * 1_000_000. / f64::from(frame.height))
        * factor;
    if need > 100_000_000. {
        return Err(SheetError::at(
            "out_of_range",
            "bounds",
            "Kapsam bu çerçeveye desteklenen ölçekte sığmıyor.",
        ));
    }
    let mut scale = if w == 0. && h == 0. {
        view.scale
    } else {
        libm::ceil(need).max(1.) as u32
    };
    if standard_scales && (w > 0. || h > 0.) {
        scale = crate::paper::standard_scales()
            .iter()
            .copied()
            .find(|s| *s >= scale)
            .unwrap_or(scale);
    }
    let center = GroundPoint {
        x: bounds[0] + w / 2.,
        y: bounds[1] + h / 2.,
    };
    let mut patch = serde_json::json!({"kind":{"view":{"center":center,"scale":scale}}});
    if view.ratio.is_some() && (w != 0. || h != 0.) {
        patch["kind"]["view"]["ratio"] = serde_json::Value::Null;
    }
    if !m.source.is_drawing() {
        patch["kind"]["source"] = serde_json::json!({"type":"drawing"});
    }
    Ok(Op::SetItemProps(SetItemProps {
        id: item.id.clone(),
        patch,
    }))
}
