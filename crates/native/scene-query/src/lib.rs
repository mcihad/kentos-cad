//! What a drawing shows and answers without changing it (plan/argon-tavsiye.md ADIM 08, docs/adr/8054 §8): an
//! object's shape (`geometry`), the blocks' contents, SHX texts and label texts, the layer filters, the spatial index
//! with picking, snapping and label places (`spatial`), the storeys' view (`level_view`), and the queries and
//! measures a host answers (`query`). No command writes here: the renderer, a viewer or a phone takes it without the
//! authoring crates. `kentos-native-application`, `kentos-interaction` and `kentos-headless` re-export these modules
//! under their old paths.

pub mod blocks;
pub mod expression_unit;
pub mod geometry;
pub mod geometry_store;
pub mod label_texts;
pub mod layer_filter;
pub mod level_view;
pub mod query;
pub mod shx_text;
pub mod spatial;
pub mod storeys;
pub mod view;
pub mod viewport_scope;
pub mod walls;

pub use query::QueryError;
pub use spatial::Spatial;
pub use view::View;
