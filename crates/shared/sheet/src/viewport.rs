//! Explicit drawing, captured and live model sources with physical CAD viewport cameras.
use crate::{
    GroundPoint, Item, ItemKind, MapView, Result, SheetError,
    ops::{Op, SetItemProps},
};
pub use kentos_contracts::{NamedView as NamedModelView, view_key};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub use kentos_contracts::PaperScaleRatio as ScaleRatio;
/// CAD presets are independent of the GIS atlas fitting table. Custom ratios remain valid.
pub fn standard_ratios() -> &'static [ScaleRatio] {
    const RATIOS: &[ScaleRatio] = &[
        ScaleRatio {
            paper: 10.,
            model: 1.,
        },
        ScaleRatio {
            paper: 5.,
            model: 1.,
        },
        ScaleRatio {
            paper: 2.,
            model: 1.,
        },
        ScaleRatio {
            paper: 1.,
            model: 1.,
        },
        ScaleRatio {
            paper: 1.,
            model: 2.,
        },
        ScaleRatio {
            paper: 1.,
            model: 5.,
        },
        ScaleRatio {
            paper: 1.,
            model: 10.,
        },
        ScaleRatio {
            paper: 1.,
            model: 20.,
        },
        ScaleRatio {
            paper: 1.,
            model: 25.,
        },
        ScaleRatio {
            paper: 1.,
            model: 50.,
        },
        ScaleRatio {
            paper: 1.,
            model: 75.,
        },
        ScaleRatio {
            paper: 1.,
            model: 100.,
        },
        ScaleRatio {
            paper: 1.,
            model: 200.,
        },
        ScaleRatio {
            paper: 1.,
            model: 250.,
        },
        ScaleRatio {
            paper: 1.,
            model: 500.,
        },
        ScaleRatio {
            paper: 1.,
            model: 1000.,
        },
        ScaleRatio {
            paper: 1.,
            model: 2000.,
        },
        ScaleRatio {
            paper: 1.,
            model: 5000.,
        },
    ];
    RATIOS
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperViewportSource"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Source {
    #[default]
    Drawing,
    /// A capture, not a live link. The stable id/name and original view are retained.
    ViewSnapshot { view: kentos_contracts::NamedView },
    /// Stable named-view link. The capture identifies the link; rendering requires the current view.
    ViewLink { view: kentos_contracts::NamedView },
}
impl Source {
    pub fn captured_view(&self) -> Option<&NamedModelView> {
        match self {
            Self::Drawing => None,
            Self::ViewSnapshot { view } | Self::ViewLink { view } => Some(view),
        }
    }
    pub fn is_link(&self) -> bool {
        matches!(self, Self::ViewLink { .. })
    }
    pub fn is_drawing(&self) -> bool {
        matches!(self, Self::Drawing)
    }
}

pub use crate::clip::Clip;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperSetViewport"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetViewport {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub center: Option<GroundPoint>,
    /// Canonical metres; an unrotated model-space rectangle to fit exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="length")))]
    pub bounds: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ratio: Option<ScaleRatio>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="scale")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="mdeg")))]
    pub rotation: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub clip: Option<Clip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frame_plot: Option<bool>,
}

