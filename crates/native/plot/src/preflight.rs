//! Native print checks: the same visible map layers and source resources as the PDF/CPU painter.
use kentos_contracts::Entity;
use kentos_domain::Document;
use kentos_interaction::Spatial;
use kentos_native_style::picture::ImageSource;
use kentos_sheet::{
    SheetBook,
    display::{self, Prim, RenderInputs},
    preflight::{Finding, Severity},
};
use crate::{book::PlotError, images::Images};

fn finding(item: &str, code: &str, message: String, fix: &str, severity: Severity) -> Finding {
    Finding {
        item: Some(item.into()),
        code: code.into(),
        message,
        fix: fix.into(),
        severity,
        fixes: vec![],
        placeholder: false,
    }
}
/// Native additions to shared layout checks. All source reads happen in the host's background job.
pub fn inspect(
    doc: &Document,
    book: &SheetBook,
    sheet: &str,
    render: &RenderInputs,
    drawing: Option<&str>,
) -> Result<Vec<Finding>, PlotError> {
    inspect_captured(doc,book,sheet,render,drawing,None)
}
pub(crate) fn inspect_captured(doc:&Document,book:&SheetBook,sheet:&str,render:&RenderInputs,drawing:Option<&str>,sources:Option<std::sync::Arc<crate::source_images::SourceImages>>)->Result<Vec<Finding>,PlotError>{
    let mut out = kentos_sheet::preflight::preflight(book, sheet, render)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    out.retain(|f| f.code != "svg_not_in_pdf");
    let list = display::display_list(book, sheet, render)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    let mut spatial = Spatial::new();
    spatial.ignore_object_isolation();
    spatial.reload(doc);
    let images=match sources{Some(sources)=>Images::captured(doc,drawing,sources),None=>Images::new(doc,drawing)};
    let factor = kentos_sheet::print_setup::plan_for(book, sheet)
        .map_err(|e| PlotError::new(&e.code, e.message))?
        .map_or(1., |p| p.factor);
    // The display primitive has the actual contain/cover/original image size, before the final plot transform.
    out.retain(|f| f.code != "low_resolution");
    let scene = kentos_sheet::scene::Scene::new(book, sheet)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    let library = crate::book::library_of(doc);
    let mut line_type_drawer = crate::book::MapDrawer::new(doc, &spatial, &library, None);
    for prim in &list.prims {
        if let Prim::Image(i) = prim
            && let Some(a) = book
                .asset(&i.asset)
                .filter(|a| a.kind != kentos_sheet::model::AssetKind::Svg)
        {
            let ppi = (f64::from(a.width) / (f64::from(i.rect.width) / 25400. * factor))
                .min(f64::from(a.height) / (f64::from(i.rect.height) / 25400. * factor));
            if ppi < 150. - 1e-9 {
                out.push(finding(
                    &i.item,
                    "low_resolution",
                    format!(
                        "Resim nihai baskı boyunda {:.0} ppi; 150 ppi'nin altında.",
                        ppi
                    ),
                    "Daha büyük bir resim kullanın veya nihai baskı boyunu küçültün.",
                    Severity::Warning,
                ));
            }
        }

        let Prim::Map(m) = prim else {
            continue;
        };
        if let Some(it) = scene.item(&m.item)
            && let kentos_sheet::kinds::ItemKind::Map(map) = &it.kind
            && !map.view_locked
            && !it.locked
        {
            let mut f = finding(
                &m.item,
                "viewport_scale_unlocked",
                format!(
                    "“{}” görünümünün ölçeği kilitli değil; yanlışlıkla değişebilir.",
                    it.name
                ),
                "Denetçide Görünümü kilitle seçeneğini açın.",
                Severity::Warning,
            );
            f.fixes.push(kentos_sheet::preflight::Fix {
                label: "Görünümü kilitle".into(),
                action: None,
                ops: vec![kentos_sheet::ops::Op::SetItemProps(
                    kentos_sheet::ops::SetItemProps {
                        id: it.id.clone(),
                        patch: serde_json::json!({"kind":{"type":"map","viewLocked":true}}),
                    },
                )],
            });
            out.push(f);
        }
        // A map's level the project no longer has (docs/adr/0716): it draws the objects on no level only.
        if let Some(level) = &m.level
            && !doc
                .settings()
                .arch
                .as_ref()
                .is_some_and(|a| a.levels.iter().any(|l| l.id == level.id))
        {
            let name = scene
                .item(&m.item)
                .map_or_else(|| m.item.clone(), |it| it.name.clone());
            let mut f = finding(
                &m.item,
                "map_level_missing",
                format!(
                    "“{name}” haritasının katı (“{}”) projede yok: harita yalnız katsız nesneleri çizer.",
                    level.id
                ),
                "Denetçide haritanın Kat'ını projenin bir katı ya da Bütün katlar yapın.",
                Severity::Warning,
            );
            f.fixes.push(kentos_sheet::preflight::Fix {
                label: "Bütün katları çiz".into(),
                action: None,
                ops: vec![kentos_sheet::ops::Op::SetItemProps(
                    kentos_sheet::ops::SetItemProps {
                        id: m.item.clone(),
                        patch: serde_json::json!({"kind":{"type":"map","level":null}}),
                    },
                )],
            });
            out.push(f);
        }
        let Some(extent) = m.extent else {
            continue;
        };
        out.extend(map_text_coverage(doc, m));
        let losses = if crate::maps::line_type_check_needed(doc, m, &library) {
            line_type_drawer.inspect_line_types(book, m, sheet)
            .map_err(|message| PlotError::new("style_compile_failed", message))?
        } else { Vec::new() };
        for message in losses {
            out.push(finding(&m.item, "linetype_unsupported", message,
                "Çizgi tipinin kaynak ve görünüş sorununu giderin veya desteklenen deseni açıkça seçin; eksik desen teslim edilmez.",
                Severity::Error));
        }
        let shown = crate::layers::map_layers(
            doc.layers().nodes(),
            &doc.settings().layer_states,
            &m.layers,
            &m.layer_overrides,
        );
        let shown_ids: std::collections::HashSet<_> = shown.iter().map(|l|l.id.as_str()).collect();
        for layer in &shown {
            if layer.service.is_some() {
                out.push(finding(&m.item,"plot_provider_unsupported",format!("“{}” harita servisi bu çıktı hattında çizilemiyor; eksik pafta üretilmeyecek.",layer.name),"Servisi yerel, desteklenen bir altlığa dönüştürün veya bu viewportta baskıdan çıkarın.",Severity::Error));
            }
            for e in doc.by_layer(&layer.id) {
                if !spatial.store().view_shown(f64::from(e.base().id)) {
                    continue;
                }
                if spatial
                    .store()
                    .get(f64::from(e.base().id))
                    .is_some_and(|v| {
                        v.bounds.max_x < extent[0]
                            || v.bounds.min_x > extent[2]
                            || v.bounds.max_y < extent[1]
                            || v.bounds.min_y > extent[3]
                    })
                {
                    continue;
                }
                let id = e.base().id;
                if matches!(e, Entity::Raster(_) | Entity::PointCloud(_)) {
                    out.push(finding(&m.item,"plot_provider_unsupported",format!("{id} numaralı raster/nokta bulutu bu çıktı hattında çizilemiyor; eksik pafta üretilmeyecek."),"Baskı için desteklenen bir resim altlığı hazırlayın veya bu katmanı viewportta baskıdan çıkarın.",Severity::Error));
                }
                // A leader's note too: its SHX text style (docs/adr/1025).
                if matches!(e, Entity::Dimension(_) | Entity::Text(_) | Entity::Table(_) | Entity::Leader(_))
                    && let Some(uid) = doc.uid(kentos_domain::Slot(id))
                {
                    for f in kentos_native_application::drawing_inspect::annotations::findings(doc, uid, e)
                        .into_iter().filter(|f| matches!(f.code.as_str(), "dimension_override" | "dimension_partial" | "dimension_invalid")
                            || f.code.starts_with("text_") || f.code.starts_with("table_field_"))
                    {
                        out.push(finding(&m.item, &f.code, format!("{id}: {}", f.message), &f.repair,
                            match f.severity { kentos_commands::InspectionSeverity::Error => Severity::Error, kentos_commands::InspectionSeverity::Warning => Severity::Warning, kentos_commands::InspectionSeverity::Info => Severity::Info }));
                    }
                }
                if matches!(e, Entity::Insert(_))
                    && let Some(uid) = doc.uid(kentos_domain::Slot(id))
                    && let Some(expanded) = spatial.store().get(f64::from(id)).and_then(|item|item.expanded.as_deref()) {
                    for (piece, origin) in &expanded.annotation_origins {
                        if !expanded.gates.get(*piece).is_none_or(|gates|gates.iter().all(|l|shown_ids.contains(l.as_str()))) {continue;}
                        let Some(shape)=expanded.shapes.get(*piece) else {continue;};
                        for f in kentos_native_application::drawing_inspect::annotations::piece_findings(doc,uid,origin,shape) {
                            if !f.points.iter().any(|p|kentos_sheet::pdf::includes_text_anchor(m,[p.x,p.y])) {continue;}
                            out.push(finding(&m.item,&f.code,format!("{id}: {}",f.message),&f.repair,match f.severity {
                                kentos_commands::InspectionSeverity::Error=>Severity::Error,
                                kentos_commands::InspectionSeverity::Warning=>Severity::Warning,
                                kentos_commands::InspectionSeverity::Info=>Severity::Info,
                            }));
                        }
                    }
                }
                for fault in doc.audit_object(kentos_domain::Slot(id)) {
                    if matches!(
                        fault.kind,
                        kentos_domain::audit::AuditKind::BrokenLink
                            | kentos_domain::audit::AuditKind::BrokenDimension
                    ) {
                        out.push(finding(
                            &m.item,
                            "broken_annotation",
                            format!("{id}: {}", fault.message),
                            "Nesnenin bağlantısını onarın veya ilişkiyi açıkça kaldırın.",
                            Severity::Warning,
                        ));
                    }
                }
                if matches!(
                    e,
                    Entity::Text(_) | Entity::Dimension(_) | Entity::Leader(_)
                ) && let Ok(v) = serde_json::to_value(e)
                {
                    let mut broken = false;
                    for ptr in [
                        "/refs",
                        "/tip",
                        "/arms",
                        "/fields",
                        "/paragraph/fields",
                        "/tag",
                        "/viewMark",
                    ] {
                        fn has_break(v: &serde_json::Value) -> bool {
                            match v {
                                serde_json::Value::Object(m) => {
                                    m.get("broken").is_some_and(|v| !v.is_null())
                                        || m.values().any(has_break)
                                }
                                serde_json::Value::Array(a) => a.iter().any(has_break),
                                _ => false,
                            }
                        }
                        broken |= v.pointer(ptr).is_some_and(has_break);
                    }
                    if broken {
                        out.push(finding(&m.item,"broken_annotation",format!("{id} numaralı açıklamanın kaynak ilişkisi kopuk; saklanan son değer çiziliyor."),"Kaynak nesneyi/bağlantıyı onarıp açıklamayı güncelleyin.",Severity::Warning));
                    }
                }
                if let Entity::Image(image) = e {
                    let i = &image.image;
                    let key = i
                        .file
                        .as_ref()
                        .map(|f| format!("file:{f}"))
                        .or_else(|| i.asset.as_ref().map(|f| format!("asset:{f}")));
                    let bitmap = key.as_deref().and_then(|k| images.bitmap(k));
                    match bitmap {
                        None => out.push(finding(
                            &m.item,
                            "missing_reference",
                            format!(
                                "“{}” resim kaynağı okunamadı (nesne {id}).",
                                i.file.as_deref().or(i.asset.as_deref()).unwrap_or("kaynak")
                            ),
                            "Resim bağlantısını düzeltin veya dosyayı projeye gömün.",
                            Severity::Error,
                        )),
                        Some(b) => {
                            let mm = 1000. / m.view.scale.max(1e-6) * factor;
                            let ppi = (f64::from(b.width) / (i.width * mm / 25.4))
                                .min(f64::from(b.height) / (i.height * mm / 25.4));
                            if ppi < 150. - 1e-9 {
                                out.push(finding(&m.item,"low_resolution",format!("{id} numaralı resim nihai baskı boyunda {:.0} ppi; 150 ppi'nin altında.",ppi),"Daha yüksek çözünürlüklü kaynak kullanın veya basılan resim boyunu küçültün.",Severity::Warning));
                            }
                        }
                    }
                }
                if let Entity::Insert(insert) = e
                    && let Some(x) = doc
                        .blocks()
                        .iter()
                        .find(|b| b.id == insert.block)
                        .and_then(|b| b.xref.as_ref())
                        .filter(|x| !x.unloaded && x.underlay.as_ref().is_none_or(|u| u.plot))
                    && kentos_native_application::links::find(
                        &x.path,
                        x.relative.as_deref(),
                        drawing,
                        &doc.settings().xref_paths,
                    )
                    .at
                    .is_none()
                {
                    out.push(finding(&m.item,"missing_reference",format!("{id} dış referansının kaynağı bulunamadı; son yüklenen içerik çiziliyor."),"Referans yolunu düzeltin veya saklanan içeriği çizime bağlayın.",Severity::Warning));
                }
            }
        }
    }
    out.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then(a.code.cmp(&b.code))
            .then(a.message.cmp(&b.message))
    });
    out.dedup_by(|a, b| a.item == b.item && a.code == b.code && a.message == b.message);
    Ok(out)
}

