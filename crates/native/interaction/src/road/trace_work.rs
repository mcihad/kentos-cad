//! İzle's graph of the visible line work (docs/adr/0161 §1, §5) for the
//! path tools: the web's `VisibleTrace` (`apps/web/src/tools/visibleTrace.ts`).
//! It is built once and rebuilt only when the view or the drawing changes,
//! as the faces are (`faces.rs`).

use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::trace::{TraceGraph, Traced, traced_kind};

use crate::Vec2;
use crate::tool::Context;

/// The graph and what it was built from: the view and the drawing's revision.
pub(crate) struct Work {
    key: ([f64; 4], u64),
    graph: TraceGraph,
}

/// A path tool's graph, which is neither `Clone` nor `Debug`: a copy of the
/// tool starts without it.
#[derive(Default)]
pub(crate) struct WorkCache(Option<Work>);

impl Clone for WorkCache {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl std::fmt::Debug for WorkCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkCache")
    }
}

fn build(b: &Bounds, cx: &Context<'_>) -> TraceGraph {
    let lines: Vec<CoreEntity> = cx
        .spatial
        .store()
        .overlapping(b, None)
        .into_iter()
        .filter(|it| traced_kind(&it.shape))
        .map(|it| CoreEntity::new(it.shape.clone()))
        .collect();
    TraceGraph::of_entities(&lines)
}

impl WorkCache {
    /// The graph of what the view shows, built again when it is stale.
    fn graph(&mut self, cx: &Context<'_>) -> &TraceGraph {
        let b = cx.view.visible();
        let key = ([b.min_x, b.min_y, b.max_x, b.max_y], cx.doc.revision());
        if self.0.as_ref().is_some_and(|w| w.key != key) {
            self.0 = None;
        }
        &self
            .0
            .get_or_insert_with(|| Work {
                key,
                graph: build(&b, cx),
            })
            .graph
    }

    /// The shortest way from `a` to `b` along the visible line work.
    pub(crate) fn path(&mut self, a: Vec2, b: Vec2, cx: &Context<'_>) -> Option<Traced> {
        self.graph(cx).path(a, b)
    }

    /// The point of the visible line work nearest to `p` within `reach` metres.
    pub(crate) fn nearest(&mut self, p: Vec2, reach: f64, cx: &Context<'_>) -> Option<Vec2> {
        self.graph(cx).nearest(p, reach)
    }
}
