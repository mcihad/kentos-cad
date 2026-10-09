//! What the network tools share (docs/adr/0209 §10; the web's
//! `tools/networkTool.ts`): Ağ (A) and, where it counts, Maliyet (M); Engel
//! (E: the next click puts a barrier); the points clicked or typed sit on the
//! network (found within 20 screen pixels, kept where they sit); Esc takes the
//! newest point back. A project without a network is told to define one
//! (Ağlar, G). Questions go to the host's network thread with a ticket; an
//! answer whose ticket the tool no longer waits for is dropped.

use kentos_contracts::{NetworkCostKind, NetworkDef, NetworkKind};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::ops::network::{Line, Refusal};

use super::{NetworkAsk, NetworkQuestion};
use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::tool::{
    Context, Marker, MarkerShape, Name, OptionChoice, Preview, Stroke, Tone, ViewChange,
};

/// How far a click looks for the network, logical pixels (docs/adr/0209 §4).
pub const REACH_PX: f64 = 20.0;

/// The command that opens Ağlar.
pub const MANAGE: &str = "network.manage";

/// What a point given to a network tool is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointKind {
    Stop,
    Barrier,
}

/// A point on the network: a stop (a facility, a start) or a barrier, where it sits on the network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetPoint {
    pub kind: PointKind,
    pub at: Vec2,
}

/// The state every network tool keeps.
#[derive(Clone, Debug, Default)]
pub struct Base {
    pub points: Vec<NetPoint>,
    pub barrier_next: bool,
    /// The network's words when it cannot be built or asked.
    pub problem: Option<String>,
    next_ticket: u64,
    /// Tickets of points being found, with what each becomes.
    finding: Vec<(u64, PointKind)>,
}

impl Base {
    /// A new ticket.
    pub fn ticket(&mut self) -> u64 {
        self.next_ticket += 1;
        self.next_ticket
    }

    /// The project's networks.
    pub fn list<'a>(cx: &'a Context<'_>) -> &'a [NetworkDef] {
        &cx.doc.settings().networks
    }

    /// The network used: the one last chosen, else the first of `prefers`' kind, else the first.
    pub fn network<'a>(cx: &'a Context<'_>, prefers: NetworkKind) -> Option<&'a NetworkDef> {
        let list = Self::list(cx);
        let chosen = cx.memory.network.as_str();
        list.iter()
            .find(|n| !chosen.is_empty() && n.id == chosen)
            .or_else(|| list.iter().find(|n| n.kind == prefers))
            .or_else(|| list.first())
    }

    /// The network's costs' names, Uzunluk first.
    pub fn cost_names(n: &NetworkDef) -> Vec<String> {
        n.cost_names().into_iter().map(str::to_owned).collect()
    }

    /// The cost used: the one last chosen when the network has it, else Uzunluk.
    pub fn cost_index(n: &NetworkDef, cx: &Context<'_>) -> usize {
        let chosen = cx.memory.network_cost.as_str();
        n.cost_names()
            .iter()
            .position(|c| *c == chosen)
            .unwrap_or(0)
    }

    /// A value of cost `c` of network `n` in its unit (metres, minutes, the field's unit).
    pub fn cost_text(v: Option<f64>, c: usize, n: &NetworkDef, f: &Format) -> String {
        let Some(v) = v.filter(|v| v.is_finite()) else {
            return "—".to_owned();
        };
        if c == 0 {
            return f.length(v);
        }
        match n.costs.get(c - 1) {
            Some(cost) if cost.kind == NetworkCostKind::Speed => format!("{} dk", fixed(v, 1)),
            Some(cost) if !cost.unit.is_empty() => format!("{} {}", fixed(v, 2), cost.unit),
            _ => fixed(v, 2),
        }
    }

    /// How far a point looks for the network, metres.
    pub fn reach(cx: &Context<'_>) -> f64 {
        cx.view.world_length(REACH_PX)
    }

    pub fn stops(&self) -> Vec<Vec2> {
        self.points
            .iter()
            .filter(|p| p.kind == PointKind::Stop)
            .map(|p| p.at)
            .collect()
    }

    pub fn barriers(&self) -> Vec<Vec2> {
        self.points
            .iter()
            .filter(|p| p.kind == PointKind::Barrier)
            .map(|p| p.at)
            .collect()
    }

