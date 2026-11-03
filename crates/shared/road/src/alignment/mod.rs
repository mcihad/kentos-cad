//! A horizontal alignment (docs/adr/0901, plan/road.md Y3): tangents, circular curves and clothoids in one analytic
//! form, built from PIs; its stations and km labels; a point's station and offset; a parallel axis; what an edit did
//! to its stations. The algorithm's id is `kentos.alignment/1`.

pub mod chain;
pub mod element;
pub mod fit;
pub mod locate;
pub mod mapping;
pub mod offset;
pub mod stationing;

pub use chain::{Alignment, ChainError, CurveReport, Join, PiInput, Residual, build};
pub use element::{Element, Kind};
pub use locate::{End, Foot, Located, Stationed};
pub use stationing::{Equation, Region, Stationing, StationingError, km_text};

/// The algorithm's id, written with what it computes.
pub const ALGORITHM: &str = "kentos.alignment/1";
