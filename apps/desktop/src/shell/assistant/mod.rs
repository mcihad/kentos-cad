//! Asistan (docs/adr/8056 §4): the user's own AI agent beside the drawing. The user sets up agents in Uygulama
//! ayarları › Yapay zekâ (settings.rs: a provider, a model, an API key, one the default) and chats with one in the
//! Asistan panel (view.rs). The agent draws through the agents' tools (`kentos_headless::agent_tools`), the same an
//! outside agent has over MCP, on the drawing in front (host.rs), under the owner's guard: a write is planned first and
//! executed as planned. A write that takes away or changes what is there (the tools' destructive ones) waits for the
//! user's Onayla in the panel; one that adds runs at once, its own undo step.
//!
//! The provider is called on the agent's own runtime (`kentos_agent`); the panel hears the turn's events through a task
//! and answers its tool calls here, on the interface thread, where the drawing is.

mod host;
mod settings;
#[cfg(test)]
mod tests;
mod view;

use std::collections::HashMap;
use std::time::Instant;

use iced::Task;
use iced::futures::stream;
use kentos_agent::{AgentProfile, Conversation, FailureKind, ProfileStore, Role, ToolSpec, Turn};
use kentos_headless::agent_tools::{self, DESTRUCTIVE, Guard};
use kentos_ui::widget::ai::{self, AiAction, AiEvent, ToolPhase, Usage};
use serde_json::{Value, json};

use crate::app::{App, Message};

pub(crate) use settings::{Agents, AgentsEdit};

/// What the panel asks.
#[derive(Debug, Clone)]
pub enum Event {
    /// The chat's own events (typing, Gönder, Durdur, Onayla …).
    Chat(AiEvent),
    /// What the turn answering message `.0` says; a stopped turn's late words are left out.
    Agent(u64, kentos_agent::Event),
    /// Yeni sohbet: the chat and what the agent was told are forgotten.
    New,
    /// Another agent from the panel's list, by its place in the list.
    Choose(usize),
    /// A suggestion of the empty chat, sent as it is.
    Suggest(&'static str),
    /// Uygulama ayarları › Yapay zekâ.
    Settings,
}

pub(crate) fn msg(event: Event) -> Message {
    Message::Assistant(event)
}

/// The panel's state.
pub(crate) struct Assistant {
    /// The device's agents (`main` opens the real file; tests and snapshots keep them in memory).
    pub(crate) store: ProfileStore,
    /// Uygulama ayarları › Yapay zekâ's form.
    pub(crate) agents: Agents,
    /// What the panel shows.
    pub(crate) chat: ai::Conversation,
    /// The agent chosen in the panel; none: the default.
    chosen: Option<String>,
    /// What the agent was told and answered so far, for the next turn.
    history: Option<Conversation>,
    running: Option<Running>,
    /// The plans the agent was shown (the owner's guard), kept across turns: a plan shown in one answer may be
    /// executed in the next, once the user says yes.
    guard: Guard,
    /// Yeniden dene is on its way: the turn goes on from the history as it is.
    retrying: bool,
    /// The agent's name and its provider and model, as the chat's header shows them ([`Assistant::refresh`]).
    pub(crate) shown: (String, String),
}

impl Default for Assistant {
    fn default() -> Self {
        Assistant {
            store: ProfileStore::in_memory(),
            agents: Agents::default(),
            chat: ai::Conversation::default(),
            chosen: None,
            history: None,
            running: None,
            guard: Guard::new(),
            retrying: false,
            shown: (String::new(), String::new()),
        }
    }
}

/// A turn under way.
struct Running {
    /// The panel's message it answers in.
    message: u64,
    turn: Turn,
    started: Instant,
    /// The reasoning so far, for the panel's thought.
    reasoning: String,
    /// The tool calls waiting for the user's Onayla: the panel's tool id → the call.
    waiting: HashMap<u64, Call>,
    /// The tools the turn called, numbered for the panel.
    tools: u64,
    input_tokens: u64,
    output_tokens: u64,
}

/// A tool call of the model.
struct Call {
    id: String,
    name: String,
    input: Value,
}

/// The most of a tool's result sent back to the model: tokens are the user's money.
const RESULT_LIMIT: usize = 48_000;

/// What the panel shows of a call's input and result.
const SHOWN_LIMIT: usize = 1_600;

impl Assistant {
    /// The agent the chat talks to: the one chosen, else the default.
    pub(crate) fn agent(&self) -> Option<AgentProfile> {
        self.chosen
            .as_deref()
            .and_then(|id| self.store.get(id))
            .or_else(|| self.store.default_profile())
    }

