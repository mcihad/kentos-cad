//! Messages a tool writes for the user, with the web's levels
//! (`apps/web/src/app/state.ts` `LogLevel`). The traces read the level of
//! the newest one (docs/adr/0018: “son iletinin düzeyi”).

/// How a message is meant: the web's `command`, `info`, `success`, `warn`, `error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// What the user asked for: a command started, a value typed.
    Command,
    Info,
    Success,
    Warn,
    Error,
}

impl Level {
    /// The web's name for the level, as the traces write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Command => "command",
            Level::Info => "info",
            Level::Success => "success",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// One message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub level: Level,
    pub text: String,
}

impl Line {
    pub fn new(level: Level, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
        }
    }
}
