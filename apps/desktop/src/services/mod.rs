//! Map services on the desktop (docs/adr/0208): layers drawn from XYZ and
//! TMS, WMS, WMTS, OGC API Tiles, ArcGIS REST, Google's 2D tiles and vector
//! tiles, their connections' secrets on this device, the answers kept on
//! the disk by HTTP's rules, and the tiles each frame shows.
//!
//! - [`hub`]: what the scene shows and what is known of it, the tiles in
//!   memory, what the frames ask for;
//! - `net`, `resolve`: the services' threads: requests with proof, the
//!   disk, answers, styles and sessions;
//! - [`decode`]: pictures into atlas slots, vector tiles read and drawn;
//! - [`cache`], [`secrets`]: the device's answers and secrets;
//! - [`systems`]: between the project's system and a service's;
//! - [`overlay`], [`app`]: the vector tiles' labels and the credits over
//!   the drawing area.

pub mod app;
pub mod basemaps;
pub mod cache;
pub mod connections;
pub mod decode;
pub mod feed_window;
pub mod hub;
pub mod info;
pub(crate) mod net;
pub mod overlay;
mod resolve;
pub mod secrets;
pub mod systems;
pub mod window;

pub use hub::{hub, in_use, key_of, ready};
