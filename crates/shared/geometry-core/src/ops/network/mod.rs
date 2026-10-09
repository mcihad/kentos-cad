//! Ağ analizi (docs/adr/0209): a network's graph built from the drawing's
//! line and point objects by the project's definition, and the analyses on
//! it: shortest routes through stops, service areas, closest facilities,
//! cost matrices, utility traces and the network's check. The apps evaluate
//! the definition's expressions and give the objects' paths and raw values;
//! everything else is here, the same on every platform.

pub mod area;
pub mod check;
pub mod closest;
pub mod graph;
pub mod input;
pub mod route;
pub mod rules;
pub mod search;
pub mod session;
pub mod trace;

pub use check::{Checked, Problem, ProblemKind};
pub use graph::{BuildReport, EdgeIn, Graph, JunctionIn, Location, Role};
pub use route::Reorder;
pub use rules::{Connect, Dir, Rules};
pub use search::{Path, Query, Searcher, Span, Tree};
pub use session::{
    Line, Missing, NetworkSession, Place, Reached, Refusal, Routed, Served, ServedArea, ServedLine,
    TraceAnswer, Valve, points_of, problem_name,
};
pub use trace::TraceKind;
