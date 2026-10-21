//! Ağ analizi for Python (`kentos.network`, docs/adr/0209 §11): a project's
//! network built from an open drawing as the İşlemler tools build it
//! (`kentos_processing::network::from_document`), then asked as the apps ask
//! it: a place on the network, a route, a service area, the nearest targets
//! (and a cost matrix), a trace and Denetle. The answers cross as the geometry
//! core's JSON (the web's network worker sends the same); a refusal is a
//! `HostError` with the apps' words. Nothing here writes: Python writes what it
//! keeps with `cad.entities.create`.

use kentos_commands::NetworkDef;
use kentos_geometry_core::api::json::{ToJson, to_string};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::ops::network::{NetworkSession, Refusal, Reorder, TraceKind, points_of};
use kentos_geometry_core::vec2::Vec2;
use kentos_headless::HeadlessError;
use kentos_processing::network::{FromDocument, from_document};
use pyo3::prelude::*;

use super::{PySession, host, text};

fn refused(code: &'static str, message: impl Into<String>) -> PyErr {
    host(HeadlessError::new(code, message.into()))
}

/// Points as Python sends them: `[[x, y], …]`.
fn points(json: &str, what: &str) -> PyResult<Vec<Vec2>> {
    points_of(json).map_err(|e| refused("invalid_input", format!("{what}: {e}")))
}

/// An answer as JSON, or its refusal in the apps' words (the search distance in metres).
fn answer<T: ToJson>(r: Result<T, Refusal>, reach: f64) -> PyResult<String> {
    r.map(|v| to_string(&v)).map_err(|e| {
        refused(
            "network_refused",
            e.words(&format!("{} m", fixed(reach, 2))),
        )
    })
}

/// A project's network, built from the drawing as it was when built.
#[pyclass(module = "kentos._native", name = "Network", unsendable)]
pub struct PyNetwork {
    def: NetworkDef,
    inner: NetworkSession,
    problems: Vec<String>,
    /// The drawing it was built from: the records the project's relations tie to a trace (docs/adr/0525 §1).
    session: Py<PySession>,
}

#[pymethods]
impl PyNetwork {
    /// The project's network `id` built from `session`'s drawing.
    #[staticmethod]
    fn build(session: PyRef<'_, PySession>, id: &str) -> PyResult<Self> {
        let FromDocument {
            def,
            session: built,
            problems,
        } = from_document(session.inner.document(), id)
            .map_err(|e| refused("network_missing", e))?;
        let drawing: Py<PySession> = session.into();
        Ok(Self {
            def,
            inner: built,
            problems,
            session: drawing,
        })
    }

    /// Its definition, its costs' names (Uzunluk first), its counts and what building it said, as JSON.
    fn summary(&self) -> PyResult<String> {
        let counts: serde_json::Value = serde_json::from_str(&self.inner.summary_json())
            .map_err(|e| refused("unwritable_result", e.to_string()))?;
        text(&serde_json::json!({
            "network": self.def,
            "costs": self.def.cost_names(),
            "counts": counts,
            "problems": self.problems,
        }))
    }

    /// Saat (docs/adr/0521 §4): the time of day the questions set off at, minutes of the day; `None`, every hour
    /// alike. Read by a cost with a time profile.
    #[pyo3(signature = (minutes=None))]
    fn set_time(&mut self, minutes: Option<f64>) {
        self.inner.time = minutes;
    }

    /// Araç profili and Tarih (docs/adr/0567): the vehicle (a profile's name) and the day (`YYYY-MM-DD`) the questions
    /// are for; `None`, neither. `{byProfile, byDate, unread}` or `{error}`.
    #[pyo3(signature = (profile=None, day=None))]
    fn set_access(&mut self, profile: Option<&str>, day: Option<&str>) -> String {
        self.inner.set_access_json(profile, day)
    }

    /// The place nearest `(x, y)` within `reach`: `{piece, offset, x, y, d}` or `null`.
    fn locate(&self, x: f64, y: f64, reach: f64) -> String {
        self.inner.locate_json(x, y, reach)
    }

    /// A route through `stops` (`[[x, y], …]`) avoiding `barriers`, with the cost at `cost` (0: the length), the stops
    /// reordered as `reorder` says (`none`, `keepFirst`, `keepFirstLast`): `{order, cost, totals, line, legs}`.
    fn route(
        &mut self,
        stops: &str,
        barriers: &str,
        reach: f64,
        cost: usize,
        reorder: &str,
    ) -> PyResult<String> {
        let (stops, barriers) = (points(stops, "duraklar")?, points(barriers, "engeller")?);
        let reorder = match reorder {
            "none" => Reorder::None,
            "keepFirst" => Reorder::KeepFirst,
            "keepFirstLast" => Reorder::KeepFirstLast,
            other => {
                return Err(refused(
                    "invalid_input",
                    format!("“{other}” bir sıra değil (none, keepFirst, keepFirstLast)."),
                ));
            }
        };
        answer(
            self.inner.route(&stops, &barriers, reach, cost, reorder),
            reach,
        )
    }