    #[cfg(test)]
    pub(crate) fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// The header's words again, after the agents or the choice changed.
    pub(crate) fn refresh(&mut self) {
        if self
            .chosen
            .as_deref()
            .is_some_and(|id| self.store.get(id).is_none())
        {
            self.chosen = None;
        }
        self.shown = self.agent().map_or_else(Default::default, |p| {
            let model = format!("{} · {}", p.preset().name, p.model);
            (p.name, model)
        });
    }
}

/// What the agent is told before everything (the house rules of the tools, in the user's language).
fn instructions(program: Option<kentos_launch::Module>) -> String {
    let program = program.map_or("KentOS", kentos_launch::Module::title);
    format!(
        "Sen {program} içinde çalışan çizim asistanısın. Kullanıcıyla Türkçe konuşursun; açık çizimi araçlarla okur, \
         komutlarla yazarsın.\n\
         \n\
         Çalışma biçimin:\n\
         - Önce durumu öğren: drawing.summary (birim, etkin katman, revision); gerekirse scene.layers ve scene.entities.\n\
         - Komutu bul: catalog.search (Türkçe sözcüklerle), sonra catalog.describe ile girdisinin şemasını oku. Şemada \
         olmayan alanı uydurma; zorunlu alanları doldur.\n\
         - Komutu kendi kapısından çalıştır: cad.run (çizimin çekirdeği), arch.run (mimari), map.run, gis.run, road.run. \
         Yanlış kapı doğrusunu söyler.\n\
         - Yazan her komut önce mode: \"plan\" ile çalışır: plan ne ekleneceğini, değişeceğini, silineceğini söyler. Plan \
         istenene uyuyorsa aynı komutu aynı girdiyle mode: \"execute\" ve o planId ile çalıştır. Birden çok adımı tek \
         geri alma adımında yazmak için command.batch.\n\
         - Silen ya da var olanı değiştiren bir yazma kullanıcının onayını bekler. Reddedilirse yazma; ne istediğini sor.\n\
         - Koordinatlar çizim biriminde (çoğunlukla metre); x doğuya, y kuzeye artar. Nokta: {{\"x\": 0, \"y\": 0}}.\n\
         - Bir araç hata verirse kodunu ve iletisini oku, girdiyi düzelt; aynı çağrıyı değiştirmeden yineleme.\n\
         - Eklenti yazman istenirse: önce extension.sdk'yı oku; extension.blueprint ile iskeleti al, \
         extension.workshop.write ile atölyeye yaz, isteğe göre değiştir, extension.verify (id ile) doğrula; sorun \
         kalmayınca extension.workshop.review kullanıcının incelemesine açar. Kurmak ve yetki vermek kullanıcınındır.\n\
         - Kısa yanıt ver: ne yaptığını ve sonucunu (kaç nesne, hangi katman, hangi ölçüde, nerede) söyle. \
         Kullanıcıya iç kimlik, sürüm ya da plan numarası (id, uid, revision, planId) ve komut kimliği yazma; \
         katman ve nesne adlarını, ölçüleri, Türkçe adları kullan. İstenmeyen değişikliği yapma."
    )
}

/// The tools offered to the model: the agents' tools this program runs (a module's program leaves out another
/// module's door).
fn tool_specs(app: &mut App) -> Vec<ToolSpec> {
    agent_tools::offered_tools(&host::AppHost(app))
        .into_iter()
        .map(|t| ToolSpec::new(t.name, t.description, t.input))
        .collect()
}

/// Whether a call writes something that takes away or changes what is there: it waits for the user's Onayla.
fn needs_approval(name: &str, input: &Value) -> bool {
    if input.get("mode").and_then(Value::as_str) != Some("execute") {
        return false;
    }
    let destructive = |command: &str| DESTRUCTIVE.contains(&command);
    match name {
        "command.batch" => input
            .get("steps")
            .and_then(Value::as_array)
            .is_some_and(|steps| {
                steps
                    .iter()
                    .filter_map(|s| s.get("command").and_then(Value::as_str))
                    .any(destructive)
            }),
        "audit.apply_fix" | "processing.run" | "processing.batch" => true,
        _ if name.ends_with(".run") => input
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(destructive),
        _ => false,
    }
}

/// A value as the panel shows it: indented JSON, cut short.
fn shown(value: &Value) -> String {
    let text = serde_json::to_string_pretty(value).unwrap_or_default();
    cut(&text, SHOWN_LIMIT)
}

fn cut(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// The end of a text that grows (the reasoning as it streams): its newest part.
fn tail(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut start = text.len() - limit;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("…{}", &text[start..])
}

/// A tool's result as the model is sent it: a long one cut, with a word on how to ask less.
fn bounded(value: Value) -> Value {
    let text = value.to_string();
    if text.len() <= RESULT_LIMIT {
        return value;
    }
    json!({
        "truncated": true,
        "bytes": text.len(),
        "note": "Sonuç çok uzun olduğu için kesildi: daha dar sorun (limit, katman, kimlik).",
        "head": cut(&text, RESULT_LIMIT),
    })
}

/// The call's line in the panel, in words (the command's title and what the call does with it); the ids are in its
/// details.
fn call_title(name: &str, input: &Value) -> String {
    agent_tools::call_words(name, input)
}

impl App {
    /// `view.assistant`: the dock shows Asistan's tab, the side panels shown if F4 hid them.
    pub(crate) fn show_assistant(&mut self) -> Task<Message> {
        if !self.open_panel(crate::app::Panel::Assistant) {
            return Task::none();
        }
        if !self.right_panel_shown() {
            self.toggle_right_panel();
        }
        if !self.docks.contains(crate::app::Panel::Assistant) {
            self.docks.show(
                crate::app::Panel::Assistant,
                kentos_ui::widget::docking::Side::Right,
            );
        }
        self.docks
            .update(kentos_ui::widget::docking::Event::Selected(
                crate::app::Panel::Assistant,
            ));
        Task::none()
    }

    pub(crate) fn assistant_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Chat(event) => {
                if matches!(event, AiEvent::Retry(_)) {
                    self.assistant.retrying = true;
                }
                let action = self.assistant.chat.update(event);
                let task = action.map_or_else(Task::none, |a| self.assistant_action(a));
                self.assistant.retrying = false;
                task
            }
            Event::Agent(message, event) => self.assistant_heard(message, event),
            Event::New => {
                self.assistant_stop();
                self.assistant.chat = ai::Conversation::default();
                self.assistant.history = None;
                self.assistant.guard = Guard::new();
                Task::none()
            }
            Event::Choose(index) => {
                if let Some(p) = self.assistant.store.list().get(index) {
                    self.assistant.chosen = Some(p.id.clone());
                }
                self.assistant.refresh();
                Task::none()
            }
            Event::Suggest(text) => {
                self.assistant.chat.prompt = ai::PromptState::with_text(text);
                self.assistant_event(Event::Chat(AiEvent::Submit))
            }
            Event::Settings => {
                self.open_settings_at(crate::settings_sections::Section::Ai);
                // The first agent's form at once: there is nothing else to see.
                if self.assistant.store.list().is_empty() {
                    return self.agents_edit(AgentsEdit::Add);
                }
                Task::none()
            }
        }
    }

