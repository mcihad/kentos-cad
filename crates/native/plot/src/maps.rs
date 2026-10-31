//! A sheet's map frame as vectors for its PDF (docs/sheet/design.md §9a,
//! `MapContent::Vector`): the shown layers through the style engine at the
//! map's scale on the paper's palette, turned into paths ([`crate::vectors`]),
//! and the drawing's texts as the host laid them out ([`MapLabel`]). The
//! desktop's sheet PDF and the headless host's give the core the same paths
//! (docs/adr/0252).

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_native_style::StylePalette;
use kentos_native_style::library::StyleLibrary;
use kentos_sheet::display::paper;
use kentos_sheet::kinds::MapLayers;
use kentos_sheet::pdf::{MapContent, MapLayerContent, MapPath, MapText, TextAnchor, VectorMap};

use crate::fonts::sheet_font;
use crate::layers::{Look, build_whole, in_order, names};
use crate::vectors::{VectorScale, layer_paths};

/// A CSS pixel on the paper, millimetres: the unit the maps' labels are laid out in (the web's
/// `PX_MM`, app/sheet/mapLabels.ts), so a label is the same size on the screen's sheet and in
/// the PDF.
pub const PX_MM: f64 = 25.4 / 96.0;

/// A label of a sheet's map as the PDF writes it (the web's mapLabels.ts `VecText`): its text
/// anchored on the ground as the drawing area anchors it (the middle of a centred label, a text's
/// baseline start, a dimension value's baseline middle), its turn (degrees, counter-clockwise on
/// the paper), its size in paper mm (the screen's pixels at 100 % are the paper's CSS pixels), its
/// face, its colour and its halo's (`#rrggbb` or `#rrggbbaa`); the paper-coloured box under a
/// masked text (docs/adr/0145), on the ground. The host lays them out (the desktop's labels.rs).
#[derive(Clone, Debug, PartialEq)]
pub struct MapLabel {
    /// The layer of the object it is written for.
    pub layer: String,
    pub text: String,
    pub at: Vec2,
    pub anchor: TextAnchor,
    pub rotation: f64,
    pub size: f64,
    pub weight: u16,
    pub italic: bool,
    /// Its own typeface (docs/adr/0183 §2); none: the project's.
    pub font: Option<kentos_contracts::DrawingFont>,
    pub color: String,
    pub halo: String,
    pub mask: Option<Vec<[f64; 2]>>,
    /// A multi-line text's underlined run's bar, on the ground, in its colour (docs/adr/0182 §3).
    pub underline: Option<Vec<[f64; 2]>>,
    /// A placed label's background or its shadow (docs/adr/0212 §3.8): a ring on the ground filled in `color`.
    pub fill: Option<Vec<[f64; 2]>>,
    /// A placed label's callout or its background's outline: a line on the ground in `color`, closed or not.
    pub stroke: Option<(Vec<[f64; 2]>, bool)>,
    /// A text's width factor and slant's tangent (docs/adr/0324 §6): 1 and 0 for any other label.
    pub width_factor: f64,
    pub lean: f64,
}

/// A label as the PDF writes it (the web's pdfExport.ts `coreText`).
fn text_of(l: &MapLabel, font: &str) -> MapText {
    MapText {
        at: [l.at.x, l.at.y],
        text: l.text.clone(),
        size: l.size,
        rotation: l.rotation,
        color: l.color.clone(),
        // A text's own typeface (docs/adr/0183 §2), else the project's.
        font: l.font.map_or(font, |f| f.id()).to_owned(),
        weight: l.weight,
        italic: l.italic,
        anchor: l.anchor,
        halo: Some(l.halo.clone()),
        width_factor: (l.width_factor != 1.0).then_some(l.width_factor),
        lean: (l.lean != 0.0).then_some(l.lean),
    }
}

/// A masked text's paper-coloured box.
fn mask_of(points: &[[f64; 2]]) -> MapPath {
    MapPath {
        points: points.to_vec(),
        closed: true,
        holes: Vec::new(),
        stroke: None,
        fill: Some(paper::PAPER.to_owned()),
    }
}

/// The paper's palette for the style engine (the web's `paperPalette`): its ink `#111111`.
pub fn paper_style() -> StylePalette {
    StylePalette {
        fg: paper::FG.to_owned(),
        fg_dim: paper::FG_DIM.to_owned(),
        ink: paper::INK.to_owned(),
        paper: paper::PAPER.to_owned(),
    }
}

