//! Map services (docs/adr/0208 §2): a layer drawn from a service (`service`:
//! XYZ and TMS tiles, WMTS, WMS, OGC API Tiles, ArcGIS REST, Google's 2D
//! tiles, vector tiles), a layer whose objects came from one (`feed`: WFS,
//! OGC API Features, an ArcGIS layer, a GeoJSON address), and the project's
//! connections, what a service asks of whoever reads it. A connection's
//! secrets are never in the drawing: the device keeps them
//! ([`ConnectionSecret`]); the drawing says only what kind of proof a
//! service wants and the names it goes under.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The longest an address or a URL template may be, in letters.
pub const MAX_SERVICE_URL: usize = 4096;
/// The most layers a WMS or ArcGIS service layer asks for.
pub const MAX_SERVICE_LAYERS: usize = 64;
/// The most matrices a tile grid has.
pub const MAX_TILE_MATRICES: usize = 40;
/// The most extra parameters a service layer sends.
pub const MAX_SERVICE_PARAMS: usize = 32;
/// The most subdomains an XYZ template takes turns on.
pub const MAX_SUBDOMAINS: usize = 16;
/// The finest zoom level a service layer may name.
pub const MAX_ZOOM: u32 = 30;
/// The least and most opacity a service layer is drawn with.
pub const MIN_SERVICE_OPACITY: f64 = 0.1;
pub const MAX_SERVICE_OPACITY: f64 = 1.0;
/// The most objects one taking from a service may bring.
pub const MAX_FEED_LIMIT: u64 = 500_000;
/// The most connections a project names.
pub const MAX_CONNECTIONS: usize = 256;
/// The most parameter or header names a connection sends.
pub const MAX_AUTH_NAMES: usize = 8;
/// Google's map types (docs/adr/0208 §8), in the menu's order.
pub const GOOGLE_MAP_TYPES: [&str; 4] = ["roadmap", "satellite", "terrain", "hybrid"];
/// The WMS versions read and asked.
pub const WMS_VERSIONS: [&str; 2] = ["1.1.1", "1.3.0"];
/// The WFS versions read and asked.
pub const WFS_VERSIONS: [&str; 3] = ["1.0.0", "1.1.0", "2.0.0"];

/// Names as the files and the wire hold them, both ways.
macro_rules! named {
    ($t:ty { $($v:ident => $n:literal),+ $(,)? }) => {
        impl $t {
            /// Every value, in the contract's order.
            pub const ALL: &'static [$t] = &[$(<$t>::$v),+];

            /// The name files and the wire hold.
            pub fn name(self) -> &'static str {
                match self {
                    $(<$t>::$v => $n),+
                }
            }

            pub fn from_name(name: &str) -> Option<$t> {
                match name {
                    $($n => Some(<$t>::$v)),+,
                    _ => None,
                }
            }
        }
    };
}

/// What a service layer draws (docs/adr/0208 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ServiceKind {
    /// Tiles by an address template, Web Mercator (TMS with `yFlip`).
    Xyz,
    Wms,
    Wmts,
    /// OGC API Tiles' map tiles.
    OgcTiles,
    /// An ArcGIS REST MapServer or ImageServer: its cached tiles, else `export`.
    Arcgis,
    /// Google's Map Tiles API, 2D tiles.
    Google,
    /// Vector tiles (MVT) by a MapLibre style, a TileJSON or a template.
    Vector,
}

named!(ServiceKind { Xyz => "xyz", Wms => "wms", Wmts => "wmts", OgcTiles => "ogcTiles", Arcgis => "arcgis", Google => "google", Vector => "vector" });

impl ServiceKind {
    /// Its name in the windows.
    pub fn label(self) -> &'static str {
        match self {
            ServiceKind::Xyz => "XYZ / TMS",
            ServiceKind::Wms => "WMS",
            ServiceKind::Wmts => "WMTS",
            ServiceKind::OgcTiles => "OGC API Tiles",
            ServiceKind::Arcgis => "ArcGIS REST",
            ServiceKind::Google => "Google Haritalar",
            ServiceKind::Vector => "Vektör karo",
        }
    }
}

/// A parameter sent with every request (`TIME`, `CQL_FILTER`, …): never a secret.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServiceParam {
    pub name: String,
    pub value: String,
}

