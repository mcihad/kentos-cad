//! Map services in the browser (docs/adr/0208): the services core's
//! wasm-bindgen boundary, a package of its own loaded the first time a
//! drawing shows a service or a services window opens (CLAUDE.md §20). The
//! page's workers (`apps/web/src/io/services/worker.ts` for the tiles,
//! `feedWorker.ts` for Servisten veri al) fetch and ask this module what to
//! fetch and what an answer means; the main thread asks it which tiles a
//! view shows and their meshes, and the windows read capabilities with it.
//!
//! What crosses is JSON in the contract's shapes (`ServiceLayer`,
//! `ServiceConnection`, `FeatureFeed`, `ImportResult`), a request as
//! `{"url", "headers": [[name, value]], "body": text | null, "media": type | null}`,
//! and typed arrays for tiles and meshes. A refusal throws an `Error` whose
//! message is the core's Turkish words.

use std::collections::HashMap;

use kentos_contracts::{
    ConnectionSecret, FeatureFeed, FeedKind, ImportResult, ServiceConnection, ServiceKind,
    ServiceLayer,
};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::crs::{Choice, System, transform_in};
use kentos_geometry_core::geom::tiles::{Grid, MeshMap, TileRef, cells_of, mesh, shown};
use kentos_services::auth::{self, Token};
use kentos_services::labels::{Candidate, Screen, place};
use kentos_services::request::Request;
use kentos_services::style::build::{TileInput, build};
use kentos_services::{attribution, connect, feed, google, info, mvt, picture, source, style};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

fn err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn parse<T: serde::de::DeserializeOwned>(text: &str, what: &str) -> Result<T, JsError> {
    serde_json::from_str(text).map_err(|e| JsError::new(&format!("{what} okunamadı: {e}")))
}

fn write<T: Serialize + ?Sized>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "null".to_owned())
}

/// A request as it crosses.
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

fn wire(r: &Request) -> String {
    write(&Wire {
        url: r.url.clone(),
        headers: r.headers.clone(),
        body: r.body.as_ref().map(|(b, _)| b.clone()),
        media: r.body.as_ref().map(|(_, m)| m.clone()),
    })
}

fn request_of(text: &str) -> Result<Request, JsError> {
    let w: Wire = parse(text, "istek")?;
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

fn service_kind(name: &str) -> Result<ServiceKind, JsError> {
    ServiceKind::ALL
        .iter()
        .copied()
        .find(|k| k.name() == name)
        .ok_or_else(|| JsError::new(&format!("bilinmeyen servis türü: {name}")))
}

fn feed_kind(name: &str) -> Result<FeedKind, JsError> {
    FeedKind::ALL
        .iter()
        .copied()
        .find(|k| k.name() == name)
        .ok_or_else(|| JsError::new(&format!("bilinmeyen veri türü: {name}")))
}

fn system(text: &str) -> Result<System, JsError> {
    System::from_json(&Json::parse(text).map_err(err)?).map_err(err)
}

fn choices(text: &str) -> Result<Vec<Choice>, JsError> {
    Vec::<Choice>::from_json(&Json::parse(text).map_err(err)?).map_err(err)
}

// ── Connections ─────────────────────────────────────────────────────────

/// Why a connection's secrets cannot prove it, in the windows' words; none when they can.
#[wasm_bindgen(js_name = authMissing)]
pub fn auth_missing(connection: &str, secret: Option<String>) -> Result<Option<String>, JsError> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    let s: Option<ConnectionSecret> = secret.map(|t| parse(&t, "gizli değer")).transpose()?;
    Ok(auth::missing(&c, s.as_ref()))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenWire {
    value: String,
    expires_ms: f64,
}

/// `request` with the connection's proof when it goes to its origin (an ArcGIS or OAuth 2 one needs its token).
#[wasm_bindgen(js_name = authApply)]
pub fn auth_apply(
    connection: &str,
    secret: &str,
    token: Option<String>,
    request: &str,
) -> Result<String, JsError> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    let s: ConnectionSecret = parse(secret, "gizli değer")?;
    let t = token
        .map(|t| parse::<TokenWire>(&t, "belirteç"))
        .transpose()?
        .map(|t| Token {
            value: t.value,
            expires_ms: t.expires_ms as u64,
        });
    let mut r = request_of(request)?;
    auth::apply(&c, &s, t.as_ref(), &mut r);
    Ok(wire(&r))
}

