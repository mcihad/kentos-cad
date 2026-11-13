//! A turn of the chat (docs/adr/8056 §4): the user's message to the model and the model's work until it answers. On
//! the agent's runtime a turn sends the conversation (streamed), passes the text and reasoning on as they come, and when
//! the model calls tools it hands each call to the desktop ([`Event::ToolCall`]), waits for every answer
//! ([`Turn::answer`]), sends the results back and goes on — at most [`MAX_TOOL_ROUNDS`] rounds. `tool_choice` is auto
//! only and no sampling parameter is sent (the research note's traps 1, 2, 23, 26, 32). The calls of an answer reach the
//! desktop only once the answer is whole, in the model's order; nothing is run for an answer that broke off.
//!
//! [`Event::Finished`] hands back the whole conversation, the providers' records included, for the next turn. After a
//! failure [`Turn::conversation`] gives it as far as it got, at its last whole step (never a call without its result),
//! so a retry does not lose the tool rounds already run on the drawing.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{mpsc, watch};

use crate::conversation::{Conversation, Message, ToolCall, ToolResult};
use crate::failure::{Failure, FailureKind, scrub};
use crate::http;
use crate::profiles::AgentProfile;
use crate::runtime;
use crate::tools::{ToolNames, ToolSpec};
use crate::wire::{self, Call, Live, Stop};

/// The tool rounds a turn may take; a model still calling tools after them is stopped and told why.
pub const MAX_TOOL_ROUNDS: usize = 25;

/// What a turn tells the desktop, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A piece of the answer's text.
    Text(String),
    /// A piece of the reasoning the provider shows.
    Reasoning(String),
    /// The model calls a tool (the app's name, the input it wrote); answer it with [`Turn::answer`].
    ToolCall {
        id: String,
        name: String,
        input: Value,
    },
    /// The tokens of one request to the model (each round has its own).
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// The model answered: the conversation for the next turn.
    Finished { conversation: Conversation },
    /// The turn stopped; the message says why and what to do, `retry_after` how long the provider asks to wait.
    Failed {
        kind: FailureKind,
        message: String,
        retry_after: Option<Duration>,
    },
}

type Answer = (String, Result<Value, String>);

/// A turn under way: its events, the way to answer its tool calls and to stop it. Dropping it stops the turn.
pub struct Turn {
    pub events: mpsc::UnboundedReceiver<Event>,
    answers: mpsc::UnboundedSender<Answer>,
    stop: watch::Sender<bool>,
    progress: Arc<Mutex<Conversation>>,
}

impl fmt::Debug for Turn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Turn")
            .field("stopped", &*self.stop.borrow())
            .finish_non_exhaustive()
    }
}

impl Turn {
    /// The result of the tool call `id`: what the tool gave, or why it failed (the model reads either).
    pub fn answer(&self, id: &str, result: Result<Value, String>) {
        let _ = self.answers.send((id.to_owned(), result));
    }

    /// The turn's events, for a task that waits on them apart from the turn (kept to answer and stop it); a second
    /// take gets a closed channel.
    pub fn take_events(&mut self) -> mpsc::UnboundedReceiver<Event> {
        let (_, closed) = mpsc::unbounded_channel();
        std::mem::replace(&mut self.events, closed)
    }

    /// Stops the turn: a request under way is dropped, calls not answered yet are told they did not run.
    pub fn cancel(&self) {
        self.stop.send_replace(true);
    }

    /// The conversation as far as the turn got, at its last whole step: the user's message once taken, every tool
    /// round with its results. After `Finished`, the same as its conversation.
    pub fn conversation(&self) -> Conversation {
        self.progress
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}

/// Starts a turn of `profile`'s agent with `key`: `user_text` after `conversation` (empty: the conversation already
/// ends with the user's message or the tools' results, a retry), `tools` offered to the model. Returns at once; the
/// turn runs on the agent's runtime.
pub fn start_turn(
    profile: &AgentProfile,
    key: Option<String>,
    conversation: Conversation,
    tools: Vec<ToolSpec>,
    user_text: String,
) -> Turn {
    let (events_tx, events) = mpsc::unbounded_channel();
    let (answers_tx, answers) = mpsc::unbounded_channel();
    let (stop_tx, stop) = watch::channel(false);
    let progress = Arc::new(Mutex::new(conversation.clone()));
    let job = Job {
        profile: profile.clone(),
        key: key.map(|k| k.trim().to_owned()).filter(|k| !k.is_empty()),
        tools,
        events: events_tx.clone(),
        answers,
        stop,
        progress: Arc::clone(&progress),
    };
    match runtime::handle() {
        Ok(rt) => {
            rt.spawn(job.run(conversation, user_text));
        }
        Err(f) => {
            let _ = events_tx.send(Event::Failed {
                kind: f.kind,
                message: f.message,
                retry_after: None,
            });
        }
    }
    Turn {
        events,
        answers: answers_tx,
        stop: stop_tx,
        progress,
    }
}

/// A turn's work on the runtime.
struct Job {
    profile: AgentProfile,
    key: Option<String>,
    tools: Vec<ToolSpec>,
    events: mpsc::UnboundedSender<Event>,
    answers: mpsc::UnboundedReceiver<Answer>,
    stop: watch::Receiver<bool>,
    progress: Arc<Mutex<Conversation>>,
}

/// What a call not answered when the turn stopped is told.
const NOT_RUN: &str = "Kullanıcı durdurdu; araç çalıştırılmadı.";

impl Job {
    async fn run(mut self, conversation: Conversation, user_text: String) {
        if let Err(f) = self.drive(conversation, user_text).await {
            let _ = self.events.send(Event::Failed {
                kind: f.kind,
                message: scrub(&f.message, self.key.as_deref()),
                retry_after: f.retry_after,
            });
        }
    }

