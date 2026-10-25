//! Ağ analizi (docs/adr/0209): a network's graph built from the drawing's
//! line and point objects by the project's definition, and the analyses on
//! it: shortest routes through stops, service areas, closest facilities,
//! cost matrices, utility traces and the network's check; with turns, levels
//! and time profiles (docs/adr/0521). The apps evaluate
//! the definition's expressions and give the objects' paths and raw values;
//! everything else is here, the same on every platform.

pub mod access;
pub mod area;
pub mod blocked;
pub mod check;
pub mod clock;
pub mod closest;
pub mod graph;
mod build;
mod rebuild;
pub mod input;
pub mod route;
pub mod rules;
pub mod search;
pub mod session;
pub mod trace;
pub mod subnetworks;
pub mod terminals;
pub mod turns;

pub use blocked::{Blocked, Blocker, Why};
pub use access::{Access, AccessRules, ClosureRule, Closing, Varied, day_of, hours_of, weekday_of, weekdays_of};
pub use check::{Checked, Problem, ProblemKind};
pub use graph::{BuildReport, EdgeIn, Graph, JunctionIn, Location, Role};
pub use rebuild::Rebuilt;
pub use route::Reorder;
pub use rules::{Connect, Dir, Rules, UTurns};
pub use search::{Path, Query, Searcher, Span, Tree};
pub use session::{
    Line, Missing, NetworkSession, Place, Reached, Refusal, Routed, Served, ServedArea, ServedLine,
    Subnet, TraceAnswer, Valve, points_of, problem_name,
};
pub use trace::TraceKind;
pub use subnetworks::{Reach, Reached as SubnetReached};
pub use terminals::{NO_DEVICE, NO_TERMINAL, Terminals};
pub use turns::{Level, TurnIn};
