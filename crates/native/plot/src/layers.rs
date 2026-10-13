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
}

/// The shown layers, bottom first (the top of the tree draws last), as the plain scene orders them.
pub fn shown_layers(nodes: &[LayerNode]) -> Vec<&LayerNode> {
    fn walk<'a>(nodes: &'a [LayerNode], visible: bool, out: &mut Vec<&'a LayerNode>) {
        for node in nodes {
            match node.kind {
                LayerNodeType::Layer => {
                    if visible && node.visible {
                        out.push(node);
                    }
                }
                LayerNodeType::Group => walk(&node.children, visible && node.visible, out),
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
    names: &HashMap<String, String>,
) -> StyledLayer {
    let opts = BuildOptions {
        origin: look.origin,
        plot_scale: look.symbol_scale,
        screen: look.screen,
        hairlines: look.hairlines,
        clip: Some(clip),
        library,
        layer_name: &|id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_owned()),
        view: look.view_build,
        frame: sheet_frame(node, look.symbol_scale),
    };
    build_layer(store, &node.style, entities, &opts)
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
        .unwrap_or_default()
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
