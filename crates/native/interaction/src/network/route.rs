//! En kısa yol (docs/adr/0209 §5; the web's `tools/networkRouteTool.ts`):
//! stops clicked in turn; from the second on, the route through them shows,
//! and the way from the last stop to the cursor follows the pointer (the
//! last stop's search kept in the host's network thread, the way read from
//! it on each move, one question at a time). Sıra (S) puts the stops in the
//! best order. Enter writes the route to Rota (opened in that step when
//! missing) as one polyline with its network, cost, stops, order, length and
//! each cost's total.

use std::collections::BTreeMap;

use kentos_contracts::NetworkKind;
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::ops::network::{Reorder, Routed};

use super::base::{Base, Changed, REACH_PX, no_network_words, outline, refusal_words, stroke};
use super::result::{ROUTE_LAYER, ResultWrite, line_geometry, write_results};
use super::{Answer, NetworkQuestion, NetworkReply};
use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};

/// The tool's id: its command is `tool.netRoute`.
pub const ID: &str = "netRoute";
pub const LABEL: &str = "En kısa yol";

/// Sıra's values in its key's order.
pub const REORDERS: [(Reorder, &str); 3] = [
    (Reorder::None, "yok"),
    (Reorder::KeepFirst, "ilk sabit"),
    (Reorder::KeepFirstLast, "ilk ve son sabit"),
];

fn reorder_name(r: Reorder) -> &'static str {
    REORDERS
        .iter()
        .find(|(v, _)| *v == r)
        .map_or("yok", |(_, n)| n)
}

/// The tickets the tool waits for.
#[derive(Clone, Copy, Debug, Default)]
struct Waiting {
    route: Option<u64>,
    tree: Option<u64>,
    path: Option<u64>,
}

/// What the prompt and the preview show of the network now (the host's world as last seen): the network's id and
/// name, the cost, its names, the format, the network, the project's networks and Sıra.
#[derive(Clone, Debug, Default)]
struct Seen {
    network: Option<(String, String)>,
    cost: usize,
    costs: Vec<String>,
    format: Option<Format>,
    def: Option<kentos_contracts::NetworkDef>,
    list: Vec<kentos_contracts::NetworkDef>,
    reorder: Option<Reorder>,
}

#[derive(Default)]
pub struct NetRoute {
    base: Base,
    route: Option<(Routed, Vec<Vec2>)>,
    refusal: Option<String>,
    /// Whether the host keeps a search from the last stop.
    tree: bool,
    hover: Option<Vec2>,
    /// The way to the cursor: its cost and points.
    band: Option<(f64, Vec<Vec2>)>,
    waiting: Waiting,
    /// The cursor's newest place while a way to it is being asked.
    path_next: Option<Vec2>,
    seen: Seen,
}

impl NetRoute {
    pub fn new() -> Self {
        Self::default()
    }

    /// What the network and its cost are now, as the prompt shows them.
    fn look(&mut self, cx: &Context<'_>) {
        let n = Base::network(cx, NetworkKind::Road);
        self.seen = Seen {
            network: n.map(|n| (n.id.clone(), n.name.clone())),
            cost: n.map_or(0, |n| Base::cost_index(n, cx)),
            costs: n.map(Base::cost_names).unwrap_or_default(),
            format: Some(cx.format()),
            def: n.cloned(),
            list: Base::list(cx).to_vec(),
            reorder: Some(cx.memory.network_reorder),
        };
    }

    fn seen_reorder(&self) -> Reorder {
        self.seen.reorder.unwrap_or(Reorder::None)
    }

    /// Something the answers depend on changed: asked again.
    fn changed(&mut self, cx: &mut Context<'_>) {
        self.look(cx);
        self.route = None;
        self.refusal = None;
        self.tree = false;
        self.band = None;
        self.waiting = Waiting::default();
        self.path_next = None;
        self.base.problem = None;
        let Some((id, _)) = self.seen.network.clone() else {
            return;
        };
        let stops = self.base.stops();
        let barriers = self.base.barriers();
        let reach = Base::reach(cx);
        let cost = self.seen.cost;
        if let Some(&last) = stops.last() {
            self.waiting.tree = Some(self.base.ask(
                &id,
                NetworkQuestion::TreeFrom {
                    at: last,
                    reach,
                    cost,
                    barriers: barriers.clone(),
                },
                cx,
            ));
        }
        if stops.len() >= 2 {
            let reorder = cx.memory.network_reorder;
            self.waiting.route = Some(self.base.ask(
                &id,
                NetworkQuestion::Route {
                    stops,
                    barriers,
                    reach,
                    cost,
                    reorder,
                },
                cx,
            ));
        }
    }

