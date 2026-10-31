//! A layer of the drawing through the style engine whole (docs/STYLE.md §6,
//! docs/adr/0090), as the drawing area's scene and a sheet's map frame build
//! it (the web's app/sheet/mapFrames.ts `styleAt` and `buildStyledLayer`):
//! the look every layer's build depends on, the shown layers bottom first,
//! and one layer built at a map's scale on a palette. The desktop's scene
//! keeps its cache over these; the headless host's PDF builds the same
//! (docs/adr/0252).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use kentos_contracts::{Entity, LayerNode, LayerNodeType};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_native_style::StylePalette;
use kentos_native_style::batches::{DecodeOptions, StyledLayer, decode};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, build_layer};

/// How the drawing is drawn: what every layer's build depends on beyond its objects.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub palette: StylePalette,
    /// The scale symbols are compiled at (`symbol_scale_of`).
    pub symbol_scale: f64,
    /// Symbol sizes on the screen (Semboller → Ekranda sabit).
    pub screen: bool,
    /// Line weights hidden (Kalınlık off).
    pub hairlines: bool,
    pub origin: Vec2,
    /// Görünüm kipleri (docs/adr/0195): what the build leaves out, and the colours.
    pub view_build: kentos_native_style::View,
    pub view_colors: kentos_native_style::color::ViewColors,
    /// The unit the layers' renderers read geometry in (`$alan`, `$uzunluk` …):
    /// the project's drawing unit (docs/adr/0165 §2, the 13 Ekim addendum;
    /// `kentos_native_style::reading_unit`), the same in the drawing area, a
    /// sheet's map and the PDF, so a layer is classed alike on all three.
    pub reading_unit: kentos_native_style::ReadingUnit,
    /// The project's line types and line type scale (docs/adr/0313 §3), as the drawing area draws them.
    pub line_types: std::sync::Arc<kentos_native_style::line_types::LineTypes>,
    /// The project's phase view's looks (docs/adr/0346 §2): the hidden objects left out, the dashed and faded drawn so.
    pub phasing: std::sync::Arc<kentos_native_style::phases::Phasing>,
}

/// [`phasing_of`] for paper: the inserts of a PDF underlay set not to plot (Yazdır off, docs/adr/0351 §2) left out
/// too. The sheet's maps and the headless PDF draw with it; the drawing area with [`phasing_of`].
pub fn plot_phasing_of(doc: &kentos_domain::Document) -> kentos_native_style::phases::Phasing {
    let mut out = phasing_of(doc);
    hide_unplotted(doc, &mut out);
    out
}

/// The design options a view with `choices` does not show (docs/adr/0788 §3), hidden in `out`: each set's options
/// but the one chosen, else the primary.
fn hide_options(
    doc: &kentos_domain::Document,
    choices: &[kentos_contracts::OptionChoice],
    out: &mut kentos_native_style::phases::Phasing,
) {
    for slot in doc.option_hidden_slots(choices) {
        out.hide(slot.0);
    }
}

/// The inserts of a PDF underlay set not to plot, hidden in `out`.
fn hide_unplotted(doc: &kentos_domain::Document, out: &mut kentos_native_style::phases::Phasing) {
    let unplotted: std::collections::HashSet<kentos_contracts::BlockId> = doc
        .blocks()
        .iter()
        .filter(|b| b.xref.as_ref().and_then(|x| x.underlay.as_ref()).is_some_and(|u| !u.plot))
        .map(|b| b.id)
        .collect();
    if !unplotted.is_empty() {
        for e in doc.entities() {
            if let Entity::Insert(i) = e
                && unplotted.contains(&i.block)
            {
                out.hide(e.base().id);
            }
        }
    }
}

/// [`plot_phasing_of`] for a map drawn by a view's recipe (docs/adr/0774 §3): its own phase view instead of the
/// project's, and the categories it leaves out hidden; the model untouched.
pub fn map_phasing_of(
    doc: &kentos_domain::Document,
    phase: Option<kentos_contracts::PhaseView>,
    hidden: &[kentos_contracts::ViewCategory],
    options: &[kentos_contracts::OptionChoice],
) -> kentos_native_style::phases::Phasing {
    map_drawing_of(doc, phase, hidden, &[], false, options)
}