/// One matrix of a tile grid (docs/adr/0208 §3): its pixel's size in the
/// grid system's units, its top left corner as east and north (longitude
/// and latitude), whatever order the service wrote them in, its tiles' and
/// its own size in tiles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TileMatrix {
    pub id: String,
    pub resolution: f64,
    pub x0: f64,
    pub y0: f64,
    pub tile_width: u32,
    pub tile_height: u32,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub matrix_width: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub matrix_height: u64,
}

/// A service's tile grid as read: its system and its matrices, coarsest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TileGrid {
    pub srid: u32,
    pub matrices: Vec<TileMatrix>,
}

/// A layer drawn from a service (docs/adr/0208 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServiceLayer {
    pub kind: ServiceKind,
    /// XYZ: the template; WMS, WMTS, ArcGIS: the service's address; OGC
    /// API Tiles: the tile set's; vector: the style's, the TileJSON's or the
    /// template; Google: empty.
    pub url: String,
    /// WMS's layer names; WMTS's one layer; ArcGIS's layers shown.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// WMS's and WMTS's style; Google's map type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    /// The image's media type (`image/png`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub format: Option<String>,
    /// The system asked: WMS's `CRS`, ArcGIS's `imageSR`; a grid's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub srid: Option<u32>,
    /// WMTS's, OGC's and a cached ArcGIS service's matrices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grid: Option<TileGrid>,
    /// WMTS's matrix set's identifier (`TILEMATRIXSET`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub matrix_set: Option<String>,
    /// WMTS REST's and OGC's tile template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<String>,
    /// XYZ's and the vector tiles' tile side, pixels; absent 256.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tile_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min_zoom: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max_zoom: Option<u32>,
    /// `{s}`'s values, taken in turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub subdomains: Vec<String>,
    /// TMS: rows counted from the bottom.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub y_flip: bool,
    /// WMS's and `export`'s `TRANSPARENT`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub transparent: bool,
    /// WMS's version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ServiceParam>>", optional))]
    pub params: Vec<ServiceParam>,
    /// WMS and `export`: one picture of the view instead of tiles.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub dynamic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attribution: Option<String>,
    /// 0.1 to 1; absent: opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opacity: Option<f64>,
    /// The project's connection whose proof goes with the requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub connection: Option<String>,
    /// The ready basemap it came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preset: Option<String>,
    /// The service's extent, WGS 84 degrees (west, south, east, north), as
    /// its capabilities give it: what “Servisin kapsamına yakınlaştır” shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bbox: Option<[f64; 4]>,
}

/// What a layer's objects were taken from (docs/adr/0208 §10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeedKind {
    Wfs,
    OgcFeatures,
    /// An ArcGIS REST layer's `query`.
    Arcgis,
    /// A GeoJSON document at an address.
    Geojson,
}

named!(FeedKind { Wfs => "wfs", OgcFeatures => "ogcFeatures", Arcgis => "arcgis", Geojson => "geojson" });

impl FeedKind {
    /// Its name in the windows.
    pub fn label(self) -> &'static str {
        match self {
            FeedKind::Wfs => "WFS",
            FeedKind::OgcFeatures => "OGC API Features",
            FeedKind::Arcgis => "ArcGIS REST",
            FeedKind::Geojson => "GeoJSON adresi",
        }
    }
}

/// Where a layer's objects came from, to take them again (docs/adr/0208 §10).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureFeed {
    pub kind: FeedKind,
    pub url: String,
    /// WFS's type, OGC's collection, ArcGIS's layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// The system asked of the service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub srid: Option<u32>,
    /// CQL, or ArcGIS's `where`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<String>,
    /// The area asked, `[x₁, y₁, x₂, y₂]` in the project's system; absent: all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bbox: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "number"))]
    pub limit: Option<u64>,
    /// WFS's version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<String>,
    /// The attribute that matches objects when they are taken again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub connection: Option<String>,
    /// When they were last taken (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fetched: Option<String>,
}

/// The proof a service asks for (docs/adr/0208 §12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AuthKind {
    None,
    /// `name=value` in the address, one or more.
    Query,
    /// `Name: value` headers, one or more.
    Header,
    /// HTTP Basic: a user name and a password.
    Basic,
    /// `Authorization: Bearer <token>`.
    Bearer,
    /// An ArcGIS token from `generateToken`, by a user name and a password.
    Arcgis,
    /// OAuth 2's client credentials at a token address.
    Oauth2,
    /// Google's API key and session.
    Google,
}

