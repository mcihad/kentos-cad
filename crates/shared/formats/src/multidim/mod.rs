//! Mesh and multidimensional data (docs/adr/0243): NetCDF classic files
//! (CDF-1, CDF-2, CDF-5) read and written by KentOS's own code, their CF
//! regular grids and UGRID meshes as rasters of one slice, Aquaveo SMS's
//! 2DM meshes and ASCII DAT datasets made one UGRID file. As the raster
//! reader, nothing here does input or output: a reader says which bytes it
//! needs and the host reads them.
//!
//! The raster reader draws a mesh's slice and a NetCDF cube's blocks, so their model (`mesh`, `ugrid`, `cf`,
//! `netcdf`) comes with the raster codec; the files' own readers and writers (HDF5, GRIB, Zarr, chunked arrays,
//! SMS, the cube and series readers, the writer) only with the `multidim` feature (docs/adr/8054 §11).

#[cfg(feature = "multidim")]
pub mod blosc;
pub mod cf;
#[cfg(feature = "multidim")]
pub mod chunked;
#[cfg(feature = "multidim")]
pub mod cube;
#[cfg(feature = "multidim")]
pub mod grib;
#[cfg(feature = "multidim")]
pub mod hdf5;
pub mod mesh;
pub mod netcdf;
#[cfg(feature = "multidim")]
pub mod series;
#[cfg(feature = "multidim")]
pub mod sms;
pub mod ugrid;
#[cfg(feature = "multidim")]
pub mod write;
#[cfg(feature = "multidim")]
pub mod zarr;

use kentos_contracts::RasterSample;

/// The floating-point samples a mesh's values are drawn in (NaN off the
/// mesh): 64 bits for 64-bit floats and 32-bit integers, 32 otherwise.
pub fn float_sample(s: RasterSample) -> RasterSample {
    match s {
        RasterSample::F64 | RasterSample::I32 | RasterSample::U32 => RasterSample::F64,
        _ => RasterSample::F32,
    }
}