/// The request that gets an ArcGIS or OAuth 2 connection its token; none for the other kinds.
#[wasm_bindgen(js_name = tokenRequest)]
pub fn token_request(
    connection: &str,
    secret: &str,
    referer: &str,
) -> Result<Option<String>, JsError> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    let s: ConnectionSecret = parse(secret, "gizli değer")?;
    Ok(auth::token_request(&c, &s, referer).map(|r| wire(&r)))
}

/// The token an answer gives, `{"value", "expiresMs"}`.
#[wasm_bindgen(js_name = readToken)]
pub fn read_token(connection: &str, body: &str, now_ms: f64) -> Result<String, JsError> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    let t = auth::read_token(&c, body, now_ms as u64).map_err(err)?;
    Ok(write(&TokenWire {
        value: t.value,
        expires_ms: t.expires_ms as f64,
    }))
}

/// The parameters that carry secrets (left out of what the page logs).
#[wasm_bindgen(js_name = secretParams)]
pub fn secret_params(connection: &str) -> Result<Vec<String>, JsError> {
    let c: ServiceConnection = parse(connection, "bağlantı")?;
    Ok(auth::secret_params(&c))
}

// ── Harita servisi's Bağlan ─────────────────────────────────────────────

/// Bağlan for a map service (`kentos_services::connect`).
#[wasm_bindgen]
pub struct Connecting {
    inner: connect::Connecting,
    first: Option<String>,
}

#[wasm_bindgen]
impl Connecting {
    #[wasm_bindgen(constructor)]
    pub fn new(kind: &str, url: &str) -> Result<Connecting, JsError> {
        let (inner, first) = connect::Connecting::start(service_kind(kind)?, url).map_err(err)?;
        Ok(Connecting {
            inner,
            first: first.map(|r| wire(&r)),
        })
    }

    /// The first request; none when the kind reads nothing (the offer is ready).
    pub fn first(&self) -> Option<String> {
        self.first.clone()
    }

    /// The answer to the last request: the next request, or none when the offer is ready.
    pub fn answer(&mut self, body: &str) -> Result<Option<String>, JsError> {
        Ok(self.inner.answer(body).map_err(err)?.map(|r| wire(&r)))
    }

    pub fn offer(&self) -> Option<String> {
        self.inner.offer().map(write)
    }

    /// The service layer a choice makes (`connect::Choice`), in the project's system `project`.
    pub fn layer(&self, choice: &str, project: u32) -> Result<String, JsError> {
        let c: connect::Choice = parse(choice, "seçim")?;
        self.inner
            .layer(&c, project)
            .map(|s| write(&s))
            .map_err(err)
    }
}

/// What Dene asks a service layer for.
#[wasm_bindgen]
pub fn probe(service: &str) -> Result<Option<String>, JsError> {
    let s: ServiceLayer = parse(service, "servis")?;
    Ok(connect::probe(&s).map(|r| wire(&r)))
}

/// The system to ask in: the project's when offered, else Web Mercator, WGS 84, the first known.
#[wasm_bindgen(js_name = suggestedSrid)]
pub fn suggested_srid(offered: Vec<u32>, project: u32) -> Option<u32> {
    connect::suggested_srid(&offered, project)
}

/// Whether KentOS's register has the system.
#[wasm_bindgen(js_name = knownSrid)]
pub fn known_srid(srid: u32) -> bool {
    kentos_services::crs::known(srid)
}

// ── Servisten veri al ───────────────────────────────────────────────────

/// Bağlan for a data service (`kentos_services::feed`).
#[wasm_bindgen]
pub struct FeedConnecting {
    inner: feed::FeedConnecting,
    first: Option<String>,
}

#[wasm_bindgen]
impl FeedConnecting {
    #[wasm_bindgen(constructor)]
    pub fn new(kind: &str, url: &str) -> Result<FeedConnecting, JsError> {
        let (inner, first) = feed::FeedConnecting::start(feed_kind(kind)?, url).map_err(err)?;
        Ok(FeedConnecting {
            inner,
            first: first.map(|r| wire(&r)),
        })
    }