named!(AuthKind { None => "none", Query => "query", Header => "header", Basic => "basic", Bearer => "bearer", Arcgis => "arcgis", Oauth2 => "oauth2", Google => "google" });

impl AuthKind {
    /// Its name in the windows.
    pub fn label(self) -> &'static str {
        match self {
            AuthKind::None => "Yok",
            AuthKind::Query => "Adreste parametre",
            AuthKind::Header => "Başlıkta değer",
            AuthKind::Basic => "Kullanıcı adı ve parola",
            AuthKind::Bearer => "Belirteç (Bearer)",
            AuthKind::Arcgis => "ArcGIS belirteci",
            AuthKind::Oauth2 => "OAuth 2 (istemci kimliği)",
            AuthKind::Google => "Google API anahtarı",
        }
    }
}

/// A connection of the project (docs/adr/0208 §2): what proof requests to
/// its origin carry, without the secrets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServiceConnection {
    pub id: String,
    pub name: String,
    /// `https://host[:port]`: proof goes to this origin only.
    pub origin: String,
    pub auth: AuthKind,
    /// The parameters' (query) or headers' (header) names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub names: Vec<String>,
    /// ArcGIS's `generateToken` or OAuth 2's token address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub token_url: Option<String>,
    /// OAuth 2's scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scope: Option<String>,
}

/// A connection's secrets as the device keeps them, never in a drawing:
/// the values of its names in order, a user name and a password, a token,
/// a client's id and secret, Google's key.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConnectionSecret {
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub client_secret: Option<String>,
}

/// Why `text` is not an HTTP or HTTPS address of at most the longest
/// length without control characters, in the commands' words; `what` names it.
pub fn http_problem(what: &str, text: &str) -> Option<String> {
    let lower = text.trim().to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    if rest.is_none_or(|r| r.is_empty() || r.starts_with('/')) {
        return Some(format!(
            "{what} bir HTTP ya da HTTPS adresi olmalı (http:// ya da https:// ile başlamalı); “{text}” değil."
        ));
    }
    if text.chars().count() > MAX_SERVICE_URL || text.chars().any(char::is_control) {
        return Some(format!(
            "{what} en çok {MAX_SERVICE_URL} harf olmalı ve denetim karakteri içermemeli."
        ));
    }
    None
}

/// Whether `name` is a parameter's or a header's name: 1 to 64 letters of
/// the token set (letters, digits, `-`, `_`, `.`).
fn token_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// A text of at most `most` letters without control characters.
fn plain(text: &str, most: usize) -> bool {
    text.chars().count() <= most && !text.chars().any(char::is_control)
}

impl TileGrid {
    /// Why the grid is not one, in the commands' words; none when it is.
    pub fn problem(&self) -> Option<String> {
        if self.srid == 0 {
            return Some("Karo ızgarasının sistemi bir EPSG kodu olmalı.".into());
        }
        if self.matrices.is_empty() || self.matrices.len() > MAX_TILE_MATRICES {
            return Some(format!(
                "Karo ızgarasının 1 ile {MAX_TILE_MATRICES} arasında matrisi olmalı."
            ));
        }
        for (i, m) in self.matrices.iter().enumerate() {
            let n = i + 1;
            if m.id.is_empty() || !plain(&m.id, 128) {
                return Some(format!("{n}. matrisin adı 1 ile 128 harf arasında olmalı."));
            }
            if self.matrices[..i].iter().any(|o| o.id == m.id) {
                return Some(format!("“{}” matrisi iki kez var.", m.id));
            }
            if !(m.resolution.is_finite() && m.resolution > 0.0) {
                return Some(format!(
                    "{n}. matrisin pikseli sıfırdan büyük, sonlu olmalı."
                ));
            }
            if !(m.x0.is_finite() && m.y0.is_finite()) {
                return Some(format!("{n}. matrisin köşesi sonlu olmalı."));
            }
            let side = |v: u32| (1..=4096).contains(&v);
            if !side(m.tile_width) || !side(m.tile_height) {
                return Some(format!(
                    "{n}. matrisin karosu 1 ile 4096 piksel arasında olmalı."
                ));
            }
            let count = |v: u64| (1..=(1u64 << 40)).contains(&v);
            if !count(m.matrix_width) || !count(m.matrix_height) {
                return Some(format!(
                    "{n}. matrisin karo sayısı 1 ile 2⁴⁰ arasında olmalı."
                ));
            }
        }
        None
    }
}

