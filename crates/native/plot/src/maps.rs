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
    scale: u32,
    extent: [f64; 4],
    labels: &[MapLabel],
) -> Result<MapContent, Vec<(String, Vec<&'static str>)>> {
    let doc = src.doc;
    let scale = f64::from(scale.max(1));
    let pad = 20.0 * scale / 1000.0;
    let reach = [
        extent[0] - pad,
        extent[1] - pad,
        extent[2] + pad,
        extent[3] + pad,
    ];
    let origin = src.origin;
    let look = Look {
        palette: paper_style(),
        symbol_scale: scale,
        screen: false,
        hairlines: false,
        origin,
        view_build: Default::default(),
        view_colors: Default::default(),
        reading_unit: kentos_native_style::reading_unit(doc.settings()),
        line_types: std::sync::Arc::new(kentos_native_style::line_types::LineTypes::of(
            doc.settings(),
        )),
        phasing: std::sync::Arc::new(crate::layers::plot_phasing_of(doc)),
    };
    let clip = Bounds {
        min_x: reach[0],
        min_y: reach[1],
        max_x: reach[2],
        max_y: reach[3],
    };
    let mut layer_names = HashMap::new();
    names(doc.layers().nodes(), &mut layer_names);
    let only = match layers {
        MapLayers::List(l) => Some(&l.layers),
        _ => None,
    };
    let font = sheet_font(
        doc.settings()
            .drawing_font
            .unwrap_or(kentos_contracts::DrawingFont::Barlow),
    );
    let o = VectorScale {
        scale,
        origin: [origin.x, origin.y],
    };
    let mut layer_styles = HashMap::new();
    crate::layers::styles(doc.layers().nodes(), &mut layer_styles);
    let mut shapes: Vec<MapLayerContent> = Vec::new();
    let mut unsupported = Vec::new();
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
        let objects: Vec<&kentos_contracts::Entity> = in_order(doc.by_layer(&node.id).collect());
        if objects.is_empty() {
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
        );
        let (paths, missing) = layer_paths(&styled, &o, Some(reach));
        if !missing.is_empty() {
            unsupported.push((node.name.clone(), missing));
        }
        shapes.push(MapLayerContent {
            id: node.id.clone(),
            name: node.name.clone(),
            paths,
            texts: Vec::new(),
        });
    }
    if !unsupported.is_empty() {
        return Err(unsupported);
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
