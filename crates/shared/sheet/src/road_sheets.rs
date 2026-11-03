//! Yol paftaları (docs/adr/0908 §10, plan/road.md Y10.5): plan–profile sheets along an alignment, as Civil 3D's view
//! frames and Create Sheets make them: one sheet per frame from one template, the template's first map showing the
//! frame's stretch in plan (centred on it, turned so the alignment runs left to right, at the set's scale) and its
//! second map, when the frame has one, the profile view drawn for the frame (unturned, at the same scale, so a station
//! lies over itself on both). Numbered on from a first sheet number; the sheets' drawing information puts them under
//! Yol › the alignment. The caller works out the frames; `road.sheets.plan` does.

use std::collections::BTreeSet;

use crate::{
    GroundPoint, ItemKind, MapView, Result, SheetError, create,
    model::SheetBook,
    ops::{Op, SetItemProps},
    template::{AssetWithBytes, InstanceIds, InstanceOptions, Template},
    units::Mdeg,
};

/// The first sheet number when none is given.
pub const FIRST_NUMBER: &str = "Y-101";

/// One sheet's frame: its title, the plan map's centre and turn, and the profile map's centre when there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct RoadFrame {
    pub title: String,
    pub center: GroundPoint,
    pub rotation: Mdeg,
    pub profile: Option<GroundPoint>,
}

/// One sheet made.
#[derive(Clone, Debug, PartialEq)]
pub struct RoadSheet {
    pub sheet: String,
    pub number: String,
}

/// The operations, in order, and what they make.
#[derive(Clone, Debug)]
pub struct RoadSheets {
    pub ops: Vec<Op>,
    pub pictures: Vec<AssetWithBytes>,
    pub made: Vec<RoadSheet>,
}

/// The maps of `template`'s sheet, in its order: the plan's first, the profile's second.
pub fn template_maps(template: &Template) -> Vec<usize> {
    template
        .sheet
        .items
        .iter()
        .enumerate()
        .filter(|(_, i)| matches!(i.kind, ItemKind::Map(_)))
        .map(|(k, _)| k)
        .collect()
}

/// The sheets of `frames` made from `template` at 1:`scale`, under `subset`, numbered from `first` (none:
/// [`FIRST_NUMBER`]), `ids` giving each sheet its ids from a suggested sheet id (`prefix-1`, `prefix-2` …).
#[allow(clippy::too_many_arguments)]
pub fn road_sheets(
    book: &SheetBook,
    template: &Template,
    options: &InstanceOptions,
    frames: &[RoadFrame],
    scale: u32,
    first: Option<&str>,
    subset: &[String],
    prefix: &str,
    mut ids: impl FnMut(&str) -> InstanceIds,
) -> Result<RoadSheets> {
    let maps = template_maps(template);
    let need = if frames.iter().any(|f| f.profile.is_some()) { 2 } else { 1 };
    if maps.len() < need {
        return Err(SheetError::at(
            "not_fixed_map",
            "source",
            if need == 2 {
                "Şablonun paftasında iki görünüm penceresi yok; plan–boykesit paftası planı birinciye, boykesiti ikinciye koyar."
            } else {
                "Şablonun paftasında görünüm penceresi yok; yol paftası haritasız yerleşmez."
            },
        ));
    }
    let first = first.unwrap_or(FIRST_NUMBER);
    let (before, start, width, after) = crate::levels::numbered(first).ok_or_else(|| {
        SheetError::at(
            "bad_drawing_info",
            "first",
            format!("“{first}” bir sayıyla bitmiyor; ilk pafta numarası Y-101 gibi olmalı."),
        )
    })?;
    let mut used: BTreeSet<String> = book
        .sheets
        .iter()
        .filter_map(|s| s.drawing_info.as_ref())
        .map(|i| crate::expr::fold(&i.number))
        .collect();
    let mut taken: BTreeSet<String> = book.sheets.iter().map(|s| s.id.clone()).collect();
    let mut next = start;
    let mut out = RoadSheets { ops: Vec::new(), pictures: Vec::new(), made: Vec::new() };
    for (k, frame) in frames.iter().enumerate() {
        let number = loop {
            let n = format!("{before}{next:0width$}{after}");
            next += 1;
            if used.insert(crate::expr::fold(&n)) {
                break n;
            }
        };
        let mut sheet = format!("{prefix}-{}", k + 1);
        let mut again = 2;
        while taken.contains(&sheet) {
            sheet = format!("{prefix}-{}-{again}", k + 1);
            again += 1;
        }
        let ids = ids(&sheet);
        taken.insert(ids.sheet.clone());
        let mut own = options.clone();
        own.name = Some(format!("{number} {}", frame.title));
        own.center = None;
        own.scale = Some(scale);
        let made = create::from_template(template, &ids, &own, None)?;
        out.ops.extend(made.ops);
        for p in made.pictures {
            if !out.pictures.iter().any(|q| q.meta.sha256 == p.meta.sha256) {
                out.pictures.push(p);
            }
        }
        let instance = crate::template::instantiate(template, &ids, &own)?;
        let views = [Some((frame.center, frame.rotation)), frame.profile.map(|c| (c, 0))];
        for (at, view) in maps.iter().zip(views) {
            let Some((center, rotation)) = view else { continue };
            let item = &instance.sheet.items[*at];
            let fixed = matches!(&item.kind, ItemKind::Map(m) if matches!(m.view, MapView::Fixed(_)));
            if !fixed {
                return Err(SheetError::at(
                    "not_fixed_map",
                    "source",
                    "Şablonun görünüm penceresi sabit görünümlü değil; yol paftası haritayı kendisi yerleştirir.",
                ));
            }
            let mut patch = serde_json::json!({"kind": {"view": {"center": center, "scale": scale, "rotation": rotation}}});
            if let ItemKind::Map(m) = &item.kind
                && let MapView::Fixed(v) = &m.view
                && v.ratio.is_some()
            {
                patch["kind"]["view"]["ratio"] = serde_json::Value::Null;
            }
            out.ops.push(Op::SetItemProps(SetItemProps { id: item.id.clone(), patch }));
        }
        out.ops.push(Op::SetDrawingInfo(crate::drawing_set::SetDrawingInfo {
            id: ids.sheet.clone(),
            info: Some(crate::drawing_set::DrawingInfo {
                number: number.clone(),
                title: frame.title.clone(),
                revision: String::new(),
                status: String::new(),
                subset: subset.to_vec(),
            }),
        }));
        out.made.push(RoadSheet { sheet: ids.sheet, number });
    }
    Ok(out)
}
