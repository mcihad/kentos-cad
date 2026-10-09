//! What a service needs before its tiles (docs/adr/0208 §5–§9, §12), read
//! on a services' thread: its connection's secrets (and a token, for ArcGIS
//! and OAuth 2), a vector service's style and its tiles' template (from the
//! style's source, its TileJSON, or the layer's own template with a plain
//! style), Google's session; then its grid, and the ways between its system
//! and the project's. What fails is said once, in the service's name.

use std::sync::{Arc, Mutex, PoisonError};

use kentos_cloud::fetch;
use kentos_contracts::{AuthKind, ConnectionSecret, ServiceConnection, ServiceKind, ServiceLayer};
use kentos_render_wgpu::styled::ServiceView;
use kentos_services::auth::{self, Token};
use kentos_services::caps::tilejson;
use kentos_services::google::{self, Session, SessionOptions};
use kentos_services::source::{self, TileSource};
use kentos_services::style::{self, Style};
use kentos_services::{attribution, presets};

use super::cache::now_ms;
use super::decode::slots_across;
use super::hub::{Entry, Ready, Shared, State};
use super::net::{name_of, to_fetch};

/// Resolves `entry`; its state says how it went.
pub(super) fn resolve(s: &Arc<Shared>, entry: &Arc<Entry>) {
    let state = match run(entry) {
        Ok(ready) => State::Ready(Arc::new(ready)),
        Err(message) => {
            s.notice(format!("“{}”: {message}", name_of(entry)));
            State::Failed {
                message,
                at_ms: now_ms(),
                secrets: super::secrets::secrets().version(),
            }
        }
    };
    *entry.state.lock().unwrap_or_else(PoisonError::into_inner) = state;
    s.tell();
}

/// A request made with the connection's proof, sent and waited for: its body, or why not.
fn ask(
    req: kentos_services::request::Request,
    conn: Option<&(ServiceConnection, ConnectionSecret)>,
    token: Option<&Token>,
    per_host: usize,
) -> Result<String, String> {
    let mut req = req;
    if let Some((c, secret)) = conn {
        auth::apply(c, secret, token, &mut req);
    }
    let res = fetch::wait(to_fetch(req), per_host).map_err(|f| f.message)?;
    if !(200..300).contains(&res.status) {
        return Err(match res.status {
            401 | 403 => format!(
                "sunucu erişimi reddetti ({}); bağlantının bilgilerini Bağlantılar penceresinden denetleyin.",
                res.status
            ),
            404 => format!("adres bulunamadı (404): {}", res.url),
            s => format!("sunucu {s} dedi."),
        });
    }
    String::from_utf8(res.body).map_err(|_| "sunucunun cevabı metin değil.".to_owned())
}

/// The connection and its secrets on this device; why not when they lack.
fn proof(entry: &Entry) -> Result<Option<(ServiceConnection, ConnectionSecret)>, String> {
    let Some(c) = &entry.connection else {
        return Ok(None);
    };
    let secret = super::secrets::secrets().get(&c.origin, &c.id);
    if let Some(why) = auth::missing(c, secret.as_ref()) {
        return Err(why);
    }
    Ok(secret.map(|s| (c.clone(), s)))
}

/// A token for an ArcGIS or OAuth 2 connection.
fn token_for(
    conn: &(ServiceConnection, ConnectionSecret),
    referer: &str,
    per_host: usize,
) -> Result<Option<Token>, String> {
    let (c, secret) = conn;
    if !matches!(c.auth, AuthKind::Arcgis | AuthKind::Oauth2) {
        return Ok(None);
    }
    let Some(req) = auth::token_request(c, secret, referer) else {
        return Ok(None);
    };
    let body = fetch::wait(to_fetch(req), per_host)
        .map_err(|f| f.message)
        .map(|r| String::from_utf8_lossy(&r.body).into_owned())?;
    auth::read_token(c, &body, now_ms()).map(Some)
}

fn run(entry: &Entry) -> Result<Ready, String> {
    let s = &entry.service;
    let per_host = presets::per_host(s) as usize;
    let conn = proof(entry)?;
    let token = match &conn {
        Some(c) => token_for(c, &s.url, per_host)?,
        None => None,
    };
    let mut attribution_text = s.attribution.clone().unwrap_or_default();
    let mut google_session: Option<Session> = None;
    let (source, style_read, source_id) = match s.kind {
        ServiceKind::Vector => {
            let (src, st, id, credit) = vector(s, conn.as_ref(), token.as_ref(), per_host)?;
            if attribution_text.is_empty() {
                attribution_text = credit;
            }
            (src, Some(Arc::new(st)), id)
        }
        ServiceKind::Google => {
            let session = google_session_for(s, conn.as_ref(), per_host)?;
            let mut src = source::tiles(s, None).ok_or("Google katmanının karoları kurulamadı.")?;
            if session.tile_size != src.grid.matrices.first().map_or(256, |m| m.tile_w) {
                src.grid = kentos_geometry_core::geom::tiles::Grid::web_mercator(
                    session.tile_size,
                    src.max_level,
                );
            }
            google_session = Some(session);
            if attribution_text.is_empty() {
                attribution_text = "Google Maps".into();
            }
            (src, None, String::new())
        }
        _ => {
            // A view-sized `export` is drawn as tiles of its own grid too.
            let tiled = ServiceLayer {
                dynamic: false,
                ..s.clone()
            };
            let src = source::tiles(&tiled, None).ok_or(
                "servisin karo ızgarası ya da istenecek sistemi yok; servisi yeniden ekleyin.",
            )?;
            (src, None, String::new())
        }
    };
    let pair = super::systems::pair(&entry.project, source.srid)?;
    let (tw, th) = source
        .grid
        .matrices
        .first()
        .map_or((256, 256), |m| (m.tile_w, m.tile_h));
    let view = ServiceView {
        id: entry.id,
        grid: source.grid.clone(),
        min_level: source.min_level,
        max_level: source.max_level,
        vector: source.vector,
        same_system: pair.same,
        to_grid: pair.to_grid,
        to_project: pair.to_project,
        metres_per_unit: pair.metres_per_unit,
        across: if source.vector { 1 } else { slots_across(tw) },
        down: if source.vector { 1 } else { slots_across(th) },
    };
    Ok(Ready {
        view: Arc::new(view),
        source,
        style: style_read,
        source_id,
        google: Mutex::new(google_session),
        token: Mutex::new(token),
        per_host,
        attribution: Mutex::new(credit_of(s, &attribution_text)),
    })
}

