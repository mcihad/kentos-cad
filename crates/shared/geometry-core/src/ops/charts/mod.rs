//! Grafikler ve alan istatistikleri (docs/adr/0227, `kentos.charts/1`): the
//! bottom panel's İstatistik and Grafik tabs on both platforms. The data
//! ([`data`]), a field's statistics ([`fields`]), the six charts' models
//! ([`model`]) and what they start with ([`specs`]), the axes ([`ticks`]),
//! the drawing list ([`layout`]) with the text's estimated width
//! ([`fonts`]), the saved picture ([`svg`]) and the work kept between calls
//! ([`work`]). The independent reference is `scripts/fixtures/chart_cases.py`.

pub mod data;
pub mod fields;
pub mod fonts;
pub mod json;
pub mod layout;
pub mod model;
pub mod specs;
pub mod svg;
pub mod ticks;
pub mod work;

/// The rules' version (docs/adr/0227 §2).
pub const POLICY: &str = "kentos.charts/1";

pub use data::{Cell, Column, DataSet, Kind};
pub use layout::{Frame, Layout, Prim, Role};
pub use model::{Aggregate, ChartKind, Model, Sort, Spec};
pub use work::Work;
