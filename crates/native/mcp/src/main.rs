//! `kentos-mcp`: KentOS's MCP server over stdio (docs/adr/0133). One
//! JSON-RPC message per line in, one answer per line out; what is not the
//! protocol goes to standard error.
//!
//!   claude mcp add kentos -- target/debug/kentos-mcp

use std::io::{BufRead, Write};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use kentos_mcp::{Profile, Server};
use serde_json::{Value, json};

/// How often the server looks, between two messages, for what a subscriber asked to hear (the installed extensions).
const LOOK: Duration = Duration::from_secs(2);

fn main() {
    // The v2 surface unless `--profile full` or KENTOS_MCP_PROFILE=full asks for every command a tool (docs/adr/8056 §2).
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env = std::env::var("KENTOS_MCP_PROFILE").ok();
    let profile = match Profile::chosen(&args, env.as_deref()) {
        Ok(profile) => profile,
        Err(why) => {
            eprintln!("{why}");
            std::process::exit(2);
        }
    };
    let mut server = Server::with_profile(profile);
    // Standard input on a thread of its own: between two messages the server tells its subscribers what changed.
    let (lines, incoming) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if lines.send(line).is_err() {
                break;
            }
        }
    });
    let mut stdout = std::io::stdout().lock();
    loop {
        let mut out: Vec<Value> = Vec::new();
        match incoming.recv_timeout(LOOK) {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => out.extend(match serde_json::from_str::<Value>(&line) {
                Ok(message) => server.handle(&message),
                Err(e) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {e}") }
                })),
            }),
            Err(RecvTimeoutError::Timeout) => {}
            // Standard input closed: the client is gone.
            Err(RecvTimeoutError::Disconnected) => break,
        }
        out.extend(server.notices());
        let written = out
            .iter()
            .try_for_each(|message| writeln!(stdout, "{message}"))
            .and_then(|()| stdout.flush());
        if written.is_err() {
            break;
        }
    }
}
