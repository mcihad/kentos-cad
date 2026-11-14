//! KentOS extension packages, `.kek` (docs/adr/8057 §1; plan/eklenti.md §3 and ADIM 16): the manifest v1, the
//! integrity list, the inspector that opens a package **without running any of its code**, and the deterministic packer.
//!
//! The extension system's model is QGIS's plugin system, its lessons taken (docs/research/2026-11-13-qgis-eklenti-sistemi.md):
//! a catalog inside the program and a low barrier, but a strict gate that is the same code on the author's computer, in
//! CI and in the catalog; every package's own locked Python environment and process; a manifest that knows its
//! extension's discovery, links, dependencies and channel from the first version, since unknown fields are refused.
//!
//! - [`manifest`]: [`Manifest`] v1 and its rules, each broken rule a [`Problem`] an agent can mend in one step.
//! - [`integrity`]: `integrity.json`, every file's size and SHA-256, and the package's content digest.
//! - [`archive`]: [`inspect`] (ZIP bounds, safe paths, no links, nothing a package must not carry, the integrity list,
//!   the manifest and what it names, the forms, the icons, the Python project) and [`pack`].
//! - [`compat`]: whether a package runs on a host: one function, no implicit upper bound (QGIS's 2026 lesson).
//! - [`version`]: semantic versions and the ranges dependencies are given in.
//! - [`catalog`]: an organisation's or an offline folder's index of packages, written from the packages themselves.
//! - [`verify`], [`blueprint`], [`sdk_pack`]: what an agent writing an extension is given: the folder's check, a
//!   starting folder that passes it, and the SDK in one read (`kentos://sdk/ai-kek-pack`).
//!
//! Pure: no file system and no network. A host reads the file and hands the bytes over.

pub mod archive;
pub mod blueprint;
pub mod catalog;
pub mod compat;
mod icon;
pub mod icon_names;
pub mod integrity;
pub mod manifest;
mod paths;
mod pointer;
mod problem;
mod python;
pub mod sdk_pack;
mod text;
pub mod verify;
pub mod version;

pub use archive::{Inspection, Limits, Opened, Packed, inspect, open, pack};
pub use compat::{Host, Os, SDK_PROTOCOL, compatible};
pub use manifest::Manifest;
pub use problem::Problem;
pub use sdk_pack::sdk_pack;
pub use verify::{Verification, verify};
