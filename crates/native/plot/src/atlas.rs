//! A sheet's atlas as one PDF with a page for each object of the coverage layer, made from a pinned state of the
//! drawing (docs/adr/0534, plan/gis.md CBS-21).
//!
//! The sheet core's planner chooses the pages (its filter, sort, names, scales: [`kentos_sheet::atlas`]) and the
//! PDF writer writes one page for each (`PdfInputs.atlas`); this module binds them to the drawing's data:
//!
//! - **the source is pinned**: the project's parts' stamps, each layer a map or table of the sheet shows and the
//!   coverage layer by its digest, each bound layer's file by its SHA-256 ([`AtlasPin`]). A pin given must be the
//!   state read (`pin_mismatch`); the files are looked at again when the pages are made, and a changed one refuses
//!   the run (`source_changed`): nothing is written from a mix of two states;
//! - **the bindings are told**: the theme the atlas maps are drawn in, the moment of the temporal layers, the
//!   coordinate system ([`AtlasBindings`]); they are written into the file's keywords;
//! - **only what the view shows and what is permitted goes out**: an object its layer's filter or the time leaves out
//!   is no page and is on no map, table or legend; an attribute a form marks `redact`, or one outside `fields`,
//!   is in no page's variables, no table and no legend.

use std::collections::{BTreeSet, HashSet};

use kentos_contracts::encoding::{sha256_hex, sha256_reader_hex};
use kentos_contracts::{
    AtlasBindings, AtlasCoverage, AtlasLayerRole, AtlasPageInfo, AtlasPin, AtlasPinLayer,
    AtlasPinSource, AtlasRead, Entity, LayerNode, SourceKind, SourceLink, Vec2 as CVec2,
};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::sources_query::{self, StageFrom};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::time::{Read, Window};
use kentos_interaction::Spatial;
use kentos_sheet::atlas::atlas_plan;
use kentos_sheet::display::{self, AtlasFeature, Attribute, Prim, RenderInputs};
use kentos_sheet::kinds::{CoordSource, ItemKind, MapLayers, MapView, TableSource, ThemeRef};
use kentos_sheet::model::{SheetBook, VarValue};
use kentos_sheet::pdf::{PdfAsset, PdfAtlas, PdfAtlasPage, PdfInputs, PdfMap, PdfOptions};
use kentos_sheet::preflight::Finding;
use kentos_sheet::project::read_project_sheets;

use crate::book::{MapDrawer, base_inputs, crs_name, library_of};
use crate::inputs::{Permit, attr_value, layer_of, pdf_crs, sheet_data_in};

/// Why an atlas could not be made: a code a program decides by, the words, and the input it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasError {
    pub code: String,
    pub message: String,
    pub path: String,
}

impl AtlasError {
    fn new(code: &str, message: impl Into<String>, path: &str) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            path: path.to_owned(),
        }
    }
}

/// A warning: a code, the words and the input it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasWarning {
    pub code: String,
    pub message: String,
    pub path: String,
}

/// What an atlas run is asked (docs/adr/0534 §1).
#[derive(Clone, Debug, Default)]
pub struct AtlasInput<'a> {
    pub sheet: &'a str,
    pub fields: Option<&'a [String]>,
    pub theme: Option<&'a str>,
    pub time: Option<&'a str>,
    pub crs: Option<u32>,
    pub pin: Option<&'a AtlasPin>,
    /// Where the coverage layer's objects come from (docs/adr/0638 §3).
    pub coverage: AtlasCoverage,
    /// The title block's user and date (`2026-10-19`).
    pub user: &'a str,
    pub date: &'a str,
}

/// The pages and what they are made from, before the PDF: what a plan says and a preview shows.
#[derive(Debug)]
pub struct AtlasPlanned {
    pub pages: Vec<AtlasPageInfo>,
    pub pin: AtlasPin,
    pub bindings: AtlasBindings,
    /// The attributes the pages carry.
    pub fields: Vec<String>,
    /// The coverage layer's attribute names the pages do not carry.
    pub withheld: Vec<String>,
    /// The coverage layer's objects the layer's filter or the time left out.
    pub filtered_out: u32,
    pub warnings: Vec<AtlasWarning>,
    /// What the planner and the run say about the pages (a scale that did not fit …).
    pub notes: Vec<String>,
    sheet: String,
    layer: String,
    book: SheetBook,
    pictures: Vec<(String, Vec<u8>)>,
    plan: Vec<display::AtlasPageInput>,
    permit: Permit,
    window: Option<Window>,
    user: String,
    date: String,
    atlas_maps: BTreeSet<String>,
    /// The paper's system when it is not the project's (docs/adr/0638 §1).
    map: Option<&'static kentos_geometry_core::crs::map::MapProjection>,
    /// Each page's object's box on the project's ground, by lasting id.
    boxes: std::collections::HashMap<String, [f64; 4]>,
    /// The coverage layer's rows read from its source (`coverage: source`), the layer's objects in the pages' drawing.
    staged: Option<Vec<(Uuid, Entity)>>,
    /// What was read from sources at the plan.
    pub read: Vec<AtlasRead>,
}

/// The atlas PDF.
#[derive(Debug)]
pub struct AtlasPdf {
    pub bytes: Vec<u8>,
    pub findings: Vec<Finding>,
    pub notes: Vec<String>,
    /// What was read from bound sources: the plan's coverage table and the pages' map layers.
    pub read: Vec<AtlasRead>,
}

fn file_of(link: &kentos_contracts::SourceLink) -> Option<String> {
    if link.kind != SourceKind::Geopackage {
        return None;
    }
    std::fs::File::open(&link.path)
        .and_then(|f| sha256_reader_hex(std::io::BufReader::new(f)))
        .ok()
        .map(|(_, sha)| sha)
}

