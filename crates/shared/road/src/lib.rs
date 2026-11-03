//! KentOS Yol's engineering core (plan/road.md): what a road project computes, apart from the module's windows, so
//! the desktop, the server, Python and KentOS Harita's zoning plans compute it alike (ROAD-MOD-08).
//!
//! - [`surface`]: the terrain surface, a constrained Delaunay TIN of spot points and breaklines, clipped by a
//!   boundary and voids, and the height of a place on it (docs/adr/0900).
//! - [`alignment`]: the horizontal alignment, tangents, curves and clothoids from PIs, its km and a point's
//!   station and offset (docs/adr/0901).
//! - [`profile`]: an alignment's terrain profile on a surface and its vertical alignment (docs/adr/0902).
//! - [`section`]: a sample line across an alignment, the terrain along it, a template's platform and its slopes tied
//!   to the terrain (docs/adr/0903).
//! - [`corridor`]: an alignment's regions with their templates, the stations they are sampled at, the feature lines
//!   and the top surface the sections make, and where they are not solved (docs/adr/0905).
//! - [`superelevation`]: the cross slope of each side along an alignment from its curves or from rows, and the
//!   curves' widening (docs/adr/0904).
//! - [`volume`]: earthwork volumes, by sections (average end area, prismoidal) and between two surfaces (the
//!   triangulations laid over each other, split where the difference changes sign) (docs/adr/0906).
//! - [`haul`]: the mass haul (Brückner) diagram read as layers: balance points, loops, haul moments, free haul and
//!   overhaul, borrow and waste (docs/adr/0907).
//! - [`cutout`]: a terrain with corridors cut out of it in plan, the planes it keeps exact (docs/adr/0910 §6).
//! - [`cloud_ground`]: a terrain's ground points from a point cloud, thinned on a grid (cells, steep cells, beside
//!   breaklines), their density and coverage and the thinning's vertical error (docs/adr/0913).
//! - [`sources`]: a surface's input gathered from the drawing by its definition, and the digest that says whether
//!   it is out of date (plan/road.md Y2.2).

pub mod alignment;
pub mod cloud_ground;
pub mod corridor;
pub mod culvert;
pub mod grading;
pub mod cutout;
pub mod guardrail;
pub mod haul;
pub mod junction;
pub mod profile;
pub mod route;
pub mod section;
pub mod sources;
pub mod superelevation;
pub mod surface;
pub mod volume;
pub mod wall;
