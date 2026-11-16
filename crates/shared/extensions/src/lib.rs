//! KentOS extension packages, `.kek` (docs/adr/8057 §1; plan/eklenti.md §3 and ADIM 16): the manifest v1, the
//! integrity list, the inspector that opens a package **without running any of its code**, and the deterministic packer.
//!
//! The extension system's model and the lessons taken from the ecosystems it was compared with are in plan/eklenti.md
//! and docs/adr/8057: a catalog inside the program and a low barrier, but a strict gate that is the same code on the
//! author's computer, in CI and in the catalog; every package's own locked Python environment and process; a manifest that knows its
//! extension's discovery, links, dependencies and channel from the first version, since unknown fields are refused.
//!
//! - [`manifest`]: [`Manifest`] v1 and its rules, each broken rule a [`Problem`] an agent can mend in one step.
//! - [`integrity`]: `integrity.json`, every file's size and SHA-256, and the package's content digest.
//! - [`archive`]: [`inspect`] (ZIP bounds, safe paths, no links, nothing a package must not carry, the integrity list,
//!   the manifest and what it names, the forms, the icons, the Python project) and [`pack`].
//! - [`compat`]: whether a package runs on a host: one function, no implicit upper bound (plan/eklenti.md's lesson).
//! - [`version`]: semantic versions and the ranges dependencies are given in.
//! - [`catalog`]: an organisation's or an offline folder's index of packages, written from the packages themselves.
//! - [`release`]: the cloud catalog's release flow (plan/eklenti.md ADIM 22A, docs/adr/8059): who moves a release from
//!   draft to published, its approval bound to its content, gates that never pass unfinished, every move audited.
//! - [`audit_facts`]: what KentOS measures in a drawing for an audit rule (`kentos.auditFacts`, plan/eklenti.md
//!   ADIM 20, docs/adr/8057 §36): its shape, its order, rounding and bounds, the same on the desktop and in a cloud job.
//! - [`verify`], [`blueprint`], [`sdk_pack`]: what an agent writing an extension is given: the folder's check, a
//!   starting folder that passes it, and the SDK in one read (`kentos://sdk/ai-kek-pack`).
//!
//! Pure: no file system and no network. A host reads the file and hands the bytes over.

pub mod archive;
pub mod audit_examples;
pub mod audit_facts;
pub mod blueprint;
pub mod catalog;
pub mod compat;
mod icon;
pub mod icon_names;
pub mod integrity;
pub mod l10n;
pub mod layer_schema;
pub mod manifest;
mod paths;
mod pointer;
mod problem;
pub mod provenance;
mod python;
pub use python::builds as python_builds;
pub mod python_names;
mod python_rules;
pub mod release;
pub mod resemblance;
pub mod revocations;
#[cfg(all(test, feature = "schema"))]
mod schema_check;
pub mod sbom;
pub mod sdk_pack;
pub mod secrets;
pub mod sdk_surface;
pub mod sdk_text;
mod text;
pub mod ui_panel;
pub mod verify;
pub mod version;

pub use archive::{Inspection, Limits, Opened, Packed, inspect, open, pack};
pub use compat::{Host, Os, SDK_PROTOCOL, compatible};
pub use manifest::Manifest;
pub use problem::Problem;

/// Why `path` cannot be a file's path in a package (`../x`, an absolute path, a reserved name …); none when it can.
pub fn path_problem(path: &str) -> Option<String> {
    paths::problem(path)
}
pub use sdk_pack::sdk_pack;
pub use verify::{Verification, verify};