/// What a map's vectors are built from: the drawing, its geometry store, the style library and
/// the local origin the style engine's numbers are taken from (the drawing area's own).
pub struct MapSource<'a> {
    pub doc: &'a kentos_domain::Document,
    pub store: &'a Store,
    pub library: &'a StyleLibrary,
    pub origin: Vec2,
    /// The paper's system when it is not the project's (docs/adr/0638 §1): the map's ground, its extent and its
    /// origin are in it, the drawing is moved there as it is built. None: the project's.
    pub map: Option<&'static kentos_geometry_core::crs::map::MapProjection>,
    /// The level the map draws (docs/adr/0716): the other levels' objects left out, the one under it faint with
    /// `below`. None: every level.
    pub level: Option<&'a kentos_sheet::kinds::MapLevel>,
    /// A linked view's recipe (docs/adr/0774 §3): its phase view (none: the project's) and the categories left out.
    pub phase: Option<kentos_contracts::PhaseView>,
    pub hidden: &'a [kentos_contracts::ViewCategory],
    /// Its pens and black and white (docs/adr/0783 §1).
    pub pens: &'a [kentos_contracts::RecipePen],
    pub mono: bool,
    /// A linked view's design options (docs/adr/0788 §3); empty: each set's primary.
    pub options: &'a [kentos_contracts::OptionChoice],
}

/// A box of the paper's system as a box of the project's (docs/adr/0638 §1): the store is searched in the project's.
pub(crate) fn project_box(
    map: Option<&'static kentos_geometry_core::crs::map::MapProjection>,
    b: [f64; 4],
) -> Option<[f64; 4]> {
    match map {
        None => Some(b),
        Some(m) => {
            kentos_geometry_core::crs::transform_bounds(&m.map, &m.project, b, 21, &m.choices)
        }
    }
}