/// The pin of the layers `layers` as the drawing holds them now; `files` reads the bound files too.
fn pin_of(doc: &Document, layers: &[(String, AtlasLayerRole)], files: bool) -> AtlasPin {
    let tree = doc.layers();
    let data: Vec<AtlasPinLayer> = layers
        .iter()
        .map(|(id, role)| AtlasPinLayer {
            layer: id.clone(),
            name: tree.get(id).map_or_else(|| id.clone(), |n| n.name.clone()),
            role: *role,
            objects: u32::try_from(doc.count(id)).unwrap_or(u32::MAX),
            digest: kentos_native_application::sources::layer_digest(doc, id),
            source: doc
                .settings()
                .sources
                .iter()
                .find(|l| l.layer == *id)
                .map(|l| AtlasPinSource {
                    id: l.id.clone(),
                    path: l.path.clone(),
                    revision: l.revision.clone(),
                    current: if files { file_of(l) } else { None },
                }),
        })
        .collect();
    let mut pin = AtlasPin {
        revision: doc.revision().to_string(),
        settings: doc.settings_stamp(),
        layers: doc.layers_stamp(),
        styles: doc.styles_stamp(),
        sheets: doc.sheets_stamp(),
        data,
        digest: String::new(),
    };
    pin.digest = digest_of(&pin);
    pin
}

/// The SHA-256 of a pin without its revision counter (an undo and a redo change that, not the content).
fn digest_of(pin: &AtlasPin) -> String {
    let body = serde_json::json!({
        "settings": pin.settings, "layers": pin.layers, "styles": pin.styles, "sheets": pin.sheets,
        "data": pin.data,
    });
    sha256_hex(kentos_contracts::canonical::canonical_json(&body).as_bytes())
}

/// What differs between a pinned state and the one read, in words; none when they are the same.
fn pin_difference(pinned: &AtlasPin, now: &AtlasPin) -> Option<String> {
    if pinned.digest == now.digest {
        return None;
    }
    let mut what: Vec<String> = Vec::new();
    for (name, a, b) in [
        ("ayarlar", &pinned.settings, &now.settings),
        ("katman ağacı", &pinned.layers, &now.layers),
        ("stiller", &pinned.styles, &now.styles),
        ("paftalar", &pinned.sheets, &now.sheets),
    ] {
        if a != b {
            what.push(name.to_owned());
        }
    }
    for l in &now.data {
        match pinned.data.iter().find(|p| p.layer == l.layer) {
            None => what.push(format!("“{}” katmanı sabitlenenler arasında yoktu", l.name)),
            Some(p) if p.digest != l.digest => {
                what.push(format!("“{}” katmanının nesneleri", l.name))
            }
            Some(p) => {
                if p.source != l.source {
                    what.push(format!("“{}” katmanının kaynak dosyası", l.name));
                }
            }
        }
    }
    for p in &pinned.data {
        if !now.data.iter().any(|l| l.layer == p.layer) {
            what.push(format!(
                "“{}” katmanı artık sabitlenen katmanlar arasında değil",
                p.name
            ));
        }
    }
    Some(if what.is_empty() {
        "sabitlenen durumdan başka".to_owned()
    } else {
        what.join(", ")
    })
}

/// The layers an atlas sheet reads: its coverage layer first, then those its maps draw, its tables list and its
/// coordinate lists take, each once.
fn layers_read(
    doc: &Document,
    book: &SheetBook,
    sheet: &kentos_sheet::model::Sheet,
    coverage: &str,
) -> Vec<(String, AtlasLayerRole)> {
    let mut out: Vec<(String, AtlasLayerRole)> =
        vec![(coverage.to_owned(), AtlasLayerRole::Coverage)];
    let mut add = |id: String| {
        if !out.iter().any(|(l, _)| *l == id) {
            out.push((id, AtlasLayerRole::Map));
        }
    };
    let master = sheet
        .master
        .as_deref()
        .and_then(|m| book.masters.iter().find(|x| x.id == m));
    for it in master
        .into_iter()
        .flat_map(|m| m.items.iter())
        .chain(sheet.items.iter())
    {
        match &it.kind {
            ItemKind::Map(m) => {
                for node in crate::layers::map_layers(
                    doc.layers().nodes(),
                    &doc.settings().layer_states,
                    &m.layers,
                    &m.layer_overrides,
                ) {
                    let only = match &m.layers {
                        MapLayers::List(l) => Some(&l.layers),
                        _ => None,
                    };
                    if only.is_none_or(|l| l.contains(&node.id)) {
                        add(node.id.clone());
                    }
                }
            }
            ItemKind::Table(t) => {
                if let TableSource::Layer(src) = &t.source
                    && let Some(l) = layer_of(doc, &src.layer)
                {
                    add(l);
                }
            }
            ItemKind::CoordinateList(c) => {
                if let CoordSource::Layer(l) = &c.source
                    && let Some(l) = layer_of(doc, &l.layer)
                {
                    add(l);
                }
            }
            _ => {}
        }
    }
    out
}

