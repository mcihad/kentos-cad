//! The expression builder's language services (docs/adr/0100 §5): one core
//! for the web's and the desktop's builder dialogs, pinned on both by
//! `fixtures/expression/v2/builder.json`.
//!
//! - `tokens`: each token's class, for the editor's colours;
//! - `check`: the error with the span to underline, and warnings (a field
//!   the objects do not have);
//! - `complete`: what can stand at the cursor; `signature`: the call the
//!   cursor is in and its argument; `bracket`: the parenthesis matching
//!   the one at the cursor;
//! - `catalog`: the builder's tree; `help` and `help_at`: an entry's
//!   signature, description, arguments and examples; `values`: a field's
//!   values as the builder lists and inserts them.
//!
//! Positions are UTF-16 code units from 0, as the web's text fields count;
//! `units` converts for the desktop's editor. Everything works on text that
//! does not compile: the dialogs call these on every key.

mod catalog;
mod check;
mod complete;
mod lex;
mod place;
pub mod units;

pub use catalog::{
    Arg, Help, Section, ValueItem, catalog, help, help_at, literal, preview, values,
};
pub use check::{Check, Diagnostic, check};
pub use complete::{Bracket, Completion, Signature, SignatureArg, bracket, complete, signature};
pub use lex::{Class, Span, tokens};
pub use place::place;

/// What an entry of the completion list or of the tree is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A field of the objects: `Parsel`, `[Tapu alanı]`.
    Field,
    /// A `$` value: `$alan`, `$katman`.
    Variable,
    Function,
    /// A sign: `=`, `||`.
    Operator,
    /// A word of the language: `ve`, `değil`, `doğru`, `boş`.
    Keyword,
}

impl Kind {
    /// A stable name (the pages and the fixture).
    pub fn id(self) -> &'static str {
        match self {
            Kind::Field => "field",
            Kind::Variable => "variable",
            Kind::Function => "function",
            Kind::Operator => "operator",
            Kind::Keyword => "keyword",
        }
    }

    /// The kind a stable name names.
    pub fn from_id(id: &str) -> Option<Kind> {
        [
            Kind::Field,
            Kind::Variable,
            Kind::Function,
            Kind::Operator,
            Kind::Keyword,
        ]
        .into_iter()
        .find(|k| k.id() == id)
    }
}

/// An entry the builder can insert, as the completion list and the tree show it.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub kind: Kind,
    /// As listed: `yuvarla`, `$alan`, `Tapu alanı`, `ve`.
    pub label: String,
    /// One line beside it: a function's signature, a field's type, what a value is.
    pub detail: String,
    /// What goes into the expression: `yuvarla()`, `$alan`, `[Tapu alanı]`.
    pub insert: String,
    /// Where the cursor goes in `insert`, in UTF-16 units: between a call's parentheses.
    pub caret: usize,
    /// The key of its help (`help`).
    pub key: String,
    /// The other name completion found it by (`round` for yuvarla).
    pub alias: Option<String>,
}