/// Inspect actual map label pieces, including block text and generated values.
/// Sheet-only preflight does not see the map's text and cannot detect its `?` substitutions.
fn map_text_coverage(doc: &Document, map: &kentos_sheet::display::MapPrim) -> Vec<Finding> {
    use std::collections::{BTreeMap, BTreeSet};
    use kentos_sheet::{style::TextStyle, text::{self, Glyph}};
    let scaled = doc.at_scale(map.view.annotation_denominator());
    let source = scaled.as_ref().unwrap_or(doc);
    let mut spatial = Spatial::new();
    spatial.ignore_object_isolation();
    spatial.reload(source);
    let mut out=Vec::new();
    let mut missing: BTreeMap<(String, u16, bool), BTreeSet<char>> = BTreeMap::new();
    for label in crate::labels::map_labels(source, &spatial, map) {
        if !kentos_sheet::pdf::includes_text_anchor(map, [label.at.x, label.at.y]) { continue; }
        // An SHX text's letters are its font's strokes (ADR 1125); its fonts were checked as the text was laid out.
        if label.shx.is_some() { continue; }
        if let Some(unicode)=&label.unicode {
            if let Err(error)=&unicode.text.plan {
                out.push(finding(&map.item,"text_layout_unsupported",error.message(&unicode.text.source),
                    "Yazı denetimindeki harf ve biçim sorununu giderin; kısmi metin teslim edilmez.",Severity::Error));
            }
            continue;
        }
        let key = (label.font.map_or("barlow", crate::fonts::sheet_font).to_owned(), label.weight, label.italic);
        let Some(face) = text::face(&TextStyle { font:key.0.clone(), weight:key.1, italic:key.2,
            size:1000, color:String::new() }) else { continue; };
        for c in label.text.chars() {
            if matches!(text::glyph(face, c), Glyph::Missing(_)) {
                missing.entry(key.clone()).or_default().insert(c);
            }
        }
    }
    out.extend(missing.into_iter().map(|((font, weight, italic), chars)| {
        let chars = chars.into_iter().take(32).map(|c| format!("U+{:04X} ({c})", c as u32)).collect::<Vec<_>>().join(", ");
        finding(&map.item, "map_text_unsupported", format!(
            "Viewport yazısı {font} {weight}{} ile PDF'ye eksiksiz yazılamıyor: {chars}. Bu karakterler soru işaretine dönüştürülemez.",
            if italic { " eğik" } else { "" }),
            "Desteklenen ortak metin yerleşimini kullanın; kaynak yazıyı silerek veya sessiz karakter değiştirerek teslim etmeyin.", Severity::Error)
    }));
    out
}

