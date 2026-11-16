//! KentOS's IFC core (ISO 16739; docs/adr/8047, docs/adr/0908 §8): every IFC operation in one crate, so the IFC work
//! KentOS grows (reading a file, checking a model against the 2026 digital project regulation's EK-9, IDS, more classes
//! and property sets) has one home.
//!
//! - [`step`]: ISO 10303-21 writing: reals, strings, GlobalIds, the header.
//! - [`crs`]: the project's coordinate reference system and the map conversion (EK-7 Tablo 7.6–7.7).
//! - [`alignment`]: a road project's alignments as IFC 4.3 `IfcAlignment`s (KentOS Yol).
//! - [`building`]: a building as IFC 4.3: its spatial structure, elements, openings, spaces, properties.
//! - [`naming`]: the regulation's names (EK-5 §5.5) and its classes' category codes (EK-5 Tablo 5.1).
//! - [`arch`]: KentOS Mimari's model as a [`building::Building`]: classes, names and the properties EK-6 asks.
//! - [`quality`]: EK-9's model quality control form, answered from the model (Md. 15).
//! - [`catalog`]: the base profile's IFC annexes as data (EK-5, EK-6, EK-7, EK-9), every row with its table and page
//!   (docs/adr/8062), and KentOS's IFC decisions.
//! - [`catalog_schema`]: the catalog compared with the IFC 4.3 schema and its official templates.
//! - [`rules`]: every rule of the base profile, the 2D annexes' and the IFC annexes', as one list a window reads.
//! - [`turns`]: a stair's and a ramp's predefined type from its walking line's flights and turns (docs/adr/8070 §2).
//! - [`read`] (feature `read`): reading an IFC file through the engine's STEP parser: summary, objects, tree, relations,
//!   sets on demand and findings (docs/adr/8061, plan/ifc-uyum.md IFC-07).

pub mod alignment;
pub mod arch;
pub mod building;
pub mod catalog;
pub mod catalog_schema;
pub mod closed;
pub mod crs;
mod math;
pub mod mesh;
pub mod naming;
pub mod quality;
pub mod quantities;
#[cfg(feature = "read")]
pub mod read;
pub mod required;
pub mod rules;
pub mod step;
pub mod turns;
pub mod under_roof;
