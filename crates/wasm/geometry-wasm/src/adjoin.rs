//! Bitişik alan across the boundary (docs/adr/0162 §5): the tool takes the
//! neighbours of the visible area once per view and drawing and asks for
//! the region on every pointer move, so they live in the core between calls
//! (as İzle's graph does, `trace.rs`).

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{self, FromJson, Json};
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::ops::adjoin;
use wasm_bindgen::prelude::*;

fn read<T: FromJson>(text: &str) -> Result<T, JsError> {
    Json::parse(text)
        .and_then(|v| T::from_json(&v))
        .map_err(|e| JsError::new(&format!("Geometri çekirdeği girdiyi okuyamadı: {e}")))
}

#[wasm_bindgen]
pub struct AdjoinWork {
    inner: adjoin::Neighbours,
}

#[wasm_bindgen]
impl AdjoinWork {
    /// The neighbours among entities (a JSON array): the ones that enclose
    /// an area (the area tools' kinds); the others are left out.
    #[wasm_bindgen(js_name = ofEntities)]
    pub fn of_entities(entities: &str) -> Result<AdjoinWork, JsError> {
        let list: Vec<Entity> = read(entities)?;
        Ok(AdjoinWork {
            inner: adjoin::Neighbours::of_entities(&list),
        })
    }

    /// How many edges the neighbours have: what the preview cuts through.
    #[wasm_bindgen(js_name = edgeCount)]
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// The region the open path closes with the neighbours as JSON areas:
    /// `xy` its corners (x0, y0, x1, y1, …), `bulges` one per segment or
    /// empty for a straight path.
    pub fn fill(&self, xy: &[f64], bulges: &[f64]) -> String {
        let pts: Vec<Vec2> = xy.chunks_exact(2).map(|c| Vec2::new(c[0], c[1])).collect();
        let bulges = (!bulges.is_empty()).then_some(bulges);
        json::to_string(&self.inner.fill(&pts, bulges))
    }

    /// A new area (JSON) less the neighbours that overlap it, and which
    /// those are, as JSON.
    pub fn avoid(&self, area: &str) -> Result<String, JsError> {
        let area: Area = read(area)?;
        Ok(json::to_string(&self.inner.avoid(&area)))
    }
}