impl ServiceLayer {
    /// Why the fields do not make a service layer, in the commands' words; none when they do.
    pub fn problem(&self) -> Option<String> {
        if self.kind == ServiceKind::Google {
            if !self.url.is_empty() {
                return Some(
                    "Google katmanının adresi boş olmalı; karoların adresi oturumdan gelir.".into(),
                );
            }
            if !self
                .style
                .as_deref()
                .is_some_and(|s| GOOGLE_MAP_TYPES.contains(&s))
            {
                return Some(format!(
                    "Google katmanının harita türü {} türlerinden biri olmalı.",
                    GOOGLE_MAP_TYPES.join(", ")
                ));
            }
            if self.connection.is_none() {
                return Some("Google katmanının API anahtarı bir bağlantıda olmalı.".into());
            }
        } else if let Some(p) = http_problem("Servisin adresi", &self.url) {
            return Some(p);
        }
        match self.kind {
            ServiceKind::Xyz => {
                let u = &self.url;
                let rows = u.contains("{y}") || u.contains("{-y}");
                if !(u.contains("{quadkey}") || (u.contains("{z}") && u.contains("{x}") && rows)) {
                    return Some(
                        "XYZ şablonunda {z}, {x} ve {y} (ya da {-y}) ya da {quadkey} olmalı."
                            .into(),
                    );
                }
                if u.contains("{s}") && self.subdomains.is_empty() {
                    return Some("Şablonda {s} var; alt alanları verin (a, b, c gibi).".into());
                }
            }
            ServiceKind::Wms => {
                if self.layers.is_empty() {
                    return Some("WMS katmanı en az bir katman adı istemeli.".into());
                }
                if self.srid.is_none() {
                    return Some("WMS katmanının istenen sistemi (srid) olmalı.".into());
                }
                if let Some(v) = &self.version
                    && !WMS_VERSIONS.contains(&v.as_str())
                {
                    return Some(format!(
                        "WMS sürümü {} olmalı; “{v}” değil.",
                        WMS_VERSIONS.join(" ya da ")
                    ));
                }
            }
            ServiceKind::Wmts => {
                if self.layers.len() != 1 {
                    return Some("WMTS katmanı tek bir katman göstermeli.".into());
                }
                if self.grid.is_none() || self.matrix_set.is_none() {
                    return Some(
                        "WMTS katmanının karo ızgarası (grid) ve matris kümesinin adı (matrixSet) olmalı.".into(),
                    );
                }
            }
            ServiceKind::OgcTiles => {
                if self.grid.is_none() || self.template.is_none() {
                    return Some(
                        "OGC API Tiles katmanının ızgarası ve karo şablonu olmalı.".into(),
                    );
                }
            }
            ServiceKind::Arcgis => {
                if self.srid.is_none() && self.grid.is_none() {
                    return Some(
                        "ArcGIS katmanının ızgarası ya da istenen sistemi (srid) olmalı.".into(),
                    );
                }
            }
            ServiceKind::Google | ServiceKind::Vector => {}
        }
        if self.layers.len() > MAX_SERVICE_LAYERS
            || self.layers.iter().any(|l| l.is_empty() || !plain(l, 256))
        {
            return Some(format!(
                "Katman adları en çok {MAX_SERVICE_LAYERS} tane ve 1 ile 256 harf arasında olmalı."
            ));
        }
        for (what, text, most) in [
            ("Stil", &self.style, 256),
            ("Biçim", &self.format, 128),
            ("Atıf", &self.attribution, 512),
            ("Sürüm", &self.version, 16),
            ("Hazır altlık", &self.preset, 64),
            ("Bağlantı", &self.connection, 64),
            ("Matris kümesi", &self.matrix_set, 256),
        ] {
            if let Some(t) = text
                && (t.is_empty() || !plain(t, most))
            {
                return Some(format!(
                    "{what} 1 ile {most} harf arasında olmalı ve denetim karakteri içermemeli."
                ));
            }
        }
        if let Some(t) = &self.template
            && let Some(p) = http_problem("Karo şablonu", t)
        {
            return Some(p);
        }
        if let Some(srid) = self.srid
            && srid == 0
        {
            return Some("İstenen sistem bir EPSG kodu olmalı.".into());
        }
        if let Some(grid) = &self.grid
            && let Some(p) = grid.problem()
        {
            return Some(p);
        }
        if let Some(t) = self.tile_size
            && !(64..=4096).contains(&t)
        {
            return Some("Karonun boyu 64 ile 4096 piksel arasında olmalı.".into());
        }
        let (lo, hi) = (
            self.min_zoom.unwrap_or(0),
            self.max_zoom.unwrap_or(MAX_ZOOM),
        );
        if lo > hi || hi > MAX_ZOOM {
            return Some(format!(
                "Katlar 0 ile {MAX_ZOOM} arasında olmalı, en küçüğü en büyüğünden büyük olmamalı."
            ));
        }
        if self.subdomains.len() > MAX_SUBDOMAINS || self.subdomains.iter().any(|s| !token_name(s))
        {
            return Some(format!(
                "Alt alanlar en çok {MAX_SUBDOMAINS} tane; her biri harf, rakam ve tire olmalı."
            ));
        }
        if self.params.len() > MAX_SERVICE_PARAMS {
            return Some(format!(
                "En çok {MAX_SERVICE_PARAMS} ek parametre olabilir."
            ));
        }
        for p in &self.params {
            if !token_name(&p.name) || !plain(&p.value, 1024) {
                return Some(format!(
                    "Ek parametre “{}”: adı harf, rakam, tire ve alt çizgi; değeri en çok 1024 harf olmalı.",
                    p.name
                ));
            }
        }
        if let Some(o) = self.opacity
            && !(o.is_finite() && (MIN_SERVICE_OPACITY..=MAX_SERVICE_OPACITY).contains(&o))
        {
            return Some(format!(
                "Servis katmanının donukluğu {MIN_SERVICE_OPACITY} ile {MAX_SERVICE_OPACITY} arasında olmalı; {o} verildi."
            ));
        }
        if let Some([w, s, e, n]) = self.bbox
            && !([w, s, e, n].iter().all(|v| v.is_finite())
                && (-180.0..=180.0).contains(&w)
                && (-180.0..=180.0).contains(&e)
                && (-90.0..=90.0).contains(&s)
                && (-90.0..=90.0).contains(&n)
                && w <= e
                && s <= n)
        {
            return Some(
                "Servisin kapsamı WGS 84 derecesinde batı, güney, doğu ve kuzey olmalı (boylam −180 ile 180, enlem −90 ile 90 arasında; batı doğudan, güney kuzeyden büyük değil).".into(),
            );
        }
        None
    }