    pub fn first(&self) -> Option<String> {
        self.first.clone()
    }

    pub fn answer(&mut self, body: &str) -> Result<Option<String>, JsError> {
        Ok(self.inner.answer(body).map_err(err)?.map(|r| wire(&r)))
    }

    pub fn offer(&self) -> Option<String> {
        self.inner.offer().map(write)
    }

    /// Whether a WFS gives its objects as GeoJSON.
    pub fn geojson(&self) -> bool {
        self.inner.geojson()
    }

    /// The systems the window lists for item `id`, the one asked when none is chosen among them.
    pub fn systems(&self, id: &str, project: u32) -> Vec<u32> {
        self.inner.systems(id, project)
    }

    /// The system item `id`'s objects are asked in for `choice` (none for a GeoJSON address).
    #[wasm_bindgen(js_name = askedSrid)]
    pub fn asked_srid(&self, id: &str, choice: Option<u32>, project: u32) -> Option<u32> {
        self.inner.asked_srid(id, choice, project)
    }

    /// The feed a choice makes (`feed::FeedChoice`).
    pub fn feed(&self, choice: &str, project: u32) -> Result<String, JsError> {
        let c: feed::FeedChoice = parse(choice, "seçim")?;
        self.inner.feed(&c, project).map(|f| write(&f)).map_err(err)
    }
}

/// Taking a feed's objects, page by page.
#[wasm_bindgen]
pub struct Taking {
    inner: feed::Taking,
    first: String,
}

#[derive(Serialize)]
struct Page {
    result: ImportResult,
    next: Option<serde_json::Value>,
}

#[wasm_bindgen]
impl Taking {
    /// `area` in the system the objects are asked in (the feed's), `[x₁, y₁, x₂, y₂]`, or empty.
    #[wasm_bindgen(constructor)]
    pub fn new(feed: &str, area: Vec<f64>, most: u32, geojson: bool) -> Result<Taking, JsError> {
        let f: FeatureFeed = parse(feed, "veri kaynağı")?;
        let area = <[f64; 4]>::try_from(area.as_slice()).ok();
        let (inner, first) = feed::Taking::start(&f, area, most, geojson);
        Ok(Taking {
            inner,
            first: wire(&first),
        })
    }

    pub fn first(&self) -> String {
        self.first.clone()
    }

    /// The system the objects come in.
    pub fn srid(&self) -> u32 {
        self.inner.srid
    }

    pub fn taken(&self) -> f64 {
        self.inner.taken as f64
    }

    /// What the pages' readers left out, summed (`ReportItem`s).
    pub fn skipped(&self) -> String {
        write(&self.inner.skipped)
    }

    pub fn matched(&self) -> Option<f64> {
        self.inner.matched.map(|m| m as f64)
    }

    /// A page's answer: `{"result": ImportResult, "next": request | null}`.
    pub fn answer(&mut self, body: &str) -> Result<String, JsError> {
        let (result, next) = self.inner.answer(body, "").map_err(err)?;
        let next = next.map(|r| serde_json::from_str(&wire(&r)).unwrap_or(serde_json::Value::Null));
        Ok(write(&Page { result, next }))
    }
}

#[derive(Serialize)]
struct Moved {
    result: ImportResult,
    dropped: usize,
}

/// A page's objects moved from `from` into `to` (the project's), its datum choices taken:
/// `{"result", "dropped"}`, the objects with a vertex that has no place there left out.
#[wasm_bindgen(js_name = featuresToProject)]
pub fn features_to_project(
    result: &str,
    from: &str,
    to: &str,
    choice_list: &str,
) -> Result<String, JsError> {
    let mut r: ImportResult = parse(result, "nesneler")?;
    let (a, b, c) = (system(from)?, system(to)?, choices(choice_list)?);
    let dropped = kentos_services::features::transform(&mut r, |p| {
        transform_in(&a, &b, Vec2 { x: p.x, y: p.y }, &c)
            .ok()
            .map(|t| kentos_contracts::Vec2 {
                x: t.point.x,
                y: t.point.y,
            })
    });
    Ok(write(&Moved { result: r, dropped }))
}

