//! Google's Map Tiles API, 2D tiles (docs/adr/0208 §8): a session is asked
//! for with the user's API key (`createSession`), its token goes with every
//! tile; the view's credits come from `viewport`. The key goes in the
//! address as `key`, added by the connection (`auth::apply`), so neither it
//! nor the session is ever in a cache key.

use crate::query::with_params;
use crate::request::Request;

pub const TILES: &str = "https://tile.googleapis.com";

/// What a session is asked with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionOptions {
    /// `roadmap`, `satellite`, `terrain` or `hybrid` (satellite with the roads' layer).
    pub map_type: String,
    pub language: String,
    pub region: String,
    /// Tiles for a high density screen (`scaleFactor2x`, `highDpi`).
    pub hidpi: bool,
}

/// The `createSession` request (the key is added by the connection).
pub fn create_session(o: &SessionOptions) -> Request {
    let (map, roads) = match o.map_type.as_str() {
        "hybrid" => ("satellite", true),
        "terrain" => ("terrain", true),
        "satellite" => ("satellite", false),
        _ => ("roadmap", false),
    };
    let mut body = serde_json::json!({
        "mapType": map,
        "language": o.language,
        "region": o.region,
    });
    if roads {
        body["layerTypes"] = serde_json::json!(["layerRoadmap"]);
    }
    if o.hidpi {
        body["scale"] = serde_json::json!("scaleFactor2x");
        body["highDpi"] = serde_json::json!(true);
    }
    Request::json(format!("{TILES}/v1/createSession"), body.to_string())
}

/// A session Google gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub token: String,
    /// When it stops, ms since the epoch.
    pub expires_ms: u64,
    pub tile_size: u32,
}

/// The session of an answer to [`create_session`], or Google's words why not.
pub fn read_session(body: &str) -> Result<Session, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "Google'ın yanıtı JSON değil.".to_owned())?;
    if let Some(m) = v["error"]["message"].as_str() {
        return Err(format!("Google oturum vermedi: {m}"));
    }
    let token = v["session"]
        .as_str()
        .ok_or_else(|| "Google oturum vermedi.".to_owned())?;
    // `expiry` is seconds since the epoch, as a text.
    let expiry = v["expiry"]
        .as_str()
        .and_then(|e| e.parse::<u64>().ok())
        .or_else(|| v["expiry"].as_u64())
        .unwrap_or(0);
    Ok(Session {
        token: token.to_owned(),
        expires_ms: expiry.saturating_mul(1000),
        tile_size: v["tileWidth"]
            .as_u64()
            .and_then(|w| u32::try_from(w).ok())
            .unwrap_or(256),
    })
}

/// Whether a session is to be asked for again: in its last half hour.
pub fn session_stale(s: &Session, now_ms: u64) -> bool {
    now_ms.saturating_add(30 * 60_000) >= s.expires_ms
}

/// A 2D tile's address (the key is added by the connection).
pub fn tile_url(session: &Session, z: u32, x: u64, y: u64) -> String {
    with_params(
        &format!("{TILES}/v1/2dtiles/{z}/{x}/{y}"),
        &[("session", &session.token)],
    )
}

/// The view's credits' address.
pub fn viewport_url(
    session: &Session,
    zoom: u32,
    south: f64,
    west: f64,
    north: f64,
    east: f64,
) -> String {
    let (z, s, w, n, e) = (
        zoom.to_string(),
        crate::query::number(south.clamp(-89.999_999, 89.999_999)),
        crate::query::number(west.clamp(-179.999_999, 179.999_999)),
        crate::query::number(north.clamp(-89.999_999, 89.999_999)),
        crate::query::number(east.clamp(-179.999_999, 179.999_999)),
    );
    with_params(
        &format!("{TILES}/tile/v1/viewport"),
        &[
            ("session", &session.token),
            ("zoom", &z),
            ("north", &n),
            ("south", &s),
            ("east", &e),
            ("west", &w),
        ],
    )
}

/// The credits of an answer to [`viewport_url`]: “Google Maps” and the
/// data's copyright, as the policies ask them to be shown.
pub fn read_copyright(body: &str) -> Result<String, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "Google'ın yanıtı JSON değil.".to_owned())?;
    if let Some(m) = v["error"]["message"].as_str() {
        return Err(format!("Google görünümün bilgisini vermedi: {m}"));
    }
    let copyright = v["copyright"].as_str().unwrap_or("").trim();
    Ok(if copyright.is_empty() {
        "Google Maps".to_owned()
    } else {
        format!("Google Maps · {copyright}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_its_tiles_and_credits() {
        let r = create_session(&SessionOptions {
            map_type: "hybrid".into(),
            language: "tr-TR".into(),
            region: "TR".into(),
            hidpi: true,
        });
        assert_eq!(r.url, "https://tile.googleapis.com/v1/createSession");
        let body: serde_json::Value = serde_json::from_str(&r.body.as_ref().unwrap().0).unwrap();
        assert_eq!(body["mapType"], "satellite");
        assert_eq!(body["layerTypes"][0], "layerRoadmap");
        assert_eq!(body["scale"], "scaleFactor2x");
        let s = read_session(r#"{"session":"IgAAAHGU","expiry":"1361828036","tileWidth":512,"tileHeight":512,"imageFormat":"png"}"#).unwrap();
        assert_eq!((s.expires_ms, s.tile_size), (1_361_828_036_000, 512));
        assert!(session_stale(&s, 1_361_828_036_000 - 60_000));
        assert_eq!(
            tile_url(&s, 3, 4, 2),
            "https://tile.googleapis.com/v1/2dtiles/3/4/2?session=IgAAAHGU"
        );
        assert!(
            read_session(r#"{"error":{"code":403,"message":"API key not valid"}}"#)
                .unwrap_err()
                .contains("API key")
        );
        assert_eq!(
            read_copyright(r#"{"copyright":"Map data ©2026 Google","maxZoomRects":[]}"#).unwrap(),
            "Google Maps · Map data ©2026 Google"
        );
        assert!(
            viewport_url(&s, 12, 39.0, 32.0, 40.0, 33.0)
                .contains("&zoom=12&north=40&south=39&east=33&west=32")
        );
    }
}