    /// The network within each of `breaks` of `facilities`: `{lines: [{facility?, band, line}], areas: [{facility?,
    /// band, shape?}]}` (the areas only when `areas`; `rings`: each band without the ones inside it).
    #[allow(clippy::too_many_arguments)]
    fn service_area(
        &mut self,
        facilities: &str,
        breaks: Vec<f64>,
        reach: f64,
        cost: usize,
        toward: bool,
        separate: bool,
        barriers: &str,
        trim: f64,
        rings: bool,
        areas: bool,
    ) -> PyResult<String> {
        let (facilities, barriers) = (
            points(facilities, "tesisler")?,
            points(barriers, "engeller")?,
        );
        answer(
            self.inner.service_area(
                &facilities,
                &breaks,
                reach,
                cost,
                toward,
                separate,
                &barriers,
                trim,
                rings,
                areas,
            ),
            reach,
        )
    }

    /// From each of `origins` the `k` cheapest of `targets` within `cutoff` (`reverse`: from the targets), with their
    /// ways when `paths`: `[[{target, cost, totals?, line?}], …]`.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (origins, targets, reach, cost, reverse, barriers, paths, k = None, cutoff = None))]
    fn nearest(
        &mut self,
        origins: &str,
        targets: &str,
        reach: f64,
        cost: usize,
        reverse: bool,
        barriers: &str,
        paths: bool,
        k: Option<usize>,
        cutoff: Option<f64>,
    ) -> PyResult<String> {
        let (origins, targets, barriers) = (
            points(origins, "başlangıçlar")?,
            points(targets, "varışlar")?,
            points(barriers, "engeller")?,
        );
        answer(
            self.inner.nearest_of(
                &origins, &targets, reach, k, cutoff, cost, reverse, &barriers, paths,
            ),
            reach,
        )
    }

    /// A trace of `kind` (`connected`, `downstream`, `upstream`, `isolation`) from `starts`: `{objects, valves,
    /// unfed, length, unfedLength, lines, unfedLines}`.
    fn trace(&self, starts: &str, barriers: &str, reach: f64, kind: &str) -> PyResult<String> {
        let (starts, barriers) = (
            points(starts, "başlangıçlar")?,
            points(barriers, "engeller")?,
        );
        let kind = match kind {
            "connected" => TraceKind::Connected,
            "downstream" => TraceKind::Downstream,
            "upstream" => TraceKind::Upstream,
            "isolation" => TraceKind::Isolation,
            other => {
                return Err(refused(
                    "invalid_input",
                    format!(
                        "“{other}” bir izleme türü değil (connected, downstream, upstream, isolation)."
                    ),
                ));
            }
        };
        answer(self.inner.trace_of(&starts, &barriers, reach, kind), reach)
    }

    /// The records the project's relations tie to the objects `ids` (`[id, …]`; a trace's objects, junctions and
    /// what it cut off): `[{relation, name, tie: attached|container, ids}]` (docs/adr/0525 §1).
    fn related(&self, py: Python<'_>, ids: Vec<u32>) -> PyResult<String> {
        let session = self.session.borrow(py);
        let slots: Vec<kentos_domain::Slot> = ids.into_iter().map(kentos_domain::Slot).collect();
        let groups: Vec<serde_json::Value> = session
            .inner
            .document()
            .related_of(&slots)
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "relation": r.relation,
                    "name": r.name,
                    "tie": match r.tie {
                        kentos_domain::Tie::Attached => "attached",
                        kentos_domain::Tie::Container => "container",
                    },
                    "ids": r.slots.iter().map(|s| s.0).collect::<Vec<_>>(),
                })
            })
            .collect();
        text(&groups)
    }

    /// Denetle: `{nodes, pieces, length, parts, deadEnds, counts, problems}`.
    fn check(&self) -> String {
        self.inner.check_json()
    }

    fn __repr__(&self) -> String {
        format!(
            "<kentos._native.Network {:?}, {} ağı>",
            self.def.name,
            if self.def.kind == kentos_commands::NetworkKind::Road {
                "yol"
            } else {
                "şebeke"
            }
        )
    }
}
