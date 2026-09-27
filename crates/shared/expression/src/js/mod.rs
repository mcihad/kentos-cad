//! JavaScript's semantics where the TypeScript relied on them: number to
//! text, text in UTF-16 code units and the Turkish locale, and the Turkish
//! text order (docs/adr/0008 “İfade dili”).

pub mod collate;
mod collation_tr;
pub mod number;
pub mod text;
