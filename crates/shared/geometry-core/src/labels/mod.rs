//! The label engine (docs/adr/0212): where each object's labels go in a
//! window at a scale, one engine for the drawing, the sheet and Etiketleri
//! yazıya çevir. The settings (`style`), the page's geometry (`geom`), the
//! words (`text`) and the placing (`engine`); the store gathers a window's
//! labels and obstacles (`crate::store::placing`).

pub mod engine;
pub mod geom;
pub mod style;
pub mod text;