pub fn configure(item: &Item, input: &SetViewport) -> Result<Op> {
    if input.id != item.id {
        return Err(SheetError::new(
            "unknown_item",
            "Görünüm penceresi kimliği uyuşmuyor.",
        ));
    }
    let ItemKind::Map(map) = &item.kind else {
        return Err(SheetError::new(
            "not_fixed_map",
            "Öğe sabit görünüm penceresi olmalı.",
        ));
    };
    let MapView::Fixed(old) = &map.view else {
        return Err(SheetError::new(
            "not_fixed_map",
            "Atlas görünümü bu işlemle düzenlenemez.",
        ));
    };
    if item.locked {
        return Err(SheetError::new(
            "item_locked",
            "Öğe kilitli; önce kilidini açın.",
        ));
    }
    let camera = input.source.is_some()
        || input.center.is_some()
        || input.bounds.is_some()
        || input.ratio.is_some()
        || input.scale.is_some()
        || input.rotation.is_some();
    if map.source.is_link() && camera && input.source.is_none() {
        return Err(SheetError::new(
            "linked_view",
            "Kamera adlı görünüme bağlı. Elle değiştirmek için kaynağı Bu çizim yapın.",
        ));
    }
    if map.view_locked && input.locked != Some(false) && camera {
        return Err(SheetError::new(
            "item_locked",
            "Görünüm kilitli; önce görünüm kilidini açın.",
        ));
    }
    if input.scale.is_some() && input.ratio.is_some() {
        return Err(SheetError::new(
            "ambiguous_source",
            "Ölçek ile oran aynı işlemde verilmez.",
        ));
    }
    if input.bounds.is_some()
        && (input.center.is_some() || input.ratio.is_some() || input.scale.is_some())
    {
        return Err(SheetError::new(
            "ambiguous_source",
            "Kapsam ile merkez/ölçek aynı işlemde verilmez.",
        ));
    }
    let mut changed = map.clone();
    let mut view = old.clone();
    let frame = crate::display::map_content_rect(item, map);
    if let Some(source) = &input.source {
        if let Some(source_view) = source.captured_view() {
            if !kentos_contracts::view_holds(source_view)
                || !kentos_contracts::view_name_ok(&source_view.name)
            {
                return Err(SheetError::new("bad_value", "Adlı görünüm geçersiz."));
            }
            if input.bounds.is_some() || input.center.is_some() {
                return Err(SheetError::new(
                    "ambiguous_source",
                    "Adlı görünüm ile merkez/kapsam aynı işlemde verilmez.",
                ));
            }
            view.center = Some(GroundPoint {
                x: source_view.center.x,
                y: source_view.center.y,
            });
            view.rotation = crate::units::norm_mdeg(
                (source_view.twist * 180_000. / core::f64::consts::PI).round() as i64,
            );
            let scale = crate::projection::fitting_denominator(
                source_view.width,
                source_view.height,
                &frame,
                0,
            );
            view.ratio = Some(ScaleRatio {
                paper: 1.,
                model: scale,
            });
        }
        changed.source = source.clone();
        if let Source::ViewSnapshot { view } | Source::ViewLink { view } = &mut changed.source
            && view.id.is_empty()
        {
            view.id = kentos_contracts::view_key(view);
        }
    }
    if let Some(rotation) = input.rotation {
        view.rotation = crate::units::norm_mdeg(i64::from(rotation));
    }
    if let Some(bounds) = input.bounds {
        if bounds.iter().any(|n| !n.is_finite()) || bounds[2] <= bounds[0] || bounds[3] <= bounds[1]
        {
            return Err(SheetError::new(
                "invalid_extent",
                "Kapsam sonlu ve iki yönde de pozitif olmalı.",
            ));
        }
        view.center = Some(GroundPoint {
            x: bounds[0] + (bounds[2] - bounds[0]) / 2.,
            y: bounds[1] + (bounds[3] - bounds[1]) / 2.,
        });
        view.ratio = Some(ScaleRatio {
            paper: 1.,
            model: crate::projection::fitting_denominator(
                bounds[2] - bounds[0],
                bounds[3] - bounds[1],
                &frame,
                view.rotation,
            ),
        });
        changed.source = Source::Drawing;
    }
    if let Some(center) = input.center {
        view.center = Some(center);
    }
    if let Some(scale) = input.scale {
        view.scale = scale;
        view.ratio = None;
    }
    if let Some(ratio) = input.ratio {
        view.ratio = Some(ratio);
    }
    if view.ratio.is_some_and(|r| !r.valid()) {
        return Err(SheetError::new(
            "out_of_range",
            "Ölçek oranı sonlu ve pozitif, paydası 0,000001–100000000 arasında olmalı.",
        ));
    }
    if let Some(locked) = input.locked {
        changed.view_locked = locked;
    }
    if let Some(clip) = &input.clip {
        clip.validate()?;
        changed.clip = clip.clone();
    }
    if let Some(plot) = input.frame_plot {
        changed.frame_plot = plot;
    }
    let ratio = view.ratio;
    changed.view = MapView::Fixed(view);
    let mut patch = serde_json::json!({"kind":changed});
    patch["kind"]["source"] = serde_json::to_value(&changed.source)
        .map_err(|e| SheetError::json("Görünüm kaynağı", &e))?;
    patch["kind"]["viewLocked"] = serde_json::json!(changed.view_locked);
    if input.clip.is_some() {
        patch["kind"]["clip"] = serde_json::json!(changed.clip);
    }
    if input.frame_plot.is_some() {
        patch["kind"]["framePlot"] = serde_json::json!(changed.frame_plot);
    }
    patch["kind"]["view"]["ratio"] = serde_json::json!(ratio);
    Ok(Op::SetItemProps(SetItemProps {
        id: item.id.clone(),
        patch,
    }))
}

