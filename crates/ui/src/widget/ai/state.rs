use std::collections::VecDeque;
use std::time::Duration;

use iced::widget::text_editor::{self, Action};

const MESSAGE_LIMIT: usize = 100;
const TEXT_LIMIT: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Thinking,
    Answering,
    Waiting,
    Complete,
    Stopped,
    Error,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Hazır",
            Self::Thinking => "Düşünüyor",
            Self::Answering => "Yanıt yazılıyor",
            Self::Waiting => "Yanıtınız bekleniyor",
            Self::Complete => "Tamamlandı",
            Self::Stopped => "Durduruldu",
            Self::Error => "Bir sorun oluştu",
        }
    }
    pub fn active(self) -> bool {
        matches!(self, Self::Thinking | Self::Answering)
    }
}

impl From<Phase> for crate::widget::animated_surface::Activity {
    fn from(phase: Phase) -> Self {
        match phase {
            Phase::Thinking | Phase::Answering => Self::Running,
            Phase::Complete => Self::Success,
            Phase::Error => Self::Error,
            _ => Self::Ready,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    Document,
    Image,
    Code,
    Drawing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub id: u64,
    pub name: String,
    pub kind: AttachmentKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    Helpful,
    Unhelpful,
}

#[derive(Debug, Clone, Default)]
pub struct Thought {
    /// A provider's displayable reasoning summary or observable progress;
    /// this field does not infer or generate a model's hidden reasoning.
    pub summary: String,
    pub elapsed: Duration,
    pub expanded: bool,
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPhase {
    Pending,
    Running,
    Approval,
    Complete,
    Stopped,
    Error,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: u64,
    pub name: String,
    pub detail: String,
    pub phase: ToolPhase,
    pub output: String,
    pub elapsed: Option<Duration>,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub title: String,
    pub location: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub elapsed: Duration,
}

#[derive(Debug, Clone)]
pub struct AnswerOption {
    pub label: String,
    pub description: String,
    pub recommended: bool,
}

impl AnswerOption {
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            recommended: false,
        }
    }
    pub fn recommended(mut self) -> Self {
        self.recommended = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub question: u64,
    pub choices: Vec<usize>,
    pub text: String,
    pub skipped: bool,
}

/// A real pending question. Nothing is submitted until the user activates
/// the answer button; recommended choices are suggestions, not approval.
#[derive(Debug, Clone)]
pub struct Question {
    pub id: u64,
    pub title: String,
    pub detail: String,
    pub options: Vec<AnswerOption>,
    pub multiple: bool,
    pub allow_skip: bool,
    pub selected: Vec<usize>,
    pub custom: String,
    pub answer: Option<Answer>,
}

impl Question {
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            detail: String::new(),
            options: Vec::new(),
            multiple: false,
            allow_skip: false,
            selected: Vec::new(),
            custom: String::new(),
            answer: None,
        }
    }
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }
    pub fn option(mut self, option: AnswerOption) -> Self {
        self.options.push(option);
        self
    }
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }
    pub fn allow_skip(mut self, skip: bool) -> Self {
        self.allow_skip = skip;
        self
    }
    pub fn ready(&self) -> bool {
        !self.selected.is_empty() || !self.custom.trim().is_empty()
    }

    pub fn update(&mut self, event: QuestionEvent) -> Option<Answer> {
        if self.answer.is_some() || event.id() != self.id {
            return None;
        }
        match event {
            QuestionEvent::Choose { index, .. } if index < self.options.len() => {
                self.custom.clear();
                if self.multiple {
                    if let Some(at) = self.selected.iter().position(|i| *i == index) {
                        self.selected.remove(at);
                    } else {
                        self.selected.push(index);
                        self.selected.sort_unstable();
                    }
                } else {
                    self.selected = vec![index];
                }
            }
            QuestionEvent::Custom { text, .. } => {
                self.custom = bounded(&text);
                self.selected.clear();
            }
            QuestionEvent::Submit { .. } if self.ready() => {
                let answer = Answer {
                    question: self.id,
                    choices: self.selected.clone(),
                    text: self.custom.trim().to_owned(),
                    skipped: false,
                };
                self.answer = Some(answer.clone());
                return Some(answer);
            }
            QuestionEvent::Skip { .. } if self.allow_skip => {
                let answer = Answer {
                    question: self.id,
                    choices: Vec::new(),
                    text: String::new(),
                    skipped: true,
                };
                self.answer = Some(answer.clone());
                return Some(answer);
            }
            _ => {}
        }
        None
    }
}

