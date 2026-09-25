import type { V1Identities } from '../contracts/generated/V1Identities';

/**
 * The formats WASM module (crates/wasm/formats-wasm → src/io/pkg, built by
 * `pnpm wasm`) loaded in this process, for tests: the page runs the same
 * module in its worker (client.ts, formatsWorker.ts). Null when the package
 * has not been built.
 */

interface FormatsModule {
  initSync(o: { module: BufferSource }): unknown;
  v1Identities(text: string): Uint8Array;
}

const glue = Object.values(import.meta.glob<FormatsModule>('./pkg/kentos_formats_wasm.js'))[0];
const fs = (globalThis as unknown as { process?: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process?.getBuiltinModule('node:fs');

let loaded: Promise<FormatsModule> | null = null;

/** Whether the module can be loaded here (built, and a Node test run). */
export const formatsBuilt = !!glue && !!fs;

function load(): Promise<FormatsModule> {
  return (loaded ??= glue!().then((m) => {
    m.initSync({ module: fs!.readFileSync(new URL('./pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
    return m;
  }));
}

/** `FormatsClient.v1Identities` without the worker. */
export async function v1IdentitiesInProcess(text: string): Promise<V1Identities> {
  const m = await load();
  return JSON.parse(new TextDecoder().decode(m.v1Identities(text))) as V1Identities;
}