/// The attribute names an atlas's pages carry and those it holds back (docs/adr/0534 §4): the layer's declared
/// fields (all the names of an undeclared layer) or `wanted`, less what a form marks `redact`.
fn carried(
    doc: &Document,
    node: &LayerNode,
    wanted: Option<&[String]>,
) -> Result<(Vec<String>, Vec<String>), AtlasError> {
    let mut present: BTreeSet<String> = BTreeSet::new();
    for e in doc.by_layer(&node.id) {
        present.extend(e.base().attrs.keys().cloned());
    }
    let declared: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let redacted: HashSet<String> = kentos_native_application::python_call::redacted_names(doc)
        .into_iter()
        .collect();
    let candidates: Vec<String> = match wanted {
        Some(w) => {
            for (i, name) in w.iter().enumerate() {
                if !present.contains(name) && !declared.contains(name) {
                    return Err(AtlasError::new(
                        "field_not_found",
                        format!(
                            "“{name}” alanı “{}” katmanında yok: katmanın alanlarından birini verin.",
                            node.name
                        ),
                        &format!("fields[{i}]"),
                    ));
                }
            }
            w.to_vec()
        }
        None if !declared.is_empty() => declared.clone(),
        None => present.iter().cloned().collect(),
    };
    let mut fields: Vec<String> = Vec::new();
    for c in candidates {
        if !redacted.contains(&c) && !fields.contains(&c) {
            fields.push(c);
        }
    }
    let mut every: BTreeSet<String> = present;
    every.extend(declared);
    let withheld: Vec<String> = every.into_iter().filter(|n| !fields.contains(n)).collect();
    Ok((fields, withheld))
}

