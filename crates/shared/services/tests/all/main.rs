//! The map services core's tests against independent references (docs/adr/0208).

// Capabilities documents read by OWSLib (scripts/fixtures/service_caps_cases.py).
mod caps;
// Adres arama's providers: requests, answers and scores (scripts/fixtures/geocode_online_cases.py).
mod geocode;
// A vector tile written and read by GDAL (scripts/fixtures/service_mvt_cases.py).
mod mvt;
