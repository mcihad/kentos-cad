//! KML and KMZ (docs/adr/0222): read into WGS 84 objects (`read`), the
//! import then moved into the project's system when the user asks
//! (`reproject`); written from the project's system into WGS 84 (`write`).
//! The independent reference is scripts/fixtures/kml_cases.py (Python's
//! xml.etree and pyproj, no KentOS code).

pub mod read;
pub mod style;
pub mod write;

pub use read::read;
pub use write::write;
