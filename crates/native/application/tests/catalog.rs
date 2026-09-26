//! The desktop runs exactly the catalog's `desktop` commands (docs/adr/0013,
//! 0022; TODOS.md CMD-03): a command added to the catalog for the desktop
//! without a handler here, or a handler without its catalog entry, fails.
//! The server (`kentos-application`, tests/catalog.rs) and the web
//! (apps/web/src/product/registry.test.ts) are held to theirs the same way.
use std::collections::BTreeSet;

use kentos_contracts::{CommandHost, catalog};
use kentos_native_application::DESKTOP_COMMANDS;

#[test]
fn the_desktop_runs_exactly_the_catalog_commands_marked_desktop() {
    let listed: BTreeSet<(String, u32)> = catalog()
        .commands
        .into_iter()
        .filter(|d| d.hosts.contains(&CommandHost::Desktop))
        .map(|d| (d.id, d.version))
        .collect();
    let handled: BTreeSet<(String, u32)> = DESKTOP_COMMANDS
        .iter()
        .map(|(name, version)| ((*name).to_string(), *version))
        .collect();
    assert_eq!(
        handled.len(),
        DESKTOP_COMMANDS.len(),
        "a command listed twice"
    );
    assert_eq!(listed, handled);
}
