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

pub mod alignment;
pub mod arch;
pub mod building;
pub mod crs;
mod math;
pub mod naming;
pub mod step;
