//! CAD paper-space ↔ the real sheet book. Model and paper coordinates never share a layer.
use kentos_commands::{
    CadLayout, CadViewport, Entity, ProjectSheets, ReportItem, ReportSeverity, Vec2,
};
use kentos_domain::{Document, Uuid};
use kentos_sheet::{
    kinds::{self, GroundPoint, ItemKind, MapView, ShapeKind},
    model::{Item, Sheet, SheetBook},
    style::{Stroke, TextStyle},
    units::RectUm,
};
use serde_json::json;

fn um(metres: f64) -> i32 {
    kentos_sheet::projection::paper_metres_to_um(metres)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32
}
fn color(e: &Entity) -> String {
    e.base()
        .color
        .as_ref()
        .filter(|s| s.starts_with('#'))
        .cloned()
        .unwrap_or_else(|| "#000000".into())
}
fn path(e: &Entity) -> Option<(Vec<Vec2>, bool)> {
    match e {
        Entity::Line(l) => Some((vec![l.a, l.b], false)),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            Some((p.pts.clone(), matches!(e, Entity::Polygon(_)) || p.closed))
        }
        _ => None,
    }
}

fn paper_item(e: &Entity, height_mm: f64) -> Option<Item> {
    let id = Uuid::now_v7().to_string();
    let name = format!("DXF {}", e.kind());
    let y = |value: f64| height_mm / 1000. - value;
    let stroke = Stroke::solid(
        &color(e),
        (e.base().line_weight.unwrap_or(0.25) * 1000.)
            .round()
            .max(1.) as i32,
    );
    if let Entity::Text(t) = e {
        let mut kind = kinds::default_kind("text")?;
        let ItemKind::Text(text) = &mut kind else {
            return None;
        };
        text.content = t.text.clone();
        text.style = TextStyle::new(um(t.height).max(1), if t.face.bold { 700 } else { 400 });
        text.style.color = color(e);
        text.style.font = kentos_commands::DrawingFont::id_of(t.face.font).into();
        text.style.italic = t.face.italic;
        text.wrap = t.paragraph.box_width.is_some();
        let width = t
            .paragraph
            .box_width
            .unwrap_or(t.height * (t.text.chars().count().max(1) as f64));
        let w = um(width).max(1000);
        let h = um(t.height * 1.3).max(1000);
        let baseline = kentos_sheet::text::face(&text.style)
            .map(|f| {
                kentos_sheet::text::baseline_in_line(
                    f,
                    text.style.size,
                    i64::from(text.style.size) * i64::from(text.line_height) / 100,
                )
            })
            .unwrap_or(i64::from(text.style.size)) as f64;
        let rotation = ((-t.rotation * 1000.).round() as i32).rem_euclid(360000);
        let angle = f64::from(rotation).to_radians() / 1000.;
        let (sn, cs) = (angle.sin(), angle.cos());
        let (dx, dy) = (-f64::from(w) / 2., baseline - f64::from(h) / 2.);
        // The sheet rotates about the frame centre; CAD text rotates about its baseline anchor.
        let left = (t.p.x * 1e6 - cs * dx + sn * dy - f64::from(w) / 2.).round() as i32;
        let top = (y(t.p.y) * 1e6 - sn * dx - cs * dy - f64::from(h) / 2.).round() as i32;
        let mut item = Item::new(&id, &name, RectUm::new(left, top, w, h), kind);
        item.rotation = rotation;
        return Some(item);
    }
    if crate::cad_paper::supports(e) {
        let (frame, cad) = crate::cad_paper::read(e, height_mm)?;
        return Some(Item::new(
            &id,
            &name,
            frame,
            ItemKind::Shape(kinds::ShapeItem {
                shape: ShapeKind::Cad(cad),
                stroke: Some(stroke),
                fill: None,
            }),
        ));
    }
    let (pts, closed) = path(e)?;
    if pts.len() < 2 || pts.len() > 65536 {
        return None;
    }
    let left = pts.iter().map(|p| um(p.x)).min()?;
    let top = pts.iter().map(|p| um(y(p.y))).min()?;
    // The designer needs a selectable frame of at least 1 mm on each side.
    // Pad that frame, then encode the original points inside it: stretching
    // a horizontal/vertical/short CAD line would change its physical length.
    let width = (i64::from(pts.iter().map(|p| um(p.x)).max()?) - i64::from(left))
        .max(i64::from(kentos_sheet::validate::ITEM_MIN))
        .min(i32::MAX as i64) as i32;
    let height = (i64::from(pts.iter().map(|p| um(y(p.y))).max()?) - i64::from(top))
        .max(i64::from(kentos_sheet::validate::ITEM_MIN))
        .min(i32::MAX as i64) as i32;
    let relative: Vec<_> = pts
        .iter()
        .map(|p| {
            [
                ((f64::from(um(p.x)) - f64::from(left)) * 1e6 / f64::from(width)).round() as i32,
                ((f64::from(um(y(p.y))) - f64::from(top)) * 1e6 / f64::from(height)).round() as i32,
            ]
        })
        .collect();
    let kind = if closed {
        ItemKind::Shape(kinds::ShapeItem {
            shape: ShapeKind::Polygon(kinds::PolygonShape { points: relative }),
            stroke: Some(stroke),
            fill: None,
        })
    } else {
        ItemKind::Line(kinds::LineItem {
            points: relative,
            stroke,
            start: Default::default(),
            end: Default::default(),
        })
    };
    Some(Item::new(
        &id,
        &name,
        RectUm::new(left, top, width, height),
        kind,
    ))
}