/// What a take brought, in the windows' words (`feed::taken_words`); `skipped` the take's `ReportItem`s.
#[wasm_bindgen(js_name = takenWords)]
pub fn taken_words(
    taken: u32,
    matched: Option<f64>,
    skipped: &str,
    dropped: u32,
    capped: bool,
    srid: u32,
) -> Result<String, JsError> {
    let skipped: Vec<kentos_contracts::ReportItem> = parse(skipped, "atlananlar")?;
    Ok(feed::taken_words(
        taken as usize,
        matched.map(|m| m as u64),
        &skipped,
        dropped as usize,
        capped,
        srid,
    ))
}

/// A new data layer's line and fill colours, by how many layers the drawing has.
#[wasm_bindgen(js_name = layerColors)]
pub fn layer_colors(layers: u32) -> Vec<String> {
    let (line, fill) = feed::layer_colors(layers as usize);
    vec![line.to_owned(), fill.to_owned()]
}

// ── Tiles ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceWire {
    srid: u32,
    vector: bool,
    min_level: u32,
    max_level: u32,
    tile_width: u32,
    tile_height: u32,
    quadtree: bool,
}

/// A service layer's tiles (`template`: a vector layer's from its style or TileJSON): its system, kind, levels and
/// first tile's size; none for one asked view by view.
#[wasm_bindgen(js_name = sourceOf)]
pub fn source_of(service: &str, template: Option<String>) -> Result<Option<String>, JsError> {
    let s: ServiceLayer = parse(service, "servis")?;
    let tiled = ServiceLayer {
        dynamic: false,
        ..s
    };
    Ok(source::tiles(&tiled, template.as_deref()).map(|src| {
        let (tw, th) = src
            .grid
            .matrices
            .first()
            .map_or((256, 256), |m| (m.tile_w, m.tile_h));
        write(&SourceWire {
            srid: src.srid,
            vector: src.vector,
            min_level: src.min_level,
            max_level: src.max_level,
            tile_width: tw,
            tile_height: th,
            quadtree: src.grid.quadtree,
        })
    }))
}

/// The address of a tile (without its connection's proof); none for Google's (its session's).
#[wasm_bindgen(js_name = tileUrl)]
pub fn tile_url(
    service: &str,
    template: Option<String>,
    level: u32,
    col: f64,
    row: f64,
    hidpi: bool,
) -> Result<Option<String>, JsError> {
    let s: ServiceLayer = parse(service, "servis")?;
    let tiled = ServiceLayer {
        dynamic: false,
        ..s
    };
    let Some(src) = source::tiles(&tiled, template.as_deref()) else {
        return Ok(None);
    };
    let t = TileRef {
        level,
        col: col as u64,
        row: row as u64,
    };
    Ok(source::tile_url(&tiled, &src, t, hidpi))
}

/// Google's `createSession` request for a map type (the key goes with the connection).
#[wasm_bindgen(js_name = googleSessionRequest)]
pub fn google_session_request(map_type: &str, hidpi: bool) -> String {
    wire(&google::create_session(&google::SessionOptions {
        map_type: map_type.to_owned(),
        language: "tr-TR".into(),
        region: "TR".into(),
        hidpi,
    }))
}

/// A Google session.
#[wasm_bindgen]
pub struct GoogleSession {
    inner: google::Session,
}

#[wasm_bindgen]
impl GoogleSession {
    /// The session an answer gives, or Google's words why not.
    pub fn read(body: &str) -> Result<GoogleSession, JsError> {
        google::read_session(body)
            .map(|inner| GoogleSession { inner })
            .map_err(err)
    }

    #[wasm_bindgen(js_name = tileSize)]
    pub fn tile_size(&self) -> u32 {
        self.inner.tile_size
    }

    pub fn stale(&self, now_ms: f64) -> bool {
        google::session_stale(&self.inner, now_ms as u64)
    }

    #[wasm_bindgen(js_name = tileUrl)]
    pub fn tile_url(&self, z: u32, x: f64, y: f64) -> String {
        google::tile_url(&self.inner, z, x as u64, y as u64)
    }