    fn assistant_action(&mut self, action: AiAction) -> Task<Message> {
        match action {
            AiAction::Send(request) => self.assistant_send(request.id, request.prompt),
            AiAction::Stop(_) => {
                self.assistant_stop();
                Task::none()
            }
            AiAction::Copy(text) => iced::clipboard::write(text),
            AiAction::ToolApproval {
                request,
                tool,
                approved,
            } => {
                self.assistant_approved(request, tool, approved);
                Task::none()
            }
            AiAction::Attach
            | AiAction::OpenSource(_)
            | AiAction::Answer { .. }
            | AiAction::Feedback { .. } => Task::none(),
        }
    }

    /// Gönder: a turn of the agent on what the user wrote.
    fn assistant_send(&mut self, message: u64, prompt: String) -> Task<Message> {
        let Some(profile) = self.assistant.agent() else {
            self.assistant.chat.fail(
                message,
                "Henüz bir yapay zekâ ajanı yok: Uygulama ayarları › Yapay zekâ'dan kendi API anahtarınızla bir ajan \
                 ekleyin.",
            );
            return Task::none();
        };
        let key = self.assistant.store.key_for(&profile);
        if key.is_none() && profile.preset().needs_key {
            let env = profile.preset().key_env.unwrap_or("ortam değişkeni");
            self.assistant.chat.fail(
                message,
                &format!(
                    "“{}” için API anahtarı yok: Uygulama ayarları › Yapay zekâ'dan anahtarı yazın ya da {env} \
                     değişkenini tanımlayın.",
                    profile.name
                ),
            );
            return Task::none();
        }
        let mut conversation = match self.assistant.history.take() {
            Some(history) => history.for_profile(&profile),
            None => Conversation::new(instructions(self.program)),
        };
        let mut text = prompt;
        if self.assistant.retrying && conversation.awaits_reply() {
            // Yeniden dene: the turn goes on where it broke off; what was run on the drawing stays known.
            text = String::new();
        } else if conversation
            .messages
            .last()
            .is_some_and(|m| m.role == Role::User)
        {
            // A message never answered, the user moved on: the new one takes its place.
            conversation.messages.pop();
        }
        let tools = tool_specs(self);
        let mut turn = kentos_agent::start_turn(&profile, key, conversation, tools, text);
        let events = turn.take_events();
        self.assistant.running = Some(Running {
            message,
            turn,
            started: Instant::now(),
            reasoning: String::new(),
            waiting: HashMap::new(),
            tools: 0,
            input_tokens: 0,
            output_tokens: 0,
        });
        let heard = stream::unfold(events, |mut events| async move {
            events.recv().await.map(|e| (e, events))
        });
        Task::run(heard, move |event| msg(Event::Agent(message, event)))
    }