#[derive(Debug, Clone)]
pub enum QuestionEvent {
    Choose { id: u64, index: usize },
    Custom { id: u64, text: String },
    Submit { id: u64 },
    Skip { id: u64 },
}
impl QuestionEvent {
    pub fn id(&self) -> u64 {
        match self {
            Self::Choose { id, .. }
            | Self::Custom { id, .. }
            | Self::Submit { id }
            | Self::Skip { id } => *id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageData {
    pub id: u64,
    pub role: Role,
    pub body: String,
    pub phase: Phase,
    pub attachments: Vec<Attachment>,
    pub thought: Option<Thought>,
    pub tools: Vec<ToolCall>,
    pub questions: Vec<Question>,
    pub sources: Vec<Source>,
    pub usage: Option<Usage>,
    pub vote: Option<Vote>,
    /// Failure details kept separately so an interrupted response stays visible.
    pub failure: Option<String>,
}

impl MessageData {
    pub fn new(id: u64, role: Role, body: impl Into<String>) -> Self {
        Self {
            id,
            role,
            body: bounded(&body.into()),
            phase: Phase::Complete,
            attachments: Vec::new(),
            thought: None,
            tools: Vec::new(),
            questions: Vec::new(),
            sources: Vec::new(),
            usage: None,
            vote: None,
            failure: None,
        }
    }

    fn awaiting_user(&self) -> bool {
        self.questions.iter().any(|q| q.answer.is_none())
            || self.tools.iter().any(|t| t.phase == ToolPhase::Approval)
    }

    fn resume(&mut self, phase: Phase) {
        self.phase = if self.awaiting_user() {
            Phase::Waiting
        } else {
            phase
        };
    }

    fn stop_activity(&mut self) {
        if let Some(thought) = &mut self.thought {
            thought.complete = true;
        }
        for tool in &mut self.tools {
            if matches!(
                tool.phase,
                ToolPhase::Pending | ToolPhase::Running | ToolPhase::Approval
            ) {
                tool.phase = ToolPhase::Stopped;
            }
        }
    }
}

/// Plain multiline composition, without Python indentation rules.
#[derive(Debug, Default)]
pub struct PromptState {
    pub content: text_editor::Content,
    pub attachments: Vec<Attachment>,
    pub revision: u64,
    undo: VecDeque<String>,
    redo: Vec<String>,
    typing: bool,
}

impl PromptState {
    pub fn with_text(text: &str) -> Self {
        Self {
            content: text_editor::Content::with_text(text),
            ..Self::default()
        }
    }
    pub fn ready(&self) -> bool {
        !self.content.text().trim().is_empty() || !self.attachments.is_empty()
    }
    pub fn perform(&mut self, action: Action) {
        let typing = matches!(action, Action::Edit(text_editor::Edit::Insert(_)));
        if action.is_edit() {
            if !typing || !self.typing {
                self.undo.push_back(self.content.text());
                while self.undo.len() > 64 {
                    self.undo.pop_front();
                }
            }
            self.redo.clear();
            self.revision = self.revision.wrapping_add(1);
        }
        self.typing = typing;
        self.content.perform(action);
    }
    pub fn undo(&mut self) {
        if let Some(text) = self.undo.pop_back() {
            self.redo.push(self.content.text());
            self.replace(text);
        }
    }
    pub fn redo(&mut self) {
        if let Some(text) = self.redo.pop() {
            self.undo.push_back(self.content.text());
            self.replace(text);
        }
    }
    fn replace(&mut self, text: String) {
        self.content = text_editor::Content::with_text(&text);
        self.content
            .perform(Action::Move(text_editor::Motion::DocumentEnd));
        self.typing = false;
        self.revision = self.revision.wrapping_add(1);
    }
}

#[derive(Debug, Clone)]
pub struct AiRequest {
    pub id: u64,
    pub prompt: String,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone)]
pub enum AiEvent {
    Edit(Action),
    Undo,
    Redo,
    Submit,
    Stop,
    Attach,
    RemoveAttachment(u64),
    Question {
        message: u64,
        event: QuestionEvent,
    },
    ToggleThought(u64),
    ToggleTool {
        message: u64,
        tool: u64,
    },
    ApproveTool {
        message: u64,
        tool: u64,
        approved: bool,
    },
    Copy(u64),
    CopyText(String),
    Retry(u64),
    Feedback {
        message: u64,
        vote: Vote,
    },
    OpenSource(String),
    Follow(bool),
}

/// Actions for the host. Widgets never start a model, execute a tool, open a
/// URL, access attachments or copy data to the OS themselves.
#[derive(Debug, Clone)]
pub enum AiAction {
    Send(AiRequest),
    Stop(u64),
    Attach,
    Copy(String),
    OpenSource(String),
    Answer {
        request: u64,
        answer: Answer,
    },
    ToolApproval {
        request: u64,
        tool: u64,
        approved: bool,
    },
    Feedback {
        message: u64,
        vote: Vote,
    },
}

#[derive(Debug, Default)]
pub struct Conversation {
    pub prompt: PromptState,
    messages: VecDeque<MessageData>,
    serial: u64,
    pending: Option<u64>,
    revision: u64,
    scroll_paused: bool,
}

impl Conversation {
    pub fn messages(&self) -> impl ExactSizeIterator<Item = &MessageData> {
        self.messages.iter()
    }
    pub fn pending(&self) -> Option<u64> {
        self.pending
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn following(&self) -> bool {
        !self.scroll_paused
    }
    pub fn phase(&self) -> Phase {
        self.messages.back().map_or(Phase::Idle, |m| m.phase)
    }

    pub fn push(&mut self, message: MessageData) {
        self.serial = self.serial.max(message.id);
        self.messages.push_back(message);
        while self.messages.len() > MESSAGE_LIMIT {
            self.messages.pop_front();
        }
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn message(&self, id: u64) -> Option<&MessageData> {
        self.messages.iter().find(|m| m.id == id)
    }

    fn active(&mut self, id: u64) -> Option<&mut MessageData> {
        if self.pending != Some(id) {
            return None;
        }
        self.messages.iter_mut().find(|m| m.id == id)
    }

    /// Late text from a stopped or older request is ignored.
    pub fn append(&mut self, id: u64, delta: &str) -> bool {
        let Some(message) = self.active(id) else {
            return false;
        };
        if message.body.len() >= TEXT_LIMIT {
            return false;
        }
        message
            .body
            .push_str(&bounded_to(delta, TEXT_LIMIT - message.body.len()));
        message.resume(Phase::Answering);
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn thought(&mut self, id: u64, summary: &str, elapsed: Duration) -> bool {
        let Some(message) = self.active(id) else {
            return false;
        };
        let thought = message.thought.get_or_insert_with(Thought::default);
        thought.summary = bounded(summary);
        thought.elapsed = elapsed;
        message.resume(Phase::Thinking);
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn tool(&mut self, id: u64, tool: ToolCall) -> bool {
        let Some(message) = self.active(id) else {
            return false;
        };
        if let Some(existing) = message
            .tools
            .iter_mut()
            .find(|existing| existing.id == tool.id)
        {
            let expanded = existing.expanded;
            *existing = tool;
            existing.expanded = expanded;
        } else if message.tools.len() < 32 {
            message.tools.push(tool);
        } else {
            return false;
        }
        message.resume(if message.body.is_empty() {
            Phase::Thinking
        } else {
            Phase::Answering
        });
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn ask(&mut self, id: u64, question: Question) -> bool {
        let Some(message) = self.active(id) else {
            return false;
        };
        if message.questions.iter().any(|q| q.id == question.id) || message.questions.len() >= 16 {
            return false;
        }
        message.questions.push(question);
        message.phase = Phase::Waiting;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn finish(&mut self, id: u64, usage: Option<Usage>) -> bool {
        let Some(message) = self.active(id) else {
            return false;
        };
        if message.questions.iter().any(|q| q.answer.is_none())
            || message.tools.iter().any(|t| {
                matches!(
                    t.phase,
                    ToolPhase::Pending | ToolPhase::Running | ToolPhase::Approval
                )
            })
        {
            return false;
        }
        message.phase = Phase::Complete;
        message.usage = usage;
        if let Some(thought) = &mut message.thought {
            thought.complete = true;
        }
        self.pending = None;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn fail(&mut self, id: u64, detail: &str) -> bool {
        let Some(message) = self.active(id) else {
            return false;
        };
        message.failure = Some(bounded(detail));
        message.phase = Phase::Error;
        message.stop_activity();
        self.pending = None;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn update(&mut self, event: AiEvent) -> Option<AiAction> {
        match event {
            AiEvent::Edit(action) => self.prompt.perform(action),
            AiEvent::Undo => self.prompt.undo(),
            AiEvent::Redo => self.prompt.redo(),
            AiEvent::Submit => {
                if self.pending.is_some() || !self.prompt.ready() {
                    return None;
                }
                let user_id = self.serial.checked_add(1)?;
                let id = user_id.checked_add(1)?;
                let request = AiRequest {
                    id,
                    prompt: self.prompt.content.text().trim().to_owned(),
                    attachments: self.prompt.attachments.clone(),
                };
                let mut user = MessageData::new(user_id, Role::User, &request.prompt);
                user.attachments = request.attachments.clone();
                self.push(user);
                let mut assistant = MessageData::new(id, Role::Assistant, "");
                assistant.phase = Phase::Thinking;
                self.push(assistant);
                self.pending = Some(id);
                self.prompt = PromptState::default();
                self.scroll_paused = false;
                return Some(AiAction::Send(request));
            }
            AiEvent::Stop => {
                if let Some(id) = self.pending.take() {
                    if let Some(message) = self.messages.iter_mut().find(|m| m.id == id) {
                        message.phase = Phase::Stopped;
                        message.stop_activity();
                    }
                    return Some(AiAction::Stop(id));
                }
            }
            AiEvent::Attach => return Some(AiAction::Attach),
            AiEvent::Follow(follow) => self.scroll_paused = !follow,
            AiEvent::RemoveAttachment(id) => self.prompt.attachments.retain(|a| a.id != id),
            AiEvent::OpenSource(location) => return Some(AiAction::OpenSource(location)),
            AiEvent::Copy(id) => return self.message(id).map(|m| AiAction::Copy(m.body.clone())),
            AiEvent::CopyText(text) => return Some(AiAction::Copy(text)),
            AiEvent::Retry(id) => {
                if self.pending.is_some() {
                    return None;
                }
                let index = self
                    .messages
                    .iter()
                    .position(|m| m.id == id && m.role == Role::Assistant)?;
                let user = self
                    .messages
                    .iter()
                    .take(index)
                    .rev()
                    .find(|m| m.role == Role::User)?;
                let prompt = user.body.clone();
                let attachments = user.attachments.clone();
                // Keep an unsent draft when retrying a failed response.
                let draft = std::mem::replace(&mut self.prompt, PromptState::with_text(&prompt));
                self.prompt.attachments = attachments;
                let action = self.update(AiEvent::Submit);
                self.prompt = draft;
                return action;
            }
            AiEvent::Feedback { message, vote } => {
                if let Some(data) = self
                    .messages
                    .iter_mut()
                    .find(|m| m.id == message && m.role == Role::Assistant)
                {
                    data.vote = Some(vote);
                    return Some(AiAction::Feedback { message, vote });
                }
            }
            AiEvent::ToggleThought(id) => {
                if let Some(thought) = self
                    .messages
                    .iter_mut()
                    .find(|m| m.id == id)
                    .and_then(|m| m.thought.as_mut())
                {
                    thought.expanded = !thought.expanded;
                }
            }
            AiEvent::ToggleTool { message, tool } => {
                if let Some(tool) = self
                    .messages
                    .iter_mut()
                    .find(|m| m.id == message)
                    .and_then(|m| m.tools.iter_mut().find(|t| t.id == tool))
                {
                    tool.expanded = !tool.expanded;
                }
            }
            AiEvent::Question { message, event } => {
                let data = self.active(message)?;
                let answer = data
                    .questions
                    .iter_mut()
                    .find(|q| q.id == event.id())?
                    .update(event)?;
                data.resume(Phase::Thinking);
                return Some(AiAction::Answer {
                    request: message,
                    answer,
                });
            }
            AiEvent::ApproveTool {
                message,
                tool,
                approved,
            } => {
                let data = self.active(message)?;
                let call = data
                    .tools
                    .iter_mut()
                    .find(|t| t.id == tool && t.phase == ToolPhase::Approval)?;
                call.phase = if approved {
                    ToolPhase::Pending
                } else {
                    ToolPhase::Error
                };
                data.resume(Phase::Thinking);
                return Some(AiAction::ToolApproval {
                    request: message,
                    tool,
                    approved,
                });
            }
        }
        None
    }
}

fn bounded(text: &str) -> String {
    bounded_to(text, TEXT_LIMIT)
}
fn bounded_to(text: &str, limit: usize) -> String {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send(state: &mut Conversation, prompt: &str) -> u64 {
        state.prompt = PromptState::with_text(prompt);
        let Some(AiAction::Send(request)) = state.update(AiEvent::Submit) else {
            panic!("request")
        };
        request.id
    }

    #[test]
    fn cancellation_rejects_late_deltas_and_preserves_partial_answers_and_drafts() {
        let mut state = Conversation::default();
        let old = send(&mut state, "ilk soru");
        state.append(old, "Yanıt 🦀");
        state.prompt = PromptState::with_text("sonraki soru");
        assert!(matches!(state.update(AiEvent::Stop), Some(AiAction::Stop(id)) if id == old));
        assert_eq!(state.message(old).unwrap().body, "Yanıt 🦀");
        assert!(!state.append(old, " eski parça"));
        let Some(AiAction::Send(request)) = state.update(AiEvent::Submit) else {
            panic!("request")
        };
        assert!(!state.finish(old, None));
        assert_eq!(state.pending(), Some(request.id));
    }

    #[test]
    fn questions_require_an_explicit_answer_and_keep_choices_during_stream_updates() {
        let mut state = Conversation::default();
        let id = send(&mut state, "katmanları incele");
        state.ask(
            id,
            Question::new(7, "Hangi katman?")
                .option(AnswerOption::new("Parseller", "").recommended()),
        );
        assert!(!state.finish(id, None));
        let choice = AiEvent::Question {
            message: id,
            event: QuestionEvent::Choose { id: 7, index: 0 },
        };
        assert!(state.update(choice).is_none());
        state.thought(id, "Katmanlar hazır.", Duration::from_secs(1));
        state.append(id, "Sonuç hazırlanıyor.");
        assert_eq!(state.phase(), Phase::Waiting);
        assert_eq!(state.message(id).unwrap().questions[0].selected, [0]);
        let answer = state.update(AiEvent::Question {
            message: id,
            event: QuestionEvent::Submit { id: 7 },
        });
        assert!(matches!(answer, Some(AiAction::Answer { request, .. }) if request == id));
        assert!(state.finish(id, None));
        assert!(
            state
                .update(AiEvent::Question {
                    message: id,
                    event: QuestionEvent::Submit { id: 7 }
                })
                .is_none()
        );
    }

    #[test]
    fn tool_approval_is_explicit_and_retry_keeps_an_unsent_prompt() {
        let mut state = Conversation::default();
        let id = send(&mut state, "alanları hesapla");
        state.tool(
            id,
            ToolCall {
                id: 3,
                name: "calculate".into(),
                detail: String::new(),
                phase: ToolPhase::Approval,
                output: String::new(),
                elapsed: None,
                expanded: false,
            },
        );
        assert!(!state.finish(id, None));
        assert_eq!(state.phase(), Phase::Waiting);
        assert!(matches!(
            state.update(AiEvent::ApproveTool {
                message: id,
                tool: 3,
                approved: true
            }),
            Some(AiAction::ToolApproval { approved: true, .. })
        ));
        assert!(!state.finish(id, None));
        state.fail(id, "bağlantı kesildi");
        state.prompt = PromptState::with_text("taslak");
        assert!(matches!(
            state.update(AiEvent::Retry(id)),
            Some(AiAction::Send(_))
        ));
        assert_eq!(state.prompt.content.text(), "taslak");
    }

    #[test]
    fn multiple_answers_free_text_and_utf8_limits_are_consistent() {
        let mut question = Question::new(1, "Seçin")
            .option(AnswerOption::new("A", ""))
            .option(AnswerOption::new("B", ""))
            .multiple(true);
        assert!(question.update(QuestionEvent::Submit { id: 1 }).is_none());
        question.update(QuestionEvent::Choose { id: 1, index: 1 });
        question.update(QuestionEvent::Choose { id: 1, index: 0 });
        assert_eq!(question.selected, [0, 1]);
        question.update(QuestionEvent::Custom {
            id: 1,
            text: "Özel yanıt 🦀".into(),
        });
        assert!(question.selected.is_empty());
        assert_eq!(
            question
                .update(QuestionEvent::Submit { id: 1 })
                .unwrap()
                .text,
            "Özel yanıt 🦀"
        );
        assert_eq!(bounded_to("🦀ç", 5), "🦀");
    }

    #[test]
    fn failure_preserves_text_and_scroll_following_respects_the_reader() {
        let mut state = Conversation::default();
        let id = send(&mut state, "soru");
        state.append(id, "Kısmi yanıt");
        state.update(AiEvent::Follow(false));
        state.append(id, " devam ediyor");
        assert!(!state.following());
        assert!(state.fail(id, "Bağlantı kesildi"));
        let message = state.message(id).unwrap();
        assert_eq!(message.body, "Kısmi yanıt devam ediyor");
        assert_eq!(message.failure.as_deref(), Some("Bağlantı kesildi"));
        assert!(!state.append(id, "geç parça"));
        send(&mut state, "yeni soru");
        assert!(state.following());
    }
}
