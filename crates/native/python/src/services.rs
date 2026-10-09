//! Map services for Python (`kentos.services`, docs/adr/0208 §15): the
//! services core's ready basemaps, Bağlan for a map service and for a data
//! service, taking a feed's objects page by page, a connection's proof, and
//! moving objects into the project's system. JSON text in the contract's
//! shapes crosses; Python sends the requests (`urllib`) and hands the answers
//! back, as the apps do. Nothing here touches a drawing: Python writes with
//! `cad.layers.service` and `cad.entities.create`.

use kentos_contracts::{
    ConnectionSecret, FeatureFeed, FeedKind, ImportResult, ProjectSettings, ReportItem,
    ServiceConnection, ServiceKind, ServiceLayer,
};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::crs::{Choice, System, transform_in};
use kentos_headless::HeadlessError;
use kentos_services::auth::{self, Token};
use kentos_services::request::Request;
use kentos_services::{connect, feed, presets};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::{host, text};

fn refused(code: &'static str, message: impl Into<String>) -> PyErr {
    host(HeadlessError::new(code, message.into()))
}

fn parse<T: serde::de::DeserializeOwned>(json: &str, what: &str) -> PyResult<T> {
    serde_json::from_str(json)
        .map_err(|e| refused("invalid_input", format!("{what} okunamadı: {e}")))
}

/// A request as it crosses: `{"url", "headers": [[name, value]], "body", "media"}`.
#[derive(Serialize, Deserialize)]
struct Wire {
    url: String,
    #[serde(default)]
    headers: Vec<(String, String)>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    media: Option<String>,
}

fn wire(r: &Request) -> PyResult<String> {
    text(&Wire {
        url: r.url.clone(),
        headers: r.headers.clone(),
        body: r.body.as_ref().map(|(b, _)| b.clone()),
        media: r.body.as_ref().map(|(_, m)| m.clone()),
    })
}

fn request_of(json: &str) -> PyResult<Request> {
    let w: Wire = parse(json, "istek")?;
    Ok(Request {
        url: w.url,
        headers: w.headers,
        body: w.body.map(|b| {
            (
                b,
                w.media.unwrap_or_else(|| "application/octet-stream".into()),
            )
        }),
    })
}

fn service_kind(name: &str) -> PyResult<ServiceKind> {
    ServiceKind::ALL
        .iter()
        .copied()
        .find(|k| k.name() == name)
        .ok_or_else(|| refused("invalid_input", format!("“{name}” bir servis türü değil (xyz, wms, wmts, ogcTiles, arcgis, google, vector).")))
}

fn feed_kind(name: &str) -> PyResult<FeedKind> {
    FeedKind::ALL
        .iter()
        .copied()
        .find(|k| k.name() == name)
        .ok_or_else(|| {
            refused(
                "invalid_input",
                format!(
                    "“{name}” bir veri servisi türü değil (wfs, ogcFeatures, arcgis, geojson)."
                ),
            )
        })
}

/// The ready basemaps' catalog: their groups and presets.
#[pyfunction]
pub fn services_presets() -> PyResult<String> {
    text(presets::catalog())
}

/// A preset's service layer (its id in `preset`, its connection's in
/// `connection`) and the connection it needs, or none for an unknown id.
#[pyfunction]
pub fn services_preset_layer(id: &str) -> PyResult<Option<(String, Option<String>)>> {
    let Some(p) = presets::preset(id) else {
        return Ok(None);
    };
    let connection = p.connection.as_ref().map(text).transpose()?;
    Ok(Some((text(&presets::layer_of(p))?, connection)))
}

/// Why a connection's secrets cannot prove it, or none when they can.
#[pyfunction]
#[pyo3(signature = (connection, secret = None))]
pub fn services_auth_missing(connection: &str, secret: Option<&str>) -> PyResult<Option<String>> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    let s: Option<ConnectionSecret> = secret.map(|s| parse(s, "gizli değerler")).transpose()?;
    Ok(auth::missing(&c, s.as_ref()))
}

/// A token as it crosses: `{"value", "expiresMs"}`.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenWire {
    value: String,
    expires_ms: u64,
}