    /// Durdur, Yeni sohbet: the turn stops; what it ran on the drawing stays in the history.
    fn assistant_stop(&mut self) {
        if let Some(running) = self.assistant.running.take() {
            running.turn.cancel();
            self.assistant.history = Some(running.turn.conversation());
        }
    }

    fn assistant_heard(&mut self, message: u64, event: kentos_agent::Event) -> Task<Message> {
        use kentos_agent::Event as Heard;
        let Some(running) = self
            .assistant
            .running
            .as_mut()
            .filter(|r| r.message == message)
        else {
            return Task::none();
        };
        match event {
            Heard::Text(text) => {
                self.assistant.chat.append(message, &text);
            }
            Heard::Reasoning(text) => {
                running.reasoning.push_str(&text);
                let elapsed = running.started.elapsed();
                let summary = tail(&running.reasoning, 4_000);
                self.assistant.chat.thought(message, &summary, elapsed);
            }
            Heard::Usage {
                input_tokens,
                output_tokens,
            } => {
                running.input_tokens += input_tokens;
                running.output_tokens += output_tokens;
            }
            Heard::ToolCall { id, name, input } => {
                running.tools += 1;
                let tool = running.tools;
                let ask = needs_approval(&name, &input);
                self.assistant.chat.tool(
                    message,
                    ai::ToolCall {
                        id: tool,
                        name: call_title(&name, &input),
                        detail: shown(&input),
                        phase: if ask {
                            ToolPhase::Approval
                        } else {
                            ToolPhase::Running
                        },
                        output: String::new(),
                        elapsed: None,
                        expanded: ask,
                    },
                );
                let call = Call { id, name, input };
                if ask {
                    running.waiting.insert(tool, call);
                } else {
                    self.assistant_run(message, tool, call);
                }
            }
            Heard::Finished { conversation } => {
                let running = self.assistant.running.take();
                let usage = running.map(|r| Usage {
                    input_tokens: r.input_tokens,
                    output_tokens: r.output_tokens,
                    elapsed: r.started.elapsed(),
                });
                self.assistant.chat.finish(message, usage);
                self.assistant.history = Some(conversation);
            }
            Heard::Failed {
                kind,
                message: why,
                retry_after,
            } => {
                let running = self.assistant.running.take();
                if let Some(r) = running {
                    self.assistant.history = Some(r.turn.conversation());
                }
                if kind != FailureKind::Cancelled {
                    let wait = retry_after
                        .map(|d| format!(" {} saniye sonra yeniden deneyin.", d.as_secs().max(1)))
                        .unwrap_or_default();
                    self.assistant.chat.fail(message, &format!("{why}{wait}"));
                }
            }
        }
        Task::none()
    }

