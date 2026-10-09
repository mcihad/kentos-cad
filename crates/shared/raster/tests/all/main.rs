//! The raster analysis core's tests: every case of the independent
//! references (`scripts/fixtures/terrain_cases.py`, `contour_cases.py`,
//! `interpolation_cases.py`) run through a whole job, its GeoTIFF read back.

mod contours;
mod host;
mod interpolation;
mod point_timing;
mod terrain;
mod timing;