/// Plans an atlas: the pages, the pin and the bindings, with no file made (what a plan answers and what the
/// desktop's window previews). Refuses in the order the command's capabilities list.
pub fn plan(doc: &Document, input: &AtlasInput<'_>) -> Result<AtlasPlanned, AtlasError> {
    let Some(kept) = doc.sheets() else {
        return Err(AtlasError::new(
            "no_sheets",
            "Projede pafta yok: önce bir pafta ekleyin (cad.sheets.edit ya da Pafta kipi).",
            "sheet",
        ));
    };
    let (mut book, pictures): (SheetBook, _) = read_project_sheets(kept)
        .map_err(|e| AtlasError::new("book_unreadable", e.message, "sheet"))?;
    let Some(si) = book.sheets.iter().position(|s| s.id == input.sheet) else {
        return Err(AtlasError::new(
            "unknown_sheet",
            format!("“{}” paftası kitapta yok.", input.sheet),
            "sheet",
        ));
    };
    let Some(atlas) = book.sheets[si].atlas.clone() else {
        return Err(AtlasError::new(
            "no_atlas",
            format!(
                "“{}” paftasının atlası yok: Pafta kipinin Atlas düğmesindeki pencereden ya da cad.sheets.edit'in setAtlas işleminden kapsam katmanını seçerek tanımlayın.",
                book.sheets[si].name
            ),
            "sheet",
        ));
    };
    let Some(layer) = (!atlas.layer.trim().is_empty())
        .then(|| layer_of(doc, &atlas.layer))
        .flatten()
    else {
        return Err(AtlasError::new(
            "atlas_no_layer",
            format!(
                "Atlasın kapsam katmanı (“{}”) seçilmemiş ya da çizimde yok.",
                atlas.layer
            ),
            "sheet",
        ));
    };
    let settings = doc.settings();
    // The paper's system (docs/adr/0638 §1): the project's, or one the project's moves to.
    let srid = input.crs.unwrap_or(settings.srid);
    let map = kentos_project::systems::paper_projection(settings, srid)
        .map_err(|why| AtlasError::new("crs_unsupported", why, "crs"))?;
    if let Some(t) = input.theme
        && !settings
            .layer_states
            .iter()
            .any(|s| s.name.trim() == t.trim())
    {
        return Err(AtlasError::new(
            "unknown_theme",
            format!(
                "“{t}” adında katman durumu (tema) projede yok: Katman durumları'ndan birini verin."
            ),
            "theme",
        ));
    }
    let window = match input.time {
        None => None,
        Some(t) => match kentos_geometry_core::time::read(t) {
            Read::Moment(m) => Some(Window::Instant(m)),
            _ => {
                return Err(AtlasError::new(
                    "invalid_time",
                    format!(
                        "“{t}” bir an olarak okunamadı: 2024-06-30 ya da 2024-06-30T12:00 gibi yazın."
                    ),
                    "time",
                ));
            }
        },
    };
    let node = doc
        .layers()
        .get(&layer)
        .cloned()
        .ok_or_else(|| AtlasError::new("atlas_no_layer", "Kapsam katmanı yok.", "sheet"))?;
    let (fields, withheld) = carried(doc, &node, input.fields)?;
    // The coverage layer read from its source whole (docs/adr/0638 §3), a part at a time as `cad.sources.query` reads.
    let mut source_read: Vec<AtlasRead> = Vec::new();
    let mut source_left = 0u64;
    let staged = match input.coverage {
        AtlasCoverage::Drawing => None,
        AtlasCoverage::Source => {
            let Some(link) = settings.sources.iter().find(|l| l.layer == layer) else {
                return Err(AtlasError::new(
                    "coverage_not_bound",
                    format!(
                        "“{}” katmanı bir kaynağa bağlı değil: kaynaktan okumak bağlı katman ister (Bağlı katman olarak aç ya da cad.sources.edit).",
                        node.name
                    ),
                    "coverage",
                ));
            };
            let filter = node.filter.as_ref().and_then(|f| f.expression.as_deref());
            let time = input.time.zip(node.time.clone()).map(|(at, fields)| {
                kentos_commands::QueryTime {
                    at: Some(at.trim().to_owned()),
                    fields: Some(fields),
                    ..Default::default()
                }
            });
            let (staging, queried) = sources_query::stage(
                settings,
                &StageFrom::Layer(link),
                filter,
                None,
                time.as_ref(),
                "coverage",
                &|| false,
            )
            .map_err(|e| match e {
                Some(e) => AtlasError::new(&e.code, e.message, "coverage"),
                None => AtlasError::new("pdf_unwritable", "Okuma durduruldu.", "coverage"),
            })?;
            source_left = queried.read.saturating_sub(staging.objects.len() as u64);
            source_read.push(AtlasRead {
                layer: layer.clone(),
                table: link.table.clone(),
                rows: staging.objects.len() as u64,
                pages: None,
            });
            Some(staging.objects)
        }
    };
    // The theme: the atlas maps draw it in place of the layers they show.
    if let Some(t) = input.theme {
        for it in &mut book.sheets[si].items {
            if let ItemKind::Map(m) = &mut it.kind
                && matches!(m.view, MapView::Atlas(_))
            {
                m.layers = MapLayers::Theme(ThemeRef {
                    name: t.trim().to_owned(),
                });
            }
        }
    }
    // A map that stands still has its middle on the project's ground: on paper of another system, there.
    if let Some(m) = map {
        for it in &mut book.sheets[si].items {
            if let ItemKind::Map(view) = &mut it.kind
                && let MapView::Fixed(f) = &mut view.view
                && let Some(c) = f.center.as_mut()
            {
                let Some(p) = m.forward(kentos_geometry_core::Vec2::new(c.x, c.y)) else {
                    return Err(AtlasError::new(
                        "crs_unsupported",
                        format!(
                            "“{}” haritasının ortası EPSG:{srid} sistemine çevrilemedi: sistem bu yeri kapsamıyor.",
                            it.name
                        ),
                        "crs",
                    ));
                };
                c.x = p.x;
                c.y = p.y;
            }
        }
    }
    let sheet = book.sheets[si].clone();
    // The pin: what the pages are read from, before anything is made.
    let read = layers_read(doc, &book, &sheet, &layer);
    let pin = pin_of(doc, &read, true);
    if let Some(p) = input.pin
        && let Some(what) = pin_difference(p, &pin)
    {
        return Err(AtlasError::new(
            "pin_mismatch",
            format!(
                "Çizim ya da bağlı kaynak, sabitlenen durumdan değişti ({what}). Planı yeniden alıp çıktıyı yenileyin."
            ),
            "pin",
        ));
    }
    let mut warnings: Vec<AtlasWarning> = Vec::new();
    for l in &pin.data {
        let Some(src) = &l.source else { continue };
        if src.current.as_deref().is_some_and(|c| c != src.revision) {
            warnings.push(AtlasWarning {
                code: "source_stale".into(),
                message: format!(
                    "“{}” katmanının dosyası ({}) katmanın okunduğu dosya değil; sayfalar katmandan, yani sabitlenen sürümden yapıldı.",
                    l.name, src.path
                ),
                path: "pin".into(),
            });
        }
        if let Some(link) = doc.settings().sources.iter().find(|s| s.id == src.id)
            && link.digest != l.digest
        {
            warnings.push(AtlasWarning {
                code: "source_unsaved".into(),
                message: format!(
                    "“{}” katmanında kaynak dosyaya yazılmamış değişiklikler var; sayfalar katmandan, yani değişiklikleriyle yapıldı.",
                    l.name
                ),
                path: "pin".into(),
            });
        }
    }
    let clip_feature = sheet.items.iter().any(
        |it| matches!(&it.kind, ItemKind::Map(m) if m.clip_to_atlas_feature && matches!(m.view, MapView::Atlas(_))),
    );
    // The coverage objects the view shows: its layer's filter and the time cut.
    let work;
    let pages_doc = match &staged {
        None => doc,
        Some(rows) => {
            work = with_rows(doc, &layer, rows.clone())?;
            &work
        }
    };
    let mut spatial = Spatial::new();
    spatial.reload(pages_doc);
    spatial.set_time_window(window);
    let carried_set: HashSet<&str> = fields.iter().map(String::as_str).collect();
    let mut features: Vec<AtlasFeature> = Vec::new();
    let (mut filtered_out, mut skipped, mut unreached) = (0u32, 0u32, 0u32);
    // Each object's box on the project's ground, by lasting id: a page's tables and its answer are of it.
    let mut boxes: std::collections::HashMap<String, [f64; 4]> = std::collections::HashMap::new();
    filtered_out += u32::try_from(source_left).unwrap_or(u32::MAX);
    for (_, slot) in pages_doc.layer_slots(&layer) {
        if !spatial.view_shown(slot) {
            filtered_out += 1;
            continue;
        }
        let (Some(e), Some(uid), Some(item)) = (
            pages_doc.get(slot),
            pages_doc.uid(slot),
            spatial.store().get(f64::from(slot.0)),
        ) else {
            skipped += 1;
            continue;
        };
        let b = item.bounds;
        if matches!(item.shape, Shape::Record {})
            || !(b.min_x.is_finite()
                && b.min_y.is_finite()
                && b.max_x.is_finite()
                && b.max_y.is_finite())
        {
            skipped += 1;
            continue;
        }
        let own = [b.min_x, b.min_y, b.max_x, b.max_y];
        // The planner fits the object on the paper's ground: its box moved there.
        let moved = match map {
            None => Some(own),
            Some(m) => kentos_geometry_core::crs::transform_bounds(&m.project, &m.map, own, 21, &m.choices),
        };
        let Some(moved) = moved else {
            unreached += 1;
            continue;
        };
        boxes.insert(uid.to_string(), own);
        // Its outline, for a map that cuts to it (docs/adr/0638 §2), on the paper's ground.
        let outline = clip_feature
            .then(|| outline_of(&item.shape, own))
            .flatten()
            .and_then(|ring| match map {
                None => Some(ring),
                Some(m) => m.path(&ring, true),
            })
            .map(|ring| ring.iter().map(|p| [p.x, p.y]).collect());
        features.push(AtlasFeature {
            id: uid.to_string(),
            bbox: moved,
            outline,
            attributes: e
                .base()
                .attrs
                .iter()
                .filter(|(name, _)| carried_set.contains(name.as_str()))
                .map(|(name, value)| Attribute {
                    name: name.clone(),
                    value: attr_value(value),
                })
                .collect(),
        });
    }
    if skipped > 0 {
        warnings.push(AtlasWarning {
            code: "atlas_note".into(),
            message: format!(
                "{skipped} nesne (geometrisiz kayıt ya da sınırı olmayan) sayfa olmadı."
            ),
            path: "sheet".into(),
        });
    }
    if clip_feature {
        let bare = features.iter().filter(|f| f.outline.is_none()).count();
        if bare > 0 {
            warnings.push(AtlasWarning {
                code: "atlas_note".into(),
                message: format!(
                    "{bare} sayfanın nesnesi alan değil: harita o sayfalarda nesneye kırpılmadan çizildi."
                ),
                path: "sheet".into(),
            });
        }
    }
    if unreached > 0 {
        warnings.push(AtlasWarning {
            code: "atlas_note".into(),
            message: format!(
                "{unreached} nesne EPSG:{srid} sistemine çevrilemediği için (sistem o yeri kapsamıyor) sayfa olmadı."
            ),
            path: "crs".into(),
        });
    }
    let planned = atlas_plan(&book, &sheet.id, &features).map_err(|e| {
        let code = if e.code == "expression_error" {
            "atlas_expression"
        } else {
            e.code.as_str()
        };
        AtlasError::new(code, e.message, "sheet")
    })?;
    if planned.pages.is_empty() {
        return Err(AtlasError::new(
            "atlas_empty",
            format!(
                "“{}” katmanında süzgeçlerden geçen nesne kalmadı: atlasın sayfası yok.",
                node.name
            ),
            "sheet",
        ));
    }
    let atlas_maps: BTreeSet<String> = sheet
        .items
        .iter()
        .filter_map(|it| match &it.kind {
            ItemKind::Map(m) if matches!(m.view, MapView::Atlas(_)) => Some(it.id.clone()),
            _ => None,
        })
        .collect();
    let pages: Vec<AtlasPageInfo> = planned
        .pages
        .iter()
        .map(|p| {
            let view = p.maps.first();
            AtlasPageInfo {
                index: p.index,
                name: p.name.clone(),
                feature: p.feature.id.clone(),
                scale: view.map_or(0, |v| v.scale),
                center: view.map_or_else(
                    || CVec2 {
                        x: (p.feature.bbox[0] + p.feature.bbox[2]) / 2.0,
                        y: (p.feature.bbox[1] + p.feature.bbox[3]) / 2.0,
                    },
                    |v| CVec2 {
                        x: v.center.x,
                        y: v.center.y,
                    },
                ),
                bbox: boxes.get(&p.feature.id).copied().unwrap_or(p.feature.bbox),
            }
        })
        .collect();
    // The planner's notes on scales that did not fit are the pages' own (its count of filtered objects is ours).
    let mut notes: Vec<String> = planned
        .warnings
        .iter()
        .filter(|w| !w.starts_with("Süzgeç "))
        .cloned()
        .collect();
    notes.extend(
        warnings
            .iter()
            .filter(|w| w.code == "atlas_note")
            .map(|w| w.message.clone()),
    );
    let redacted: HashSet<String> = kentos_native_application::python_call::redacted_names(doc)
        .into_iter()
        .collect();
    let name = match map {
        None => crs_name(settings),
        Some(_) => kentos_project::crs::system(srid).map(|s| s.name.clone()),
    };
    let bindings = AtlasBindings {
        theme: input.theme.map(|t| t.trim().to_owned()),
        time: input.time.map(|t| t.trim().to_owned()),
        srid,
        crs_name: name
            .clone()
            .unwrap_or_else(|| "Yerel (koordinat sistemi yok)".to_owned()),
        geo: name.is_some() && pdf_crs(srid).is_some(),
        project_srid: map.map(|_| settings.srid),
    };
    Ok(AtlasPlanned {
        pages,
        pin,
        bindings,
        fields: fields.clone(),
        withheld,
        filtered_out,
        warnings: warnings
            .into_iter()
            .filter(|w| w.code != "atlas_note")
            .collect(),
        notes,
        sheet: sheet.id.clone(),
        layer: layer.clone(),
        book,
        pictures: pictures.into_iter().collect(),
        plan: planned.pages,
        permit: Permit {
            redacted,
            coverage: Some((layer, fields.into_iter().collect())),
            atlas_box: None,
        },
        window,
        user: input.user.to_owned(),
        date: input.date.to_owned(),
        atlas_maps,
        map,
        boxes,
        staged,
        read: source_read,
    })
}