/// Styled layers shared by vector PDF and the headless/desktop CPU painter.
pub(crate) fn built_layers_profile(
    src: &MapSource<'_>,
    layers: &MapLayers,
    overrides: &[kentos_sheet::kinds::LayerOverride],
    scale: f64,
    extent: [f64; 4],
    profile: Option<&kentos_sheet::plot_profile::PlotProfile>,
    labels: &[MapLabel],
) -> Result<Vec<(String, String, kentos_native_style::StyledLayer)>, String> {
    let doc = src.doc;
    let scale = scale.max(1e-6);
    let pen_mm = profile
        .into_iter()
        .flat_map(|p| &p.entries)
        .filter_map(|e| e.style.line_weight)
        .map(|w| f64::from(w) / 1000. * 5.)
        .fold(20., f64::max);
    let pad = kentos_sheet::projection::Scale::for_rendering(scale).model_metres_from_mm(pen_mm);
    let reach = [
        extent[0] - pad,
        extent[1] - pad,
        extent[2] + pad,
        extent[3] + pad,
    ];
    // The objects are searched in the project's system; the map's box moved there.
    let (Some(searched), Some(shown)) = (project_box(src.map, reach), project_box(src.map, extent))
    else {
        return Ok(Vec::new());
    };
    let origin = src.origin;
    // A level's plan (docs/adr/0716): the other levels' objects out, the one under it faint.
    let (left_out, faint) = src
        .level
        .map(|l| crate::layers::level_looks(doc, l))
        .unwrap_or_default();
    let mut phasing = crate::layers::map_drawing_of(doc, src.phase, src.hidden, src.pens, src.mono, src.options);
    for &id in &faint {
        phasing.fade(id);
    }
    let look = Look {
        palette: paper_style(),
        symbol_scale: scale,
        screen: false,
        hairlines: false,
        hold_weights: false,
        origin,
        view_build: kentos_native_style::View {
            plot_precision: true,
            map: src.map,
            ..Default::default()
        },
        view_colors: Default::default(),
        reading_unit: kentos_native_style::reading_unit(doc.settings()),
        line_types: std::sync::Arc::new(kentos_native_style::line_types::LineTypes::of(
            doc.settings(),
        )),
        phasing: std::sync::Arc::new(phasing),
    };
    let clip = Bounds {
        min_x: searched[0],
        min_y: searched[1],
        max_x: searched[2],
        max_y: searched[3],
    };
    let mut layer_names = HashMap::new();
    names(doc.layers().nodes(), &mut layer_names);
    let only = match layers {
        MapLayers::List(l) => Some(&l.layers),
        _ => None,
    };
    let mut layer_styles = HashMap::new();
    crate::layers::styles(doc.layers().nodes(), &mut layer_styles);
    // What the box reaches, by layer in the document's order, through the store's tree: a page of an atlas over a
    // large layer reads its own objects, not the layer's every one (docs/adr/0638 §4).
    let mut reaching: HashMap<&str, Vec<&kentos_contracts::Entity>> = HashMap::new();
    for id in src.store.reaching(&clip) {
        if !src.store.view_shown(id) || left_out.contains(&(id as u32)) {
            continue;
        }
        if let Some(e) = doc.get(kentos_domain::Slot(id as u32)) {
            reaching
                .entry(e.base().layer_id.as_str())
                .or_default()
                .push(e);
        }
    }
    let mut built = Vec::new();
    // The map's layer state and overrides (docs/adr/0314 §1): its own layers, the drawing's untouched.
    for node in crate::layers::map_layers(
        doc.layers().nodes(),
        &doc.settings().layer_states,
        layers,
        overrides,
    ) {
        let node: &kentos_contracts::LayerNode = &node;
        if only.is_some_and(|l| !l.contains(&node.id)) {
            continue;
        }
        if node.service.is_some() {
            return Err(format!(
                "“{}” harita servisi henüz baskıya aktarılamıyor; eksik içerikle çıktı üretilmeyecek.",
                node.name
            ));
        }
        // Only what the view shows (docs/adr/0534 §4): an object its layer's filter or the time leaves out is on
        // no map and no paper, and a layer with none of them has no content group in the file.
        let objects: Vec<&kentos_contracts::Entity> =
            in_order(reaching.remove(node.id.as_str()).unwrap_or_default());
        if objects.iter().any(|e| {
            matches!(
                e,
                kentos_contracts::Entity::Raster(_) | kentos_contracts::Entity::PointCloud(_)
            ) && src.store.get(f64::from(e.base().id)).is_none_or(|v| {
                v.bounds.max_x >= shown[0]
                    && v.bounds.min_x <= shown[2]
                    && v.bounds.max_y >= shown[1]
                    && v.bounds.min_y <= shown[3]
            })
        }) {
            return Err(format!(
                "“{}” katmanında henüz baskıya aktarılamayan raster/nokta bulutu var; eksik içerikle çıktı üretilmeyecek.",
                node.name
            ));
        }
        if objects.is_empty() {
            if labels.iter().any(|l| l.layer == node.id) {
                built.push((node.id.clone(), node.name.clone(), Default::default()));
            }
            continue;
        }
        let styled = build_whole(
            src.store,
            src.library,
            node,
            &objects,
            &look,
            clip,
            crate::layers::LayerTables {
                names: &layer_names,
                styles: &layer_styles,
            },
        )?;
        built.push((node.id.clone(), node.name.clone(), styled));
    }
    Ok(built)
}

