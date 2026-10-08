//! Point cloud operations (docs/adr/0207 §7): each a machine fed a source's
//! records in their order, in pieces, by the host's full read, so that a
//! cloud of any size goes through in bounded memory; the host reads and
//! writes, the machines decide.
//!
//! - [`query`]: XYZ sor's nearest point;
//! - [`region`] and [`stats`]: the areas points are read against, Alan sorgusu;
//! - [`convert`]: the result's file for one or more sources, their records moved to it;
//! - [`thin`]: Seyrelt; [`grid`], [`ground`] and [`height`]: Zemin süzgeci and
//!   Yüksekliğe göre sınıfla;
//! - [`tile`]: Karola's tiles; [`raster`] and [`boundary`]: Rasterleştir and Sınır çıkar.

pub mod boundary;
pub mod convert;
pub mod grid;
pub mod ground;
pub mod height;
pub mod query;
pub mod raster;
pub mod region;
pub mod stats;
pub mod thin;
pub mod tile;
