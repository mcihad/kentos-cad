import init, { decodeKcad, encodeKcad, formatsVersion, readCoords, readDxf, v1Identities, writeCoords, writeDxf } from './pkg/kentos_formats_wasm.js';
import wasmUrl from './pkg/kentos_formats_wasm_bg.wasm?url';
import { KcadError, decodeWith, encodeWith } from './kcad';
import type { FormatsReply, FormatsRequest } from './protocol';
import { FORMATS_VERSION } from './version';

/**
 * Entry of the formats Web Worker (started by client.ts the first time a
 * file is imported or exported, or a drawing opened or saved). It loads the
 * Rust formats module (crates/wasm/formats-wasm) on its first message;
 * parsing a large file, and writing and checking a `.kcad` v2 (io/kcad.ts),
 * never block the page. One request at a time, in order.
 */
const scope = self as unknown as {
  onmessage: ((e: MessageEvent<FormatsRequest>) => void) | null;
  postMessage(m: FormatsReply, transfer?: Transferable[]): void;
};

let ready: Promise<void> | null = null;
const start = () =>
  (ready ??= init({ module_or_path: wasmUrl }).then(() => {
    const v = formatsVersion();
    if (v !== FORMATS_VERSION) throw new Error(`dosya biçimi modülü sürüm ${v}, uygulama ${FORMATS_VERSION} bekliyor; modülü yeniden derleyin (pnpm wasm)`);
  }));

/** The bytes wasm-bindgen returned, as a buffer that can be transferred whole. */
const own = (a: Uint8Array): ArrayBuffer => (a.byteOffset === 0 && a.byteLength === a.buffer.byteLength ? a.buffer : a.slice().buffer) as ArrayBuffer;

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

const kcad = { encodeKcad, decodeKcad };

let queue = Promise.resolve();

scope.onmessage = (e) => {
  const m = e.data;
  queue = queue.then(async () => {
    try {
      await start();
    } catch (err) {
      ready = null;
      scope.postMessage({ id: m.id, ok: false, fatal: true, message: `Dosya biçimi modülü yüklenemedi: ${message(err)}.` });
      return;
    }
    try {
      if (m.op === 'readCoords' || m.op === 'readDxf') {
        const read = m.op === 'readCoords' ? readCoords : readDxf;
        const json = own(read(new Uint8Array(m.bytes), JSON.stringify(m.options)));
        scope.postMessage({ id: m.id, ok: true, json }, [json]);
      } else if (m.op === 'v1Identities') {
        const json = own(v1Identities(m.text));
        scope.postMessage({ id: m.id, ok: true, json }, [json]);
      } else if (m.op === 'encodeKcad') {
        const { bytes, dropped } = encodeWith(kcad, m.snapshot);
        const buffer = own(bytes);
        scope.postMessage({ id: m.id, ok: true, kcad: buffer, dropped }, [buffer]);
      } else if (m.op === 'decodeKcad') {
        scope.postMessage({ id: m.id, ok: true, snapshot: decodeWith(kcad, new Uint8Array(m.bytes)) });
      } else {
        const written = m.op === 'writeCoords' ? writeCoords(JSON.stringify(m.input)) : writeDxf(JSON.stringify(m.input));
        try {
          const file = own(written.takeBytes());
          scope.postMessage({ id: m.id, ok: true, file, report: written.report }, [file]);
        } finally {
          written.free();
        }
      }
    } catch (err) {
      // A trap is a bug in the module, never the file's fault (it reports bad data as values):
      // the module cannot run again, so the page starts a fresh worker.
      const fatal = err instanceof WebAssembly.RuntimeError;
      const code = err instanceof KcadError ? err.code : undefined;
      scope.postMessage({ id: m.id, ok: false, fatal, ...(code ? { code } : {}), message: fatal ? `Dosya biçimi modülü beklenmedik biçimde durdu (${message(err)}). Yeniden deneyin; sürerse dosyayla birlikte bildirin.` : message(err) });
    }
  });
};
