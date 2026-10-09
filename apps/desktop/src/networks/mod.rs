//! Ağ analizi on the desktop (docs/adr/0209 §10, §12; the web's
//! `app/networks.ts`): the tools' questions to the project's networks. A
//! network is built when a question finds it missing or out of date: on the
//! interface's thread what the drawing says (its layers' objects in document
//! order, the definition's expressions evaluated, the edges' paths from the
//! geometry store), on the network thread the graph; it is built again only
//! when its definition, the layer tree or one of its layers' objects changed
//! (the document's change journal). Questions go to the network thread in
//! order; their answers come back to the running tool, or to Ağlar's Denetle.

mod engine;
#[cfg(test)]
mod tests;
pub mod window;

use std::collections::{HashMap, HashSet};

use iced::Task;
use iced::futures::channel::oneshot;
use kentos_contracts::{Entity, NetworkDef};
use kentos_domain::{ChangeMark, Changes, Document};
use kentos_geometry_core::ops::network::input::from_store;
use kentos_interaction::Level;
use kentos_interaction::network::{Answer, NetworkAsk, NetworkReply};
use kentos_processing::network::{NetworkInput, Source, network_input, rules_of};

pub use engine::Built;
use engine::{Engine, Job};

use crate::app::{App, Message};

#[derive(Clone, Debug)]
pub enum Event {
    /// A question's answer, for the tool (or Ağlar's Denetle) that asked it.
    Answered(NetworkReply),
    /// A network was built: what building it said.
    Built(Built),
    /// Ağlar's window.
    Window(window::Event),
}

/// What a network was last built from: its definition, the drawing's change mark, the layer tree's names and the
/// objects it took (a change to one of them, or a new object on its layers, builds it again).
struct Stamp {
    def: NetworkDef,
    mark: ChangeMark,
    layers: Vec<(String, String)>,
    members: HashSet<u32>,
}

#[derive(Default)]
pub struct State {
    engine: Engine,
    built: HashMap<String, Stamp>,
    pub(crate) window: Option<window::Window>,
    /// The Denetle problem shown over the drawing (its row chosen in Ağlar).
    pub(crate) mark: Option<crate::topology::ProblemMark>,
}

impl State {
    /// Whether the project's network `id` was built for the tools.
    #[cfg(test)]
    pub(crate) fn built_for(&self, id: &str) -> bool {
        self.built.contains_key(id)
    }
}

/// The layer tree's leaves, by id and name: `$katman` reads the names.
fn leaves(doc: &Document) -> Vec<(String, String)> {
    doc.layers()
        .leaves()
        .into_iter()
        .map(|l| (l.id.clone(), l.name.clone()))
        .collect()
}

impl State {
    /// Whether network `key` is built as `def` says over the drawing as it is now.
    fn fresh(&self, key: &str, def: &NetworkDef, doc: &Document) -> bool {
        let Some(s) = self.built.get(key) else {
            return false;
        };
        if s.def != *def || s.layers != leaves(doc) {
            return false;
        }
        let layers: HashSet<&str> = def.layers().into_iter().collect();
        match doc.changes_since(s.mark) {
            Changes::All => false,
            Changes::Slots(slots) => slots.iter().all(|slot| {
                !s.members.contains(&slot.0)
                    && doc
                        .get(*slot)
                        .is_none_or(|e| !layers.contains(e.base().layer_id.as_str()))
            }),
        }
    }
}

impl App {
    /// The networks' questions the tools asked: each network built first when it must be, the questions to its thread.
    pub(crate) fn network_tasks(&mut self) -> Task<Message> {
        let asks = std::mem::take(&mut self.network_wanted);
        Task::batch(
            asks.into_iter()
                .map(|ask| self.network_ask(ask))
                .collect::<Vec<_>>(),
        )
    }

    /// One question: its network built when it must be, then asked; the answer comes back as a message.
    pub(crate) fn network_ask(&mut self, ask: NetworkAsk) -> Task<Message> {
        let ticket = ask.ticket;
        let failed = |why: String| {
            Task::done(Message::Networks(Event::Answered(NetworkReply {
                ticket,
                answer: Answer::Failed(why),
            })))
        };
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let def = ask.def.clone().or_else(|| {
            doc.model
                .settings()
                .networks
                .iter()
                .find(|n| n.id == ask.network)
                .cloned()
        });
        let Some(def) = def else {
            return failed(format!(
                "“{}” kimlikli ağ projede yok; Ağlar penceresinden tanımlayın.",
                ask.network
            ));
        };
        let build = match self.network_build(&ask.network, &def) {
            Ok(task) => task,
            Err(why) => return failed(why),
        };
        let (done, wait) = oneshot::channel();
        self.networks.engine.send(Job::Ask {
            network: ask.network,
            ticket,
            question: ask.question,
            done,
        });
        let answer = Task::perform(wait, move |r| {
            Message::Networks(Event::Answered(r.unwrap_or(NetworkReply {
                ticket,
                answer: Answer::Failed("Ağların iş parçacığı yanıt vermedi.".into()),
            })))
        });
        Task::batch([build, answer])
    }