/// Live tables, legends and coordinates must be populated before judging their layout.
pub fn page_inputs(
    doc: &Document,
    book: &SheetBook,
    id: &str,
    base: &RenderInputs,
) -> Result<RenderInputs, PlotError> {
    let sheet = book
        .sheet(id)
        .ok_or_else(|| PlotError::new("unknown_sheet", "Pafta yok."))?;
    let mut spatial = Spatial::new();
    spatial.ignore_object_isolation();
    spatial.reload(doc);
    let library = crate::book::library_of(doc);
    let mut render = base.clone();
    let list =
        display::display_list(book, id, &render).map_err(|e| PlotError::new(&e.code, e.message))?;
    let data = crate::inputs::sheet_data(doc, &spatial, &library, &[], book, sheet, Some(&list));
    render.tables.extend(data.tables);
    render.coordinates.extend(data.coordinates);
    render.legends.extend(data.legends);
    Ok(render)
}
/// Measured pixels of the actual fallback image include output caps and final plot enlargement.
pub(crate) fn raster_resolution(
    book: &SheetBook,
    inputs: &kentos_sheet::pdf::PdfInputs,
    id: &str,
) -> Result<Vec<Finding>, PlotError> {
    let list = display::display_list(book, id, &inputs.render)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    let factor = kentos_sheet::print_setup::plan_for(book, id)
        .map_err(|e| PlotError::new(&e.code, e.message))?
        .map_or(1., |p| p.factor);
    let mut out = Vec::new();
    for prim in list.prims {
        let Prim::Map(m) = prim else { continue };
        let Some(map) = inputs
            .maps
            .iter()
            .find(|x| x.item == m.item && x.sheet.as_deref().is_none_or(|s| s == id))
        else {
            continue;
        };
        let kentos_sheet::pdf::MapContent::Raster(r) = &map.content else {
            continue;
        };
        let Some(header) = r.png.get(16..24) else {
            continue;
        };
        let w = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let h = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
        let ppi = (f64::from(w) / (f64::from(m.clip.width) / 25400. * factor))
            .min(f64::from(h) / (f64::from(m.clip.height) / 25400. * factor));
        if ppi < 150. - 1e-9 {
            out.push(finding(&m.item,"low_resolution",format!("Görünümün gerçek raster çıktısı nihai baskıda {ppi:.0} ppi; hedef dpi, bellek sınırı ve baskı büyütmesi birlikte hesaba katıldı."),"Baskı büyütmesini azaltın veya görünümü daha küçük fiziksel paftalara bölün.",Severity::Warning));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn sheet_preflight_provider_guard_also_protects_direct_cpu_and_atlas_map_drawers() {
        let fx: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/sheet/v1/native-preflight.json"
        ))
        .unwrap();
        for name in [
            "raster-visible-refused",
            "cloud-visible-refused",
            "visible-map-service-refused",
        ] {
            let c = fx["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == name)
                .unwrap();
            let mut model = fx["model"].clone();
            model["layers"][1]["visible"] = json!(true);
            if let Some(e) = c.get("entity") {
                model["entities"].as_array_mut().unwrap().push(e.clone());
            } else {
                model["layers"][1]["service"] =
                    json!({"kind":"xyz","url":"https://invalid.example/{z}/{x}/{y}.png"});
            }
            let doc = Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
            let mut book = fx["book"].clone();
            if let Some(center) = c.get("center") {
                book["sheets"][0]["items"][0]["kind"]["view"]["center"] = center.clone();
            }
            let book: SheetBook = serde_json::from_value(book).unwrap();
            let render = crate::book::base_inputs(&doc, "Test", "2026-10-23");
            let list = display::display_list(&book, "ground", &render).unwrap();
            let map = list
                .prims
                .iter()
                .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
                .unwrap();
            let mut spatial = Spatial::new();
            spatial.ignore_object_isolation();
            spatial.reload(&doc);
            let library = crate::book::library_of(&doc);
            let center = map.view.center.unwrap();
            let src = crate::maps::MapSource {
                map: None,
                level: map.level.as_ref(),
                phase: map.phase,
                hidden: &map.hidden,
                pens: &map.pens,
                mono: map.mono,
                options: &map.options,
                doc: &doc,
                store: spatial.store(),
                library: &library,
                origin: kentos_geometry_core::Vec2::new(center.x, center.y),
            };
            let result = crate::render_map_image(&src, map, &[], [64, 64], None);
            assert!(
                result.is_err(),
                "{name}: unsupported data must not become a successful blank map"
            );
        }
    }
}
