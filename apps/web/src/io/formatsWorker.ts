import init, {
  decodeKcad,
  encodeKcad,
  formatsVersion,
  readCoords,
  readDxf,
  readGeoJson,
  readShapefile,
  v1Identities,
  writeCoords,
  writeDxf,
  writeGeoJson,
} from './pkg/kentos_formats_wasm.js';
import wasmUrl from './pkg/kentos_formats_wasm_bg.wasm?url';
import { KcadError, decodeWith, encodeWith, transferables, type KcadProgress } from './kcad';
import type { FormatsReply, FormatsRequest } from './protocol';
import { FORMATS_VERSION } from './version';

/**
 * Entry of the formats Web Worker (started by client.ts the first time a
 * file is imported or exported, or a drawing opened or saved). It loads the
 * Rust formats module (crates/wasm/formats-wasm) on its first message;
 * parsing a large file, and writing and checking a `.kcad` v2 (io/kcad.ts),
 * never block the page. One request at a time, in order. A drawing comes and
 * goes as typed columns whose buffers are transferred (io/columns.ts); a
 * long KCAD read or write posts its progress as it goes. The page stops one
 * by ending the worker: a call into the module cannot be interrupted.
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

/** Posts a request's progress to the page. */
const progress = (id: number) => (p: KcadProgress) => scope.postMessage({ id, progress: p });

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
      if (m.op === 'readCoords' || m.op === 'readDxf' || m.op === 'readGeoJson') {
        const read = m.op === 'readCoords' ? readCoords : m.op === 'readDxf' ? readDxf : readGeoJson;
        const json = own(read(new Uint8Array(m.bytes), JSON.stringify(m.options)));
        scope.postMessage({ id: m.id, ok: true, json }, [json]);
      } else if (m.op === 'readShapefile') {
        const f = m.files;
        const part = (b?: ArrayBuffer) => (b ? new Uint8Array(b) : undefined);
        const json = own(readShapefile(new Uint8Array(f.shp), part(f.shx), part(f.dbf), part(f.prj), part(f.cpg), JSON.stringify(m.options)));
        scope.postMessage({ id: m.id, ok: true, json }, [json]);
      } else if (m.op === 'v1Identities') {
        const json = own(v1Identities(m.text));
        scope.postMessage({ id: m.id, ok: true, json }, [json]);
      } else if (m.op === 'encodeKcad') {
        const buffer = own(encodeWith(kcad, m.drawing, progress(m.id)));
        scope.postMessage({ id: m.id, ok: true, kcad: buffer }, [buffer]);
      } else if (m.op === 'decodeKcad') {
        const drawing = decodeWith(kcad, new Uint8Array(m.bytes), progress(m.id));
        scope.postMessage({ id: m.id, ok: true, drawing }, transferables(drawing.columns));
      } else {
        const input = JSON.stringify(m.input);
        const written = m.op === 'writeCoords' ? writeCoords(input) : m.op === 'writeGeoJson' ? writeGeoJson(input) : writeDxf(input);
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