    /// Its tile side, pixels.
    pub fn tile_side(&self) -> u32 {
        self.tile_size.unwrap_or(256)
    }
}

impl FeatureFeed {
    /// Why the fields do not make a feed, in the commands' words; none when they do.
    pub fn problem(&self) -> Option<String> {
        if let Some(p) = http_problem("Verinin adresi", &self.url) {
            return Some(p);
        }
        match (&self.name, self.kind) {
            (None, FeedKind::Wfs | FeedKind::OgcFeatures | FeedKind::Arcgis) => {
                return Some(format!(
                    "{} kaynağının tür, koleksiyon ya da katman adı (name) olmalı.",
                    self.kind.label()
                ));
            }
            (Some(n), _) if n.is_empty() || !plain(n, 256) => {
                return Some("Tür adı 1 ile 256 harf arasında olmalı.".into());
            }
            _ => {}
        }
        if let Some(v) = &self.version {
            if self.kind != FeedKind::Wfs {
                return Some("Sürüm yalnız WFS kaynağının olur.".into());
            }
            if !WFS_VERSIONS.contains(&v.as_str()) {
                return Some(format!(
                    "WFS sürümü {} olmalı; “{v}” değil.",
                    WFS_VERSIONS.join(", ")
                ));
            }
        }
        if let Some(srid) = self.srid
            && srid == 0
        {
            return Some("İstenen sistem bir EPSG kodu olmalı.".into());
        }
        if let Some(b) = &self.bbox
            && !(b.iter().all(|v| v.is_finite()) && b[0] <= b[2] && b[1] <= b[3])
        {
            return Some(
                "İstenen alan sonlu dört sayı olmalı, en küçükler en büyüklerden büyük olmamalı."
                    .into(),
            );
        }
        if let Some(l) = self.limit
            && !(1..=MAX_FEED_LIMIT).contains(&l)
        {
            return Some(format!(
                "En çok nesne 1 ile {MAX_FEED_LIMIT} arasında olmalı."
            ));
        }
        for (what, text, most) in [
            ("Süzgeç", &self.filter, 8192),
            ("Anahtar alan", &self.key, 128),
            ("Bağlantı", &self.connection, 64),
            ("Son alınış", &self.fetched, 40),
        ] {
            if let Some(t) = text
                && (t.is_empty() || !plain(t, most))
            {
                return Some(format!(
                    "{what} 1 ile {most} harf arasında olmalı ve denetim karakteri içermemeli."
                ));
            }
        }
        None
    }
}