    /// Builds network `key` as `def` says when it is not built so already: the drawing read here, the graph on the
    /// network thread. What building says comes back as a message.
    fn network_build(&mut self, key: &str, def: &NetworkDef) -> Result<Task<Message>, String> {
        let Some(doc) = self.document.as_ref() else {
            return Ok(Task::none());
        };
        let model = &doc.model;
        if self.networks.fresh(key, def, model) {
            return Ok(Task::none());
        }
        self.spatial.sync(model);
        let rules = rules_of(def)?;
        let layers: HashSet<&str> = def.layers().into_iter().collect();
        let mut by_layer: HashMap<&str, Vec<&Entity>> = HashMap::new();
        for l in &layers {
            by_layer.insert(l, model.by_layer(l).collect());
        }
        let store = self.spatial.store();
        let names: HashMap<String, String> = leaves(model).into_iter().collect();
        let by = |l: &str| by_layer.get(l).cloned().unwrap_or_default();
        let name = |id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_owned());
        let measures = |list: &[&Entity]| {
            store.measures(
                &list
                    .iter()
                    .map(|e| f64::from(e.base().id))
                    .collect::<Vec<_>>(),
            )
        };
        let input: NetworkInput = network_input(
            def,
            &Source {
                by_layer: &by,
                layer_name: &name,
                measures: &measures,
            },
        );
        let (edges, junctions, skipped) = from_store(store, &input.edges, &input.junctions);
        let members: HashSet<u32> = input
            .edges
            .iter()
            .map(|(id, _)| *id as u32)
            .chain(input.junctions.iter().map(|(id, _)| *id as u32))
            .chain(skipped.iter().map(|(id, _)| *id as u32))
            .collect();
        self.networks.built.insert(
            key.to_owned(),
            Stamp {
                def: def.clone(),
                mark: model.change_mark(),
                layers: leaves(model),
                members,
            },
        );
        let (done, wait) = oneshot::channel();
        self.networks.engine.send(Job::Build {
            network: key.to_owned(),
            name: def.name.clone(),
            rules,
            edges,
            junctions,
            skipped,
            problems: input.problems,
            done,
        });
        Ok(Task::perform(wait, |r| match r {
            Ok(built) => Message::Networks(Event::Built(built)),
            Err(_) => Message::Networks(Event::Built(Built {
                network: String::new(),
                name: String::new(),
                problems: vec!["Ağ kurulamadı: ağların iş parçacığı yanıt vermedi.".into()],
                skipped: 0,
                nodes: 0,
                pieces: 0,
                millis: 0.0,
            })),
        }))
    }

    pub(crate) fn networks_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Answered(reply) => {
                if self
                    .networks
                    .window
                    .as_ref()
                    .is_some_and(|w| w.waits_for(reply.ticket))
                {
                    return self.networks_window_answered(reply);
                }
                self.with_tool(|s, cx| s.network_answered(reply, cx));
                Task::none()
            }
            Event::Built(b) => {
                // What building said, once a build (not every question) says it (the web's service).
                for p in &b.problems {
                    self.say(Level::Warn, format!("{}: {p}", b.name));
                }
                if b.skipped > 0 {
                    self.say(
                        Level::Warn,
                        format!("{}: ağın katmanlarında {} nesne çizgi, çoklu çizgi, yay ya da nokta değil; ağa alınmadı.", b.name, b.skipped),
                    );
                }
                Task::none()
            }
            Event::Window(e) => self.networks_window_event(e),
        }
    }

    /// The commands the tools asked for, run after the update.
    pub(crate) fn command_tasks(&mut self) -> Task<Message> {
        let ids = std::mem::take(&mut self.command_wanted);
        Task::batch(ids.into_iter().map(|id| self.run(id)).collect::<Vec<_>>())
    }

    /// Forgets the networks the project no longer has (their threads' copies go too).
    /// The networks built from the drawing left go with it, and Denetle's mark (docs/adr/0209 §12): a network of
    /// the next drawing is built from that drawing whatever it is called.
    pub(crate) fn networks_reset(&mut self) {
        for k in std::mem::take(&mut self.networks.built).into_keys() {
            self.networks.engine.send(Job::Drop(k));
        }
        self.networks.mark = None;
    }

    /// The networks taken out of the project (Ağlar's Kaydet) are let go.
    pub(crate) fn networks_follow(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let ids: HashSet<&str> = doc
            .model
            .settings()
            .networks
            .iter()
            .map(|n| n.id.as_str())
            .collect();
        let gone: Vec<String> = self
            .networks
            .built
            .keys()
            .filter(|k| !ids.contains(k.as_str()) && !k.starts_with('#'))
            .cloned()
            .collect();
        for k in gone {
            self.networks.built.remove(&k);
            self.networks.engine.send(Job::Drop(k));
        }
    }
}
