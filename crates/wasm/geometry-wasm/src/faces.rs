//! Faces of line work across the boundary (docs/adr/0008, S3): the hatch and
//! "click inside" tools build the faces of the visible lines once per view
//! and ask for the face under the cursor on every pointer move, so the
//! index lives in the core between calls.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{self, FromJson, Json};
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::geom::arrangement::Source;
use kentos_geometry_core::geom::region;
use kentos_geometry_core::ops::areas::line_source;
use wasm_bindgen::prelude::*;

fn read<T: FromJson>(text: &str) -> Result<T, JsError> {
    Json::parse(text)
        .and_then(|v| T::from_json(&v))
        .map_err(|e| JsError::new(&format!("Geometri çekirdeği girdiyi okuyamadı: {e}")))
}

#[wasm_bindgen]
pub struct FaceIndex {
    inner: region::FaceIndex,
}

#[wasm_bindgen]
impl FaceIndex {
    /// Faces of overlay sources (a JSON array, as `faceIndex` takes them).
    #[wasm_bindgen(constructor)]
    pub fn new(sources: &str) -> Result<FaceIndex, JsError> {
        let lines: Vec<Source> = read(sources)?;
        Ok(FaceIndex {
            inner: region::FaceIndex::new(&lines),
        })
    }

    /// Faces of the line work of entities (a JSON array): `lineSource` and
    /// the index in one step, without the source crossing back.
    #[wasm_bindgen(js_name = ofEntities)]
    pub fn of_entities(entities: &str) -> Result<FaceIndex, JsError> {
        let list: Vec<Entity> = read(entities)?;
        Ok(FaceIndex {
            inner: region::FaceIndex::new(&[line_source(&list)]),
        })
    }

    /// The face around (x, y) as JSON (`null` outside every closed shape).
    pub fn at(&self, x: f64, y: f64, islands: bool) -> String {
        json::to_string(&self.inner.at(Vec2::new(x, y), islands))
    }

    /// Every bounded face as JSON.
    pub fn all(&self) -> String {
        json::to_string(&self.inner.all())
    }
}