impl ServiceConnection {
    /// Why the fields do not make a connection, in the commands' words; none when they do.
    pub fn problem(&self) -> Option<String> {
        let id_ok = !self.id.is_empty()
            && self.id.len() <= 64
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
        if !id_ok {
            return Some(
                "Bağlantının kimliği 1 ile 64 harf, rakam, tire ya da alt çizgi olmalı.".into(),
            );
        }
        if self.name.trim().is_empty() || !plain(&self.name, 128) {
            return Some("Bağlantının adı 1 ile 128 harf arasında olmalı.".into());
        }
        if let Some(p) = origin_problem(&self.origin) {
            return Some(p);
        }
        let named = matches!(self.auth, AuthKind::Query | AuthKind::Header);
        if named && (self.names.is_empty() || self.names.len() > MAX_AUTH_NAMES) {
            return Some(format!(
                "{} için 1 ile {MAX_AUTH_NAMES} arasında ad olmalı.",
                self.auth.label()
            ));
        }
        if !named && !self.names.is_empty() {
            return Some(format!("{} ad taşımaz.", self.auth.label()));
        }
        if self.names.iter().any(|n| !token_name(n)) {
            return Some(
                "Parametre ve başlık adları harf, rakam, tire, nokta ve alt çizgiden olmalı."
                    .into(),
            );
        }
        match (&self.token_url, self.auth) {
            (None, AuthKind::Oauth2) => {
                return Some("OAuth 2 bağlantısının belirteç adresi olmalı.".into());
            }
            (Some(_), a) if !matches!(a, AuthKind::Arcgis | AuthKind::Oauth2) => {
                return Some("Belirteç adresi yalnız ArcGIS ve OAuth 2 bağlantısının olur.".into());
            }
            (Some(u), _) => {
                if let Some(p) = http_problem("Belirteç adresi", u) {
                    return Some(p);
                }
            }
            _ => {}
        }
        if let Some(s) = &self.scope {
            if self.auth != AuthKind::Oauth2 {
                return Some("Kapsam (scope) yalnız OAuth 2 bağlantısının olur.".into());
            }
            if s.is_empty() || !plain(s, 256) {
                return Some("Kapsam 1 ile 256 harf arasında olmalı.".into());
            }
        }
        None
    }
}

/// An authority's host and port: `[::1]:8080` or `ornek.gov.tr:8443`;
/// none when the brackets of an IPv6 host do not close.
fn split_authority(a: &str) -> Option<(&str, Option<&str>)> {
    if let Some(inner) = a.strip_prefix('[') {
        let close = inner.find(']')?;
        let host = &a[..close + 2];
        let after = &inner[close + 1..];
        return match after.strip_prefix(':') {
            Some(port) => Some((host, Some(port))),
            None if after.is_empty() => Some((host, None)),
            None => None,
        };
    }
    Some(match a.rsplit_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (a, None),
    })
}

/// Why `origin` is not `http(s)://host[:port]` in lower case without a
/// path, in the commands' words; none when it is.
pub fn origin_problem(origin: &str) -> Option<String> {
    let rest = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"));
    let ok = rest
        .filter(|r| !r.contains(['/', '?', '#', '@']))
        .and_then(split_authority)
        .is_some_and(|(host, port)| {
            let name = if host.starts_with('[') {
                host.len() > 2
                    && host[1..host.len() - 1].chars().all(|c| {
                        (c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) || matches!(c, ':' | '.')
                    })
            } else {
                !host.is_empty()
                    && host.len() <= 253
                    && host.chars().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.')
                    })
            };
            name && port.is_none_or(|p| p.parse::<u16>().is_ok_and(|n| n > 0))
        });
    (!ok).then(|| {
        format!(
            "Bağlantının kökeni küçük harfle şema ve makine (https://ornek.gov.tr, isteğe bağlı :kapı) olmalı, yolu olmamalı; “{origin}” değil."
        )
    })
}

