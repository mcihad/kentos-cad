//! Apache Parquet, our own (docs/adr/0628 §1): the footer's metadata in Thrift's compact protocol, the schema as a
//! tree, column chunks read page by page — the dictionary, data pages v1 and v2, every encoding of format 2.11 and
//! the codecs of `crate::compress` — and nested columns assembled from their levels; files written with PLAIN values,
//! RLE levels and Snappy. GeoParquet's meaning (the `geo` key, WKB, GeoArrow) is the caller's.

pub mod encoding;
pub mod meta;
pub mod nest;
pub mod read;
pub mod record;
pub mod rle;
pub mod schema;
pub mod thrift;
pub mod write;

pub use encoding::{Bytes, Values};
pub use meta::{Logical, Physical, Repetition, TimeUnit};
pub use nest::{Nested, nest};
pub use read::{File, FileSource, Leaf, Source};
pub use schema::Schema;