/// The map's content as vectors (the web's app/sheet/mapContent.ts `vectorMap` and pdfExport.ts
/// `coreLayers`): the shown layers bottom first (only those a layer list names), each through
/// the style engine at the map's scale on the paper's palette, what reaches the map's box with
/// a margin of 20 paper mm; then every layer's text masks and texts over all of them. A layer
/// that draws something with no vector form is named with what (the map then goes as a picture).
pub fn vector_content(
    src: &MapSource<'_>,
    layers: &MapLayers,
    overrides: &[kentos_sheet::kinds::LayerOverride],
    scale: f64,
    extent: [f64; 4],
    labels: &[MapLabel],
) -> Result<MapContent, Vec<(String, Vec<&'static str>)>> {
    vector_content_profile(src, layers, overrides, scale, extent, labels, None)
}
pub fn vector_content_profile(
    src: &MapSource<'_>,
    layers: &MapLayers,
    overrides: &[kentos_sheet::kinds::LayerOverride],
    scale: f64,
    extent: [f64; 4],
    labels: &[MapLabel],
    profile: Option<&kentos_sheet::plot_profile::PlotProfile>,
) -> Result<MapContent, Vec<(String, Vec<&'static str>)>> {
    collect_vector_profile(src, layers, overrides, scale, extent, labels, profile).map_err(|e| {
        match e {
            VectorIssue::Unsupported(why) => why,
            VectorIssue::Invalid(message) => {
                vec![(message, vec!["baskı geometrisi hesaplanamadı"])]
            }
        }
    })
}
/// Missing vector support can use raster; a failed geometry calculation must refuse the export.
#[derive(Debug)]
pub enum VectorIssue {
    Unsupported(Vec<(String, Vec<&'static str>)>),
    Invalid(String),
}
pub fn collect_vector_profile(
    src: &MapSource<'_>,
    layers: &MapLayers,
    overrides: &[kentos_sheet::kinds::LayerOverride],
    scale: f64,
    extent: [f64; 4],
    labels: &[MapLabel],
    profile: Option<&kentos_sheet::plot_profile::PlotProfile>,
) -> Result<MapContent, VectorIssue> {
    let doc = src.doc;
    let scale = scale.max(1e-6);
    let pen_mm = profile
        .into_iter()
        .flat_map(|p| &p.entries)
        .filter_map(|e| e.style.line_weight)
        .map(|w| f64::from(w) / 1000. * 5.)
        .fold(20., f64::max);
    let pad = kentos_sheet::projection::Scale::for_rendering(scale).model_metres_from_mm(pen_mm);
    let reach = [
        extent[0] - pad,
        extent[1] - pad,
        extent[2] + pad,
        extent[3] + pad,
    ];
    let origin = src.origin;
    let font = sheet_font(
        doc.settings()
            .drawing_font
            .unwrap_or(kentos_contracts::DrawingFont::Barlow),
    );
    let o = VectorScale {
        scale,
        origin: [origin.x, origin.y],
    };
    let mut shapes = Vec::new();
    let mut unsupported = Vec::new();
    for (id, name, styled) in
        built_layers_profile(src, layers, overrides, scale, extent, profile, labels)
            .map_err(VectorIssue::Invalid)?
    {
        let (paths, missing) = layer_paths(&styled, &o, Some(reach));
        if !missing.is_empty() {
            unsupported.push((name.clone(), missing));
        }
        shapes.push(MapLayerContent {
            id,
            name,
            paths,
            texts: Vec::new(),
        });
    }
    if !unsupported.is_empty() {
        return Err(VectorIssue::Unsupported(unsupported));
    }
    // The texts and their masks, by layer, over every layer's drawing.
    let writing: Vec<MapLayerContent> = shapes
        .iter()
        .filter_map(|l| {
            let mine: Vec<&MapLabel> = labels.iter().filter(|t| t.layer == l.id).collect();
            if mine.is_empty() {
                return None;
            }
            Some(MapLayerContent {
                id: l.id.clone(),
                name: l.name.clone(),
                // Placed labels' shadows and backgrounds, then the texts' masks, the underlines and
                // the callouts and outlines (docs/adr/0212 §4), all under the texts.
                paths: mine
                    .iter()
                    .filter_map(|t| {
                        t.fill.as_deref().map(|ring| MapPath {
                            points: ring.to_vec(),
                            closed: true,
                            holes: Vec::new(),
                            stroke: None,
                            fill: Some(t.color.clone()),
                        })
                    })
                    .chain(mine.iter().filter_map(|t| t.mask.as_deref().map(mask_of)))
                    .chain(mine.iter().filter_map(|t| {
                        t.stroke.as_ref().map(|(line, closed)| MapPath {
                            points: line.clone(),
                            closed: *closed,
                            holes: Vec::new(),
                            stroke: Some(kentos_sheet::pdf::MapStroke {
                                color: t.color.clone(),
                                width: PX_MM,
                                dash: Vec::new(),
                                cap: kentos_sheet::LineCap::Butt,
                                join: kentos_sheet::LineJoin::Miter,
                            }),
                            fill: None,
                        })
                    }))
                    .chain(mine.iter().filter_map(|t| {
                        t.underline.as_deref().map(|bar| MapPath {
                            points: bar.to_vec(),
                            closed: true,
                            holes: Vec::new(),
                            stroke: None,
                            fill: Some(t.color.clone()),
                        })
                    }))
                    .collect(),
                // A multi-line text's mask has no words (docs/adr/0182 §3).
                texts: mine
                    .iter()
                    .filter(|t| !t.text.is_empty())
                    .map(|t| text_of(t, font))
                    .collect(),
            })
        })
        .collect();
    let shown: Vec<MapLayerContent> = shapes
        .into_iter()
        .filter(|l| !l.paths.is_empty() || writing.iter().any(|w| w.id == l.id))
        .collect();
    Ok(MapContent::Vector(VectorMap {
        layers: shown.into_iter().chain(writing).collect(),
    }))
}
