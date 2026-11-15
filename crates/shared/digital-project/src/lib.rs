//! The 2026 digital project regulation as data (Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması
//! Hakkında Yönetmelik, RG 5.8.2026/33331; plan/bim-plan.md BIM-01, docs/adr/8062). A rule is data, not code: every row
//! KentOS keeps of the annexes carries where it was read — the annex, its table or clause, the row, the annexes PDF's
//! page — so a finding or an exported field can say it, and an administration's reading or the ministry's next
//! decision is a change of data, not of code.
//!
//! - [`source`]: where a rule was read ([`Source`]) and how much it is asked ([`Level`]).
//! - [`catalog`]: the base profile's EK-1…EK-4 and EK-8 rows: the clauses, the name templates, the code lists (EK-2's
//!   disciplines, items, sub-disciplines, file kinds and types, storeys; EK-3's rooms), EK-4's fonts and marks.
//! - [`decision`]: KentOS's decisions on the regulation's open points: read from a page, settled by a schema, waiting
//!   for the administration or KentOS's own reading.
//! - [`profile`]: the layers over the base profile — an administration's (its own sub-disciplines, the optional fields
//!   it asks, its readings) and a project's — and the parameters they settle.
//!
//! The IFC annexes (EK-5…EK-7) and the model quality control form (EK-9) are `kentos-ifc`'s, over these types
//! (CLAUDE.md §14: everything IFC is there).

pub mod catalog;
pub mod decision;
pub mod profile;
pub mod source;

pub use source::{Level, Source};
