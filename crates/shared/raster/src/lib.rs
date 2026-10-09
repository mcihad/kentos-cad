//! Raster analysis (docs/adr/0231–0236): a raster worked out cell by cell
//! into a new raster or into lines. The host opens the raster with the
//! formats core's reader and hands over the blocks a [`job::Job`] asks for;
//! the job gives the bytes of the result's tiled GeoTIFF (with its reduced
//! levels) to append, or the result's lines. One core for the web's analysis
//! worker (WASM, one thread) and the desktop's processing thread (its rows
//! and tiles on the machine's cores).
//!
//! - [`terrain`]: slope, aspect, the shaded relief, curvature and
//!   ruggedness, from a 3 × 3 window (docs/adr/0231 §3–§7).
//! - [`relief`]: Renkli kabartma's colour table (§5).
//! - [`insolation`]: Güneşlenme's suns and energy (§8).
//! - [`contours`]: Eş yükselti eğrileri, marching squares (§9).
//! - [`out`]: the result's GeoTIFF, written as it is worked out (§2).
//! - [`job`]: a run: the host's settings, the strips, the result.

pub mod contours;
pub mod frame;
pub mod insolation;
pub mod job;
pub mod out;
pub mod par;
pub mod relief;
pub mod terrain;

pub use job::{Finished, Job, Spec, Tool};