    #[wasm_bindgen(js_name = viewportUrl)]
    pub fn viewport_url(&self, zoom: u32, south: f64, west: f64, north: f64, east: f64) -> String {
        google::viewport_url(&self.inner, zoom, south, west, north, east)
    }
}

/// Google's credits for a view's answer.
#[wasm_bindgen(js_name = googleCopyright)]
pub fn google_copyright(body: &str) -> Result<String, JsError> {
    google::read_copyright(body).map_err(err)
}

/// A service as a view draws it: its grid and the ways between the project's system and its own.
#[wasm_bindgen]
pub struct View {
    grid: Grid,
    min: u32,
    max: u32,
    srid: u32,
    vector: bool,
    same: bool,
    ours: Option<System>,
    theirs: Option<System>,
    choices: Vec<Choice>,
    metres_per_unit: f64,
    across: u32,
    down: u32,
}

impl View {
    fn to_grid_xy(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        if self.same {
            return Some((x, y));
        }
        let (a, b) = (self.ours.as_ref()?, self.theirs.as_ref()?);
        transform_in(a, b, Vec2 { x, y }, &self.choices)
            .ok()
            .map(|t| (t.point.x, t.point.y))
    }

    fn to_project_xy(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        if self.same {
            return Some((x, y));
        }
        let (a, b) = (self.theirs.as_ref()?, self.ours.as_ref()?);
        transform_in(a, b, Vec2 { x, y }, &self.choices)
            .ok()
            .map(|t| (t.point.x, t.point.y))
    }

    fn tile(level: u32, col: f64, row: f64) -> TileRef {
        TileRef {
            level,
            col: col as u64,
            row: row as u64,
        }
    }
}

#[wasm_bindgen]
impl View {
    /// `service`'s view (`template` a vector layer's; `tile_size` a Google session's side, 0: none) between the
    /// project's system `ours` and the service's `theirs` (the core's JSON; absent for a project with none), the
    /// project's datum choices; `same` when the two are one (the tiles drawn as they are).
    #[wasm_bindgen(constructor)]
    pub fn new(
        service: &str,
        template: Option<String>,
        ours: Option<String>,
        theirs: Option<String>,
        choice_list: &str,
        same: bool,
        tile_size: u32,
    ) -> Result<View, JsError> {
        let s: ServiceLayer = parse(service, "servis")?;
        let tiled = ServiceLayer {
            dynamic: false,
            ..s
        };
        let mut src = source::tiles(&tiled, template.as_deref()).ok_or_else(|| {
            JsError::new(
                "servisin karo ızgarası ya da istenecek sistemi yok; servisi yeniden ekleyin.",
            )
        })?;
        if tile_size > 0
            && src.srid == 3857
            && src
                .grid
                .matrices
                .first()
                .is_some_and(|m| m.tile_w != tile_size)
        {
            src.grid = Grid::web_mercator(tile_size, src.max_level);
        }
        let ours = ours.map(|t| system(&t)).transpose()?;
        let theirs = theirs.map(|t| system(&t)).transpose()?;
        if !same && (ours.is_none() || theirs.is_none()) {
            return Err(JsError::new(
                "Projenin ya da servisin koordinat sistemi yok: harita servisi gösterilemez.",
            ));
        }
        let degrees = matches!(theirs, Some(System::Geographic { .. }));
        let (tw, th) = src
            .grid
            .matrices
            .first()
            .map_or((256, 256), |m| (m.tile_w, m.tile_h));
        Ok(View {
            metres_per_unit: if degrees { 111_320.0 } else { 1.0 },
            across: if src.vector {
                1
            } else {
                picture::slots_across(tw)
            },
            down: if src.vector {
                1
            } else {
                picture::slots_across(th)
            },
            grid: src.grid,
            min: src.min_level,
            max: src.max_level,
            srid: src.srid,
            vector: src.vector,
            same,
            ours,
            theirs,
            choices: choices(choice_list)?,
        })
    }

    pub fn srid(&self) -> u32 {
        self.srid
    }

    pub fn vector(&self) -> bool {
        self.vector
    }

    pub fn same(&self) -> bool {
        self.same
    }

    pub fn across(&self) -> u32 {
        self.across
    }

