//! Detay paftaları (docs/adr/1090 §2): general 2D detail sheets from one template, each detail a locked model viewport
//! of a model extent at an exact paper:model ratio, with its own layer state, viewport overrides and annotation
//! representation, and a title text that reads the viewport's own scale. No architectural level is needed. Details
//! are packed into the template's first fixed map frame in their order, row by row, and run on to further sheets of
//! the set when a sheet is full. The same operations come from `cad.sheets.details`, Python and MCP.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{
    GroundPoint, Item, ItemKind, MapView, Result, SheetError, create,
    kinds::{LayerOverride, MapLayers},
    model::{Owner, SheetBook},
    ops::{AddItems, ItemIds, Op},
    template::{AssetWithBytes, InstanceIds, InstanceOptions, Template},
    units::RectUm,
    viewport::{AnnotationScale, ScaleRatio},
};

/// The first sheet number when none is given.
pub const FIRST_NUMBER: &str = "D-501";
/// The sheets' title when none is given.
pub const TITLE: &str = "Detaylar";
/// The hierarchy the sheets' drawing information puts them in.
pub const SUBSET: [&str; 1] = ["Detaylar"];
/// Between two details, paper µm (both directions).
pub const GAP_UM: i32 = 10_000;
/// Under a viewport: the space to its title and the title's own height, paper µm.
pub const TITLE_GAP_UM: i32 = 1_500;
pub const TITLE_HEIGHT_UM: i32 = 5_000;
/// The title's letters, paper µm.
pub const TITLE_SIZE_UM: i32 = 2_500;
/// A detail's cell is at least this wide, so its title reads (paper µm).
pub const MIN_CELL_UM: i32 = 40_000;
/// A viewport is at least this big on the paper in both directions (paper µm).
pub const MIN_VIEWPORT_UM: i32 = 1_000;
/// At most this many details in one call.
pub const MAX_DETAILS: usize = 200;

fn yes() -> bool {
    true
}

/// One detail: the model extent it shows, at an exact ratio, with its layer and annotation representation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperSheetDetail"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SheetDetail {
    /// Its title (“Ankraj detayı”); the sheet writes “<name> — Ölçek 1/N” under it, N read from the viewport.
    pub name: String,
    /// The model extent, canonical metres: min east, min north, max east, max north.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub bounds: [f64; 4],
    /// Paper:model, both in the same unit (1:5 is paper 1, model 5; 2:1 is paper 2, model 1).
    pub ratio: ScaleRatio,
    /// Which layers it draws; absent: the drawing's (a layer state keeps the print from following anyone's
    /// layer switching, docs/adr/1090 §5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<MapLayers>,
    /// Layers drawn otherwise in this viewport only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<LayerOverride>>"))]
    pub layer_overrides: Vec<LayerOverride>,
    /// The annotation representation; absent: the viewport's own scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub annotation: Option<AnnotationScale>,
    /// The camera locked (zooming on the screen never changes the print scale); default true.
    #[serde(default = "yes")]
    pub locked: bool,
}

/// How the sheets are made.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DetailSheetsOptions {
    /// The first sheet's number; its last digits count on; none: [`FIRST_NUMBER`].
    pub first: Option<String>,
    /// The sheets' title; none: [`TITLE`].
    pub title: Option<String>,
}

/// Where one detail went.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperDetailPlaced"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase")]
pub struct DetailPlaced {
    pub name: String,
    pub sheet: String,
    /// The viewport's item id and its title's.
    pub item: String,
    pub title: String,
    /// The viewport's frame on the paper, µm.
    pub frame: RectUm,
    /// model / paper.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub denominator: f64,
}

/// A sheet made.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperDetailSheet"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase")]
pub struct DetailSheet {
    pub sheet: String,
    pub number: String,
    pub name: String,
}

/// The operations, in order, and what they make.
#[derive(Clone, Debug)]
pub struct DetailSheets {
    pub ops: Vec<Op>,
    pub pictures: Vec<AssetWithBytes>,
    pub sheets: Vec<DetailSheet>,
    pub details: Vec<DetailPlaced>,
}

/// The paper size of a detail's viewport, µm: its extent times paper/model, rounded to the nearest µm (half away
/// from zero).
pub fn viewport_size(d: &SheetDetail) -> (f64, f64) {
    let k = 1e6 * d.ratio.paper / d.ratio.model;
    (
        ((d.bounds[2] - d.bounds[0]) * k).round(),
        ((d.bounds[3] - d.bounds[1]) * k).round(),
    )
}