/// `request` with the connection's proof added (`token`: an ArcGIS or OAuth 2 token, `{"value", "expiresMs"}`).
#[pyfunction]
#[pyo3(signature = (connection, secret, request, token = None))]
pub fn services_auth_apply(
    connection: &str,
    secret: &str,
    request: &str,
    token: Option<&str>,
) -> PyResult<String> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    let s: ConnectionSecret = parse(secret, "gizli değerler")?;
    let t = token
        .map(|t| parse::<TokenWire>(t, "belirteç"))
        .transpose()?
        .map(|t| Token {
            value: t.value,
            expires_ms: t.expires_ms,
        });
    let mut r = request_of(request)?;
    auth::apply(&c, &s, t.as_ref(), &mut r);
    wire(&r)
}

/// The request for an ArcGIS or OAuth 2 connection's token, or none for another kind.
#[pyfunction]
pub fn services_token_request(
    connection: &str,
    secret: &str,
    referer: &str,
) -> PyResult<Option<String>> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    let s: ConnectionSecret = parse(secret, "gizli değerler")?;
    auth::token_request(&c, &s, referer)
        .map(|r| wire(&r))
        .transpose()
}

/// A token's answer read: `{"value", "expiresMs"}`.
#[pyfunction]
pub fn services_read_token(connection: &str, body: &str, now_ms: f64) -> PyResult<String> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    let t = auth::read_token(&c, body, now_ms as u64).map_err(|e| refused("auth_failed", e))?;
    text(&TokenWire {
        value: t.value,
        expires_ms: t.expires_ms,
    })
}

/// What a take brought, in the apps' words (`feed::taken_words`).
#[pyfunction]
pub fn services_taken_words(
    taken: usize,
    matched: Option<u64>,
    skipped: &str,
    dropped: usize,
    capped: bool,
    srid: u32,
) -> PyResult<String> {
    let skipped: Vec<ReportItem> = parse(skipped, "atlananlar")?;
    Ok(feed::taken_words(
        taken, matched, &skipped, dropped, capped, srid,
    ))
}

/// A new data layer's line and fill colours, by how many layers the drawing has.
#[pyfunction]
pub fn services_layer_colors(layers: usize) -> (String, String) {
    let (line, fill) = feed::layer_colors(layers);
    (line.to_owned(), fill.to_owned())
}

/// The project's system and the service system `srid` as the transforms
/// read them, with the project's datum choices; none when they are the
/// same; why not when there is no way between them.
type Pair = Option<(System, System, Vec<Choice>)>;

fn pair_of(settings: &str, srid: u32) -> PyResult<Pair> {
    let s: ProjectSettings = parse(settings, "proje ayarları")?;
    let Some(theirs) = kentos_project::crs::system(srid).and_then(|x| x.transform_system()) else {
        return Err(refused(
            "unknown_system",
            format!(
                "Servisin koordinat sistemi (EPSG:{srid}) KentOS'un kaydında yok; servisi başka bir sistemde isteyin."
            ),
        ));
    };
    if s.srid == srid && s.custom_crs.is_none() {
        return Ok(None);
    }
    let Some(ours) = kentos_project::systems::own(&s).and_then(|n| n.system) else {
        return Err(refused(
            "no_system",
            "Projenin koordinat sistemi yok: harita servisi kullanılamaz. Proje ayarlarında projeye bir koordinat sistemi verin.",
        ));
    };
    Ok(Some((ours, theirs, kentos_project::systems::choices(&s))))
}

/// Whether the project (its settings) and the service system `srid` are the
/// same; why not when there is no way between them.
#[pyfunction]
pub fn services_same_system(settings: &str, srid: u32) -> PyResult<bool> {
    Ok(pair_of(settings, srid)?.is_none())
}

/// A page's objects moved from the service system `srid` into the
/// project's (its settings): `{"result", "dropped"}`.
#[pyfunction]
pub fn services_to_project(result: &str, settings: &str, srid: u32) -> PyResult<String> {
    let mut r: ImportResult = parse(result, "nesneler")?;
    let dropped = match pair_of(settings, srid)? {
        None => 0,
        Some((ours, theirs, choices)) => kentos_services::features::transform(&mut r, |p| {
            transform_in(&theirs, &ours, Vec2 { x: p.x, y: p.y }, &choices)
                .ok()
                .map(|t| kentos_contracts::Vec2 {
                    x: t.point.x,
                    y: t.point.y,
                })
        }),
    };
    #[derive(Serialize)]
    struct Moved {
        result: ImportResult,
        dropped: usize,
    }
    text(&Moved { result: r, dropped })
}

