//! Map services (docs/adr/0208): what a host asks a service for and how it
//! reads the answer, the same on the desktop, in the browser (WASM) and on
//! the server, so the platforms ask for the same tiles with the same
//! addresses and draw what comes back the same way.
//!
//! - [`query`], [`template`], [`crs`]: addresses and their parameters, URL
//!   templates, coordinate system names and axis orders.
//! - [`request`]: the requests of each kind (WMS, WMTS, WFS, OGC API, ArcGIS
//!   REST, Google), [`source`]: a service layer's tile grid and the address of
//!   each tile, [`auth`]: a connection's proof added to a request.
//! - [`caps`]: capabilities documents read (WMS, WMTS, WFS, OGC API,
//!   ArcGIS REST, TileJSON).
//! - [`mvt`]: Mapbox vector tiles read; [`style`]: a MapLibre style's subset
//!   applied to them into the style engine's batches and label candidates;
//!   [`labels`]: which labels a frame shows.
//! - [`gml`], [`features`]: a service's objects read as the contract's
//!   geometries; [`info`]: a feature info answer as rows.
//! - [`picture`]: a picture tile cut into the raster atlas's slots.
//! - [`cache`]: HTTP's caching rules; [`presets`]: the ready basemaps;
//!   [`attribution`]: credits as text.
//! - [`connect`]: Harita servisi's Bağlan (the requests in turn, the offer)
//!   and the service layer a choice makes; [`feed`]: Servisten veri al's,
//!   and the pages that take a feed's objects.

pub mod attribution;
pub mod auth;
pub mod cache;
pub mod caps;
pub mod connect;
pub mod crs;
pub mod features;
pub mod feed;
pub mod gml;
pub mod google;
pub mod info;
pub mod labels;
pub mod mvt;
pub mod picture;
pub mod presets;
pub mod query;
pub mod request;
pub mod source;
pub mod style;
pub mod template;

/// The User-Agent every host sends (OpenStreetMap's tile usage policy asks
/// for one that names the application): `KentOS-CAD/<version> (+address)`.
pub fn user_agent(version: &str) -> String {
    format!("KentOS-CAD/{version} (+https://github.com/mcihad/kentos-cad)")
}
