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
pub(crate) mod read;
#[cfg(test)]
mod read_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod terminal_tests;
pub mod window;

use std::collections::{HashMap, HashSet};

use iced::Task;
use iced::futures::channel::oneshot;
use kentos_commands::NetworkDef;
use kentos_domain::{ChangeMark, Changes, Document};
use kentos_interaction::Level;
use kentos_interaction::network::{Answer, NetworkAsk, NetworkReply};
use kentos_processing::network::rules_of;

pub use engine::Built;
use engine::Reading;
use read::{Members, read_changed, read_whole};
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
/// objects it took (a change to one of them, or a new object on its layers, builds it again; only they are read again,
/// docs/adr/0641 §3).
struct Stamp {
    def: NetworkDef,
    mark: ChangeMark,
    layers: Vec<(String, String)>,
    members: Members,
    /// The network's bound layers' links (layer, file's revision when read): a link read again builds anew.
    sources: Vec<(String, String)>,
    /// The bound layers whose files have changed since they were read, as last said (docs/adr/0640 §4).
    stale: Vec<String>,
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

/// The links of the network's bound layers: (layer, the file's revision when the working copy was read).
fn source_links(doc: &Document, def: &NetworkDef) -> Vec<(String, String)> {
    let layers = def.layers();
    doc.settings()
        .sources
        .iter()
        .filter(|l| layers.contains(&l.layer.as_str()))
        .map(|l| (l.layer.clone(), l.revision.clone()))
        .collect()
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
        if s.def != *def || s.layers != leaves(doc) || s.sources != source_links(doc, def) {
            return false;
        }
        let layers: HashSet<&str> = def.layers().into_iter().collect();
        match doc.changes_since(s.mark) {
            Changes::All => false,
            Changes::Slots(slots) => slots.iter().all(|slot| {
                !s.members.holds(slot.0)
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
        // A bound layer's file changed since its working copy was read: the answer is the copy's, said once a change
        // (docs/adr/0640 §4).
        let stale = self
            .document
            .as_ref()
            .map(|d| kentos_native_application::sources::stale_layers(&d.model, &def.layers()))
            .unwrap_or_default();
        if let Some(stamp) = self.networks.built.get_mut(&ask.network)
            && stale != stamp.stale
        {
            stamp.stale.clone_from(&stale);
            if !stale.is_empty() {
                self.say(
                    Level::Warn,
                    format!(
                        "{}: {} katmanının kaynağı çalışma kopyası okunduktan sonra değişti; ağ çizimdeki kopyadan kuruldu. Güncel ağ için önce Kaynak ▸ Kaynaktan oku.",
                        def.name,
                        stale.join(", ")
                    ),
                );
            }
        }
        let (done, wait) = oneshot::channel();
        self.networks.engine.send(Job::Ask {
            network: ask.network,
            ticket,
            question: ask.question,
            at: ask.at,
            profile: ask.profile,
            day: ask.day,
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
        let store = self.spatial.store();
        // Read again only the objects that changed since the last read of this definition over this layer tree.
        let part = self
            .networks
            .built
            .get_mut(key)
            .filter(|s| {
                s.def == *def && s.layers == leaves(model) && s.sources == source_links(model, def)
            })
            .and_then(|s| match model.changes_since(s.mark) {
                Changes::Slots(slots) => {
                    let changed: HashSet<u32> = slots.iter().map(|slot| slot.0).collect();
                    read_changed(def, model, store, std::mem::take(&mut s.members), &changed)
                }
                Changes::All => None,
            });
        let (reading, members, skipped, problems) = match part {
            Some((mut changed, members)) => {
                let skipped = std::mem::take(&mut changed.skipped);
                let problems = std::mem::take(&mut changed.problems);
                (Reading::Changed(Box::new(changed)), members, skipped, problems)
            }
            None => {
                let (mut whole, members) = read_whole(def, model, store);
                let skipped = std::mem::take(&mut whole.skipped);
                let problems = std::mem::take(&mut whole.problems);
                (
                    Reading::Whole(read::Inputs {
                        edges: whole.edges,
                        junctions: whole.junctions,
                        turns: whole.turns,
                    }),
                    members,
                    skipped,
                    problems,
                )
            }
        };
        self.networks.built.insert(
            key.to_owned(),
            Stamp {
                def: def.clone(),
                mark: model.change_mark(),
                layers: leaves(model),
                members,
                sources: source_links(model, def),
                stale: Vec::new(),
            },
        );
        let (done, wait) = oneshot::channel();
        self.networks.engine.send(Job::Build {
            network: key.to_owned(),
            name: def.name.clone(),
            rules: Box::new(rules),
            reading,
            skipped,
            problems,
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
                again: false,
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
                // The network thread had no last input for a read of the changed objects: read whole next time.
                if b.again {
                    self.networks.built.remove(&b.network);
                }
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