/// A service's credits as text and links; a ready basemap's own page when the text names none.
fn credit_of(s: &ServiceLayer, html: &str) -> attribution::Credit {
    let mut c = attribution::credit(html);
    if c.links.is_empty()
        && !c.text.is_empty()
        && let Some(url) = s
            .preset
            .as_deref()
            .and_then(presets::preset)
            .and_then(|p| p.attribution_url.clone())
    {
        let shown = url
            .split_once("://")
            .map_or(url.as_str(), |(_, rest)| rest)
            .trim_end_matches('/')
            .to_owned();
        c.links.push((shown, url));
    }
    c
}

/// A vector service's tiles, style, source and credits: its address is a
/// style, a TileJSON or a template.
fn vector(
    s: &ServiceLayer,
    conn: Option<&(ServiceConnection, ConnectionSecret)>,
    token: Option<&Token>,
    per_host: usize,
) -> Result<(TileSource, Style, String, String), String> {
    if s.url.contains("{z}") {
        let st = style::basic(
            "kaynak",
            &s.url,
            s.min_zoom.unwrap_or(0),
            s.max_zoom.unwrap_or(14),
            &[],
        );
        let src = source::tiles(s, Some(&s.url)).ok_or("vektör karoların şablonu okunamadı.")?;
        return Ok((src, st, "kaynak".into(), String::new()));
    }
    let text = ask(
        kentos_services::request::Request::get(s.url.clone()),
        conn,
        token,
        per_host,
    )?;
    let is_style = serde_json::from_str::<serde_json::Value>(&text)
        .map(|v| v["layers"].is_array() && v["sources"].is_object())
        .unwrap_or(false);
    let (st, tiles, min, max, credit, id) = if is_style {
        let st = style::parse(&text, &s.url)?;
        let main = st
            .main_vector()
            .cloned()
            .ok_or("stilin vektör kaynağı yok.")?;
        let (tiles, min, max, credit) = if main.tiles.is_empty() {
            let tj_url = main
                .url
                .clone()
                .ok_or("stilin vektör kaynağının adresi yok.")?;
            let tj = tilejson::read(
                &ask(
                    kentos_services::request::Request::get(tj_url.clone()),
                    conn,
                    token,
                    per_host,
                )?,
                &tj_url,
            )?;
            (
                tj.tiles,
                tj.min_zoom,
                tj.max_zoom,
                tj.attribution.unwrap_or_default(),
            )
        } else {
            (
                main.tiles.clone(),
                main.min_zoom,
                main.max_zoom,
                main.attribution.clone().unwrap_or_default(),
            )
        };
        (st, tiles, min, max, credit, main.id.clone())
    } else {
        let tj = tilejson::read(&text, &s.url)?;
        let st = style::basic("kaynak", &tj.tiles[0], tj.min_zoom, tj.max_zoom, &tj.layers);
        (
            st,
            tj.tiles.clone(),
            tj.min_zoom,
            tj.max_zoom,
            tj.attribution.unwrap_or_default(),
            "kaynak".into(),
        )
    };
    let template = tiles
        .first()
        .cloned()
        .ok_or("vektör karoların adresi yok.")?;
    let bounded = ServiceLayer {
        min_zoom: Some(s.min_zoom.unwrap_or(min)),
        max_zoom: Some(s.max_zoom.unwrap_or(max).min(max)),
        ..s.clone()
    };
    let src =
        source::tiles(&bounded, Some(&template)).ok_or("vektör karoların şablonu okunamadı.")?;
    Ok((src, st, id, credit))
}

/// A Google session for the layer's map type.
fn google_session_for(
    s: &ServiceLayer,
    conn: Option<&(ServiceConnection, ConnectionSecret)>,
    per_host: usize,
) -> Result<Session, String> {
    let conn = conn.ok_or("Google katmanının API anahtarı bir bağlantıda olmalı.")?;
    let req = google::create_session(&SessionOptions {
        map_type: s.style.clone().unwrap_or_else(|| "roadmap".into()),
        language: "tr-TR".into(),
        region: "TR".into(),
        hidpi: false,
    });
    let body = ask(req, Some(conn), None, per_host)?;
    google::read_session(&body)
}
