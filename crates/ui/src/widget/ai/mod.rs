//! Native AI surfaces: conversation, composer, streaming response, reasoning
//! summaries, questions, tools, citations and feedback. No provider SDK or
//! network access lives here. The host supplies text and observable state.
//!
//! ```ignore
//! AiConversation::new(&conversation, Message::Ai)
//!     .title("KentOS Asistan")
//!     .model("Çizim asistanı")
//!     .output_id("ai-output")
//!
//! if let Some(AiAction::Send(request)) = conversation.update(event) {
//!     // Run the host's model, then feed deltas using request.id.
//! }
//! ```

mod content;
mod disclosure;
#[cfg(all(test, feature = "snapshot"))]
mod interaction_tests;
mod motion;
mod paragraph;
mod scroll;
mod state;
mod view;

pub use content::AiContent;
pub use motion::{AiActivity, AiStream};
pub use state::{
    AiAction, AiEvent, AiRequest, Answer, AnswerOption, Attachment, AttachmentKind, Conversation,
    MessageData, Phase, PromptState, Question, QuestionEvent, Role, Source, Thought, ToolCall,
    ToolPhase, Usage, Vote,
};
pub use view::{AiConversation, AiMessage, AiPrompt, AiQuestion, AiThinking, AiToolCall};
