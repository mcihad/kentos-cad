import { describe, expect, it } from 'vitest';
import catalogText from '../contracts/generated/commandCatalog.json?raw';
import type { CommandCatalog } from '../contracts/generated/CommandCatalog';
import { WEB_COMMANDS } from './registry';

/**
 * The web runs exactly the catalog's `web` commands (docs/adr/0013, 0022;
 * TODOS.md CMD-03): a command the catalog marks for the web without a
 * handler here, or a handler the catalog does not list for the web, fails.
 * The desktop (crates/native/application/tests/catalog.rs) and the server
 * (crates/server/application/tests/catalog.rs) are held to theirs the same way.
 */
describe('web product command registry', () => {
  const catalog = JSON.parse(catalogText) as CommandCatalog;
  const key = (id: string, version: number) => `${id} v${version}`;

  it('runs exactly the catalog commands marked web', () => {
    const listed = catalog.commands.filter((d) => d.hosts.includes('web')).map((d) => key(d.id, d.version));
    const handled = WEB_COMMANDS.map((c) => key(c.id, c.version));
    expect(new Set(handled).size, 'bir komut iki kez kayıtlı').toBe(handled.length);
    expect([...handled].sort()).toEqual([...listed].sort());
  });
});
