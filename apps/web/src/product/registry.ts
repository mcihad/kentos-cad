import type { ProductCommand } from './command';
import { lineCreate } from './lineCreate';
import { polygonCreate } from './polygonCreate';
import { polylineCreate } from './polylineCreate';

/**
 * The product commands the web runs (docs/adr/0013, 0022, 0027).
 * registry.test.ts keeps the list equal to the catalog's commands marked
 * `web`, as the desktop's `DESKTOP_COMMANDS` and the server's
 * `SERVER_COMMANDS` are kept to theirs: a command in the catalog without a
 * handler here, or a handler without its catalog entry, fails there.
 */
export const WEB_COMMANDS: readonly ProductCommand<never, unknown, unknown>[] = [polygonCreate, lineCreate, polylineCreate];

/** The handler of a command id and version; undefined for one the web does not run (never guessed). */
export function findProductCommand(id: string, version: number): ProductCommand<never, unknown, unknown> | undefined {
  return WEB_COMMANDS.find((c) => c.id === id && c.version === version);
}