    pub fn down(&self) -> u32 {
        self.down
    }

    pub fn quadtree(&self) -> bool {
        self.grid.quadtree
    }

    /// What the view `[x₁, y₁, x₂, y₂]` (the project's, `px_per_unit` pixels a unit) shows:
    /// `[level, unitsPerPx, x₁, y₁, x₂, y₂, cx, cy, col, row, col, row, …]` in the grid's system, the centre's
    /// tiles first; empty when nothing.
    pub fn tiles(&self, x1: f64, y1: f64, x2: f64, y2: f64, px_per_unit: f64) -> Vec<f64> {
        let Some(s) = shown(
            &self.grid,
            self.min,
            self.max,
            [x1, y1, x2, y2],
            px_per_unit,
            |x, y| self.to_grid_xy(x, y),
        ) else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(8 + 2 * s.tiles.len());
        out.extend([s.level as f64, s.view.units_per_px]);
        out.extend(s.view.bbox);
        out.extend(s.view.center);
        for t in &s.tiles {
            out.push(t.col as f64);
            out.push(t.row as f64);
        }
        out
    }

    /// How finely a tile at `level` is divided.
    pub fn cells(&self, level: u32) -> u32 {
        cells_of(
            &self.grid,
            level,
            self.same,
            self.metres_per_unit,
            self.across.max(self.down),
        )
    }

    /// A tile's mesh: `(n + 1)²` nodes, row by row from its top left, in the project's system (NaN: no place).
    pub fn mesh(&self, level: u32, col: f64, row: f64) -> Vec<f64> {
        let mut out = Vec::new();
        let n = self.cells(level);
        mesh(
            &self.grid,
            Self::tile(level, col, row),
            n,
            |x, y| self.to_project_xy(x, y),
            &mut out,
        );
        out
    }

    /// A tile's box in the grid's system.
    pub fn bounds(&self, level: u32, col: f64, row: f64) -> Vec<f64> {
        self.grid
            .matrices
            .get(level as usize)
            .map(|m| m.bounds(col as u64, row as u64).to_vec())
            .unwrap_or_default()
    }

    /// A vector tile's style zoom (MapLibre's 512-pixel tiles) for `units_per_px` grid units a pixel.
    #[wasm_bindgen(js_name = displayZoom)]
    pub fn display_zoom(&self, units_per_px: f64) -> f64 {
        let Some(m) = self.grid.matrices.first() else {
            return 0.0;
        };
        let z = (m.resolution * f64::from(m.tile_w) / 512.0 / units_per_px).log2();
        if z.is_finite() {
            z.clamp(0.0, 24.0)
        } else {
            0.0
        }
    }

    #[wasm_bindgen(js_name = toGrid)]
    pub fn to_grid(&self, x: f64, y: f64) -> Vec<f64> {
        self.to_grid_xy(x, y)
            .map(|(a, b)| vec![a, b])
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = toProject)]
    pub fn to_project(&self, x: f64, y: f64) -> Vec<f64> {
        self.to_project_xy(x, y)
            .map(|(a, b)| vec![a, b])
            .unwrap_or_default()
    }
}

// ── Pictures ────────────────────────────────────────────────────────────

/// A picture tile cut into the atlas's slots.
#[wasm_bindgen]
pub struct PictureCut {
    across: u32,
    down: u32,
    bytes: Vec<u8>,
}

#[wasm_bindgen]
impl PictureCut {
    pub fn across(&self) -> u32 {
        self.across
    }

    pub fn down(&self) -> u32 {
        self.down
    }