/// The detail's checks, in this order: `bad_value` (an empty or long name), `invalid_extent` (bounds not finite or
/// not positive both ways), `invalid_ratio`, `bad_value` (an annotation scale out of range), `detail_too_small`.
pub fn validate_detail(d: &SheetDetail, path: &str) -> Result<()> {
    if d.name.trim().is_empty()
        || d.name.trim() != d.name
        || d.name.chars().count() > 120
        || d.name.chars().any(char::is_control)
    {
        return Err(SheetError::at(
            "bad_value",
            format!("{path}.name"),
            "Detayın adı boş olamaz, başta/sonda boşluk ya da denetim karakteri taşıyamaz; en çok 120 karakter girin.",
        ));
    }
    let b = d.bounds;
    if b.iter().any(|v| !v.is_finite()) || b[2] <= b[0] || b[3] <= b[1] {
        return Err(SheetError::at(
            "invalid_extent",
            format!("{path}.bounds"),
            "Detayın kapsamı sonlu ve iki yönde de pozitif olmalı: [en küçük doğu, en küçük kuzey, en büyük doğu, en büyük kuzey] metre.",
        ));
    }
    if !d.ratio.valid() {
        return Err(SheetError::at(
            "invalid_ratio",
            format!("{path}.ratio"),
            "Oranın iki sayısı da sonlu ve pozitif olmalı; payda (model/kâğıt) 0,000001 ile 100 000 000 arasında kalmalı.",
        ));
    }
    if let Some(AnnotationScale::Fixed { scale }) = d.annotation
        && !crate::viewport::valid_annotation_scale(scale)
    {
        return Err(SheetError::at(
            "bad_value",
            format!("{path}.annotation.scale"),
            "Açıklama temsili paydası sonlu, pozitif ve en çok 1 000 000 olmalı.",
        ));
    }
    let (w, h) = viewport_size(d);
    if w < f64::from(MIN_VIEWPORT_UM) || h < f64::from(MIN_VIEWPORT_UM) {
        return Err(SheetError::at(
            "detail_too_small",
            format!("{path}.ratio"),
            format!(
                "“{}” bu oranda kâğıtta {:.1}×{:.1} mm kalıyor; en az 1 mm olmalı. Daha büyük bir ölçek seçin.",
                d.name,
                w / 1000.,
                h / 1000.
            ),
        ));
    }
    Ok(())
}

/// The template's first fixed map item: where details go.
pub fn region_index(template: &Template) -> Option<usize> {
    template
        .sheet
        .items
        .iter()
        .position(|i| matches!(&i.kind, ItemKind::Map(m) if matches!(m.view, MapView::Fixed(_))))
}

/// The placement of `sizes` (width, height of each viewport, µm) into `region`: each detail's frame and the index of
/// the sheet it goes on (0, 1, …). A row holds details while they fit its width; a sheet holds rows while they fit its
/// height; a detail that fits no empty region is refused (`detail_does_not_fit`, at `details[i]`).
pub fn pack(region: &RectUm, sizes: &[(f64, f64)], names: &[&str]) -> Result<Vec<(usize, RectUm)>> {
    let band = TITLE_GAP_UM + TITLE_HEIGHT_UM;
    let (right, bottom) = (region.left + region.width, region.top + region.height);
    let mut out = Vec::with_capacity(sizes.len());
    let (mut sheet, mut x, mut y, mut row_h, mut in_row, mut on_sheet) =
        (0usize, region.left, region.top, 0i32, 0usize, 0usize);
    for (i, (w, h)) in sizes.iter().enumerate() {
        let (w, h) = (*w as i32, *h as i32);
        let cell_w = w.max(MIN_CELL_UM);
        let cell_h = h + band;
        if cell_w > region.width || cell_h > region.height {
            return Err(SheetError::at(
                "detail_does_not_fit",
                format!("details[{i}]"),
                format!(
                    "“{}” kâğıtta {:.1}×{:.1} mm (başlığıyla {:.1} mm); şablonun çizim alanı {:.1}×{:.1} mm. Daha küçük bir ölçek ya da daha büyük kâğıt seçin.",
                    names.get(i).copied().unwrap_or(""),
                    f64::from(w) / 1000.,
                    f64::from(h) / 1000.,
                    f64::from(cell_h) / 1000.,
                    f64::from(region.width) / 1000.,
                    f64::from(region.height) / 1000.
                ),
            ));
        }
        if in_row > 0 && x + cell_w > right {
            y += row_h + GAP_UM;
            x = region.left;
            row_h = 0;
            in_row = 0;
        }
        if on_sheet > 0 && y + cell_h > bottom {
            sheet += 1;
            x = region.left;
            y = region.top;
            row_h = 0;
            in_row = 0;
            on_sheet = 0;
        }
        out.push((sheet, RectUm::new(x, y, w, h)));
        x += cell_w + GAP_UM;
        row_h = row_h.max(cell_h);
        in_row += 1;
        on_sheet += 1;
    }
    Ok(out)
}