/// A box of the project's system in the service system `srid`: its edges,
/// five points each, moved and boxed; none where an edge has no place there.
#[pyfunction]
pub fn services_box_in(
    bbox: (f64, f64, f64, f64),
    settings: &str,
    srid: u32,
) -> PyResult<Option<(f64, f64, f64, f64)>> {
    let Some((ours, theirs, choices)) = pair_of(settings, srid)? else {
        return Ok(Some(bbox));
    };
    let (x1, y1, x2, y2) = bbox;
    let mut out = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for i in 0..=4 {
        let t = f64::from(i) / 4.0;
        for (x, y) in [
            (x1 + (x2 - x1) * t, y1),
            (x1 + (x2 - x1) * t, y2),
            (x1, y1 + (y2 - y1) * t),
            (x2, y1 + (y2 - y1) * t),
        ] {
            let Ok(p) = transform_in(&ours, &theirs, Vec2 { x, y }, &choices) else {
                return Ok(None);
            };
            out = [
                out[0].min(p.point.x),
                out[1].min(p.point.y),
                out[2].max(p.point.x),
                out[3].max(p.point.y),
            ];
        }
    }
    Ok(out
        .iter()
        .all(|v| v.is_finite())
        .then_some((out[0], out[1], out[2], out[3])))
}

/// Bağlan for a map service (`connect::Connecting`).
#[pyclass(module = "kentos._native", name = "ServiceReading")]
pub struct PyServiceReading {
    inner: connect::Connecting,
    first: Option<String>,
}

#[pymethods]
impl PyServiceReading {
    #[new]
    fn new(kind: &str, url: &str) -> PyResult<Self> {
        let (inner, first) = connect::Connecting::start(service_kind(kind)?, url)
            .map_err(|e| refused("invalid_service", e))?;
        Ok(Self {
            inner,
            first: first.as_ref().map(wire).transpose()?,
        })
    }

    /// The first request; none when the kind reads nothing (the offer is ready).
    fn first(&self) -> Option<String> {
        self.first.clone()
    }

    /// The answer to the last request: the next request, or none when the offer is ready.
    fn answer(&mut self, body: &str) -> PyResult<Option<String>> {
        let next = self
            .inner
            .answer(body)
            .map_err(|e| refused("service_failed", e))?;
        next.as_ref().map(wire).transpose()
    }

    fn offer(&self) -> PyResult<Option<String>> {
        self.inner.offer().map(text).transpose()
    }

    /// The service layer a choice (`connect::Choice`) makes, in the project's system `project`.
    fn layer(&self, choice: &str, project: u32) -> PyResult<String> {
        let c: connect::Choice = parse(choice, "seçim")?;
        let layer: ServiceLayer = self
            .inner
            .layer(&c, project)
            .map_err(|e| refused("invalid_service", e))?;
        text(&layer)
    }
}

/// Bağlan for a data service (`feed::FeedConnecting`).
#[pyclass(module = "kentos._native", name = "FeedReading")]
pub struct PyFeedReading {
    inner: feed::FeedConnecting,
    first: Option<String>,
}

#[pymethods]
impl PyFeedReading {
    #[new]
    fn new(kind: &str, url: &str) -> PyResult<Self> {
        let (inner, first) = feed::FeedConnecting::start(feed_kind(kind)?, url)
            .map_err(|e| refused("invalid_feed", e))?;
        Ok(Self {
            inner,
            first: first.as_ref().map(wire).transpose()?,
        })
    }

    fn first(&self) -> Option<String> {
        self.first.clone()
    }

    fn answer(&mut self, body: &str) -> PyResult<Option<String>> {
        let next = self
            .inner
            .answer(body)
            .map_err(|e| refused("service_failed", e))?;
        next.as_ref().map(wire).transpose()
    }

    fn offer(&self) -> PyResult<Option<String>> {
        self.inner.offer().map(text).transpose()
    }

    /// Whether a WFS gives its objects as GeoJSON.
    fn geojson(&self) -> bool {
        self.inner.geojson()
    }

    /// The systems listed for item `id`, the one asked when none is chosen among them.
    fn systems(&self, id: &str, project: u32) -> Vec<u32> {
        self.inner.systems(id, project)
    }

    /// The system item `id`'s objects are asked in for `choice`.
    #[pyo3(signature = (id, project, choice = None))]
    fn asked_srid(&self, id: &str, project: u32, choice: Option<u32>) -> Option<u32> {
        self.inner.asked_srid(id, choice, project)
    }