    /// The slots one after another, each `258 × 258 × 4` bytes, row by row.
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

/// `rgba` (straight alpha, `width` × `height`) cut as a tile of `want_w` × `want_h` is; none when clear.
#[wasm_bindgen(js_name = cutPicture)]
pub fn cut_picture(
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    want_w: u32,
    want_h: u32,
) -> Option<PictureCut> {
    let want = (want_w > 0 && want_h > 0).then_some((want_w, want_h));
    let cut = picture::slots(width as usize, height as usize, rgba, want)?;
    Some(PictureCut {
        across: cut.across,
        down: cut.down,
        bytes: cut.slots.concat(),
    })
}

// ── Vector tiles ────────────────────────────────────────────────────────

/// A vector service's style and the source its tiles are of.
#[wasm_bindgen]
pub struct VectorStyle {
    style: style::Style,
    source: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceInfo {
    id: String,
    tiles: Vec<String>,
    url: Option<String>,
    min_zoom: u32,
    max_zoom: u32,
    attribution: Option<String>,
}

#[wasm_bindgen]
impl VectorStyle {
    /// A MapLibre style read from `url`.
    pub fn parse(text: &str, url: &str) -> Result<VectorStyle, JsError> {
        let style = style::parse(text, url).map_err(err)?;
        let source = style
            .main_vector()
            .map(|s| s.id.clone())
            .ok_or_else(|| JsError::new("Stilin vektör kaynağı yok."))?;
        Ok(VectorStyle { style, source })
    }

    /// The plain style a template or a TileJSON gets (its `layers`).
    pub fn basic(template: &str, min: u32, max: u32, layers: Vec<String>) -> VectorStyle {
        VectorStyle {
            style: style::basic("kaynak", template, min, max, &layers),
            source: "kaynak".into(),
        }
    }

    /// The style's main vector source: `{id, tiles, url, minZoom, maxZoom, attribution}`.
    pub fn source(&self) -> Option<String> {
        self.style.source(&self.source).map(|s| {
            write(&SourceInfo {
                id: s.id.clone(),
                tiles: s.tiles.clone(),
                url: s.url.clone(),
                min_zoom: s.min_zoom,
                max_zoom: s.max_zoom,
                attribution: s.attribution.clone(),
            })
        })
    }

    /// What the style uses that KentOS does not draw.
    pub fn notes(&self) -> Vec<String> {
        self.style.notes.clone()
    }

    /// A vector tile's bytes (gzipped or not) drawn at `zoom` into the style engine's batches (their origin
    /// `ox`, `oy`), its label candidates kept in `labels` under `id`.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        &self,
        view: &View,
        bytes: &[u8],
        level: u32,
        col: f64,
        row: f64,
        zoom: f64,
        ox: f64,
        oy: f64,
        id: u32,
        labels: &mut LabelStore,
    ) -> Result<BuiltTile, JsError> {
        let raw = mvt::inflate(bytes).map_err(err)?;
        let layers = mvt::read(&raw).map_err(err)?;
        // The vertices through the tile's mesh, as a picture tile is drawn (docs/adr/0208 §9).
        let t = View::tile(level, col, row);
        let n = cells_of(&view.grid, level, view.same, view.metres_per_unit, 1);
        let map = MeshMap::new(&view.grid, t, n, |x, y| view.to_project_xy(x, y));
        let to_project = |u: f64, v: f64| map.at(u, v).map(|(x, y)| Vec2 { x, y });
        let out = build(&TileInput {
            layers: &layers,
            style: &self.style,
            source: &self.source,
            zoom,
            to_project: &to_project,
            origin: Vec2 { x: ox, y: oy },
            tile: u64::from(id),
        });
        labels.tiles.insert(id, out.labels);
        Ok(BuiltTile {
            json: out.batches.json,
            data: out.batches.data,
        })
    }
}

/// A built vector tile: the style engine's batches, as a layer's are.
#[wasm_bindgen]
pub struct BuiltTile {
    json: String,
    data: Vec<f32>,
}

#[wasm_bindgen]
impl BuiltTile {
    pub fn json(&self) -> String {
        self.json.clone()
    }

    pub fn data(&self) -> Vec<f32> {
        self.data.clone()
    }
}

/// A TileJSON read from `base`.
#[wasm_bindgen(js_name = readTileJson)]
pub fn read_tile_json(text: &str, base: &str) -> Result<String, JsError> {
    kentos_services::caps::tilejson::read(text, base)
        .map(|t| write(&t))
        .map_err(err)
}

// ── Labels ──────────────────────────────────────────────────────────────

/// The label candidates of the vector tiles built, by tile.
#[wasm_bindgen]
#[derive(Default)]
pub struct LabelStore {
    tiles: HashMap<u32, Vec<Candidate>>,
}

#[derive(Serialize)]
struct Line<'a> {
    text: &'a str,
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct Glyph {
    ch: String,
    x: f64,
    y: f64,
    angle: f64,
}

#[derive(Serialize)]
struct Shown<'a> {
    lines: Vec<Line<'a>>,
    glyphs: Vec<Glyph>,
    size: f64,
    /// `[r, g, b, a]`, 0 to 1, the opacity in `a`.
    color: [f64; 4],
    /// `[r, g, b, a, width]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    halo: Option<[f64; 5]>,
    bold: bool,
    italic: bool,
}

