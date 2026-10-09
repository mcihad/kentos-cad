//! Ağ analizi on the desktop (docs/adr/0209 §10; the web's `tools/network*.ts`
//! and `model/networkForm.ts`): Ağlar's form rules, and the three tools (En
//! kısa yol, Hizmet alanı, Şebeke izleme) that ask the project's networks
//! through the host ([`crate::tool::ViewChange::Network`]); the host's
//! network thread answers ([`crate::tool::Tool::network_answered`]).

pub mod area;
mod ask;
pub mod base;
pub mod form;
pub mod result;
pub mod route;
pub mod trace;

pub use ask::{Answer, NetworkAsk, NetworkQuestion, NetworkReply};

/// Kenar payı at the start, metres (Hizmet alanı, docs/adr/0209 §6).
pub const FIRST_TRIM: f64 = 50.0;