/// The viewport item of a detail (`id`) in `frame`, centred on its extent at its exact ratio, and its title text.
pub fn detail_items(d: &SheetDetail, id: &str, frame: RectUm) -> Result<(Item, Item)> {
    let center = GroundPoint {
        x: d.bounds[0] + (d.bounds[2] - d.bounds[0]) / 2.,
        y: d.bounds[1] + (d.bounds[3] - d.bounds[1]) / 2.,
    };
    let denominator = d.ratio.denominator();
    let mut kind = serde_json::json!({
        "type": "map",
        "view": {
            "type": "fixed",
            "center": center,
            "scale": (denominator.round().clamp(1., f64::from(u32::MAX))) as u32,
            "ratio": d.ratio,
        },
    });
    if d.locked {
        kind["viewLocked"] = true.into();
    }
    if let Some(layers) = &d.layers {
        kind["layers"] = serde_json::to_value(layers).unwrap_or_default();
    }
    if !d.layer_overrides.is_empty() {
        kind["layerOverrides"] = serde_json::to_value(&d.layer_overrides).unwrap_or_default();
    }
    if let Some(AnnotationScale::Fixed { scale }) = d.annotation {
        kind["annotationScale"] = scale.into();
    }
    let map: Item = serde_json::from_value(serde_json::json!({
        "id": id,
        "name": d.name,
        "frame": frame,
        // A detail's frame is printed: a thin line around it, in the paper's ink (Stroke's own default colour).
        "border": {"width": 250},
        "kind": kind,
    }))
    .map_err(|e| SheetError::new("bad_value", format!("Detay penceresi kurulamadı: {e}")))?;
    let title: Item = serde_json::from_value(serde_json::json!({
        "id": format!("{id}-baslik"),
        "name": format!("{} başlığı", d.name),
        "frame": RectUm::new(frame.left, frame.top + frame.height + TITLE_GAP_UM, frame.width.max(MIN_CELL_UM), TITLE_HEIGHT_UM),
        "kind": {
            "type": "text",
            "content": format!("{} — Ölçek [% @olcek %]", d.name),
            "style": {"size": TITLE_SIZE_UM, "weight": 600},
            "wrap": false,
            "map": id,
        },
    }))
    .map_err(|e| SheetError::new("bad_value", format!("Detay başlığı kurulamadı: {e}")))?;
    Ok((map, title))
}

/// The detail of a placed viewport, read back from the book (docs/adr/1090 §2): its extent from its centre and its
/// content's size at its ratio, its layers, overrides, annotation choice and lock. None for a map that is not a fixed,
/// placed model viewport of the drawing.
pub fn detail_of(item: &Item) -> Option<SheetDetail> {
    let ItemKind::Map(m) = &item.kind else {
        return None;
    };
    let MapView::Fixed(v) = &m.view else {
        return None;
    };
    if !m.source.is_drawing() || m.overview_of.is_some() || v.rotation != 0 {
        return None;
    }
    let center = v.center.as_ref()?;
    let ratio = v.ratio.unwrap_or(ScaleRatio {
        paper: 1.,
        model: f64::from(v.scale.max(1)),
    });
    let content = crate::display::map_content_rect(item, m);
    let k = ratio.model / ratio.paper / 1e6;
    let (hw, hh) = (
        f64::from(content.width) * k / 2.,
        f64::from(content.height) * k / 2.,
    );
    Some(SheetDetail {
        name: item.name.clone(),
        bounds: [center.x - hw, center.y - hh, center.x + hw, center.y + hh],
        ratio,
        layers: (m.layers != MapLayers::default()).then(|| m.layers.clone()),
        layer_overrides: m.layer_overrides.clone(),
        annotation: m
            .annotation_scale
            .map(|scale| AnnotationScale::Fixed { scale }),
        locked: m.view_locked,
    })
}