    /// Asks network `network` a question with a new ticket.
    pub fn ask(&mut self, network: &str, question: NetworkQuestion, cx: &mut Context<'_>) -> u64 {
        let ticket = self.ticket();
        cx.view_changes
            .push(ViewChange::Network(Box::new(NetworkAsk {
                ticket,
                network: network.to_owned(),
                def: None,
                question,
            })));
        ticket
    }

    /// A clicked or typed point: put on the network as a stop (a facility, a start) or, after E, a barrier.
    pub fn place(&mut self, at: Vec2, network: Option<&str>, label: &str, cx: &mut Context<'_>) {
        let Some(id) = network else {
            cx.say(Level::Warn, format!("{label}: {}", no_network_words()));
            return;
        };
        let kind = if self.barrier_next {
            PointKind::Barrier
        } else {
            PointKind::Stop
        };
        self.barrier_next = false;
        let reach = Self::reach(cx);
        let ticket = self.ask(id, NetworkQuestion::Locate { at, reach }, cx);
        self.finding.push((ticket, kind));
    }

    /// A point found (or not) for a ticket this tool waits for: true when it was one (the point is added when found).
    pub fn found(
        &mut self,
        ticket: u64,
        place: Option<kentos_geometry_core::ops::network::Place>,
        label: &str,
        cx: &mut Context<'_>,
    ) -> bool {
        let Some(at) = self.finding.iter().position(|(t, _)| *t == ticket) else {
            return false;
        };
        let (_, kind) = self.finding.remove(at);
        match place {
            Some(p) => self.points.push(NetPoint { kind, at: Vec2::new(p.x, p.y) }),
            None => cx.say(
                Level::Warn,
                format!("{label}: tıklanan yerin {REACH_PX} piksel yakınında ağ yok; ağın çizgisine yakın tıklayın."),
            ),
        }
        true
    }

    /// Forgets the points being found and the points (another network was chosen, the tool leaves).
    pub fn reset(&mut self) {
        self.points.clear();
        self.finding.clear();
        self.barrier_next = false;
        self.problem = None;
    }

    /// The options every network tool has: Ağ, Maliyet where it counts, Engel.
    pub fn common(
        &self,
        prompt: crate::prompt::Prompt,
        n: &NetworkDef,
        cost: Option<&str>,
    ) -> crate::prompt::Prompt {
        let prompt = prompt.option_with("Ağ", "A", n.name.clone());
        let prompt = match cost {
            Some(c) => prompt.option_with("Maliyet", "M", c.to_owned()),
            None => prompt,
        };
        if self.barrier_next {
            prompt.option_with("Engel", "E", "sonraki tıklama")
        } else {
            prompt.option("Engel", "E")
        }
    }

    /// The shared options' keys: what became of them, `None` when it is none of them.
    pub fn option(
        &mut self,
        key: &str,
        prefers: NetworkKind,
        costs: bool,
        cx: &mut Context<'_>,
    ) -> Option<Changed> {
        match key {
            "A" => {
                let list = Self::list(cx);
                if list.is_empty() {
                    return None;
                }
                let now = Self::network(cx, prefers).map(|n| n.id.clone());
                let at = list
                    .iter()
                    .position(|n| Some(&n.id) == now.as_ref())
                    .unwrap_or(0);
                let next = list[(at + 1) % list.len()].id.clone();
                cx.memory.network = Name::new(&next).unwrap_or(Name::EMPTY);
                self.reset();
                Some(Changed::Network)
            }
            "M" if costs => {
                let n = Self::network(cx, prefers)?;
                let names = Self::cost_names(n);
                let at = Self::cost_index(n, cx);
                let next = names[(at + 1) % names.len()].clone();
                cx.memory.network_cost = Name::new(&next).unwrap_or(Name::EMPTY);
                Some(Changed::Ask)
            }
            "E" => {
                self.barrier_next = !self.barrier_next;
                Some(Changed::Show)
            }
            "G" => {
                cx.view_changes.push(ViewChange::Command(MANAGE));
                Some(Changed::Show)
            }
            _ => None,
        }
    }