/// The origin of an address: its scheme, host and port in lower case
/// (`https://ornek.gov.tr`, the scheme's own port left out); none for what
/// is not an HTTP or HTTPS address.
pub fn origin_of(url: &str) -> Option<String> {
    let text = url.trim();
    let colon = text.find("://")?;
    let scheme = text[..colon].to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let rest = &text[colon + 3..];
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    // A user's name and password in the address are not part of its origin.
    let authority = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let (host, port) = split_authority(authority)?;
    if host.is_empty() {
        return None;
    }
    let host = host.to_ascii_lowercase();
    let default = if scheme == "https" { "443" } else { "80" };
    Some(match port {
        Some(p) if p != default && !p.is_empty() => format!("{scheme}://{host}:{p}"),
        _ => format!("{scheme}://{host}"),
    })
}

/// Why the project's connections are not ones: a connection's own rules,
/// the most there may be, a repeated id; none when they are.
pub fn connections_problem(list: &[ServiceConnection]) -> Option<String> {
    if list.len() > MAX_CONNECTIONS {
        return Some(format!(
            "Projenin en çok {MAX_CONNECTIONS} bağlantısı olabilir."
        ));
    }
    for (i, c) in list.iter().enumerate() {
        if let Some(p) = c.problem() {
            return Some(format!("“{}” bağlantısı: {p}", c.name));
        }
        if list[..i].iter().any(|o| o.id == c.id) {
            return Some(format!("“{}” kimlikli bağlantı iki kez var.", c.id));
        }
    }
    None
}

/// How a drawing's services break their links (docs/adr/0208 §2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceLinkFault {
    /// A layer's service or feed names a connection the project does not have.
    Connection { layer: String, connection: String },
    /// A layer is drawn from a service and has a feed too.
    Both { layer: String },
    /// The object at this index of the drawing is on a layer drawn from a service.
    Object { index: usize },
}

impl ServiceLinkFault {
    /// What is wrong, in the readers' and commands' words.
    pub fn words(&self) -> String {
        match self {
            ServiceLinkFault::Connection { layer, connection } => format!(
                "“{layer}” katmanının bağlantısı “{connection}” projenin bağlantıları arasında yok"
            ),
            ServiceLinkFault::Both { layer } => format!(
                "“{layer}” katmanı hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz"
            ),
            ServiceLinkFault::Object { .. } => {
                "nesne bir servis katmanında; servis katmanı nesne tutmaz".to_owned()
            }
        }
    }
}