/// The sheets of `details` made from `template` with `options` (its paper and answers; the names are the set's own),
/// `ids` giving each sheet its ids from a suggested sheet id. The operations make them in one change.
pub fn detail_sheets(
    book: &SheetBook,
    template: &Template,
    options: &InstanceOptions,
    details: &[SheetDetail],
    how: &DetailSheetsOptions,
    mut ids: impl FnMut(&str) -> InstanceIds,
) -> Result<DetailSheets> {
    if details.is_empty() || details.len() > MAX_DETAILS {
        return Err(SheetError::at(
            "bad_value",
            "details",
            format!("1 ile {MAX_DETAILS} arasında detay verin."),
        ));
    }
    let mut names = BTreeSet::new();
    for (i, d) in details.iter().enumerate() {
        validate_detail(d, &format!("details[{i}]"))?;
        if !names.insert(crate::expr::fold(&d.name)) {
            return Err(SheetError::at(
                "bad_value",
                format!("details[{i}].name"),
                format!(
                    "“{}” adı iki detayda var; her detaya ayrı ad verin.",
                    d.name
                ),
            ));
        }
    }
    let at = region_index(template).ok_or_else(|| {
        SheetError::at(
            "not_fixed_map",
            "source",
            "Şablonun paftasında sabit görünüm penceresi yok; detaylar onun alanına yerleşir.",
        )
    })?;
    let title = how.title.as_deref().unwrap_or(TITLE).trim().to_owned();
    if title.is_empty() || title.chars().count() > 200 || title.chars().any(char::is_control) {
        return Err(SheetError::at(
            "bad_value",
            "title",
            "Pafta başlığı boş olamaz; en çok 200 karakter girin.",
        ));
    }
    let first = how.first.as_deref().unwrap_or(FIRST_NUMBER);
    let (before, start, width, after) = crate::levels::numbered(first).ok_or_else(|| {
        SheetError::at(
            "bad_drawing_info",
            "first",
            format!("“{first}” bir sayıyla bitmiyor; ilk pafta numarası D-501 gibi olmalı."),
        )
    })?;
    // The region on this paper: the template's map frame as the instance lays it out.
    let probe = crate::template::instantiate(template, &ids("detay-yerlesim"), options)?;
    let region = probe.sheet.items[at].content_rect();
    let sizes: Vec<(f64, f64)> = details.iter().map(viewport_size).collect();
    let labels: Vec<&str> = details.iter().map(|d| d.name.as_str()).collect();
    let placed = pack(&region, &sizes, &labels)?;
    let count = placed.last().map_or(0, |(s, _)| s + 1);
    let mut used: BTreeSet<String> = book
        .sheets
        .iter()
        .filter_map(|s| s.drawing_info.as_ref())
        .map(|i| crate::expr::fold(&i.number))
        .collect();
    let mut taken: BTreeSet<String> = book.sheets.iter().map(|s| s.id.clone()).collect();
    let mut next = start;
    let mut out = DetailSheets {
        ops: Vec::new(),
        pictures: Vec::new(),
        sheets: Vec::new(),
        details: Vec::new(),
    };
    for n in 0..count {
        let number = loop {
            let v = format!("{before}{next:0width$}{after}");
            next += 1;
            if used.insert(crate::expr::fold(&v)) {
                break v;
            }
        };
        let mut sheet = "detay-1".to_owned();
        let mut k = 1;
        while taken.contains(&sheet) {
            k += 1;
            sheet = format!("detay-{k}");
        }
        let ids = ids(&sheet);
        taken.insert(ids.sheet.clone());
        let name = if count > 1 {
            format!("{number} {title} ({}/{count})", n + 1)
        } else {
            format!("{number} {title}")
        };
        let mut own = options.clone();
        own.name = Some(name.clone());
        let made = create::from_template(template, &ids, &own, None)?;
        out.ops.extend(made.ops);
        for p in made.pictures {
            if !out.pictures.iter().any(|q| q.meta.sha256 == p.meta.sha256) {
                out.pictures.push(p);
            }
        }
        // The template's own map gives way to the details.
        if let Some(map) = ids.items.get(at) {
            out.ops.push(Op::RemoveItems(ItemIds {
                ids: vec![map.clone()],
            }));
        }
        let mut items = Vec::new();
        for (i, ((s, frame), d)) in placed.iter().zip(details).enumerate() {
            if *s != n {
                continue;
            }
            let id = format!("{}-detay-{}", ids.sheet, i + 1);
            let (map, label) = detail_items(d, &id, *frame)?;
            out.details.push(DetailPlaced {
                name: d.name.clone(),
                sheet: ids.sheet.clone(),
                item: map.id.clone(),
                title: label.id.clone(),
                frame: *frame,
                denominator: d.ratio.denominator(),
            });
            items.push(map);
            items.push(label);
        }
        out.ops.push(Op::AddItems(AddItems {
            to: Owner::sheet(&ids.sheet),
            items,
            index: None,
        }));
        out.ops
            .push(Op::SetDrawingInfo(crate::drawing_set::SetDrawingInfo {
                id: ids.sheet.clone(),
                info: Some(crate::drawing_set::DrawingInfo {
                    number: number.clone(),
                    title: title.clone(),
                    revision: String::new(),
                    status: String::new(),
                    subset: SUBSET.iter().map(|s| (*s).to_owned()).collect(),
                }),
            }));
        out.sheets.push(DetailSheet {
            sheet: ids.sheet,
            number,
            name,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(name: &str, w: f64, h: f64, model: f64) -> SheetDetail {
        SheetDetail {
            name: name.into(),
            bounds: [0., 0., w, h],
            ratio: ScaleRatio { paper: 1., model },
            layers: None,
            layer_overrides: Vec::new(),
            annotation: None,
            locked: true,
        }
    }

    #[test]
    fn a_viewport_is_its_extent_at_its_ratio() {
        assert_eq!(viewport_size(&d("a", 0.1, 0.05, 1.)), (100_000., 50_000.));
        assert_eq!(viewport_size(&d("b", 0.5, 0.25, 5.)), (100_000., 50_000.));
        assert_eq!(viewport_size(&d("c", 2., 1., 20.)), (100_000., 50_000.));
        assert_eq!(viewport_size(&d("d", 10., 6., 100.)), (100_000., 60_000.));
        let mut two = d("e", 0.01, 0.01, 1.);
        two.ratio = ScaleRatio {
            paper: 2.,
            model: 1.,
        };
        assert_eq!(viewport_size(&two), (20_000., 20_000.));
    }

    #[test]
    fn details_fill_rows_then_sheets() {
        let region = RectUm::new(30_000, 20_000, 370_000, 208_000);
        let sizes = vec![(100_000., 50_000.); 7];
        let names = vec!["x"; 7];
        let got = pack(&region, &sizes, &names).unwrap();
        // Three a row (3·100 + 2·10 = 320 ≤ 370), two rows a sheet (2·56.5 + 10 = 123 ≤ 208; a third: 189.5 ≤ 208).
        assert_eq!(got[0], (0, RectUm::new(30_000, 20_000, 100_000, 50_000)));
        assert_eq!(got[1].1.left, 140_000);
        assert_eq!(got[2].1.left, 250_000);
        assert_eq!(got[3], (0, RectUm::new(30_000, 86_500, 100_000, 50_000)));
        assert_eq!(got[6], (0, RectUm::new(30_000, 153_000, 100_000, 50_000)));
        let big = pack(&region, &[(400_000., 10_000.)], &["büyük"]).unwrap_err();
        assert_eq!(big.code, "detail_does_not_fit");
        let tall = vec![(300_000., 150_000.); 2];
        let got = pack(&region, &tall, &["a", "b"]).unwrap();
        assert_eq!(got[1], (1, RectUm::new(30_000, 20_000, 300_000, 150_000)));
    }

    #[test]
    fn a_detail_reads_back_from_its_viewport() {
        let one = d("Ankraj", 0.5, 0.25, 5.);
        let (map, title) = detail_items(&one, "v", RectUm::new(0, 0, 100_000, 50_000)).unwrap();
        let back = detail_of(&map).unwrap();
        assert_eq!(back.ratio, one.ratio);
        for (a, b) in back.bounds.iter().zip(one.bounds) {
            assert!((a - b).abs() < 1e-12, "{a} {b}");
        }
        assert!(back.locked);
        let ItemKind::Text(t) = &title.kind else {
            panic!()
        };
        assert_eq!(t.map.as_deref(), Some("v"));
        assert!(t.content.contains("@olcek"));
    }
}
