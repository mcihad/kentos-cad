//! The faces the visible line work closes, for Alan hesapla's İçine tıkla
//! (the web's `VisibleFaces`, `apps/web/src/tools/visibleFaces.ts`). The
//! faces are the ones İçine tıklayarak alan's and Tarama's command finds
//! (`kentos_native_application::region::faces` over the view's objects,
//! docs/adr/0254 §6), built once and rebuilt only when the view, the drawing
//! (its layers too) or the boundary layer changes.

use kentos_commands::TrimBoundaries;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::FaceIndex;
use kentos_native_application::region;

use crate::Vec2;
use crate::points::wire;
use crate::tool::Context;

/// The faces of the visible line work, and what they were built from: the
/// view, the boundary layer and the drawing's revision.
pub(crate) struct Faces {
    key: ([f64; 4], Option<String>, u64),
    index: FaceIndex,
}

/// The face around `p`, with its closed groups as holes when `islands`;
/// `layer`, when given, is the only one whose line work counts. The index
/// in `cache` is kept while what it was built from stays.
pub(crate) fn face_at(
    cache: &mut Option<Faces>,
    p: Vec2,
    islands: bool,
    layer: Option<&str>,
    cx: &Context<'_>,
) -> Option<Area> {
    let b = cx.view.visible();
    let key = (
        [b.min_x, b.min_y, b.max_x, b.max_y],
        layer.map(str::to_owned),
        cx.doc.revision(),
    );
    if cache.as_ref().is_none_or(|f| f.key != key) {
        let view = TrimBoundaries::Area {
            min: wire(Vec2::new(b.min_x, b.min_y)),
            max: wire(Vec2::new(b.max_x, b.max_y)),
        };
        *cache = Some(Faces {
            key,
            index: region::faces(cx.doc, cx.spatial.store(), Some(&view), layer),
        });
    }
    cache.as_ref().and_then(|f| f.index.at(p, islands))
}