/// The first broken link of a drawing's services: a connection a service
/// or a feed names that the project does not have, a layer with both, an
/// object on a layer drawn from a service; none when all hold.
pub fn service_links(
    layers: &[crate::LayerNode],
    connections: &[ServiceConnection],
    entities: &[crate::Entity],
) -> Option<ServiceLinkFault> {
    fn walk<'a>(nodes: &'a [crate::LayerNode], out: &mut Vec<&'a crate::LayerNode>) {
        for n in nodes {
            out.push(n);
            walk(&n.children, out);
        }
    }
    let mut all = Vec::new();
    walk(layers, &mut all);
    let mut served = std::collections::HashSet::new();
    for n in &all {
        if n.service.is_some() && n.feed.is_some() {
            return Some(ServiceLinkFault::Both {
                layer: n.name.clone(),
            });
        }
        let named = [
            n.service.as_ref().and_then(|s| s.connection.as_deref()),
            n.feed.as_ref().and_then(|f| f.connection.as_deref()),
        ];
        for c in named.into_iter().flatten() {
            if !connections.iter().any(|k| k.id == c) {
                return Some(ServiceLinkFault::Connection {
                    layer: n.name.clone(),
                    connection: c.to_owned(),
                });
            }
        }
        if n.service.is_some() {
            served.insert(n.id.as_str());
        }
    }
    if served.is_empty() {
        return None;
    }
    entities
        .iter()
        .position(|e| served.contains(e.base().layer_id.as_str()))
        .map(|index| ServiceLinkFault::Object { index })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn osm() -> ServiceLayer {
        ServiceLayer {
            kind: ServiceKind::Xyz,
            url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".into(),
            layers: Vec::new(),
            style: None,
            format: None,
            srid: None,
            grid: None,
            matrix_set: None,
            template: None,
            tile_size: None,
            min_zoom: None,
            max_zoom: Some(19),
            subdomains: Vec::new(),
            y_flip: false,
            transparent: false,
            version: None,
            params: Vec::new(),
            dynamic: false,
            attribution: Some("© OpenStreetMap katkıda bulunanlar".into()),
            opacity: None,
            connection: None,
            preset: Some("osm-standard".into()),
            bbox: None,
        }
    }

    #[test]
    fn a_service_layer_is_checked_by_its_kind() {
        assert_eq!(osm().problem(), None);
        let mut s = osm();
        s.url = "https://tile.openstreetmap.org/{z}/{x}.png".into();
        assert!(s.problem().unwrap().contains("{y}"));
        let mut s = osm();
        s.url = "ftp://x/{z}/{x}/{y}".into();
        assert!(s.problem().unwrap().contains("HTTP"));
        let mut s = osm();
        s.url = "https://{s}.tile.example/{z}/{x}/{y}.png".into();
        assert!(s.problem().unwrap().contains("alt alan"));
        s.subdomains = vec!["a".into(), "b".into()];
        assert_eq!(s.problem(), None);
        let mut s = osm();
        s.opacity = Some(0.05);
        assert!(s.problem().unwrap().contains("donukluğu"));
        let mut s = osm();
        s.min_zoom = Some(12);
        s.max_zoom = Some(4);
        assert!(s.problem().is_some());
        let mut g = osm();
        g.kind = ServiceKind::Google;
        g.url = String::new();
        g.style = Some("satellite".into());
        assert!(g.problem().unwrap().contains("bağlantı"));
        g.connection = Some("google".into());
        assert_eq!(g.problem(), None);
        let mut w = osm();
        w.kind = ServiceKind::Wms;
        w.url = "https://ornek.gov.tr/wms".into();
        assert!(w.problem().unwrap().contains("katman adı"));
        w.layers = vec!["parseller".into()];
        assert!(w.problem().unwrap().contains("srid"));
        w.srid = Some(5254);
        w.version = Some("1.2.0".into());
        assert!(w.problem().unwrap().contains("sürüm"));
        w.version = Some("1.3.0".into());
        assert_eq!(w.problem(), None);
    }

    #[test]
    fn origins_are_read_and_checked() {
        assert_eq!(
            origin_of("HTTPS://Kbs.Belediye.gov.tr:443/arcgis/rest?f=json").as_deref(),
            Some("https://kbs.belediye.gov.tr")
        );
        assert_eq!(
            origin_of("http://user:pw@10.0.0.5:8080/geoserver/wms").as_deref(),
            Some("http://10.0.0.5:8080")
        );
        assert_eq!(origin_of("ftp://x"), None);
        assert_eq!(origin_problem("https://ornek.gov.tr"), None);
        assert_eq!(origin_problem("http://10.0.0.5:8080"), None);
        assert!(origin_problem("https://ornek.gov.tr/wms").is_some());
        assert!(origin_problem("https://Ornek.gov.tr").is_some());
        assert!(origin_problem("https://ornek.gov.tr:0").is_some());
        assert_eq!(origin_problem("http://[::1]:8080"), None);
        assert_eq!(
            origin_of("http://[::1]:8080/wms").as_deref(),
            Some("http://[::1]:8080")
        );
    }

    #[test]
    fn a_connection_names_what_its_kind_sends() {
        let c = ServiceConnection {
            id: "hgm".into(),
            name: "HGM ATLAS".into(),
            origin: "https://atlas.harita.gov.tr".into(),
            auth: AuthKind::Query,
            names: vec!["apikey".into()],
            token_url: None,
            scope: None,
        };
        assert_eq!(c.problem(), None);
        let mut d = c.clone();
        d.names.clear();
        assert!(d.problem().is_some());
        let mut d = c.clone();
        d.auth = AuthKind::Basic;
        assert!(d.problem().unwrap().contains("ad taşımaz"));
        let mut d = c.clone();
        d.auth = AuthKind::Oauth2;
        d.names.clear();
        assert!(d.problem().unwrap().contains("belirteç adresi"));
        d.token_url = Some("https://ornek.gov.tr/oauth/token".into());
        assert_eq!(d.problem(), None);
        assert!(
            connections_problem(&[c.clone(), c])
                .unwrap()
                .contains("iki kez")
        );
    }
}
