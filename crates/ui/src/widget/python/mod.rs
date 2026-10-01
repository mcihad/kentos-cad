//! Python authoring surfaces, with JetBrains Mono and frame driven motion.
//!
//! [`PythonEditor`] works with Iced's `text_editor::Content`; [`EditorState`]
//! adds Python indentation and undo/redo. [`PythonRepl`] renders a [`ReplState`].
//! Call `ReplState::update`, execute the returned [`Request`] in your runtime,
//! and pass its result to `ReplState::finish`. Execution stays outside the UI
//! library, so an application's interpreter and permissions remain its own.
//!
//! ```ignore
//! PythonEditor::new(&state.content, Message::Edit)
//!     .title("analysis.py")
//!     .modified(state.modified())
//!     .on_run(Message::Run)
//!
//! PythonRepl::new(&repl, Message::Repl).height(320)
//! ```

mod completion;
mod completion_menu;
mod editor;
mod repl;
mod state;
use crate::widget::animated_surface as surface;
pub mod syntax;

pub use completion::{
    CompletionEvent, CompletionItem, CompletionRequest, CompletionState, SymbolKind,
};
pub use editor::PythonEditor;
pub use repl::{CodeView, PythonRepl};
pub use state::{
    EditorState, Entry, EntryKind, ReplEvent, ReplState, Request, RunResult, RunStatus,
};

use iced::{Font, font};

/// Code always uses the embedded JetBrains Mono family, independently of
/// the application's general monospace preference. Call `typography::load()`
/// at application startup (or install the font when the `fonts` feature is off).
pub fn font() -> Font {
    Font::with_name("JetBrains Mono")
}

pub fn strong_font() -> Font {
    Font {
        weight: font::Weight::Semibold,
        ..font()
    }
}