    /// The feed a choice (`feed::FeedChoice`) makes.
    fn feed(&self, choice: &str, project: u32) -> PyResult<String> {
        let c: feed::FeedChoice = parse(choice, "seçim")?;
        let f: FeatureFeed = self
            .inner
            .feed(&c, project)
            .map_err(|e| refused("invalid_feed", e))?;
        text(&f)
    }
}

/// Taking a feed's objects, page by page (`feed::Taking`).
#[pyclass(module = "kentos._native", name = "FeedTaking")]
pub struct PyFeedTaking {
    inner: feed::Taking,
    first: String,
}

#[pymethods]
impl PyFeedTaking {
    /// `area` in the feed's system, or none.
    #[new]
    #[pyo3(signature = (feed, most, geojson, area = None))]
    fn new(
        feed: &str,
        most: u32,
        geojson: bool,
        area: Option<(f64, f64, f64, f64)>,
    ) -> PyResult<Self> {
        let f: FeatureFeed = parse(feed, "veri kaynağı")?;
        let area = area.map(|(a, b, c, d)| [a, b, c, d]);
        let (inner, first) = feed::Taking::start(&f, area, most, geojson);
        Ok(Self {
            inner,
            first: wire(&first)?,
        })
    }

    fn first(&self) -> String {
        self.first.clone()
    }

    /// A page's answer: `{"result": ImportResult, "next": request | null}`.
    fn answer(&mut self, body: &str) -> PyResult<String> {
        let (result, next) = self
            .inner
            .answer(body, "")
            .map_err(|e| refused("service_failed", e))?;
        #[derive(Serialize)]
        struct Page {
            result: ImportResult,
            next: Option<serde_json::Value>,
        }
        let next = next
            .as_ref()
            .map(|r| wire(r).and_then(|w| parse::<serde_json::Value>(&w, "istek")))
            .transpose()?;
        text(&Page { result, next })
    }

    #[getter]
    fn srid(&self) -> u32 {
        self.inner.srid
    }

    #[getter]
    fn taken(&self) -> u64 {
        self.inner.taken
    }

    #[getter]
    fn matched(&self) -> Option<u64> {
        self.inner.matched
    }

    /// What the pages' readers left out, summed (`ReportItem`s).
    fn skipped(&self) -> PyResult<String> {
        text(&self.inner.skipped)
    }
}

/// A take's objects as `cad.entities.create`'s `NewObject`s: each object's
/// shape as its geometry, its attributes, label, colour, weight and symbol
/// kept (the reader's ids and layer left out).
#[pyfunction]
pub fn services_new_objects(entities: &str) -> PyResult<String> {
    let list: Vec<serde_json::Value> = parse(entities, "nesneler")?;
    let mut out: Vec<kentos_contracts::NewObject> = Vec::with_capacity(list.len());
    for e in list {
        let serde_json::Value::Object(fields) = e else {
            return Err(refused("invalid_input", "Bir nesne JSON nesnesi değil."));
        };
        let mut geometry = serde_json::Map::new();
        let mut object = serde_json::Map::new();
        for (k, v) in fields {
            match k.as_str() {
                "id" | "layerId" => {}
                "attrs" | "label" | "color" | "symbol" | "lineWeight" => {
                    object.insert(k, v);
                }
                _ => {
                    geometry.insert(k, v);
                }
            }
        }
        object.insert("geometry".into(), serde_json::Value::Object(geometry));
        let o = serde_json::from_value(serde_json::Value::Object(object)).map_err(|e| {
            refused(
                "invalid_input",
                format!("Servisin bir nesnesi çizime yazılamıyor: {e}"),
            )
        })?;
        out.push(o);
    }
    text(&out)
}

/// A new data layer's fields from its objects' attributes (`fields::infer_fields`), as the apps make them.
#[pyfunction]
pub fn services_infer_fields(rows: &str) -> PyResult<String> {
    let rows: Vec<std::collections::BTreeMap<String, String>> = parse(rows, "öznitelikler")?;
    let rows: Vec<Vec<(String, String)>> =
        rows.into_iter().map(|r| r.into_iter().collect()).collect();
    let fields = kentos_contracts::fields::infer_fields(rows.iter().map(Vec::as_slice), |a, b| {
        kentos_geometry_core::text::natural::natural_cmp(a, b)
    });
    text(&fields)
}
