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
#[derive(Clone,Debug,PartialEq)]
pub struct UnicodeLabel {
    pub text: std::rc::Rc<kentos_interaction::spatial::UnicodeText>,
    pub scale: f64,
    pub rotation: f64,
    pub colors: Vec<Option<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapLabel {
    /// The exact glyph plan; absent for legacy scalar text.
    pub unicode: Option<UnicodeLabel>,
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
        glyphs: glyphs_of(l),
        at: [l.at.x, l.at.y],
        text: pdf_text(l),
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

/// Stack separators are authoring syntax, not visible PDF characters.
fn pdf_text(l: &MapLabel) -> String {
    let Some(u) = &l.unicode else {
        return l.text.clone();
    };
    l.text
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if matches!(c, '#' | '^')
                && u.text
                    .runs
                    .get(u.text.runs.partition_point(|r| r.end <= i as u32))
                    .is_some_and(|r| {
                        r.script == Some(kentos_geometry_core::text::paragraph::Script::Stack)
                            && r.start <= i as u32
                    })
            {
                if c == '#' { '/' } else { ' ' }
            } else {
                c
            }
        })
        .collect()
}

/// Convert the prepared model-relative plan to paper-relative em matrices.
/// The world anchor never participates in these small-coordinate calculations.
fn glyphs_of(l: &MapLabel) -> Option<kentos_sheet::pdf::GlyphText> {
    use kentos_geometry_core::geom::affine;
    use kentos_sheet::pdf::{GlyphText, PdfGlyph, PdfGlyphRule};
    let u = l.unicode.as_ref()?;
    let plan = u.text.plan.as_ref().ok()?;
    let k = l.size / u.text.height * u.scale;
    let outer = affine::compose(
        &affine::rotation(u.rotation, Vec2::new(0., 0.)),
        &affine::scaling(k, Vec2::new(0., 0.)),
    );
    let color = |scalar| {
        let i = u.text.runs.partition_point(|r| r.end <= scalar);
        u.text
            .runs
            .get(i)
            .filter(|r| r.start <= scalar)
            .and_then(|_| u.colors.get(i))
            .and_then(|c| c.clone())
            .unwrap_or_else(|| l.color.clone())
    };
    let owners = glyph_owners(&plan.glyphs);
    let glyphs = plan
        .glyphs
        .iter()
        .enumerate()
        .map(|(index, placed)| {
            let g = &placed.glyph;
            let em = [
                g.size,
                0.,
                g.slant * g.size,
                g.size,
                g.origin[0],
                g.origin[1],
            ];
            let matrix = affine::compose(&outer, &affine::compose(&placed.transform, &em));
            let mut text = if owners.get(&(g.bytes.start, g.bytes.end)) == Some(&index) {
                u.text.source[g.bytes.clone()].to_owned()
            } else {
                String::new()
            };
            if text == "#"
                && u.text
                    .runs
                    .get(u.text.runs.partition_point(|r| r.end <= g.scalars.start))
                    .is_some_and(|r| {
                        r.script == Some(kentos_geometry_core::text::paragraph::Script::Stack)
                            && r.start <= g.scalars.start
                            && g.scalars.end <= r.end
                    })
            {
                text = "/".into();
            }
            PdfGlyph {
                font: g.font.0,
                glyph: g.id,
                text,
                matrix,
                color: color(g.scalars.start),
            }
        })
        .collect();
    let rules = plan
        .rules
        .iter()
        .map(|r| PdfGlyphRule {
            corners: r.corners.map(|[x, y]| {
                let p = affine::apply(&outer, Vec2::new(x, y));
                [p.x, p.y]
            }),
            color: u
                .text
                .runs
                .get(r.run)
                .map_or_else(|| l.color.clone(), |r| color(r.start)),
        })
        .collect();
    Some(GlyphText {
        profile: "unicode-v1".into(),
        glyphs,
        rules,
    })
}