pub(crate) fn guard(before: &Item, after: &Item) -> Result<()> {
    if let (ItemKind::Map(a), ItemKind::Map(b)) = (&before.kind, &after.kind)
        && a.source.is_link()
        && a.source == b.source
        && a.view != b.view
    {
        return Err(SheetError::new(
            "linked_view",
            "Kamera canlı adlı görünüme bağlı; önce kaynağı ayırın.",
        ));
    }
    if let (ItemKind::Map(a), ItemKind::Map(b)) = (&before.kind, &after.kind)
        && a.view_locked
        && b.view_locked
        && (a.view != b.view || a.source != b.source)
    {
        return Err(SheetError::new(
            "item_locked",
            "Görünüm kilitli; önce görünüm kilidini açın.",
        ));
    }
    Ok(())
}

/// Item property inverses use null for a previously absent default field.
pub(crate) fn default_on_null<'de, D, T>(deserializer: D) -> core::result::Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Preserve the old display-list JSON spelling for integer denominators while allowing true fractions.
pub(crate) fn serialize_denominator<S: serde::Serializer>(
    value: &f64,
    serializer: S,
) -> core::result::Result<S::Ok, S::Error> {
    if value.is_finite() && value.fract() == 0. && value.abs() <= 9_007_199_254_740_991. {
        serializer.serialize_i64(*value as i64)
    } else {
        serializer.serialize_f64(*value)
    }
}

/// Current camera for a linked view. Missing links fail closed instead of showing an old capture.
pub fn effective_fixed_view(
    item: &Item,
    views: &[NamedModelView],
) -> Result<crate::kinds::FixedView> {
    let ItemKind::Map(map) = &item.kind else {
        return Err(SheetError::new(
            "not_fixed_map",
            "Öğe model görünümü değil.",
        ));
    };
    let MapView::Fixed(stored) = &map.view else {
        return Err(SheetError::new(
            "not_fixed_map",
            "Atlas görünümü sabit kamera değil.",
        ));
    };
    let Source::ViewLink { view: link } = &map.source else {
        return Ok(stored.clone());
    };
    let view = views
        .iter()
        .find(|v| view_key(v) == view_key(link))
        .ok_or_else(|| {
            SheetError::new(
                "missing_named_view",
                format!("{} adlı görünüm bağlantısı bulunamadı.", link.name),
            )
        })?;
    if !kentos_contracts::view_holds(view) {
        return Err(SheetError::new("bad_value", "Kaynak görünüm geçersiz."));
    }
    let frame = crate::display::map_content_rect(item, map);
    let denominator = crate::projection::fitting_denominator(view.width, view.height, &frame, 0);
    if crate::projection::Scale::new(denominator).is_none() {
        return Err(SheetError::new(
            "bad_scale",
            "Bağlı görünüm fiziksel ölçek sınırını aşıyor.",
        ));
    }
    let mut result = stored.clone();
    result.center = Some(GroundPoint {
        x: view.center.x,
        y: view.center.y,
    });
    result.rotation =
        crate::units::norm_mdeg((view.twist * 180000. / core::f64::consts::PI).round() as i64);
    result.ratio = Some(ScaleRatio {
        paper: 1.,
        model: denominator,
    });
    Ok(result)
}

/// Detaching a live source retains its current effective camera when the registry is available.
/// The operation remains explicit; replay and undo do not need a host registry.
pub fn prepare_source(
    book: &crate::SheetBook,
    input: &SetViewport,
    views: &[NamedModelView],
) -> Result<SetViewport> {
    let mut request = input.clone();
    if matches!(input.source, Some(Source::Drawing))
        && input.center.is_none()
        && input.bounds.is_none()
        && input.ratio.is_none()
        && input.scale.is_none()
        && input.rotation.is_none()
        && let Some(item) = book.item(&input.id)
        && matches!(&item.kind,ItemKind::Map(m) if m.source.is_link())
        && let Ok(view) = effective_fixed_view(item, views)
    {
        request.center = view.center;
        request.ratio = view.ratio;
        request.rotation = Some(view.rotation);
    }
    Ok(request)
}