/// `doc` with its layer `layer`'s objects in place of the rows `rows` (docs/adr/0638 §3): a reading copy, the
/// drawing untouched. An object of another layer with a row's lasting id gives way (the same table imported twice).
fn with_rows(doc: &Document, layer: &str, rows: Vec<(Uuid, Entity)>) -> Result<Document, AtlasError> {
    let mut work = doc.reading_copy();
    let own: Vec<Slot> = work.layer_slots(layer).map(|(_, s)| s).collect();
    work.remove(&own);
    let taken: Vec<Slot> = rows.iter().filter_map(|(u, _)| work.slot_of(*u)).collect();
    work.remove(&taken);
    let rows = rows
        .into_iter()
        .map(|(u, mut e)| {
            e.base_mut().layer_id = layer.to_owned();
            (u, e)
        })
        .collect();
    work.add_many_with_uids(rows, "Kaynaktan oku")
        .map_err(|r| AtlasError::new("pdf_unwritable", r.0, "coverage"))?;
    Ok(work)
}

/// The outer ring of an area's largest part on its ground (curved edges as chords a two-thousandth of its box's
/// diagonal off at most); none for what has no area.
fn outline_of(shape: &Shape, bbox: [f64; 4]) -> Option<Vec<kentos_geometry_core::Vec2>> {
    use kentos_geometry_core::geom::plot_outline;
    let tol = ((bbox[2] - bbox[0]).hypot(bbox[3] - bbox[1]) / 2000.0).max(1e-3);
    let area = |r: &[kentos_geometry_core::Vec2]| {
        let n = r.len();
        (0..n)
            .map(|i| {
                let (a, b) = (r[i], r[(i + 1) % n]);
                a.x * b.y - b.x * a.y
            })
            .sum::<f64>()
            .abs()
    };
    let ring = match shape {
        Shape::Polygon {
            pts, bulges, parts, ..
        } => {
            let mut rings = vec![plot_outline::bulges(pts, bulges.as_deref(), true, tol).ok()?];
            for p in parts.iter().flatten() {
                rings.push(plot_outline::bulges(&p.pts, p.bulges.as_deref(), true, tol).ok()?);
            }
            rings.into_iter().max_by(|a, b| area(a).total_cmp(&area(b)))?
        }
        Shape::Polyline {
            pts,
            bulges,
            closed: true,
            parts: None,
            ..
        } => plot_outline::bulges(pts, bulges.as_deref(), true, tol).ok()?,
        Shape::Circle { .. } | Shape::Ellipse { .. } => match plot_outline::outline(shape, tol) {
            Ok(Some((ring, true))) => ring,
            _ => return None,
        },
        _ => return None,
    };
    (ring.len() >= 3).then_some(ring)
}

