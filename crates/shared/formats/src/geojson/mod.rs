//! GeoJSON (RFC 7946; docs/adr/0046): read into the app's objects and
//! written from them. A GeoJSON file is WGS 84 longitude and latitude
//! unless it carries the 2008 specification's `crs` member; the reader says
//! which (`declaredCrs`) and never transforms a coordinate, the app asks.

mod read;
mod write;

pub use read::read;
pub use write::{WriteInput, input_from_json, write};