    /// The choices of Ağ (with Ağlar…) and Maliyet.
    pub fn option_choices(
        key: &str,
        list: &[NetworkDef],
        network: Option<&NetworkDef>,
        cost: usize,
        costs: bool,
    ) -> Vec<OptionChoice> {
        let choice = |label: String, checked: bool| OptionChoice {
            label: label.clone(),
            typed: label,
            icon: None,
            preview: None,
            checked,
            command: None,
        };
        match key {
            "A" => {
                let mut out: Vec<OptionChoice> = list
                    .iter()
                    .map(|n| choice(n.name.clone(), Some(&n.id) == network.map(|x| &x.id)))
                    .collect();
                out.push(OptionChoice {
                    label: "Ağlar…".into(),
                    typed: String::new(),
                    icon: None,
                    preview: None,
                    checked: false,
                    command: Some(MANAGE),
                });
                out
            }
            "M" if costs => network
                .map(|n| {
                    Self::cost_names(n)
                        .into_iter()
                        .enumerate()
                        .map(|(i, c)| choice(c, i == cost))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// One of the shared options chosen by name: Ağ's network, Maliyet's cost.
    pub fn choose(
        &mut self,
        key: &str,
        typed: &str,
        prefers: NetworkKind,
        costs: bool,
        cx: &mut Context<'_>,
    ) -> Option<Changed> {
        let t = typed.trim();
        match key {
            "A" => {
                let id = Self::list(cx).iter().find(|n| n.name == t)?.id.clone();
                let now = Self::network(cx, prefers).map(|n| n.id.clone());
                if now.as_deref() != Some(id.as_str()) {
                    cx.memory.network = Name::new(&id).unwrap_or(Name::EMPTY);
                    self.reset();
                    return Some(Changed::Network);
                }
                Some(Changed::Ask)
            }
            "M" if costs => {
                let n = Self::network(cx, prefers)?;
                if !n.cost_names().contains(&t) {
                    return None;
                }
                cx.memory.network_cost = Name::new(t).unwrap_or(Name::EMPTY);
                Some(Changed::Ask)
            }
            _ => None,
        }
    }

    /// Esc: E waiting, then the newest point, go first; false with none (the tool leaves).
    pub fn cancel(&mut self) -> Option<Changed> {
        if self.barrier_next {
            self.barrier_next = false;
            return Some(Changed::Show);
        }
        self.points.pop().map(|_| Changed::Ask)
    }

    /// The points: stops numbered (when `numbered`), barriers as crosses.
    pub fn draw_points(&self, out: &mut Preview, numbered: bool) {
        let mut n = 0;
        for p in &self.points {
            match p.kind {
                PointKind::Barrier => out.markers.push(Marker {
                    at: p.at,
                    shape: MarkerShape::Cross(6.0),
                    tone: Tone::Danger,
                }),
                PointKind::Stop => {
                    n += 1;
                    out.markers.push(Marker {
                        at: p.at,
                        shape: if numbered {
                            MarkerShape::Numbered(n)
                        } else {
                            MarkerShape::Dot(5.0)
                        },
                        tone: Tone::Accent,
                    });
                }
            }
        }
    }
}

/// What an option changed: the network (its points go and it is built as chosen), what the answers depend on (asked
/// again), or only what the prompt shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Changed {
    Network,
    Ask,
    Show,
}

/// The prompt's step when the project has no network.
pub fn no_network_words() -> &'static str {
    "projede ağ tanımlı değil; Ağlar ile yol ya da şebeke ağını tanımlayın"
}

/// A way's points with its arcs as short segments (the web's `bulgePathOutline`).
pub fn outline(line: &Line) -> Vec<Vec2> {
    let pts: Vec<Vec2> = line.pts.iter().map(|p| Vec2::new(p[0], p[1])).collect();
    if line.bulges.iter().all(|b| *b == 0.0) {
        return pts;
    }
    bulge_path_outline(&pts, Some(&line.bulges), false, std::f64::consts::PI / 36.0)
}

/// A way drawn as the tools draw it.
pub fn stroke(pts: Vec<Vec2>, width: f32, dash: Option<[f32; 2]>, tone: Tone) -> Stroke {
    Stroke {
        pts,
        closed: false,
        dash,
        width,
        tone,
    }
}

/// A refusal in words (`within`: the search distance as written, “20 piksel” in the tools); the web's `refusalWords`.
pub fn refusal_words(r: Refusal, within: &str) -> String {
    r.words(within)
}

/// A document's slot from an object's id as the core gives it back (a whole number).
pub fn slot_of(id: f64) -> Option<kentos_domain::Slot> {
    (id >= 0.0 && id.fract() == 0.0 && id <= f64::from(u32::MAX))
        .then_some(kentos_domain::Slot(id as u32))
}
