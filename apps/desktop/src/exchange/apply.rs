//! Puts what a reader produced into the drawing: the rules are the
//! application layer's (`kentos_native_application::import_apply`), shared
//! with `file.import.dxf` (docs/adr/0257 §3), so the import windows and a
//! script write alike.

pub use kentos_native_application::import_apply::*;