#[wasm_bindgen]
impl LabelStore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> LabelStore {
        LabelStore::default()
    }

    pub fn remove(&mut self, id: u32) {
        self.tiles.remove(&id);
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
    }

    pub fn has(&self, id: u32) -> bool {
        self.tiles.contains_key(&id)
    }

    /// The labels of the tiles `ids` placed on a screen `width` × `height` whose top left is the project's
    /// (`x0`, `y0`), `px_per_unit` pixels a unit: each its lines or its letters, size, colour and halo.
    pub fn place(
        &self,
        ids: Vec<u32>,
        x0: f64,
        y0: f64,
        px_per_unit: f64,
        width: f64,
        height: f64,
    ) -> String {
        let cands: Vec<&Candidate> = ids
            .iter()
            .filter_map(|id| self.tiles.get(id))
            .flat_map(|t| t.iter())
            .collect();
        let screen = Screen {
            x0,
            y0,
            px_per_unit,
            width,
            height,
        };
        let mut placed = Vec::new();
        place(&cands, &screen, &mut placed);
        let shown: Vec<Shown<'_>> = placed
            .iter()
            .filter_map(|p| {
                let c = cands.get(p.candidate)?;
                let s = &c.style;
                Some(Shown {
                    lines: p
                        .lines
                        .iter()
                        .map(|l| Line {
                            text: &l.text,
                            x: l.x,
                            y: l.y,
                        })
                        .collect(),
                    glyphs: p
                        .glyphs
                        .iter()
                        .map(|g| Glyph {
                            ch: g.ch.to_string(),
                            x: g.x,
                            y: g.y,
                            angle: g.angle,
                        })
                        .collect(),
                    size: s.size,
                    color: [
                        s.color[0],
                        s.color[1],
                        s.color[2],
                        (s.color[3] * s.opacity).clamp(0.0, 1.0),
                    ],
                    halo: s
                        .halo
                        .filter(|(c, w)| c[3] > 0.0 && *w > 0.0)
                        .map(|(c, w)| {
                            [
                                c[0],
                                c[1],
                                c[2],
                                (c[3] * s.opacity).clamp(0.0, 1.0),
                                w.min(3.0),
                            ]
                        }),
                    bold: s.bold,
                    italic: s.italic,
                })
            })
            .collect();
        write(&shown)
    }
}

// ── Info, credits ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct Asked {
    media: String,
    request: serde_json::Value,
}

/// Servis bilgisi's requests for `service` about (`x`, `y`) in its system `srid`, `units` its units a pixel.
#[wasm_bindgen(js_name = infoRequests)]
pub fn info_requests(
    service: &str,
    srid: u32,
    x: f64,
    y: f64,
    units: f64,
) -> Result<String, JsError> {
    let s: ServiceLayer = parse(service, "servis")?;
    let list: Vec<Asked> = info::requests(&s, srid, x, y, units)
        .into_iter()
        .map(|(media, r)| Asked {
            media,
            request: serde_json::from_str(&wire(&r)).unwrap_or(serde_json::Value::Null),
        })
        .collect();
    Ok(write(&list))
}

/// An info answer's rows.
#[wasm_bindgen(js_name = infoRead)]
pub fn info_read(media: &str, body: &str) -> String {
    write(&info::read(media, body))
}

/// A credit as text and links.
#[wasm_bindgen]
pub fn credit(html: &str) -> String {
    write(&attribution::credit(html))
}

/// Credits joined without repeats.
#[wasm_bindgen]
pub fn joined(credits: Vec<String>) -> Vec<String> {
    attribution::joined(credits.iter().map(String::as_str))
}
