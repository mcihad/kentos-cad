//! Terminals (docs/adr/0557 §1): a device (a junction layer whose points have terminals) is where edges attach to
//! its ends: a valve's in and out, a transformer's two sides. An edge's start and end each name the terminal they are
//! attached to (an edge field's value: the terminal's id or its name). A trace arriving at a device by a terminal goes
//! on only along the edges attached to a terminal on a path with it (the same terminal always connects). An end that
//! names no terminal, or one the device has not, and an edge that runs through a device instead of ending at it, pass
//! to every terminal: Denetle says them and a trace over them is not one to rely on (docs/adr/0525 §2).
//!
//! The graph keeps for each node its device and for each arc (a piece travelled one way, `piece * 2` the way it is
//! drawn, `piece * 2 + 1` against it) the terminal at its tail; a trace's states are a node and the terminal it was
//! reached by, which is what `state` numbers.

use super::rules::Rules;

/// No terminal: an end that names none, an edge running through a device, a trace's start.
pub const NO_TERMINAL: u8 = u8::MAX;

/// A node that is no device.
pub const NO_DEVICE: u32 = u32::MAX;

/// The devices' terminals at a graph's nodes and arcs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Terminals {
    /// Each node's device (an index into the rules' devices), `NO_DEVICE` for a point.
    pub node_device: Vec<u32>,
    /// Each arc's terminal at its tail: `NO_TERMINAL` where the tail is no device or names none.
    pub arc_terminal: Vec<u8>,
    /// Each node's first state: a device has one state for each terminal and one for none, any other node one.
    slot: Vec<u32>,
    /// All the states of the graph's nodes.
    pub states: u32,
}

impl Terminals {
    /// The terminals of nodes with `node_device` and arcs with `arc_terminal`, by `rules`' devices.
    pub fn new(rules: &Rules, node_device: Vec<u32>, arc_terminal: Vec<u8>) -> Terminals {
        let devices = rules.terminals.as_ref().map_or(&[][..], |t| &t.devices);
        let mut slot = Vec::with_capacity(node_device.len());
        let mut at = 0u32;
        for &d in &node_device {
            slot.push(at);
            at += if d == NO_DEVICE {
                1
            } else {
                devices[d as usize].count() as u32 + 1
            };
        }
        Terminals {
            node_device,
            arc_terminal,
            slot,
            states: at,
        }
    }

    /// Whether node `n` is a device.
    pub fn is_device(&self, n: u32) -> bool {
        self.node_device
            .get(n as usize)
            .is_some_and(|&d| d != NO_DEVICE)
    }

    /// The state of node `n` reached by terminal `t` (`NO_TERMINAL`: by none).
    pub fn state(&self, n: u32, t: u8) -> usize {
        let base = self.slot[n as usize] as usize;
        if self.node_device[n as usize] == NO_DEVICE || t == NO_TERMINAL {
            base
        } else {
            base + 1 + t as usize
        }
    }

    /// Whether a way arriving at node `n` by terminal `from` goes on by terminal `out`: any way at a point, from or
    /// to no terminal; else along a path (or the same terminal).
    pub fn passes(&self, rules: &Rules, n: u32, from: u8, out: u8) -> bool {
        let d = self.node_device[n as usize];
        if d == NO_DEVICE || from == NO_TERMINAL || out == NO_TERMINAL {
            return true;
        }
        rules.terminals.as_ref().is_some_and(|t| {
            t.devices
                .get(d as usize)
                .is_some_and(|dev| dev.connects(from, out))
        })
    }
}