/// What the file's keywords say of the output: what it was made from and bound to (docs/adr/0534 §5).
fn keywords(p: &AtlasPlanned) -> String {
    let mut text = format!(
        "kentos.atlas/1; paftalar={}; durum={}; revizyon={}",
        p.pages.len(),
        p.pin.digest,
        p.pin.revision
    );
    for l in &p.pin.data {
        let Some(s) = &l.source else { continue };
        text.push_str(&format!("; kaynak={}:{}", s.id, s.revision));
    }
    if let Some(t) = &p.bindings.theme {
        text.push_str(&format!("; tema={t}"));
    }
    if let Some(t) = &p.bindings.time {
        text.push_str(&format!("; zaman={t}"));
    }
    text.push_str(&format!("; koordinat=EPSG:{}", p.bindings.srid));
    if let Some(from) = p.bindings.project_srid {
        text.push_str(&format!("; proje=EPSG:{from}"));
    }
    text
}

impl AtlasPlanned {
    /// The coverage layer, by id.
    pub fn layer(&self) -> &str {
        &self.layer
    }

    /// The sheet the atlas is of.
    pub fn sheet(&self) -> &str {
        &self.sheet
    }

    /// Makes the PDF; refuses `source_changed` when a bound file changed while the pages were made.
    pub fn pdf(&self, doc: &Document) -> Result<AtlasPdf, AtlasError> {
        self.pdf_with(doc, &|| {})
    }

