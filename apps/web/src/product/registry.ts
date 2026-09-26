import { arcCreate } from './arcCreate';
import { circleCreate } from './circleCreate';
import type { ProductCommand } from './command';
import { entitiesDelete } from './entitiesDelete';
import { entitiesTransform } from './entitiesTransform';
import { lineCreate } from './lineCreate';
import { pointCreate } from './pointCreate';
import { polygonCreate } from './polygonCreate';
import { polylineCreate } from './polylineCreate';

/**
 * The product commands the web runs (docs/adr/0013, 0022, 0027, 0029, 0032, 0037).
 * registry.test.ts keeps the list equal to the catalog's commands marked
 * `web`, as the desktop's `DESKTOP_COMMANDS` and the server's
 * `SERVER_COMMANDS` are kept to theirs: a command in the catalog without a
 * handler here, or a handler without its catalog entry, fails there.
 */
export const WEB_COMMANDS: readonly ProductCommand<never, unknown, unknown>[] = [polygonCreate, lineCreate, polylineCreate, entitiesDelete, pointCreate, circleCreate, arcCreate, entitiesTransform];

/** The handler of a command id and version; undefined for one the web does not run (never guessed). */
export function findProductCommand(id: string, version: number): ProductCommand<never, unknown, unknown> | undefined {
  return WEB_COMMANDS.find((c) => c.id === id && c.version === version);
}
