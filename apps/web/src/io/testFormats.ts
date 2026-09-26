import type { V1Identities } from '../contracts/generated/V1Identities';
import { decodeWith, encodeWith, type KcadCodec, type KcadModule } from './kcad';

/**
 * The formats WASM module (crates/wasm/formats-wasm → src/io/pkg, built by
 * `pnpm wasm`) loaded in this process, for tests: the page runs the same
 * module in its worker (client.ts, formatsWorker.ts), with the same
 * surroundings (io/kcad.ts). Null when the package has not been built.
 */

interface FormatsModule extends KcadModule {
  initSync(o: { module: BufferSource }): unknown;
  v1Identities(text: string): Uint8Array;
}

const glue = Object.values(import.meta.glob<FormatsModule>('./pkg/kentos_formats_wasm.js'))[0];
const fs = (globalThis as unknown as { process?: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process?.getBuiltinModule('node:fs');

let loaded: Promise<FormatsModule> | null = null;

/** Whether the module can be loaded here (built, and a Node test run). */
export const formatsBuilt = !!glue && !!fs;

/** The module itself (its KCAD calls and the rest), loaded once. */
export function formatsModule(): Promise<FormatsModule> {
  return (loaded ??= glue!().then((m) => {
    m.initSync({ module: fs!.readFileSync(new URL('./pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
    return m;
  }));
}

/** `FormatsClient.v1Identities` without the worker. */
export async function v1IdentitiesInProcess(text: string): Promise<V1Identities> {
  const m = await formatsModule();
  return JSON.parse(new TextDecoder().decode(m.v1Identities(text))) as V1Identities;
}

/**
 * The `.kcad` v2 codec as `DocumentFiles` takes it, without the worker: the
 * worker's own code (io/kcad.ts) around the same module. `encode` works
 * before it returns, as the worker client hands the drawing over before it returns.
 */
export async function kcadInProcess(): Promise<KcadCodec> {
  const m = await formatsModule();
  return {
    encode(drawing, progress) {
      try {
        return Promise.resolve(encodeWith(m, drawing, progress));
      } catch (e) {
        return Promise.reject(e);
      }
    },
    decode: async (bytes, progress) => decodeWith(m, bytes, progress),
  };
}