    /// Onayla or Reddet on a call waiting for the user.
    fn assistant_approved(&mut self, message: u64, tool: u64, approved: bool) {
        let Some(running) = self
            .assistant
            .running
            .as_mut()
            .filter(|r| r.message == message)
        else {
            return;
        };
        let Some(call) = running.waiting.remove(&tool) else {
            return;
        };
        if approved {
            self.assistant_run(message, tool, call);
            return;
        }
        running.turn.answer(
            &call.id,
            Err("Kullanıcı bu değişikliği onaylamadı; çizime bir şey yazılmadı. Ne istediğini sorun.".into()),
        );
        self.assistant_tool_done(
            message,
            tool,
            &call,
            ToolPhase::Error,
            "Reddedildi.".into(),
            None,
        );
    }

    /// Runs a call on the drawing in front and answers the model.
    fn assistant_run(&mut self, message: u64, tool: u64, call: Call) {
        let started = Instant::now();
        let mut guard = std::mem::take(&mut self.assistant.guard);
        let result = agent_tools::call(
            &mut host::AppHost(self),
            &mut guard,
            &call.name,
            &call.input,
        );
        self.assistant.guard = guard;
        let (phase, output, answer) = match result {
            Ok(value) => (ToolPhase::Complete, shown(&value), Ok(bounded(value))),
            Err(e) => {
                let why = format!("{}: {}", e.code, e.message);
                (ToolPhase::Error, why.clone(), Err(why))
            }
        };
        if let Some(running) = self
            .assistant
            .running
            .as_ref()
            .filter(|r| r.message == message)
        {
            running.turn.answer(&call.id, answer);
        }
        self.assistant_tool_done(message, tool, &call, phase, output, Some(started.elapsed()));
    }

    fn assistant_tool_done(
        &mut self,
        message: u64,
        tool: u64,
        call: &Call,
        phase: ToolPhase,
        output: String,
        elapsed: Option<std::time::Duration>,
    ) {
        self.assistant.chat.tool(
            message,
            ai::ToolCall {
                id: tool,
                name: call_title(&call.name, &call.input),
                detail: shown(&call.input),
                phase,
                output,
                elapsed,
                expanded: false,
            },
        );
    }
}
