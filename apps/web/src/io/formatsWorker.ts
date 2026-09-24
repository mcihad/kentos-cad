import init, { formatsVersion, readCoords, readDxf, writeCoords, writeDxf } from './pkg/kentos_formats_wasm.js';
import wasmUrl from './pkg/kentos_formats_wasm_bg.wasm?url';
import type { FormatsReply, FormatsRequest } from './protocol';
import { FORMATS_VERSION } from './version';

/**
 * Entry of the formats Web Worker (started by client.ts the first time a
 * file is imported or exported). It loads the Rust formats module
 * (crates/wasm/formats-wasm) on its first message; parsing a large file never
 * blocks the page. One request at a time, in order.
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
      scope.postMessage({ id: m.id, ok: false, fatal, message: fatal ? `Dosya biçimi modülü beklenmedik biçimde durdu (${message(err)}). Yeniden deneyin; sürerse dosyayla birlikte bildirin.` : message(err) });
    }
  });
};