#[derive(Clone, Debug)]
pub struct Source {
    pub file: String,
    pub sha256: Option<String>,
}
pub fn merge(doc: &Document, layouts: &[CadLayout]) -> Result<Option<ProjectSheets>, String> {
    merge_with_source(doc, layouts, None)
}
pub fn merge_with_source(
    doc: &Document,
    layouts: &[CadLayout],
    source: Option<&Source>,
) -> Result<Option<ProjectSheets>, String> {
    if layouts.is_empty() {
        return Ok(None);
    }
    let mut book = match doc.sheets() {
        Some(s) => {
            kentos_sheet::project::read_project_sheets(s)
                .map_err(|e| e.message)?
                .0
        }
        None => SheetBook::default(),
    };
    for layout in layouts {
        let mut name = layout.name.clone();
        let mut n = 2;
        while book
            .sheets
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(&name))
        {
            name = format!("{} ({n})", layout.name);
            n += 1;
        }
        let mut sheet: Sheet = serde_json::from_value(json!({"id": Uuid::now_v7().to_string(), "name": name,
            "page": {"paper": "custom", "orientation": if layout.width_mm >= layout.height_mm { "landscape" } else { "portrait" },
                "size": {"width": (layout.width_mm * 1000.).round() as i32, "height": (layout.height_mm * 1000.).round() as i32},
                "margins": {"top":0,"right":0,"bottom":0,"left":0}}, "items": []})).map_err(|e| e.to_string())?;
        for (i, entity) in layout.entities.iter().enumerate() {
            if let Some(mut item) = paper_item(entity, layout.height_mm) {
                item.name = format!("DXF {} {}", entity.kind(), i + 1);
                item.cad_origin = Some(kentos_sheet::cad::Origin {
                    layout: layout.name.clone(),
                    record: i as u32,
                    entity_kind: entity.kind().into(),
                    file: source.map(|s| s.file.clone()),
                    sha256: source.and_then(|s| s.sha256.clone()),
                    handle: None,
                    layer: Some(entity.base().layer_id.clone()),
                });
                sheet.items.push(item);
            }
        }
        for (i, view) in layout.viewports.iter().enumerate() {
            let scale =
                kentos_sheet::projection::Scale::from_lengths(view.view_height, view.height_mm)
                    .ok_or_else(|| {
                        format!(
                            "{} paftasının görünüm ölçeği desteklenen aralıkta değil.",
                            layout.name
                        )
                    })?
                    .denominator();
            let mut kind = kinds::default_kind("map").ok_or("Pafta görünüm türü bulunamadı")?;
            if let ItemKind::Map(map) = &mut kind {
                map.view_locked = view.locked;
                map.frame_plot = view.frame_plot;
                if !view.paper_clip_mm.is_empty() {
                    map.clip = kentos_sheet::clip::Clip::Polygon {
                        points: view
                            .paper_clip_mm
                            .iter()
                            .map(|p| {
                                [
                                    ((p.x - view.paper_center_mm.x + view.width_mm / 2.)
                                        / view.width_mm
                                        * 1e6)
                                        .round() as i32,
                                    ((view.paper_center_mm.y + view.height_mm / 2. - p.y)
                                        / view.height_mm
                                        * 1e6)
                                        .round() as i32,
                                ]
                            })
                            .collect(),
                    };
                    map.clip
                        .validate()
                        .map_err(|e| format!("{}: {}", layout.name, e.message))?;
                }
                map.view = MapView::Fixed(kinds::FixedView {
                    center: Some(GroundPoint {
                        x: view.view_center.x,
                        y: view.view_center.y,
                    }),
                    scale: scale.round().clamp(1., u32::MAX as f64) as u32,
                    ratio: (scale.fract() != 0.).then_some(kentos_sheet::viewport::ScaleRatio {
                        paper: 1.,
                        model: scale,
                    }),
                    rotation: (view.twist.to_degrees() * 1000.).round() as i32,
                });
            }
            let frame = RectUm::new(
                ((view.paper_center_mm.x - view.width_mm / 2.) * 1000.).round() as i32,
                ((layout.height_mm - view.paper_center_mm.y - view.height_mm / 2.) * 1000.).round()
                    as i32,
                (view.width_mm * 1000.).round().max(1.) as i32,
                (view.height_mm * 1000.).round().max(1.) as i32,
            );
            let mut item = Item::new(
                &Uuid::now_v7().to_string(),
                &format!("DXF görünümü {}", i + 1),
                frame,
                kind,
            );
            item.border = Some(Stroke::solid("#000000", 250));
            sheet.items.push(item);
        }
        book.sheets.push(sheet);
    }
    kentos_sheet::validate::validate_book(&book)
        .map_err(|e| format!("CAD paftası geçersiz: {}", e.message))?;
    Ok(Some(ProjectSheets {
        book: serde_json::to_value(book).map_err(|e| e.to_string())?,
        pictures: doc.sheets().map(|s| s.pictures.clone()).unwrap_or_default(),
    }))
}

