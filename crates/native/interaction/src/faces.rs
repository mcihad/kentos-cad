//! The faces the visible line work closes: the web's `VisibleFaces`
//! (`apps/web/src/tools/visibleFaces.ts`), for the tools that fill or make an
//! area by a click inside (Tarama by lines, İçine tıklayarak alan). The
//! faces are the shared core's (`FaceIndex`), built once and rebuilt only
//! when the view, the drawing (its layers too) or the boundary layer changes.

use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::FaceIndex;
use kentos_geometry_core::ops::areas::line_source;

use crate::Vec2;
use crate::spatial::slot;
use crate::tool::Context;

/// Kinds whose line work bounds faces: fills, texts and dimensions do not
/// (the web's `isBoundaryKind`).
fn bounds_faces(shape: &Shape) -> bool {
    !matches!(
        shape,
        Shape::Point { .. } | Shape::Text { .. } | Shape::Dimension { .. } | Shape::Hatch { .. }
    )
}

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
        let doc = &*cx.doc;
        let lines: Vec<CoreEntity> = cx
            .spatial
            .store()
            .overlapping(&b, None)
            .into_iter()
            .filter(|it| bounds_faces(&it.shape))
            .filter(|it| {
                layer.is_none_or(|layer| {
                    slot(it.id)
                        .and_then(|s| doc.get(s))
                        .is_some_and(|e| e.base().layer_id == layer)
                })
            })
            .map(|it| CoreEntity::new(it.shape.clone()))
            .collect();
        *cache = Some(Faces {
            key,
            index: FaceIndex::new(&[line_source(&lines)]),
        });
    }
    cache.as_ref().and_then(|f| f.index.at(p, islands))
}