/// [`map_phasing_of`] with a recipe's pens and black and white (docs/adr/0783 §1).
pub fn map_drawing_of(
    doc: &kentos_domain::Document,
    phase: Option<kentos_contracts::PhaseView>,
    hidden: &[kentos_contracts::ViewCategory],
    pens: &[kentos_contracts::RecipePen],
    mono: bool,
    options: &[kentos_contracts::OptionChoice],
) -> kentos_native_style::phases::Phasing {
    let mut out = match phase {
        None => {
            let mut own = phase_only(doc);
            hide_unplotted(doc, &mut own);
            own
        }
        Some(view) => {
            let mut own = kentos_native_style::phases::Phasing::of(doc.entities(), view, |u| {
                kentos_domain::Uuid::parse_str(u)
                    .ok()
                    .and_then(|u| doc.slot_of(u))
                    .and_then(|s| doc.get(s))
            });
            hide_unplotted(doc, &mut own);
            own
        }
    };
    hide_options(doc, options, &mut out);
    if !hidden.is_empty() {
        for e in doc.entities() {
            if kentos_contracts::ViewCategory::of_kind(e.kind()).is_some_and(|c| hidden.contains(&c)) {
                out.hide(e.base().id);
            }
        }
    }
    out.with_pens(pens, mono)
}

/// Kat görünümü as a map of `level` shows it (docs/adr/0716): Etkin kat ve alt kat with `below`, else Yalnız etkin kat.
pub fn level_view(level: &kentos_sheet::kinds::MapLevel) -> kentos_interaction::level_view::LevelView {
    use kentos_interaction::level_view::LevelView;
    if level.below { LevelView::Below } else { LevelView::Active }
}

/// What a map of `level` leaves out and draws faint (docs/adr/0716): Kat görünümü's rule (docs/adr/0715) for that level,
/// the level under it faint with `below`. The stairs under it seen from above (docs/adr/0724 §4) are neither: a store
/// of the map's own holds them so ([`shows_from_above`]).
pub fn level_looks(
    doc: &kentos_domain::Document,
    level: &kentos_sheet::kinds::MapLevel,
) -> (std::collections::HashSet<u32>, std::collections::HashSet<u32>) {
    let l = kentos_interaction::level_view::looks(doc, level_view(level), Some(&level.id));
    (l.hidden.into_iter().collect(), l.faded.into_iter().collect())
}

/// Whether a map of `level` shows a stair of the level under it from above (docs/adr/0724 §4): then it draws from a
/// store whose level view is the map's.
pub fn shows_from_above(doc: &kentos_domain::Document, level: &kentos_sheet::kinds::MapLevel) -> bool {
    !kentos_interaction::level_view::looks(doc, level_view(level), Some(&level.id))
        .upper
        .is_empty()
}

/// A colour on white paper as faint as a faded object (`PHASE_FADED`, docs/adr/0716): its mix with white, its own
/// opacity kept.
pub fn faint_on_paper(hex: &str) -> String {
    let p = kentos_native_style::color::parse_hex(hex, 1.0);
    let k = kentos_contracts::PHASE_FADED;
    let mix = |c: f64| ((c * (1.0 - k) + k) * 255.0).round() as u8;
    let a = (p[3] * 255.0).round() as u8;
    if a == 255 {
        format!("#{:02x}{:02x}{:02x}", mix(p[0]), mix(p[1]), mix(p[2]))
    } else {
        format!("#{:02x}{:02x}{:02x}{a:02x}", mix(p[0]), mix(p[1]), mix(p[2]))
    }
}

/// The phase view's looks of `doc`'s objects (docs/adr/0346 §2), the design options not shown hidden too
/// (docs/adr/0788 §3: each set's primary is shown); nothing to change in Tümü without options.
pub fn phasing_of(doc: &kentos_domain::Document) -> kentos_native_style::phases::Phasing {
    let mut out = phase_only(doc);
    hide_options(doc, &[], &mut out);
    out
}

/// The phase view's looks alone.
fn phase_only(doc: &kentos_domain::Document) -> kentos_native_style::phases::Phasing {
    let view = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.phase_view)
        .unwrap_or_default();
    kentos_native_style::phases::Phasing::of(doc.entities(), view, |u| {
        kentos_domain::Uuid::parse_str(u)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| doc.get(s))
    })
}

