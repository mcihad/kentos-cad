//! Ağ analizi across the boundary (docs/adr/0209 §12): the network worker
//! builds a network from its own geometry store (the network layers' objects,
//! packed) and asks it again and again; answers are JSON but the way to the
//! cursor, asked on every pointer move, which is numbers.

use kentos_geometry_core::ops::network::NetworkSession;
use wasm_bindgen::prelude::*;

use crate::store::GeometryStore;

fn err(e: String) -> JsError {
    JsError::new(&e)
}

#[wasm_bindgen]
pub struct NetworkGraph {
    inner: NetworkSession,
    buf: Vec<f64>,
}

#[wasm_bindgen]
impl NetworkGraph {
    /// The network of definition `def` (`NetworkDef` JSON) over `store`'s objects: `edges` with their values
    /// (`[[direction | null, [cost values], closed], …]`) and `junctions` with theirs (`[[role, closed], …]`).
    pub fn build(
        store: &GeometryStore,
        def: &str,
        edges: &[f64],
        edge_values: &str,
        junctions: &[f64],
        junction_values: &str,
    ) -> Result<NetworkGraph, JsError> {
        NetworkSession::from_store(
            store.store(),
            def,
            edges,
            edge_values,
            junctions,
            junction_values,
        )
        .map(|inner| NetworkGraph {
            inner,
            buf: Vec::new(),
        })
        .map_err(err)
    }

    /// Its counts and what building it said (JSON).
    pub fn summary(&self) -> String {
        self.inner.summary_json()
    }

    /// The place nearest `(x, y)` within `reach` (JSON, `null` when none).
    pub fn locate(&self, x: f64, y: f64, reach: f64) -> String {
        self.inner.locate_json(x, y, reach)
    }

    /// A route through `stops` (`[[x, y], …]`) not passing `barriers` (JSON).
    pub fn route(
        &mut self,
        stops: &str,
        barriers: &str,
        reach: f64,
        cost: u32,
        reorder: &str,
    ) -> Result<String, JsError> {
        self.inner
            .route_json(stops, barriers, reach, cost as usize, reorder)
            .map_err(err)
    }

    /// Searches from `(x, y)` and keeps the tree for `pathTo`; false when no network is near.
    #[wasm_bindgen(js_name = treeFrom)]
    pub fn tree_from(
        &mut self,
        x: f64,
        y: f64,
        reach: f64,
        cost: u32,
        barriers: &str,
    ) -> Result<bool, JsError> {
        self.inner
            .tree_from(x, y, reach, cost as usize, barriers)
            .map_err(err)
    }

    /// The kept tree's way to `(x, y)`: `[cost, n, x0, y0, …, bulge0, …]`, empty when there is none.
    #[wasm_bindgen(js_name = pathTo)]
    pub fn path_to(&mut self, x: f64, y: f64, reach: f64) -> Vec<f64> {
        let mut buf = std::mem::take(&mut self.buf);
        self.inner.path_to(x, y, reach, &mut buf);
        let out = buf.clone();
        self.buf = buf;
        out
    }

    /// A service area (JSON): its lines and, when `areas`, its areas.
    #[allow(clippy::too_many_arguments)]
    pub fn area(
        &mut self,
        facilities: &str,
        breaks: &str,
        reach: f64,
        cost: u32,
        toward: bool,
        separate: bool,
        barriers: &str,
        trim: f64,
        rings: bool,
        areas: bool,
    ) -> Result<String, JsError> {
        self.inner
            .area_json(
                facilities,
                breaks,
                reach,
                cost as usize,
                toward,
                separate,
                barriers,
                trim,
                rings,
                areas,
            )
            .map_err(err)
    }

    /// From each origin the `k` (below 0: all) cheapest targets within `cutoff` (NaN: none), with their ways when `paths`.
    #[allow(clippy::too_many_arguments)]
    pub fn nearest(
        &mut self,
        origins: &str,
        targets: &str,
        reach: f64,
        k: i32,
        cutoff: f64,
        cost: u32,
        reverse: bool,
        barriers: &str,
        paths: bool,
    ) -> Result<String, JsError> {
        let k = (k >= 0).then_some(k as usize);
        let cutoff = cutoff.is_finite().then_some(cutoff);
        self.inner
            .nearest_json(
                origins,
                targets,
                reach,
                k,
                cutoff,
                cost as usize,
                reverse,
                barriers,
                paths,
            )
            .map_err(err)
    }

    /// A trace of `kind` from `starts` (JSON).
    pub fn trace(
        &self,
        starts: &str,
        barriers: &str,
        reach: f64,
        kind: &str,
    ) -> Result<String, JsError> {
        self.inner
            .trace_json(starts, barriers, reach, kind)
            .map_err(err)
    }

    /// Denetle (JSON).
    pub fn check(&self) -> String {
        self.inner.check_json()
    }
}
