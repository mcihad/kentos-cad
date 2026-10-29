//! Adres arama (docs/adr/0229): a project's own address data searched by
//! address and by place, the same on every platform. An address is folded
//! and cut into words (`fold`), read into its parts (`parse`); names are
//! compared by a similarity of their own (`similar`). A locator is built
//! from the doors, streets and areas a host reads from its drawing
//! (`locator`) and asked by address (`forward`) or by place (`reverse`);
//! an address is written back as text (`format`). `fields` guesses which
//! attributes say what from their names; `check` counts what a locator
//! holds and what it left out; `register` compares its doors with an
//! address register (docs/adr/0736). The online providers' requests and answers
//! are the services core's (`kentos_services::geocode`), scored by the same
//! rules. The independent reference is `scripts/fixtures/geocode_cases.py`
//! (fixtures/geocode/v1/cases.json).

pub mod check;
pub mod fields;
pub mod fold;
pub mod format;
pub mod forward;
pub mod locator;
pub mod parse;
pub mod register;
pub mod reverse;
pub mod similar;

pub use fold::{Tag, Token, fold, tokens};
pub use forward::{Candidate, Found, Kind, Status, find};
pub use locator::{AreaInput, DoorInput, Input, Level, Locator, StreetInput};
pub use parse::{Parsed, parse};
pub use reverse::{Place, PlaceKind};
pub use similar::{numbered, similarity};