    fn keep(&self, conversation: &Conversation) {
        *self.progress.lock().unwrap_or_else(PoisonError::into_inner) = conversation.clone();
    }

    async fn drive(
        &mut self,
        mut conversation: Conversation,
        user_text: String,
    ) -> Result<(), Failure> {
        let preset = self.profile.preset();
        let who = preset.name;
        if preset.needs_key && self.key.is_none() {
            return Err(Failure::no_key(&self.profile.name, preset.key_env));
        }
        let endpoint = http::endpoint(&self.profile)?;
        if !user_text.trim().is_empty() {
            conversation.push_user(user_text);
        } else if !conversation.awaits_reply() {
            return Err(Failure::bad_request(
                "Gönderilecek bir ileti yok; bir şey yazıp yeniden deneyin.",
            ));
        }
        self.keep(&conversation);
        let names = Arc::new(ToolNames::new(
            self.tools
                .iter()
                .map(|t| t.name.as_str())
                .chain(conversation.tool_names()),
        ));
        let mut round = 0;
        loop {
            let call = Call {
                profile: &self.profile,
                preset,
                base: &endpoint.base,
                key: self.key.as_deref(),
                conversation: &conversation,
                tools: &self.tools,
                names: &names,
            };
            let request = wire::request(&call);
            let decoder = wire::decoder(&call);
            let events = self.events.clone();
            let mut live = move |l: Live| {
                let _ = events.send(match l {
                    Live::Text(t) => Event::Text(t),
                    Live::Reasoning(r) => Event::Reasoning(r),
                });
            };
            let key = self.key.as_deref();
            let reply = tokio::select! {
                biased;
                () = stopped(&mut self.stop) => return Err(Failure::cancelled()),
                reply = http::stream(&endpoint, request, decoder, who, key, &mut live) => reply?,
            };
            if let Some((input_tokens, output_tokens)) = reply.usage {
                let _ = self.events.send(Event::Usage {
                    input_tokens,
                    output_tokens,
                });
            }
            if reply.stop == Stop::ContextFull {
                return Err(Failure::context_full());
            }
            let calls: Vec<ToolCall> = reply.message.tool_calls().cloned().collect();
            conversation.messages.push(reply.message);
            if calls.is_empty() {
                self.keep(&conversation);
                let _ = self.events.send(Event::Finished { conversation });
                return Ok(());
            }
            if round == MAX_TOOL_ROUNDS {
                let refused = format!(
                    "Araç turu sınırına ({MAX_TOOL_ROUNDS}) ulaşıldı; araç çalıştırılmadı."
                );
                let results = calls
                    .iter()
                    .map(|c| ToolResult::from_answer(c, Err(refused.clone())))
                    .collect();
                conversation.messages.push(Message::tool_results(results));
                self.keep(&conversation);
                return Err(Failure::bad_request(format!(
                    "Ajan {MAX_TOOL_ROUNDS} araç turunda işi bitiremedi ve durduruldu. İsteği daha küçük adımlara \
                     bölün ya da “devam et” yazın."
                )));
            }
            for c in &calls {
                let _ = self.events.send(Event::ToolCall {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    input: c.input.clone(),
                });
            }
            let (results, stopped) = self.collect(&calls).await;
            conversation.messages.push(Message::tool_results(results));
            self.keep(&conversation);
            if stopped {
                return Err(Failure::cancelled());
            }
            round += 1;
        }
    }

    /// Waits for every call's answer, in any order; the turn stopped, the calls not answered are told they did not
    /// run. The results in the calls' order, and whether the turn stopped.
    async fn collect(&mut self, calls: &[ToolCall]) -> (Vec<ToolResult>, bool) {
        let mut got: HashMap<String, Result<Value, String>> = HashMap::new();
        let mut stop = false;
        while got.len() < calls.len() {
            tokio::select! {
                biased;
                () = stopped(&mut self.stop) => {
                    stop = true;
                    break;
                }
                answer = self.answers.recv() => match answer {
                    Some((id, result)) => {
                        if calls.iter().any(|c| c.id == id) {
                            got.entry(id).or_insert(result);
                        }
                    }
                    None => {
                        stop = true;
                        break;
                    }
                },
            }
        }
        let results = calls
            .iter()
            .map(|c| {
                ToolResult::from_answer(
                    c,
                    got.remove(&c.id).unwrap_or_else(|| Err(NOT_RUN.to_owned())),
                )
            })
            .collect();
        (results, stop)
    }
}

/// Ends when the turn is stopped or dropped.
async fn stopped(stop: &mut watch::Receiver<bool>) {
    let _ = stop.wait_for(|s| *s).await;
}
