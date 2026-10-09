//! The raster analysis core's tests: every case of the independent
//! references (`scripts/fixtures/terrain_cases.py`, `contour_cases.py`,
//! `interpolation_cases.py`; the raster operations' `raster_ops.rs`, raster
//! and vector's `raster_vector.rs`) run
//! through a whole job, its GeoTIFF read back.

mod contours;
mod host;
mod interpolation;
mod ops_timing;
mod point_timing;
mod raster_ops;
mod raster_vector;
mod terrain;
mod timing;
mod vector_timing;
