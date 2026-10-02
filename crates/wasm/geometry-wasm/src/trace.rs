//! İzle across the boundary (docs/adr/0161 §5): the path tools build the
//! trace graph of the visible line work once per view and ask for the way to
//! the cursor on every pointer move, so the graph lives in the core between
//! calls (as the faces do, `faces.rs`).

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{self, FromJson, Json};
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::ops::trace;
use wasm_bindgen::prelude::*;

fn read<T: FromJson>(text: &str) -> Result<T, JsError> {
    Json::parse(text)
        .and_then(|v| T::from_json(&v))
        .map_err(|e| JsError::new(&format!("Geometri çekirdeği girdiyi okuyamadı: {e}")))
}

#[wasm_bindgen]
pub struct TraceGraph {
    inner: trace::TraceGraph,
}

#[wasm_bindgen]
impl TraceGraph {
    /// The graph of the line work of entities (a JSON array); the kinds İzle
    /// does not follow are left out.
    #[wasm_bindgen(js_name = ofEntities)]
    pub fn of_entities(entities: &str) -> Result<TraceGraph, JsError> {
        let list: Vec<Entity> = read(entities)?;
        Ok(TraceGraph {
            inner: trace::TraceGraph::of_entities(&list),
        })
    }

    /// The shortest way from (ax, ay) to (bx, by) along the line work as
    /// JSON (`null` when there is none).
    pub fn path(&self, ax: f64, ay: f64, bx: f64, by: f64) -> String {
        json::to_string(&self.inner.path(Vec2::new(ax, ay), Vec2::new(bx, by)))
    }

    /// The point of the line work nearest to (x, y) within `reach` metres as
    /// JSON (`null` when none is that near).
    pub fn nearest(&self, x: f64, y: f64, reach: f64) -> String {
        json::to_string(&self.inner.nearest(Vec2::new(x, y), reach))
    }
}