    /// [`Self::pdf`], with `after` called once the pages are made and before the files are looked at again.
    pub fn pdf_with(&self, doc: &Document, after: &dyn Fn()) -> Result<AtlasPdf, AtlasError> {self.pdf_with_at(doc,None,after)}
    /// The file host supplies the source drawing so relative image versions are frozen too.
    pub fn pdf_at(&self,doc:&Document,drawing:Option<&str>)->Result<AtlasPdf,AtlasError>{self.pdf_with_at(doc,drawing,&||{})}
    fn pdf_with_at(&self,original:&Document,drawing:Option<&str>,after:&dyn Fn())->Result<AtlasPdf,AtlasError>{
        let captured=crate::provenance::Capture::new(original,&self.user,&self.date,drawing).map_err(|e|AtlasError::new(&e.code,e.message,"sources"))?;
        // The pages' drawing: the captured one, its coverage layer the source's rows when read from it.
        let mut work = match &self.staged {
            Some(rows) => with_rows(&captured.doc, &self.layer, rows.clone())?,
            None => captured.doc.reading_copy(),
        };
        // The bound parts the maps draw, read page by page in the page's reach (docs/adr/0638 §3).
        let parts: Vec<SourceLink> = match &self.staged {
            None => Vec::new(),
            Some(_) => work
                .settings()
                .sources
                .iter()
                .filter(|l| {
                    l.extent.is_some()
                        && l.layer != self.layer
                        && self
                            .pin
                            .data
                            .iter()
                            .any(|d| d.layer == l.layer && d.role == AtlasLayerRole::Map)
                })
                .cloned()
                .collect(),
        };
        let mut book = self.book.clone();
        // The sheet's source note says what the pages are made from, when the book keeps it for that.
        let from = format!(
            "{} (çizim durumu {})",
            work.name(),
            self.pin.digest.get(..12).unwrap_or(&self.pin.digest)
        );
        for vars in std::iter::once(&mut book.variables).chain(
            book.sheets
                .iter_mut()
                .filter(|s| s.id == self.sheet)
                .map(|s| &mut s.variables),
        ) {
            for v in vars.iter_mut().filter(|v| v.name == "kaynak") {
                if matches!(&v.value, VarValue::Null)
                    || matches!(&v.value, VarValue::Text(t) if t.trim().is_empty())
                {
                    v.value = VarValue::Text(from.clone());
                }
            }
        }
        let mut spatial = Spatial::new();
        spatial.reload(&work);
        spatial.set_time_window(self.window);
        // A part's band holds room for its labels (docs/adr/0636); rows read from the source are the objects themselves.
        let band_notes = match &self.staged {
            None => crate::book::with_bands(&work, &mut spatial),
            Some(_) => Vec::new(),
        };
        let library = library_of(&work);
        let mut base: RenderInputs = base_inputs(&work, &self.user, &self.date);
        if self.map.is_some() {
            // The title block, the grid and the north say the paper's system (docs/adr/0638 §1).
            base.project.crs_name = self.bindings.crs_name.clone();
            base.crs = Some(
                crate::inputs::crs_info(self.bindings.srid).unwrap_or(kentos_sheet::display::CrsInfo {
                    name: self.bindings.crs_name.clone(),
                    tm: None,
                }),
            );
        }
        let sheet = book
            .sheet(&self.sheet)
            .cloned()
            .ok_or_else(|| AtlasError::new("unknown_sheet", "Pafta kitapta yok.", "sheet"))?;
        let mut notes = self.notes.clone();
        let mut read = self.read.clone();
        // The maps that are not the atlas's draw the same on every page: once.
        let mut shared: Vec<PdfMap> = Vec::new();
        let mut pages: Vec<PdfAtlasPage> = Vec::with_capacity(self.plan.len());
        let list_of = |page: &display::AtlasPageInput| {
            let mut render = base.clone();
            render.atlas = Some(page.clone());
            display::display_list(&book, &self.sheet, &render)
                .map_err(|e| AtlasError::new("pdf_unwritable", e.message, "path"))
        };
        let said = |drawer: MapDrawer<'_>, notes: &mut Vec<String>| -> Result<(), AtlasError> {
            for n in drawer
                .finish()
                .map_err(|e| AtlasError::new(&e.code, e.message, "maps"))?
            {
                if !notes.contains(&n) {
                    notes.push(n);
                }
            }
            Ok(())
        };
        let images = captured.images.clone();
        if parts.is_empty() {
            let mut drawer = self.drawer(&work, &spatial, &library, &images, drawing);
            drawer.notes.extend(band_notes);
            for page in &self.plan {
                let list = list_of(page)?;
                let (maps, data) = self.page_maps(
                    &work, &spatial, &library, &book, &sheet, page, &list, &mut drawer, &mut shared, true,
                );
                pages.push(PdfAtlasPage {
                    atlas: page.clone(),
                    maps,
                    tables: data.tables,
                    coordinates: data.coordinates,
                    legends: data.legends,
                });
            }
            said(drawer, &mut notes)?;
        } else {
            let mut added: Vec<Slot> = Vec::new();
            let mut rows_read = vec![0u64; parts.len()];
            for (k, page) in self.plan.iter().enumerate() {
                let list = list_of(page)?;
                if k == 0 {
                    // The maps that stand still, of the drawing as it is, before any page's rows come in.
                    let mut drawer = self.drawer(&work, &spatial, &library, &images, drawing);
                    self.page_maps(
                        &work, &spatial, &library, &book, &sheet, page, &list, &mut drawer, &mut shared, false,
                    );
                    said(drawer, &mut notes)?;
                }
                work.remove(&added);
                added.clear();
                if let Some(reach) = self.reach_of(&list) {
                    for (i, link) in parts.iter().enumerate() {
                        let (staging, queried) = sources_query::stage(
                            work.settings(),
                            &StageFrom::Layer(link),
                            None,
                            Some(reach),
                            None,
                            "coverage",
                            &|| false,
                        )
                        .map_err(|e| match e {
                            Some(e) => AtlasError::new(&e.code, e.message, "coverage"),
                            None => AtlasError::new("pdf_unwritable", "Okuma durduruldu.", "coverage"),
                        })?;
                        let _ = queried;
                        rows_read[i] += staging.objects.len() as u64;
                        // The part's own objects (edits not yet written among them) stay: the rows they are are left out.
                        let rows: Vec<(Uuid, Entity)> = staging
                            .objects
                            .into_iter()
                            .filter(|(u, _)| work.slot_of(*u).is_none())
                            .map(|(u, mut e)| {
                                e.base_mut().layer_id = link.layer.clone();
                                (u, e)
                            })
                            .collect();
                        added.extend(
                            work.add_many_with_uids(rows, "Kaynaktan oku")
                                .map_err(|r| AtlasError::new("pdf_unwritable", r.0, "coverage"))?,
                        );
                    }
                }
                spatial.sync(&work);
                let mut drawer = self.drawer(&work, &spatial, &library, &images, drawing);
                let (maps, data) = self.page_maps(
                    &work, &spatial, &library, &book, &sheet, page, &list, &mut drawer, &mut shared, true,
                );
                said(drawer, &mut notes)?;
                pages.push(PdfAtlasPage {
                    atlas: page.clone(),
                    maps,
                    tables: data.tables,
                    coordinates: data.coordinates,
                    legends: data.legends,
                });
            }
            for (link, rows) in parts.iter().zip(rows_read) {
                read.push(AtlasRead {
                    layer: link.layer.clone(),
                    table: link.table.clone(),
                    rows,
                    pages: Some(u32::try_from(self.plan.len()).unwrap_or(u32::MAX)),
                });
            }
        }
        let doc = &work;
        if !self.bindings.geo && self.bindings.srid != kentos_project::crs::LOCAL_SRID {
            notes.push(
                "Projenin sistemi dönüşü çekirdekçe bilinen bir TM sistemi değil: PDF'e coğrafi görüntü penceresi (GeoPDF) yazılmadı."
                    .to_owned(),
            );
        }
        let assets = self
            .pictures
            .iter()
            .map(|(sha256, data)| PdfAsset {
                sha256: sha256.clone(),
                data: data.clone(),
                raster: None,
            })
            .collect();
        let mut inputs = PdfInputs {
            render: base,
            fonts: crate::fonts::pdf_fonts(),
            assets,
            maps: shared,
            crs: pdf_crs(self.bindings.srid),
            atlas: Some(PdfAtlas {
                sheet: self.sheet.clone(),
                pages,
            }),
            keywords: Some(format!("{}; snapshot={}",keywords(self),captured.keywords().map_err(|e|AtlasError::new(&e.code,e.message,"sources"))?)),
        };
        let options = PdfOptions {
            sheets: vec![self.sheet.clone()],
            geo: inputs.crs.is_some(),
            layers: true,
        };
        crate::images::svg_assets(&book, &mut inputs, &options, sheet.export.dpi)
            .map_err(|e| AtlasError::new("plot_asset_unreadable", e, "assets"))?;
        let bytes = kentos_sheet::pdf::to_pdf(&book, &inputs, &options)
            .map_err(|e| AtlasError::new("pdf_unwritable", e.message, "path"))?;
        let findings = kentos_sheet::pdf::findings(&book, &inputs, &options)
            .map_err(|e| AtlasError::new("pdf_unwritable", e.message, "path"))?;
        after();
        // The files again: a bound file that is not the one pinned means the pages mix two states.
        let mut changed: Vec<String> = Vec::new();
        for l in &self.pin.data {
            let Some(src) = &l.source else { continue };
            let Some(link) = doc.settings().sources.iter().find(|s| s.id == src.id) else {
                continue;
            };
            if file_of(link) != src.current {
                changed.push(format!("{} ({})", l.name, src.path));
            }
        }
        if !changed.is_empty() {
            return Err(AtlasError::new(
                "source_changed",
                format!(
                    "Sayfalar yapılırken bağlı kaynak değişti: {}. Hiçbir şey yazılmadı; çıktıyı yenileyin.",
                    changed.join("; ")
                ),
                "pin",
            ));
        }
        Ok(AtlasPdf {
            bytes,
            findings,
            notes,
            read,
        })
    }
}

