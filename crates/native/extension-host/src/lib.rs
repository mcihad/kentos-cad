//! The extension registry (docs/adr/8057 §2; plan/eklenti.md ADIM 17): the `.kek` packages a user installed, their
//! states, leases, updates and rollbacks, the watchdog, the organisation's policy and the revocation list.
//!
//! - [`Registry`]: install (or update) → enable → run → disable → go back → remove, each a separate operation; the
//!   files checked against their content digest before they run; dependencies in order.
//! - [`ExtensionLease`]: every activation's, and every contribution is registered under it, so revoking it leaves
//!   nothing; a late callback of an older lease is not applied.
//! - [`watchdog`]: crashes in a row stop an extension for good; "Sorunlu eklentiyi bul" by halves.
//! - [`policy`]: what IT allows (the user cannot loosen it) and the versions withdrawn.
//!
//! The registry reads and writes the user's extension folder only (`<veri>/kentos-cad/eklentiler` on the desktop);
//! it runs no extension code: Python is ADIM 18A/19's.

mod lease;
pub mod places;
pub mod policy;
mod registry;
pub mod store;
mod view;
pub mod watchdog;

pub use lease::{ContributionKind, ExtensionLease, Registered, Skipped};
pub use policy::{Policy, Revocations, Revoked};
pub use registry::{Activation, Choice, Crash, Installed, Options, Registry};
pub use store::{Channel, Grant, Source, SourceKind};
pub use view::{
    CapabilityLine, Change, DependencyStatus, ExtensionView, NeedStatus, ProjectNeed, Review,
    State, Trust,
};

#[cfg(test)]
mod tests;
