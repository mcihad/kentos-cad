//! KentOS's data drivers' common ground (docs/adr/9100, plan/veri.md VY-01): what a layer asks of the place its rows
//! live in, whatever that place is. A driver (GeoPackage, PostGIS; Oracle, MySQL or SQL Server tomorrow) is a crate of
//! its own that implements [`Driver`], [`Source`] and [`Transaction`] and registers itself in the [`Registry`]; the
//! application speaks only these traits.
//!
//! - [`Source`]: an opened file or database: its tables ([`TableInfo`]), a table's [`Schema`] and [`Capabilities`],
//!   its extent and row count, and its rows as a stream ([`Source::features`]) for a [`Request`]: a box, a
//!   [`Condition`], the columns, an order, a page, a geometry simplified for the screen. What the source can do itself
//!   it does; the request says nothing of SQL.
//! - [`Transaction`]: inserts, updates and deletes over one or more tables of one source, written together or not at
//!   all ([`Transaction::commit`], [`Transaction::rollback`]); new tables for exports.
//! - [`Registry`]: the drivers a host registered and the sources opened through them, one per address, so a database's
//!   connections are shared by every layer and window that reads it.
//!
//! Nothing here reads a disk or a network: the crate is pure, and its [`conformance`] suite (feature `conformance`)
//! is how a driver proves it answers the same questions the same way.
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod cancel;
pub mod catalog;
pub mod coerce;
pub mod condition;
#[cfg(any(test, feature = "conformance"))]
pub mod conformance;
pub mod driver;
pub mod edit;
pub mod error;
pub mod feature;
pub mod fit;
pub mod geom;
pub mod lossy;
pub mod projects;
pub mod query;
pub mod registry;
pub mod request;
pub mod schema;
pub mod secrets;
pub mod simplify;
pub mod turkish;
pub mod value;
pub mod wkt;

pub use cancel::Cancel;
pub use catalog::{TableInfo, TableKind};
pub use condition::{Condition, Op};
pub use driver::{Capabilities, Driver, RECEIPTS_TABLE, Read, Receipt, Source, Transaction};
pub use edit::{Change, NewFeature, NewTable, SystemText};
pub use error::{ConstraintKind, DataError, Reach};
pub use feature::Feature;
pub use geom::{
    Blob, BlobError, Coord, Curve, Geometry, read_blob, read_wkb, write_blob, write_wkb,
};
pub use projects::{ProjectStore, StoredProject};
pub use query::{QueryColumn, QueryTable};
pub use registry::{Registry, registry};
pub use request::{Order, Request};
pub use schema::{ColumnRule, Family, Field, FieldKind, GeometryField, RuledColumn, Schema, family_of};
pub use secrets::{Secrets, set_secrets};
pub use value::Value;
