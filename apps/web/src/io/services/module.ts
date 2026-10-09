import type * as Pkg from './pkg/kentos_services_wasm';

/**
 * The map services' module (docs/adr/0208; crates/wasm/services-wasm), fetched and started the first time a drawing
 * shows a service or a services window opens (CLAUDE.md §20), in the page and in its services worker. A failed load is
 * not kept: the next ask tries again.
 */
export type ServicesModule = typeof Pkg;

let loading: Promise<ServicesModule> | null = null;

export function loadServices(): Promise<ServicesModule> {
  return (loading ??= import('./pkg/kentos_services_wasm')
    .then(async (m) => {
      await m.default();
      return m;
    })
    .catch((e: unknown) => {
      loading = null;
      throw e;
    }));
}