    /// The way from the last stop to the cursor, one question at a time (a newer place waits for the answer).
    fn follow(&mut self, cx: &mut Context<'_>) {
        let (Some(at), true) = (self.hover, self.tree) else {
            return;
        };
        if self.waiting.path.is_some() {
            self.path_next = Some(at);
            return;
        }
        let Some((id, _)) = self.seen.network.clone() else {
            return;
        };
        let reach = Base::reach(cx);
        self.waiting.path = Some(
            self.base
                .ask(&id, NetworkQuestion::PathTo { at, reach }, cx),
        );
    }

    fn totals(&self, r: &Routed) -> String {
        let (Some(def), Some(f)) = (&self.seen.def, &self.seen.format) else {
            return String::new();
        };
        self.seen
            .costs
            .iter()
            .enumerate()
            .map(|(c, name)| {
                format!(
                    "{name} {}",
                    Base::cost_text(r.totals.get(c).copied().flatten(), c, def, f)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> Option<Changed> {
        if key == "S" {
            let at = REORDERS
                .iter()
                .position(|(r, _)| *r == cx.memory.network_reorder)
                .unwrap_or(0);
            cx.memory.network_reorder = REORDERS[(at + 1) % REORDERS.len()].0;
            return Some(Changed::Ask);
        }
        self.base.option(key, NetworkKind::Road, true, cx)
    }

    fn after(&mut self, did: Changed, cx: &mut Context<'_>) {
        match did {
            Changed::Network => {
                self.changed(cx);
                self.prepare(cx);
            }
            Changed::Ask => self.changed(cx),
            Changed::Show => self.look(cx),
        }
    }

    /// The network built as the tool starts or another is chosen.
    fn prepare(&mut self, cx: &mut Context<'_>) {
        if let Some((id, _)) = self.seen.network.clone() {
            self.base.ask(&id, NetworkQuestion::Prepare, cx);
        }
    }

    /// Enter: the route written; the stops go.
    fn write(&mut self, cx: &mut Context<'_>) {
        let Some((r, _)) = &self.route else {
            let why = self
                .refusal
                .clone()
                .unwrap_or_else(|| "yazılacak bir yol yok; en az iki durak verin.".to_owned());
            cx.say(Level::Warn, format!("{LABEL}: {why}"));
            return;
        };
        let Some(def) = self.seen.def.clone() else {
            return;
        };
        let decimals = cx.format().length_decimals;
        let mut attrs = BTreeMap::from([
            ("Ağ".to_owned(), def.name.clone()),
            (
                "Maliyet".to_owned(),
                self.seen
                    .costs
                    .get(self.seen.cost)
                    .cloned()
                    .unwrap_or_default(),
            ),
            ("Duraklar".to_owned(), self.base.stops().len().to_string()),
        ]);
        if cx.memory.network_reorder != Reorder::None {
            attrs.insert(
                "Sıra".to_owned(),
                r.order
                    .iter()
                    .map(|i| (i + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        for (c, name) in self.seen.costs.iter().enumerate() {
            let v = r.totals.get(c).copied().flatten();
            attrs.insert(
                name.clone(),
                v.map_or_else(String::new, |v| fixed(v, if c == 0 { decimals } else { 2 })),
            );
        }
        let totals = self.totals(r);
        let objects = vec![(line_geometry(&r.line), attrs)];
        if write_results(
            LABEL,
            &[ResultWrite {
                layer: &ROUTE_LAYER,
                objects,
            }],
            cx,
        )
        .is_none()
        {
            return;
        }
        cx.say(
            Level::Success,
            format!(
                "{LABEL}: {totals}; “{}” katmanına yazıldı.",
                ROUTE_LAYER.name
            ),
        );
        self.base.points.clear();
        self.changed(cx);
    }
}

impl Tool for NetRoute {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let Some((_, name)) = &self.seen.network else {
            return Prompt::new(LABEL, no_network_words()).option("Ağlar", "G");
        };
        let n = self.base.stops().len();
        let step = if let Some(p) = &self.base.problem {
            p.clone()
        } else if n == 0 {
            "ilk durağa tıklayın ya da Y,X yazın".to_owned()
        } else if n == 1 {
            "sonraki durağa tıklayın; imlece kadar yol izlenir".to_owned()
        } else if let Some(why) = &self.refusal {
            why.clone()
        } else if let Some((r, _)) = &self.route {
            format!(
                "{n} durak; {}; sonraki durağa tıklayın ya da Enter ile yazın",
                self.totals(r)
            )
        } else {
            format!("{n} durak; yol aranıyor")
        };
        let cost = self
            .seen
            .costs
            .get(self.seen.cost)
            .cloned()
            .unwrap_or_default();
        let prompt = Prompt::new(LABEL, step)
            .option_with("Ağ", "A", name.clone())
            .option_with("Maliyet", "M", cost);
        let prompt = if self.base.barrier_next {
            prompt.option_with("Engel", "E", "sonraki tıklama")
        } else {
            prompt.option("Engel", "E")
        };
        let prompt = prompt.option_with("Sıra", "S", reorder_name(self.seen_reorder()));
        if self.route.is_some() {
            prompt.option("Uygula", "Enter")
        } else {
            prompt
        }
    }

    fn point_count(&self) -> usize {
        self.base.points.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.look(cx);
        self.prepare(cx);
        Flow::Stay
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.follow(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let id = self.seen.network.as_ref().map(|(id, _)| id.clone());
        self.base.place(p.world, id.as_deref(), LABEL, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = kentos_geometry_core::tools::point_text::js_trim(text);
        let last = self.base.points.last().map(|p| p.at);
        if let Some(at) = cx.typed_point(t, last, None) {
            let id = self.seen.network.as_ref().map(|(id, _)| id.clone());
            self.base.place(at, id.as_deref(), LABEL, cx);
            return true;
        }
        let Some(did) = self.option(&upper_tr(t), cx) else {
            return false;
        };
        self.after(did, cx);
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key == "S" {
            return REORDERS
                .iter()
                .map(|&(r, name)| OptionChoice {
                    label: name.to_owned(),
                    typed: name.to_owned(),
                    icon: None,
                    preview: None,
                    checked: r == self.seen_reorder(),
                    command: None,
                })
                .collect();
        }
        Base::option_choices(
            key,
            &self.seen.list,
            self.seen.def.as_ref(),
            self.seen.cost,
            true,
        )
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key == "S" {
            let Some(&(r, _)) = REORDERS.iter().find(|(_, n)| *n == typed.trim()) else {
                return false;
            };
            cx.memory.network_reorder = r;
            self.after(Changed::Ask, cx);
            return true;
        }
        match self.base.choose(key, typed, NetworkKind::Road, true, cx) {
            Some(did) => {
                self.after(did, cx);
                true
            }
            None => false,
        }
    }

    /// Enter: with a route it is written and the stops go; with no stops the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.base.points.is_empty() {
            return Flow::Exit;
        }
        self.write(cx);
        Flow::Stay
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        match self.base.cancel() {
            Some(did) => {
                self.after(did, cx);
                true
            }
            None => false,
        }
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    fn network_answered(&mut self, reply: NetworkReply, cx: &mut Context<'_>) {
        match reply.answer {
            Answer::Place(place) => {
                if self.base.found(reply.ticket, place, LABEL, cx) && place.is_some() {
                    self.changed(cx);
                }
            }
            Answer::Route(r) if self.waiting.route == Some(reply.ticket) => {
                self.waiting.route = None;
                match r {
                    Ok(r) => {
                        let line = outline(&r.line);
                        self.route = Some((r, line));
                    }
                    Err(why) => {
                        let words = refusal_words(why, &format!("{REACH_PX} piksel"));
                        cx.say(Level::Warn, format!("{LABEL}: {words}"));
                        self.refusal = Some(words);
                    }
                }
            }
            Answer::Tree(kept) if self.waiting.tree == Some(reply.ticket) => {
                self.waiting.tree = None;
                self.tree = kept;
                self.follow(cx);
            }
            Answer::Path(way) if self.waiting.path == Some(reply.ticket) => {
                self.waiting.path = None;
                self.band = way;
                if let Some(at) = self.path_next.take() {
                    self.hover = Some(at);
                    self.follow(cx);
                }
            }
            Answer::Failed(why) => {
                if self.base.problem.as_deref() != Some(why.as_str()) {
                    cx.say(Level::Warn, why.clone());
                }
                self.base.problem = Some(why);
                self.waiting = Waiting::default();
            }
            _ => {}
        }
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = Preview::default();
        if let Some((_, line)) = &self.route {
            out.strokes
                .push(stroke(line.clone(), 3.0, None, Tone::Accent));
        }
        if let Some((cost, way)) = &self.band {
            out.strokes
                .push(stroke(way.clone(), 2.0, Some([7.0, 5.0]), Tone::Accent));
            if let (Some(at), Some(def)) = (self.hover, &self.seen.def) {
                let c = self.seen.cost;
                let mut lines = vec![format!("+{}", Base::cost_text(Some(*cost), c, def, format))];
                if let Some((r, _)) = &self.route {
                    lines.push(format!(
                        "Toplam {}",
                        Base::cost_text(Some(r.cost + cost), c, def, format)
                    ));
                }
                out.tag = Some(Tag { at, lines });
            }
        }
        self.base.draw_points(&mut out, true);
        out
    }
}
