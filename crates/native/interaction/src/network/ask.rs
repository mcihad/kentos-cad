//! What a network tool asks and what it is answered (docs/adr/0209 §12; the
//! web's `io/network/protocol.ts`): a question names the network and carries
//! a ticket the answer comes back with; the host's network thread builds the
//! network from the drawing when it must and answers with the core's typed
//! answers (`ops::network::session`).

use kentos_contracts::NetworkDef;
use kentos_geometry_core::ops::network::{
    Checked, Place, Refusal, Reorder, Routed, Served, TraceAnswer, TraceKind,
};

use crate::Vec2;

/// A question to a network.
#[derive(Clone, Debug, PartialEq)]
pub enum NetworkQuestion {
    /// Nothing but the network built, as a tool starts (what building says is said by the host).
    Prepare,
    /// Where the network is nearest to `at`, within `reach`.
    Locate { at: Vec2, reach: f64 },
    Route {
        stops: Vec<Vec2>,
        barriers: Vec<Vec2>,
        reach: f64,
        cost: usize,
        reorder: Reorder,
    },
    /// A search from `at` kept for [`NetworkQuestion::PathTo`] (En kısa yol's rubber band).
    TreeFrom {
        at: Vec2,
        reach: f64,
        cost: usize,
        barriers: Vec<Vec2>,
    },
    /// The kept search's way to `at`.
    PathTo { at: Vec2, reach: f64 },
    Area {
        facilities: Vec<Vec2>,
        breaks: Vec<f64>,
        reach: f64,
        cost: usize,
        toward: bool,
        separate: bool,
        barriers: Vec<Vec2>,
        trim: f64,
        rings: bool,
        areas: bool,
    },
    Trace {
        starts: Vec<Vec2>,
        barriers: Vec<Vec2>,
        reach: f64,
        kind: TraceKind,
    },
    /// Denetle.
    Check,
}

/// A question as the host takes it: its ticket, the network's id and, for a definition not saved yet (Ağlar's
/// Denetle), that definition.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkAsk {
    pub ticket: u64,
    pub network: String,
    pub def: Option<NetworkDef>,
    pub question: NetworkQuestion,
}

/// A network's answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// The network is built.
    Ready,
    Place(Option<Place>),
    Route(Result<Routed, Refusal>),
    /// Whether the search was kept (the point found the network).
    Tree(bool),
    /// The way to the cursor: its cost and its points (arcs as short segments); none when there is no way.
    Path(Option<(f64, Vec<Vec2>)>),
    Area(Result<Served, Refusal>),
    Trace(Result<TraceAnswer, Refusal>),
    Check(Box<Checked>),
    /// The network could not be built or asked: why, in words.
    Failed(String),
}

/// An answer to the question of a ticket.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkReply {
    pub ticket: u64,
    pub answer: Answer,
}
