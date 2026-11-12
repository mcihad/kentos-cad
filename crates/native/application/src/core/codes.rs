//! The stable codes of the drawing commands' answers (`CommandError.code`,
//! `CommandWarning.code`; docs/adr/0022 … 0253). Programs branch on these,
//! never on the Turkish message. They are the contracts' (`kentos_commands::codes`),
//! where each has its meaning, kind and whether calling again may help
//! (docs/adr/0253 §7); the catalog lists each command's.

pub use kentos_commands::codes::*;