pub fn diagnostics(layouts: &[CadLayout]) -> Vec<ReportItem> {
    let skipped = layouts
        .iter()
        .flat_map(|l| l.entities.iter().map(move |e| (l, e)))
        .filter(|(l, e)| paper_item(e, l.height_mm).is_none())
        .count();
    let formatted_text = layouts.iter().flat_map(|l| &l.entities).filter(|e| matches!(e, Entity::Text(t) if t.align.is_some() || t.width_factor.is_some_and(|w| w != 1.) || t.face.oblique.is_some() || !t.paragraph.runs.is_empty() || t.paragraph.layout.is_some() || t.mask)).count();
    let mut out = Vec::new();
    let paths: Vec<_> = layouts
        .iter()
        .flat_map(|l| &l.entities)
        .filter_map(|e| match e {
            Entity::Polyline(p) | Entity::Polygon(p) => Some(p),
            _ => None,
        })
        .collect();
    for (property, reason, affected) in [(
        "paper_elevation",
        "Pafta kitabının çizgi öğesi köşe kotlarını taşımaz.",
        paths
            .iter()
            .filter(|p| {
                p.zs.as_ref()
                    .is_some_and(|z| z.iter().any(|v| v.is_some_and(|v| v != 0.)))
            })
            .collect::<Vec<_>>(),
    )] {
        if !affected.is_empty() {
            out.push(ReportItem {
                severity: Some(ReportSeverity::Warning),
                what: "Pafta çoklu çizgisi".into(),
                count: affected.len() as u32,
                reason: reason.into(),
                lines: Vec::new(),
                details: vec![kentos_commands::ReportDetail {
                    entity_type: "LWPOLYLINE/POLYLINE".into(),
                    property: format!("polyline.loss.{property}"),
                    source_id: None,
                    source_line: None,
                }],
            });
        }
    }
    for (count, reason) in [
        (
            skipped,
            "Desteklenmeyen kâğıt uzayı öğeleri pafta kitabına alınmadı; kaynak dosyada durur.",
        ),
        (
            formatted_text,
            "Pafta yazısının özel hizası, genişletme/eğiklik, karakter biçimleri veya zemini yerel sade metne dönüştürüldü; metin içeriği korundu.",
        ),
    ] {
        if count > 0 {
            out.push(ReportItem {
                severity: Some(ReportSeverity::Warning),
                details: Vec::new(),
                what: "Pafta nesnesi".into(),
                count: count as u32,
                reason: reason.into(),
                lines: Vec::new(),
            });
        }
    }
    out
}

