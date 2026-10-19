//! Turns and levels (docs/adr/0521 §2, §3). A turn is from an arc (a piece
//! travelled one way, `piece * 2` the way it is drawn, `piece * 2 + 1`
//! against it) arriving at a node to an arc leaving it. It is forbidden by a
//! turn layer's line or by the U-turn policy; else it costs the speed costs
//! its line's delay, or at a crossing of three or more pieces its angle's
//! (straight on within 30° of the way one came, right, left) and a U-turn
//! (back along the piece one came) its own wherever it is made. A level
//! says which places connect: an edge's start, end and inside each have one,
//! a junction point its own or every one.

use super::rules::{TurnRule, UTurns, number};
use crate::geom::intersect::Edge;
use crate::jsmath::{cos, sin};
use crate::vec2::Vec2;

/// A turn layer's line as the host gives it: its object's id, its points (the first on the piece one comes along,
/// the last on the piece one goes on, the second, when there are three or more, near their node), whether its
/// layer reads a delay field and the field's value.
#[derive(Clone, Debug, PartialEq)]
pub struct TurnIn {
    pub id: f64,
    pub pts: Vec<Vec2>,
    pub field: bool,
    pub value: Option<String>,
}

/// A place's level: every level (a junction point that gives none), one level, or an edge's own inside (its ends'
/// levels differ and no inside level is given: it connects nothing but itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Any,
    At(i64),
    Own(u32),
}

impl Level {
    /// Whether two places at these levels connect.
    pub fn meets(self, other: Level) -> bool {
        self == Level::Any || other == Level::Any || self == other
    }
}

/// A level field's value that is not a level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unreadable;

/// The greatest level a field may give, either way.
pub const LEVEL_LIMIT: f64 = 1000.0;

/// A level field's value: none for no value, an error for one that is not a whole number within ±1000.
pub fn level_value(text: Option<&str>) -> Result<Option<i64>, Unreadable> {
    let Some(t) = text.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    match number(t) {
        Some(v) if v.fract() == 0.0 && v.abs() <= LEVEL_LIMIT => Ok(Some(v as i64)),
        _ => Err(Unreadable),
    }
}

/// The turn table of a graph: the rule, the turns its layers name (by `in_arc << 32 | out_arc`, sorted: their delay
/// in minutes, infinite when forbidden), each piece's heading leaving its start and arriving at its end, each node's
/// pieces count (a crossing has three or more), which costs are times, and each node's turns
/// coded once (`codes[code_start[n] + k * d + m]` from the arc arriving along its `k`th piece to the arc leaving along
/// its `m`th, `d` its pieces; none past [`CODED_DEGREE`] pieces): a search asks a turn on every step.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TurnTable {
    pub rule: Option<TurnRule>,
    pub named: Vec<(u64, f64)>,
    pub heads: Vec<[Vec2; 2]>,
    pub degree: Vec<u32>,
    pub times: Vec<bool>,
    pub(crate) codes: Vec<u8>,
    pub(crate) code_start: Vec<u32>,
}

/// The most pieces of a node whose turns are coded.
pub const CODED_DEGREE: usize = 16;

/// A turn's kind (`TurnTable::code`).
const FREE: u8 = 0;
const STRAIGHT: u8 = 1;
const RIGHT: u8 = 2;
const LEFT: u8 = 3;
const BACK: u8 = 4;
const NAMED: u8 = 5;
const FORBIDDEN: u8 = 6;
const UNCODED: u32 = u32::MAX;

/// The way an edge primitive goes at its start (`end` false) or its end, not of unit length.
pub fn heading(e: &Edge, end: bool) -> Vec2 {
    match *e {
        Edge::Seg { a, b } => Vec2::new(b.x - a.x, b.y - a.y),
        Edge::Arc { a0, sweep, .. } => {
            let a = if end { a0 + sweep } else { a0 };
            let (s, c) = (sin(a), cos(a));
            if sweep >= 0.0 {
                Vec2::new(-s, c)
            } else {
                Vec2::new(s, -c)
            }
        }
    }
}

/// The angle class of a turn from the way `came` to the way `goes`: 0 straight (within 30°, cos² ≥ 3/4 of the way
/// one came), 1 right, 2 left. Without a trigonometric function: a turn is asked on every step of a search.
pub fn angle_class(came: Vec2, goes: Vec2) -> usize {
    let cross = came.x * goes.y - came.y * goes.x;
    let dot = came.x * goes.x + came.y * goes.y;
    if dot > 0.0 && 3.0 * cross * cross <= dot * dot {
        0
    } else if cross < 0.0 {
        1
    } else {
        2
    }
}

