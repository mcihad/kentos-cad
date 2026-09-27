//! İşlemler on the desktop (docs/PROCESSING.md, docs/adr/0084): batch
//! operations over many objects, declared as data so that the dialog, the
//! toolbox, the command line, history and models run the same tool.
//!
//! It is the native counterpart of the web's `apps/web/src/processing`, not a
//! facade over it: what makes the two agree is the shared cases in
//! `fixtures/processing/v1`, which both play (`tests/cases.rs`,
//! `apps/web/src/processing/cases.test.ts`).
//!
//! - A tool ([`Tool`]) never touches the document. It receives resolved,
//!   read-only inputs and returns a [`ChangeSet`]; the [`Runner`] applies it
//!   as one undo step, creates the layers it writes to and leaves locked
//!   layers alone.
//! - A model ([`Model`]) wires tools into a flow; it runs step by step and is
//!   one undo step as a whole.
//! - Geometry is the shared core's (corner numbering, edge lengths, the
//!   expressions' measures), the expression language the style core's.
//!   What stays here is text, counting and flow, as in the web's tools.
//! - Texts are the web's, word for word.
//!
//! Pure Rust, native only: no UI, GPU, async runtime or network
//! (scripts/arch/deps.mjs).
#![forbid(unsafe_code)]
// User data (a drawing, typed values) must never crash a run.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod builtin;
pub mod categories;
pub mod expression;
pub mod features;
pub mod geometry;
pub mod model;
pub mod model_runner;
pub mod parameters;
pub mod registry;
pub mod runner;
pub mod text;
pub mod types;
pub mod values;

pub use features::{Host, InputSummary, Scene};
pub use kentos_geometry_core::geometry::Bounds;
pub use kentos_geometry_core::store::Store;
pub use model::{Model, ModelStep, ValueSource};
pub use parameters::Issue;
pub use registry::Registry;
pub use runner::{Level, LogLine, Outcome, RunRecord, Runner, Status};
pub use types::{
    ChangeSet, DefaultValue, Defaults, EnumOption, FeatureSet, Feedback, OutputDef, OutputKind,
    ParamDef, ParamKind, Patch, Resolved, Returns, RunContext, RunResult, ScopeKind, Target,
    TargetLayer, Tool, Values,
};
pub use values::{FeaturesValue, LayerValue, Scope};
