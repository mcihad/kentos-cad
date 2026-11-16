//! The standards packages' rules called from a window (catalog/standards; plan/bim-plan.md K10, docs/adr/8063): the
//! shared runner (`kentos_standards`, docs/adr/8075 §2.6), which the sheet publication and the command line
//! use too.

pub use kentos_standards::call;
