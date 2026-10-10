//! Mesh and multidimensional data (docs/adr/0243): NetCDF classic files
//! (CDF-1, CDF-2, CDF-5) read and written by KentOS's own code, their CF
//! regular grids and UGRID meshes as rasters of one slice, Aquaveo SMS's
//! 2DM meshes and ASCII DAT datasets made one UGRID file. As the raster
//! reader, nothing here does input or output: a reader says which bytes it
//! needs and the host reads them.

pub mod cf;
pub mod cube;
pub mod mesh;
pub mod netcdf;
pub mod series;
pub mod sms;
pub mod ugrid;
pub mod write;

use kentos_contracts::RasterSample;

/// The floating-point samples a mesh's values are drawn in (NaN off the
/// mesh): 64 bits for 64-bit floats and 32-bit integers, 32 otherwise.
pub fn float_sample(s: RasterSample) -> RasterSample {
    match s {
        RasterSample::F64 | RasterSample::I32 | RasterSample::U32 => RasterSample::F64,
        _ => RasterSample::F32,
    }
}
