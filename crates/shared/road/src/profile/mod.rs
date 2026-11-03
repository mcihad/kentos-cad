//! A road's longitudinal profile (docs/adr/0902, plan/road.md Y4): the terrain along an alignment, and its vertical
//! alignment. The algorithm's id is `kentos.profile/1`.

pub mod terrain;
pub mod trace;
pub mod vertical;

pub use terrain::{ProfilePoint, TOLERANCE, terrain};
pub use vertical::{CurveReport, Piece, Pvi, VCurve, Vertical, VerticalError};

/// The algorithm's id, written with what it computes.
pub const ALGORITHM: &str = "kentos.profile/1";
