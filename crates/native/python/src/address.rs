//! Adres arama for Python (`kentos.address`, docs/adr/0229 §14): an
//! address's parts (§4), and a project's address data built from an open
//! drawing as the apps build it (`kentos_native_application::addresses`)
//! and asked what the apps' Adres tab asks: an address's candidates (in one
//! line or in columns), a place's address, Denetle. The answers cross as
//! JSON; `kentos.address` makes dataclasses of them. Online providers are
//! not here (§1): their terms are accepted in the apps, on a device.

use kentos_commands::AddressLocator;
use kentos_geometry_core::ops::geocode::check::check;
use kentos_geometry_core::ops::geocode::locator::Texts;
use kentos_geometry_core::ops::geocode::parse::from_columns;
use kentos_geometry_core::ops::geocode::{Found, Locator, Parsed, Place, find, parse};
use kentos_geometry_core::vec2::Vec2;
use kentos_headless::HeadlessError;
use pyo3::prelude::*;
use serde_json::{Value, json};

use super::{PySession, host, text};

/// How many candidates an answer lists at most (the Adres tab's).
const LISTED: usize = 20;

fn refused(code: &'static str, message: impl Into<String>) -> PyErr {
    host(HeadlessError::new(code, message.into()))
}

fn texts_json(t: &Texts) -> Value {
    json!({
        "street": t.street,
        "door": t.door,
        "neighbourhood": t.neighbourhood,
        "district": t.district,
        "province": t.province,
        "postcode": t.postcode,
        "building": t.building,
        "code": t.code,
    })
}

fn parsed_json(p: &Parsed) -> Value {
    json!({
        "postcode": p.postcode,
        "street": p.street,
        "streetKind": p.street_kind.map(|k| k.name()),
        "door": p.door,
        "neighbourhood": p.neighbourhood,
        "district": p.district,
        "province": p.province,
        "rest": p.rest,
        "untagged": p.untagged,
    })
}

fn found_json(f: &Found) -> Value {
    json!({
        "status": f.status.word(),
        "candidates": f.candidates.iter().map(|c| json!({
            "kind": c.kind.word(),
            "score": c.score,
            "at": [c.at.x, c.at.y],
            "label": c.label,
            "texts": texts_json(&c.texts),
        })).collect::<Vec<_>>(),
    })
}

fn place_json(p: &Place) -> Value {
    json!({
        "kind": p.kind.word(),
        "at": [p.at.x, p.at.y],
        "distance": p.distance,
        "label": p.label,
        "texts": texts_json(&p.texts),
    })
}

/// An address's parts (§4), as JSON.
#[pyfunction]
pub fn address_parse(address: &str) -> PyResult<String> {
    text(&parsed_json(&parse(address)))
}

/// A project's address data, built from the drawing as it was when built.
#[pyclass(module = "kentos._native", name = "Locator", unsendable)]
pub struct PyLocator {
    def: AddressLocator,
    inner: Locator,
}

#[pymethods]
impl PyLocator {
    /// The project's address data `id` (none: its first) built from `session`'s drawing.
    #[staticmethod]
    #[pyo3(signature = (session, id = None))]
    fn build(session: PyRef<'_, PySession>, id: Option<&str>) -> PyResult<Self> {
        let doc = session.inner.document();
        let all = &doc.settings().address_locators;
        let def = match id {
            Some(id) => all.iter().find(|l| l.id == id).ok_or_else(|| {
                refused(
                    "unknown_locator",
                    format!(
                        "“{id}” adres verisi projede yok; kentos.address.define ile tanımlayın."
                    ),
                )
            })?,
            None => all.first().ok_or_else(|| {
                refused(
                    "no_locator",
                    "Projede adres verisi yok; kentos.address.define ile tanımlayın.",
                )
            })?,
        };
        Ok(Self {
            inner: kentos_native_application::addresses::build(doc, def),
            def: def.clone(),
        })
    }

    /// The definition, as JSON.
    fn definition(&self) -> PyResult<String> {
        text(&self.def)
    }

    /// An address in one line: its candidates, the best first, and the best one's status (§7). An address code
    /// answers its door.
    #[pyo3(signature = (address, min_score = None))]
    fn find(&self, address: &str, min_score: Option<u32>) -> PyResult<String> {
        let min = min_score.unwrap_or(self.def.min_score);
        text(&found_json(&find(
            &self.inner,
            address,
            &parse(address),
            min,
            LISTED,
        )))
    }

    /// An address in columns (neighbourhood, street, door, district, province, postcode).
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (neighbourhood, street, door, district, province, postcode, min_score = None))]
    fn find_columns(
        &self,
        neighbourhood: &str,
        street: &str,
        door: &str,
        district: &str,
        province: &str,
        postcode: &str,
        min_score: Option<u32>,
    ) -> PyResult<String> {
        let min = min_score.unwrap_or(self.def.min_score);
        let q = from_columns(neighbourhood, street, door, district, province, postcode);
        text(&found_json(&find(&self.inner, "", &q, min, LISTED)))
    }

    /// A place's address within `reach` metres (none: the definition's), as JSON; null when none is near (§8).
    #[pyo3(signature = (east, north, reach = None))]
    fn reverse(&self, east: f64, north: f64, reach: Option<f64>) -> PyResult<String> {
        let reach = reach.unwrap_or(self.def.reach);
        if !(reach.is_finite() && reach > 0.0) {
            return Err(refused(
                "invalid_input",
                format!("Uzaklık sıfırdan büyük olmalı; {reach} yazıldı."),
            ));
        }
        if !(east.is_finite() && north.is_finite()) {
            return Err(refused(
                "invalid_input",
                "Yerin doğusu ve kuzeyi sayı olmalı.",
            ));
        }
        let place = self.inner.reverse(Vec2::new(east, north), reach);
        text(&place.as_ref().map(place_json))
    }

    /// Denetle (§13): what the index holds and what it left out, as JSON.
    fn check(&self) -> PyResult<String> {
        let s = check(&self.inner);
        let k = &s.counts;
        text(&json!({
            "doors": k.doors,
            "streets": k.streets,
            "lines": k.lines,
            "neighbourhoods": k.neighbourhoods,
            "areas": k.areas,
            "notDoors": s.not_doors,
            "emptyStreets": s.empty_streets,
            "unreadDoors": s.unread_doors,
            "notStreets": s.not_streets,
            "unnamedLines": s.unnamed_lines,
            "notAreas": s.not_areas,
            "unnamedAreas": s.unnamed_areas,
            "repeated": s.repeated,
            "repeats": s.repeats.iter().map(|r| json!({
                "street": r.street,
                "neighbourhood": r.neighbourhood,
                "door": r.door,
                "count": r.orders.len(),
            })).collect::<Vec<_>>(),
        }))
    }
}