fn point(item: &Item, p: [i32; 2], height: f64) -> Vec2 {
    let p = kinds::frame_point(&item.frame, p);
    world_point(item, p, height)
}
pub(crate) fn world_point(item: &Item, p: [f64; 2], height: f64) -> Vec2 {
    let c = [
        f64::from(item.frame.left) + f64::from(item.frame.width) / 2.,
        f64::from(item.frame.top) + f64::from(item.frame.height) / 2.,
    ];
    let a = f64::from(item.rotation).to_radians() / 1000.;
    let (sn, cs) = (a.sin(), a.cos());
    let (x, y) = (p[0] - c[0], p[1] - c[1]);
    Vec2 {
        x: (c[0] + cs * x - sn * y) / 1e6,
        y: height / 1000. - (c[1] + sn * x + cs * y) / 1e6,
    }
}

pub fn export(doc: &Document) -> (Vec<CadLayout>, Vec<ReportItem>) {
    let Some((book, _)) = doc
        .sheets()
        .and_then(|s| kentos_sheet::project::read_project_sheets(s).ok())
    else {
        return (Vec::new(), Vec::new());
    };
    let mut result = Vec::new();
    let mut omitted = 0u32;
    for (i, sheet) in book.sheets.iter().enumerate() {
        let mut layout = CadLayout {
            name: sheet.name.clone(),
            width_mm: f64::from(sheet.page.size.width) / 1000.,
            height_mm: f64::from(sheet.page.size.height) / 1000.,
            tab_order: i as u32 + 1,
            ..Default::default()
        };
        if sheet.master.is_some() {
            omitted += 1;
        }
        if sheet.atlas.is_some() {
            omitted += 1;
        }
        for item in &sheet.items {
            let mut at = Some(item);
            let mut printable = true;
            for _ in 0..=sheet.items.len() {
                let Some(parent) = at else {
                    break;
                };
                if parent.hidden || !parent.printable {
                    printable = false;
                    break;
                }
                at = parent
                    .group
                    .as_deref()
                    .and_then(|id| sheet.items.iter().find(|p| p.id == id));
            }
            if !printable {
                continue;
            }
            if item.opacity != 100
                || item.border.is_some()
                || item.fill.is_some()
                || item.padding != 0
                || !item.bindings.is_empty()
            {
                omitted += 1;
            }
            let mut base = json!({"id": layout.entities.len() + 1, "layerId": "0", "attrs": {}});
            let raw = match &item.kind {
                ItemKind::Map(map) => {
                    if !matches!(map.layers, kinds::MapLayers::All(_))
                        || !map.layer_overrides.is_empty()
                    {
                        omitted += 1;
                    }
                    if let Ok(view) = kentos_sheet::viewport::effective_fixed_view(item,&doc.settings().named_views)
                        && let Some(center) = &view.center
                    {
                        let rect = kentos_sheet::display::map_content_rect(item, map);
                        let c = world_point(item, rect.center(), layout.height_mm);
                        let angled = item.rotation.rem_euclid(90000) != 0;
                        let (width, height) = if angled {
                            let (sn, cs) = kentos_sheet::units::sin_cos(item.rotation);
                            (
                                f64::from(rect.width) * cs.abs()
                                    + f64::from(rect.height) * sn.abs(),
                                f64::from(rect.width) * sn.abs()
                                    + f64::from(rect.height) * cs.abs(),
                            )
                        } else if item.rotation.rem_euclid(180000) == 90000 {
                            (f64::from(rect.height), f64::from(rect.width))
                        } else {
                            (f64::from(rect.width), f64::from(rect.height))
                        };
                        layout.viewports.push(CadViewport {
                            paper_clip_mm: if map.clip.is_frame() && !angled {
                                Vec::new()
                            } else {
                                map.clip
                                    .paper_points(&rect)
                                    .iter()
                                    .map(|p| {
                                        let point = world_point(item, *p, layout.height_mm);
                                        Vec2 {
                                            x: point.x * 1000.,
                                            y: point.y * 1000.,
                                        }
                                    })
                                    .collect()
                            },
                            frame_plot: map.frame_plot && item.border.is_some(),
                            locked: map.view_locked,
                            paper_center_mm: Vec2 {
                                x: c.x * 1000.,
                                y: c.y * 1000.,
                            },
                            width_mm: kentos_sheet::projection::paper_um_to_mm(width),
                            height_mm: kentos_sheet::projection::paper_um_to_mm(height),
                            view_center: Vec2 {
                                x: center.x,
                                y: center.y,
                            },
                            view_height: kentos_sheet::projection::Scale::for_rendering(
                                view.denominator(),
                            )
                            .model_metres(height),
                            twist: f64::from(kentos_sheet::units::norm_mdeg(
                                i64::from(view.rotation) + i64::from(item.rotation),
                            ))
                            .to_radians()
                                / 1000.,
                        });
                    } else {
                        omitted += 1;
                    }
                    continue;
                }
                ItemKind::Text(t) => {
                    let laid = kentos_sheet::text::layout(
                        &t.content,
                        &t.style,
                        &item.frame,
                        &kentos_sheet::text::Layout {
                            align: t.align,
                            valign: t.valign,
                            line_height: t.line_height,
                            wrap: t.wrap,
                            fit: t.fit,
                        },
                    );
                    let Some(first) = laid.lines.first() else {
                        omitted += 1;
                        continue;
                    };
                    let p = world_point(
                        item,
                        [first.x as f64, first.baseline as f64],
                        layout.height_mm,
                    );
                    base["kind"] = json!("text");
                    base["p"] = json!(p);
                    base["text"] = json!(t.content);
                    base["height"] = json!(f64::from(t.style.size) / 1e6);
                    base["rotation"] = json!(-f64::from(item.rotation) / 1000.);
                    base["align"] = json!(null);
                    base["color"] = json!(t.style.color);
                    base["bold"] = json!(t.style.weight >= 600);
                    base["italic"] = json!(t.style.italic);
                    if let Ok(font) =
                        serde_json::from_value::<kentos_commands::DrawingFont>(json!(t.style.font))
                    {
                        base["font"] = json!(font);
                    }
                    base
                }
                ItemKind::Line(line) => {
                    base["kind"] = json!("polyline");
                    base["pts"] = json!(
                        line.points
                            .iter()
                            .map(|p| point(item, *p, layout.height_mm))
                            .collect::<Vec<_>>()
                    );
                    base["color"] = json!(line.stroke.color);
                    base["lineWeight"] = json!(f64::from(line.stroke.width) / 1000.);
                    base
                }
                ItemKind::Shape(shape) => {
                    if let ShapeKind::Cad(cad) = &shape.shape {
                        match crate::cad_paper::write(cad, item, layout.height_mm, &base) {
                            Some((entities, loss)) => {
                                layout.entities.extend(entities);
                                omitted += loss;
                            }
                            None => omitted += 1,
                        }
                        if shape.fill.is_some() {
                            omitted += 1;
                        }
                        continue;
                    }

                    if matches!(shape.shape, ShapeKind::Ellipse(_)) {
                        let center = point(item, [500000, 500000], layout.height_mm);
                        let width = f64::from(item.frame.width) / 1e6;
                        let height = f64::from(item.frame.height) / 1e6;
                        let angle = -f64::from(item.rotation).to_radians() / 1000.
                            + if height > width {
                                std::f64::consts::FRAC_PI_2
                            } else {
                                0.
                            };
                        let major = width.max(height) / 2.;
                        base["kind"] = json!("ellipse");
                        base["c"] = json!(center);
                        base["major"] = json!({"x": major * angle.cos(), "y": major * angle.sin()});
                        base["ratio"] = json!(width.min(height) / width.max(height));
                        base["t0"] = json!(0.);
                        base["t1"] = json!(std::f64::consts::TAU);
                        if let Some(s) = &shape.stroke {
                            base["color"] = json!(s.color);
                            base["lineWeight"] = json!(f64::from(s.width) / 1000.);
                        }
                        if shape.fill.is_some() {
                            omitted += 1;
                        }
                        if let Ok(entity) = serde_json::from_value(base) {
                            layout.entities.push(entity);
                        }
                        continue;
                    }
                    let pts = match &shape.shape {
                        ShapeKind::Polygon(p) => p.points.clone(),
                        ShapeKind::Rect(_) => vec![
                            [0, 0],
                            [1_000_000, 0],
                            [1_000_000, 1_000_000],
                            [0, 1_000_000],
                        ],
                        _ => {
                            omitted += 1;
                            continue;
                        }
                    };
                    if shape.fill.is_some() {
                        omitted += 1;
                    }
                    base["kind"] = json!("polyline");
                    base["closed"] = json!(true);
                    base["pts"] = json!(
                        pts.iter()
                            .map(|p| point(item, *p, layout.height_mm))
                            .collect::<Vec<_>>()
                    );
                    if let Some(s) = &shape.stroke {
                        base["color"] = json!(s.color);
                        base["lineWeight"] = json!(f64::from(s.width) / 1000.);
                    }
                    base
                }
                ItemKind::Group(_) => {
                    omitted += 1;
                    continue;
                }
                _ => {
                    omitted += 1;
                    continue;
                }
            };
            match serde_json::from_value(raw) {
                Ok(entity) => layout.entities.push(entity),
                Err(_) => omitted += 1,
            }
        }
        result.push(layout);
    }
    let notes = if omitted == 0 {
        Vec::new()
    } else {
        vec![ReportItem { severity: Some(ReportSeverity::Warning), details: Vec::new(), what: "Pafta dışa aktarımı".into(), count: omitted,
        reason: "Desteklenmeyen pafta öğesi/özelliği (ana pafta, atlas, grup, dolgu, kenarlık, opaklık, veri bağlama, iç boşluk, görünüm katman ezmesi) aktarılmadı; desteklenen metin, çizgi, çokgen ve sabit görünüm aktarıldı.".into(), lines: Vec::new() }]
    };
    (result, notes)
}