/// ToUnicode belongs to the advancing body of a cluster. HarfBuzz can emit
/// an Arabic dot/mark before that body; assigning the letter to that zero-width
/// mark makes PDFKit infer spaces inside a word. Geometry and paint order stay
/// exactly as shaped. All-zero clusters keep their first glyph as the owner.
fn glyph_owners(
    glyphs: &[kentos_geometry_core::text::shaping::placed::PlacedGlyph],
) -> HashMap<(usize, usize), usize> {
    let mut owners: HashMap<_, usize> = HashMap::new();
    for (index, placed) in glyphs.iter().enumerate() {
        let g = &placed.glyph;
        owners
            .entry((g.bytes.start, g.bytes.end))
            .and_modify(|old| {
                if g.advance.abs() > glyphs[*old].glyph.advance.abs() {
                    *old = index;
                }
            })
            .or_insert(index);
    }
    owners
}

#[cfg(test)]
mod glyph_owner_tests {
    #[test]
    fn arabic_source_letters_belong_to_the_body_not_its_preceding_dots() {
        use kentos_geometry_core::text::shaping::{self, Style, placed::PlacedGlyph};
        let line = shaping::line("العربية", Style {family:"arimo",..Style::default()}).unwrap();
        let glyphs:Vec<_> = line.glyphs.into_iter().map(|glyph|PlacedGlyph {
            glyph,transform:[1.,0.,0.,1.,0.,0.],
        }).collect();
        let owners=super::glyph_owners(&glyphs);
        let mut actual:Vec<_>=owners.values().map(|&i| {
            let g=&glyphs[i].glyph;(g.scalars.start,g.id)
        }).collect();actual.sort();
        // Independent C HarfBuzz 15.0.0, frozen NotoSansArabic-400 face:
        // body GIDs in logical order. GIDs 278/309/307 are preceding dots,
        // and must not receive the source letter's searchable bounding box.
        assert_eq!(actual,[(0,8),(1,72),(2,48),(3,31),(4,19),(5,18),(6,83)]);
        assert_eq!(glyphs.len(),10);
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
    clip: Option<&[[f64; 2]]>,
) -> Result<Vec<(String, String, kentos_native_style::StyledLayer)>, String> {
    let built = built_layers_profile_unchecked(src, layers, overrides, scale, extent, profile, labels)?;
    let losses = emitted_line_type_losses(&built, scale, extent, src.origin, clip);
    if !losses.is_empty() {
        return Err(format!("{} Eksik çizgi deseniyle çıktı üretilmedi.", losses.join("\n")));
    }
    Ok(built)
}

/// Uses the same resolved primitives as delivery, including BYBLOCK, layer
/// overrides, custom symbols, filters and hidden expanded pieces.
pub(crate) fn line_type_losses(
    src: &MapSource<'_>,
    map: &kentos_sheet::display::MapPrim,
) -> Result<Vec<String>, String> {
    let Some(extent) = map.extent else { return Ok(Vec::new()); };
    if !line_type_check_needed(src.doc, map, src.library) {
        return Ok(Vec::new());
    }
    let built = built_layers_profile_unchecked(src, &map.layers, &map.layer_overrides,
        map.view.scale, extent, None, &[])?;
    let clip = ground_clip(map);
    Ok(emitted_line_type_losses(&built, map.view.scale, extent, src.origin, clip.as_deref()))
}

/// Cheap source gate, not the finding itself. The ordinary built-in/short
/// pattern project must not compile its map a second time for this check.
pub(crate) fn line_type_check_needed(
    doc: &kentos_domain::Document, map: &kentos_sheet::display::MapPrim, library: &StyleLibrary,
) -> bool {
    let types =
        kentos_native_style::line_types::LineTypes::with_resources(doc.settings(), doc.styles());
    if types.needs_preflight() {
        return true;
    }
    // Geometric-width dashes are split before filling and can reject an
    // excessive/dotted recipe. Inspect their actual emitted diagnostics too.
    let wide = |e: &kentos_contracts::Entity| matches!(e,
        kentos_contracts::Entity::Polyline(p) if p.widths.as_ref().is_some_and(|w|w.iter().any(|[a,b]|*a>0. || *b>0.)));
    if doc.entities().any(wide) || doc.blocks().iter().flat_map(|b|&b.entities).any(wide) { return true; }

    let bad = |name: Option<&str>| name.is_some_and(|n| types.loss(n).is_some());
    fn long_dash(v: &serde_json::Value) -> bool {
        match v {
            serde_json::Value::Object(m) => m.get("lineTypeLoss").is_some() || m.get("cadMarks").is_some()
                || m.get("dash").and_then(serde_json::Value::as_array)
                    .is_some_and(|d| d.len() > 4096)
                || m.values().any(long_dash),
            serde_json::Value::Array(a) => a.iter().any(long_dash),
            _ => false,
        }
    }
    fn layers(nodes: &[kentos_contracts::LayerNode], bad: &impl Fn(Option<&str>)->bool) -> bool {
        nodes.iter().any(|n| bad(n.style.line_pattern.as_deref())
            || n.style.renderer.as_ref().is_some_and(long_dash) || layers(&n.children,bad))
    }
    layers(doc.layers().nodes(), &bad)
        || doc.entities().any(|e| bad(e.base().look.line_pattern.as_deref()))
        || doc.blocks().iter().flat_map(|b| &b.entities).any(|e| bad(e.base().look.line_pattern.as_deref()))
        || doc.settings().layer_states.iter().flat_map(|s| &s.nodes)
            .any(|n| n.style.as_ref().is_some_and(|s|bad(s.line_pattern.as_deref()) || s.renderer.as_ref().is_some_and(long_dash)))
        || map.layer_overrides.iter().any(|o|bad(o.line_pattern.as_deref()))
        || library.items(None).iter().any(|(item,_)|item.symbol().is_some_and(long_dash))
}

/// The map's visible ground (docs/adr/1007): its clip shape (the frame, a rectangle or a polygon) on its turned
/// content, in world coordinates. None for a map with no centre.
pub fn ground_clip(map: &kentos_sheet::display::MapPrim) -> Option<Vec<[f64; 2]>> {
    let center = map.view.center?;
    let t = kentos_sheet::projection::Transform::paper(&map.clip, center, map.view.scale.max(1e-6), map.view.rotation);
    Some(map.clip_shape.paper_points(&map.clip).into_iter().map(|p| t.inverse(p)).collect())
}

/// Whether the box `[x0, y0, x1, y1]` and the simple polygon `ring` share a point.
fn box_meets_ring(b: [f64; 4], ring: &[[f64; 2]]) -> bool {
    let inside_box = |p: &[f64; 2]| p[0] >= b[0] && p[0] <= b[2] && p[1] >= b[1] && p[1] <= b[3];
    if ring.iter().any(inside_box) {
        return true;
    }
    let inside_ring = |x: f64, y: f64| {
        let mut inside = false;
        for (i, a) in ring.iter().enumerate() {
            let c = ring[(i + 1) % ring.len()];
            if (a[1] > y) != (c[1] > y) && x < a[0] + (y - a[1]) * (c[0] - a[0]) / (c[1] - a[1]) {
                inside = !inside;
            }
        }
        inside
    };
    if [[b[0], b[1]], [b[2], b[1]], [b[2], b[3]], [b[0], b[3]]].iter().any(|p| inside_ring(p[0], p[1])) {
        return true;
    }
    let cross = |p: [f64; 2], q: [f64; 2], r: [f64; 2], s: [f64; 2]| {
        let d = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        let (d1, d2, d3, d4) = (d(r, s, p), d(r, s, q), d(p, q, r), d(p, q, s));
        (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
    };
    let corners = [[b[0], b[1]], [b[2], b[1]], [b[2], b[3]], [b[0], b[3]]];
    ring.iter().enumerate().any(|(i, a)| {
        let c = ring[(i + 1) % ring.len()];
        (0..4).any(|k| cross(*a, c, corners[k], corners[(k + 1) % 4]))
    })
}

fn emitted_line_type_losses(
    built: &[(String, String, kentos_native_style::StyledLayer)],
    scale: f64,
    extent: [f64; 4],
    origin: Vec2,
    clip: Option<&[[f64; 2]]>,
) -> Vec<String> {
    // The clip in the batches' origin-relative metres: a loss outside the visible shape is not delivered.
    let ring: Option<Vec<[f64; 2]>> = clip.map(|c| c.iter().map(|p| [p[0] - origin.x, p[1] - origin.y]).collect());
    let seen = [extent[0]-origin.x, extent[1]-origin.y, extent[2]-origin.x, extent[3]-origin.y];
    let px_per_m = 1. / (scale.max(1e-6) * kentos_native_style::program::METRES_PER_PX);
    let ring = ring.as_deref();
    built.iter().flat_map(|(_, name, layer)| layer.batches.iter().filter_map(move |b| {
        b.line_type_loss.as_ref().filter(|_| {
            let ink = match &b.kind {
                kentos_native_style::batches::BatchKind::Stroke { color, .. } => color[3] > 0.,
                kentos_native_style::batches::BatchKind::Fill { paint: kentos_native_style::FillPaintBatch::Hatch { color, .. } } => color[3] > 0.,
                _ => false,
            };
            ink && b.in_scale(scale) && b.in_view(seen, px_per_m, 1.)
                && ring.is_none_or(|r| r.len() < 3 || box_meets_ring(b.reach_bounds(px_per_m, 1.), r))
        })
            .map(|loss| format!("{name}: {loss}"))
    })).collect::<std::collections::BTreeSet<_>>().into_iter().collect()
}

fn built_layers_profile_unchecked(
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
            // CAD line types along the source curve, as the screen (docs/adr/1122).
            cad_phase: true,
            ..Default::default()
        },
        view_colors: Default::default(),
        reading_unit: kentos_native_style::reading_unit(doc.settings()),
        line_types: std::sync::Arc::new(
            kentos_native_style::line_types::LineTypes::at_scale_with_resources(
                doc.settings(),
                doc.styles(),
                scale,
            ),
        ),
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
    vector_content_clipped(src, layers, overrides, scale, extent, labels, profile, None)
}

/// [`vector_content_profile`] with the map's visible ground (docs/adr/1007).
#[allow(clippy::too_many_arguments)]
pub fn vector_content_clipped(
    src: &MapSource<'_>,
    layers: &MapLayers,
    overrides: &[kentos_sheet::kinds::LayerOverride],
    scale: f64,
    extent: [f64; 4],
    labels: &[MapLabel],
    profile: Option<&kentos_sheet::plot_profile::PlotProfile>,
    clip: Option<&[[f64; 2]]>,
) -> Result<MapContent, Vec<(String, Vec<&'static str>)>> {
    collect_vector_clipped(src, layers, overrides, scale, extent, labels, profile, clip).map_err(|e| {
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
    collect_vector_clipped(src, layers, overrides, scale, extent, labels, profile, None)
}

/// [`collect_vector_profile`] with the map's visible ground (docs/adr/1007): a line type loss outside it does not
/// refuse the delivery.
#[allow(clippy::too_many_arguments)]
pub fn collect_vector_clipped(
    src: &MapSource<'_>,
    layers: &MapLayers,
    overrides: &[kentos_sheet::kinds::LayerOverride],
    scale: f64,
    extent: [f64; 4],
    labels: &[MapLabel],
    profile: Option<&kentos_sheet::plot_profile::PlotProfile>,
    clip: Option<&[[f64; 2]]>,
) -> Result<MapContent, VectorIssue> {
    for l in labels {
        if let Some(u) = &l.unicode
            && let Err(error) = &u.text.plan {
            return Err(VectorIssue::Invalid(error.message(&u.text.source)));
        }
    }
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
        built_layers_profile(src, layers, overrides, scale, extent, profile, labels, clip)
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
                                dash_offset: None,
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