/// The shown layers a sheet prints, bottom first: [`shown_layers`] without
/// those a node not printed holds (Yazdır off, docs/adr/0260 §4).
pub fn printed_layers(nodes: &[LayerNode]) -> Vec<&LayerNode> {
    fn walk<'a>(nodes: &'a [LayerNode], on: bool, out: &mut Vec<&'a LayerNode>) {
        for node in nodes {
            let on = on && node.visible && node.plot != Some(false) && node.frozen != Some(true);
            match node.kind {
                LayerNodeType::Layer => {
                    if on {
                        out.push(node);
                    }
                }
                LayerNodeType::Group => walk(&node.children, on, out),
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, true, &mut out);
    out.reverse();
    out
}

/// The layers one map of a sheet prints, bottom first (docs/adr/0314 §1): the map's layer state (`MapLayers::Theme`,
/// by name) in place of the drawing's visibility, freezing, printing and, when the state kept them, looks; then the
/// map's overrides (frozen, printed or not, a colour, a line type or pattern, a weight). The drawing's layers stay as
/// they are; a layer drawn otherwise is a copy. `MapLayers::List` is filtered here too, including labels and their obstacles.
pub use kentos_sheet::layers::map_layers;

/// A layer's objects in its drawing order (docs/adr/0260 §5): by their
/// order, absent 0, equals in the document's order.
pub fn in_order(mut list: Vec<&kentos_contracts::Entity>) -> Vec<&kentos_contracts::Entity> {
    if list.iter().any(|e| e.base().look.order.is_some()) {
        list.sort_by_key(|e| e.base().look.order.unwrap_or(0));
    }
    list
}

/// The shown layers, bottom first (the top of the tree draws last), as the plain scene orders them.
pub fn shown_layers(nodes: &[LayerNode]) -> Vec<&LayerNode> {
    fn walk<'a>(nodes: &'a [LayerNode], visible: bool, out: &mut Vec<&'a LayerNode>) {
        for node in nodes {
            match node.kind {
                LayerNodeType::Layer => {
                    if visible && node.visible && node.frozen != Some(true) {
                        out.push(node);
                    }
                }
                // A frozen group's are frozen with it (docs/adr/0311 §1).
                LayerNodeType::Group => walk(
                    &node.children,
                    visible && node.visible && node.frozen != Some(true),
                    out,
                ),
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, true, &mut out);
    out.reverse();
    out
}

/// Every layer's name by id (`$katman`).
pub fn names(nodes: &[LayerNode], out: &mut HashMap<String, String>) {
    for n in nodes {
        out.insert(n.id.clone(), n.name.clone());
        names(&n.children, out);
    }
}

/// Every layer's look by id: what a block's piece that follows its own layer draws with (docs/adr/0260 §1).
pub fn styles(nodes: &[LayerNode], out: &mut HashMap<String, kentos_contracts::LayerStyle>) {
    for n in nodes {
        out.insert(n.id.clone(), n.style.clone());
        styles(&n.children, out);
    }
}

/// The layer tree's tables a build reads: every layer's name (`$katman`) and look (a block's piece that follows its
/// own layer, docs/adr/0260 §1), by id.
#[derive(Clone, Copy)]
pub struct LayerTables<'a> {
    pub names: &'a HashMap<String, String>,
    pub styles: &'a HashMap<String, kentos_contracts::LayerStyle>,
}

/// One layer through the style engine whole, as a sheet's map frame draws it (the web's
/// app/sheet/mapFrames.ts `styleAt` and `buildStyledLayer`): its objects' symbols at `look`'s
/// scale and palette, its construction lines clipped to `clip`; empty where the engine refuses it.
pub fn build_whole(
    store: &Store,
    library: &StyleLibrary,
    node: &LayerNode,
    entities: &[&Entity],
    look: &Look,
    clip: Bounds,
    tables: LayerTables<'_>,
) -> Result<StyledLayer,String> {
    let names = tables.names;
    let opts = BuildOptions {
        line_types: Some(&look.line_types),
        layer_styles: Some(tables.styles),
        origin: look.origin,
        plot_scale: look.symbol_scale,
        screen: look.screen,
        hairlines: look.hairlines,
        clip: Some(clip),
        library,
        layer_name: &|id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_owned()),
        view: look.view_build,
        frame: sheet_frame(node, look.symbol_scale),
        reading_unit: look.reading_unit,
    };
    let phased = kentos_native_style::phases::phased(entities, &look.phasing);
    let entities: Vec<&Entity> = phased.iter().map(|e| e.as_ref()).collect();
    build_layer(store, &node.style, &entities, &opts)
        .and_then(|(_, batches)| {
            decode(
                batches,
                &DecodeOptions {
                    palette: &look.palette,
                    plot_scale: look.symbol_scale,
                    library,
                    view: look.view_colors,
                },
            )
        })
}

static SHEET_MADE: AtomicU64 = AtomicU64::new(0);

/// A layer's build on a sheet's map (the web's `layerStyleAt`): a view-dependent renderer's
/// (docs/adr/0213 §3) at the paper's CSS px per ground metre (96 dpi over the scale), a heat map's
/// picture under a key of its own.
fn sheet_frame(node: &LayerNode, scale: f64) -> Option<(f64, String)> {
    let needs = kentos_native_style::renderer::needs_of(node.style.renderer.as_ref());
    (needs.scale || needs.frame.is_some()).then(|| {
        (
            96_000.0 / (25.4 * scale.max(1.0)),
            format!(
                "heat:sheet:{}:{}",
                node.id,
                SHEET_MADE.fetch_add(1, Ordering::Relaxed) + 1
            ),
        )
    })
}
