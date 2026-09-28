//! KentOS's MCP server (docs/adr/0133): the drawing commands of the command
//! catalog, a drawing's queries and its `.kcad` files as tools an AI agent
//! calls over stdio, on the headless command host (`kentos-headless`).
//!
//! - [`Server::handle`] answers one JSON-RPC message: MCP 2026-07-28's
//!   per-request `_meta` (stateless, `server/discover`) and, for older
//!   clients, the `initialize` handshake of 2025-11-25 and before.
//! - Drawings are named by handles a tool gives (`drawing.new`,
//!   `drawing.open`) and every later call passes: the protocol keeps no
//!   session.
//! - A command's refusal is a tool result with `isError`, its code and the
//!   field it is about, so the model can correct itself; an unknown tool or
//!   a malformed request is a JSON-RPC error.
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod server;
mod tools;

pub use server::{LEGACY, MODERN, NAME, Server};