impl AtlasPlanned {
    /// The maps' drawer of `doc` as the pages draw it: the paper's system, the captured pictures.
    fn drawer<'d>(
        &self,
        doc: &'d Document,
        spatial: &'d Spatial,
        library: &'d kentos_native_style::StyleLibrary,
        images: &std::sync::Arc<crate::source_images::SourceImages>,
        drawing: Option<&str>,
    ) -> MapDrawer<'d> {
        let mut d = MapDrawer::new(doc, spatial, library, self.window);
        d.map = self.map;
        d.sources = Some(images.clone());
        d.drawing = drawing.map(str::to_owned);
        d
    }

    /// A page's maps and data: its atlas maps drawn when `atlas`, and the maps that stand still, once.
    #[allow(clippy::too_many_arguments)]
    fn page_maps(
        &self,
        doc: &Document,
        spatial: &Spatial,
        library: &kentos_native_style::StyleLibrary,
        book: &SheetBook,
        sheet: &kentos_sheet::model::Sheet,
        page: &display::AtlasPageInput,
        list: &display::DisplayList,
        drawer: &mut MapDrawer<'_>,
        shared: &mut Vec<PdfMap>,
        atlas: bool,
    ) -> (Vec<PdfMap>, crate::inputs::SheetData) {
        let permit = Permit {
            atlas_box: Some(
                self.boxes
                    .get(&page.feature.id)
                    .copied()
                    .unwrap_or(page.feature.bbox),
            ),
            ..self.permit.clone()
        };
        let data = sheet_data_in(doc, spatial, library, &[], book, sheet, Some(list), &permit);
        let mut maps: Vec<PdfMap> = Vec::new();
        for p in &list.prims {
            let Prim::Map(m) = p else { continue };
            if m.view.center.is_none() {
                continue;
            }
            if self.atlas_maps.contains(&m.item) {
                if atlas && let Some(map) = drawer.draw(book, m, &sheet.id) {
                    maps.push(map);
                }
            } else if !shared.iter().any(|x| x.item == m.item)
                && let Some(map) = drawer.draw(book, m, &sheet.id)
            {
                shared.push(map);
            }
        }
        (maps, data)
    }

    /// The box a page's atlas maps show, on the project's ground: what a bound part is read in for it.
    fn reach_of(&self, list: &display::DisplayList) -> Option<[f64; 4]> {
        let mut out: Option<[f64; 4]> = None;
        for p in &list.prims {
            let Prim::Map(m) = p else { continue };
            if !self.atlas_maps.contains(&m.item) {
                continue;
            }
            let Some(b) = m.extent.and_then(|e| crate::maps::project_box(self.map, e)) else {
                continue;
            };
            out = Some(match out {
                None => b,
                Some(o) => [o[0].min(b[0]), o[1].min(b[1]), o[2].max(b[2]), o[3].max(b[3])],
            });
        }
        out
    }
}

/// The digest of the state `planned` pinned, as the drawing holds it now: a window asks it to know the drawing moved on.
pub fn current_digest(doc: &Document, planned: &AtlasPlanned) -> String {
    let layers: Vec<(String, AtlasLayerRole)> = planned
        .pin
        .data
        .iter()
        .map(|l| (l.layer.clone(), l.role))
        .collect();
    pin_of(doc, &layers, true).digest
}
