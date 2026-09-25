//! The server runs exactly the catalog's `server` commands (docs/adr/0013,
//! TODOS.md CMD-03): a command added to the catalog without a handler, or a
//! handler without a catalog entry, fails here.
use std::collections::BTreeSet;

use kentos_application::SERVER_COMMANDS;
use kentos_contracts::{CommandHost, catalog};

#[test]
fn the_server_runs_exactly_the_catalog_commands_marked_server() {
    let listed: BTreeSet<(String, u32)> = catalog()
        .commands
        .into_iter()
        .filter(|d| d.hosts.contains(&CommandHost::Server))
        .map(|d| (d.id, d.version))
        .collect();
    let handled: BTreeSet<(String, u32)> = SERVER_COMMANDS
        .iter()
        .map(|(name, version)| ((*name).to_string(), *version))
        .collect();
    assert_eq!(listed, handled);
}
