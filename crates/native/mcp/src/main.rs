//! `kentos-mcp`: KentOS's MCP server over stdio (docs/adr/0133). One
//! JSON-RPC message per line in, one answer per line out; what is not the
//! protocol goes to standard error.
//!
//!   claude mcp add kentos -- target/debug/kentos-mcp

use std::io::{BufRead, Write};

use kentos_mcp::Server;
use serde_json::{Value, json};

fn main() {
    let mut server = Server::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(&line) {
            Ok(message) => server.handle(&message),
            Err(e) => Some(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("Parse error: {e}") }
            })),
        };
        if let Some(answer) = answer {
            let written = writeln!(stdout, "{answer}").and_then(|()| stdout.flush());
            if written.is_err() {
                break;
            }
        }
    }
}
