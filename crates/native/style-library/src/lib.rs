//! The style library (docs/adr/0092): system, user and project items with their categories; the .kstil file that
//! moves them (`file`), checked as untrusted data; object templates (docs/adr/0176). Below the product commands, so
//! `cad.library.edit` (docs/adr/0314 §3) and the desktop's style page (`kentos_native_style`, which re-exports these
//! modules under their old paths) keep one rule.

pub mod file;
pub mod library;
pub mod number;
pub mod object_template;