impl TurnTable {
    /// Codes every node's turns (`adj_start`, `adj`: the graph's pieces at each node, in their order).
    pub fn code_all(&mut self, adj_start: &[u32], adj: &[u32]) {
        let nodes = adj_start.len().saturating_sub(1);
        self.code_start = Vec::with_capacity(nodes);
        self.codes.clear();
        for n in 0..nodes {
            let at = &adj[adj_start[n] as usize..adj_start[n + 1] as usize];
            if at.len() > CODED_DEGREE {
                self.code_start.push(UNCODED);
                continue;
            }
            self.code_start.push(self.codes.len() as u32);
            for &k in at {
                for &m in at {
                    let code = self.code(k ^ 1, m, n as u32);
                    self.codes.push(code);
                }
            }
        }
    }

    /// The way arc `arc` arrives (`arrive`) at its head or leaves its tail.
    fn way(&self, arc: u32, arrive: bool) -> Vec2 {
        let [start, end] = self.heads[(arc / 2) as usize];
        let back = |v: Vec2| Vec2::new(-v.x, -v.y);
        match (arc.is_multiple_of(2), arrive) {
            (true, true) => end,
            (true, false) => start,
            (false, true) => back(start),
            (false, false) => back(end),
        }
    }

    fn named_delay(&self, in_arc: u32, out_arc: u32) -> Option<f64> {
        if self.named.is_empty() {
            return None;
        }
        let key = (u64::from(in_arc) << 32) | u64::from(out_arc);
        self.named
            .binary_search_by_key(&key, |t| t.0)
            .ok()
            .map(|k| self.named[k].1)
    }

    /// The kind of the turn at `node` from `in_arc` to `out_arc`: a turn layer's line forbids it or gives its own
    /// delay (the U-turn policy still holds); else a U-turn, a turn where fewer than three pieces meet, or its angle's.
    fn code(&self, in_arc: u32, out_arc: u32, node: u32) -> u8 {
        let Some(rule) = self.rule else {
            return FREE;
        };
        let named = self.named_delay(in_arc, out_arc);
        if named.is_some_and(|d| !d.is_finite()) {
            return FORBIDDEN;
        }
        let degree = self.degree[node as usize];
        if out_arc == in_arc ^ 1 {
            let ok = match rule.u_turns {
                UTurns::Allowed => true,
                UTurns::DeadEnds => degree == 1,
                UTurns::Never => false,
            };
            return match (ok, named) {
                (false, _) => FORBIDDEN,
                (true, Some(_)) => NAMED,
                (true, None) => BACK,
            };
        }
        if named.is_some() {
            return NAMED;
        }
        if degree < 3 {
            return FREE;
        }
        match angle_class(self.way(in_arc, true), self.way(out_arc, false)) {
            0 => STRAIGHT,
            1 => RIGHT,
            _ => LEFT,
        }
    }

    /// What a turn of kind `code` adds to cost `c`; none when it is forbidden.
    fn delay(&self, code: u8, in_arc: u32, out_arc: u32, c: usize) -> Option<f64> {
        let rule = self.rule?;
        if code == FORBIDDEN {
            return None;
        }
        if !self.times.get(c).copied().unwrap_or(false) {
            return Some(0.0);
        }
        Some(match code {
            STRAIGHT => rule.straight,
            RIGHT => rule.right,
            LEFT => rule.left,
            BACK => rule.back,
            NAMED => self.named_delay(in_arc, out_arc).unwrap_or(0.0),
            _ => 0.0,
        })
    }

    /// What turning at `node` from `in_arc` to `out_arc` adds to cost `c` (its unit, minutes for a time); none when
    /// the turn is forbidden.
    pub fn turn(&self, in_arc: u32, out_arc: u32, node: u32, c: usize) -> Option<f64> {
        self.delay(self.code(in_arc, out_arc, node), in_arc, out_arc, c)
    }

    /// The turn at `node` from the arc arriving along its `k`th piece to the arc leaving along its `m`th (the
    /// graph's order: `at` its pieces), from the coded table.
    pub fn turn_at(&self, node: u32, at: &[u32], k: usize, m: usize, c: usize) -> Option<f64> {
        let (in_arc, out_arc) = (at[k] ^ 1, at[m]);
        let start = self.code_start.get(node as usize).copied().unwrap_or(UNCODED);
        let code = if start == UNCODED {
            self.code(in_arc, out_arc, node)
        } else {
            self.codes[start as usize + k * at.len() + m]
        };
        self.delay(code, in_arc, out_arc, c)
    }
}
