//! KentOS Yol's engineering core (plan/road.md): what a road project computes, apart from the module's windows, so
//! the desktop, the server, Python and KentOS Harita's zoning plans compute it alike (ROAD-MOD-08).
//!
//! - [`surface`]: the terrain surface, a constrained Delaunay TIN of spot points and breaklines, clipped by a
//!   boundary and voids, and the height of a place on it (docs/adr/0900).
//! - [`alignment`]: the horizontal alignment, tangents, curves and clothoids from PIs, its km and a point's
//!   station and offset (docs/adr/0901).
//! - [`sources`]: a surface's input gathered from the drawing by its definition, and the digest that says whether
//!   it is out of date (plan/road.md Y2.2).

pub mod alignment;
pub mod sources;
pub mod surface;
