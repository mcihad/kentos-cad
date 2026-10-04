import { KcadError, decodeWith, encodeWith, transferables, type KcadProgress } from './kcad';
import type { FormatsReply, FormatsRequest } from './protocol';
import { FORMATS_VERSION } from './version';

/**
 * Entry of the formats Web Worker (started by client.ts the first time a
 * file is imported or exported, or a drawing opened or saved). The Rust
 * modules load with the first message that needs them, each once: the
 * formats module (crates/wasm/formats-wasm) for coordinate lists, GIS files
 * and `.kcad`, the DXF module (crates/wasm/dxf-wasm) the first time a DXF is
 * read or written, the NCZ module (crates/wasm/ncz-wasm) the first time a
 * Netcad drawing is imported (docs/adr/0138, CLAUDE.md §20): a user who never
 * imports one never downloads its reader. Parsing a large file, and writing
 * and checking a `.kcad` v2 (io/kcad.ts), never block the page. One request
 * at a time, in order. A drawing comes and goes as typed columns whose
 * buffers are transferred (io/columns.ts); a long KCAD read or write, and a
 * drawing's import, post their progress as they go. The page stops one by
 * ending the worker: a call into a module cannot be interrupted.
 */
const scope = self as unknown as {
  onmessage: ((e: MessageEvent<FormatsRequest>) => void) | null;
  postMessage(m: FormatsReply, transfer?: Transferable[]): void;
};

type FormatsModule = typeof import('./pkg/kentos_formats_wasm.js');
type DxfModule = typeof import('./dxf/pkg/kentos_dxf_wasm.js');
type NczModule = typeof import('./ncz/pkg/kentos_ncz_wasm.js');

/** A module's code and its WebAssembly, fetched together and started once; a failed start is tried again next time. */
function loader<M>(name: string, load: () => Promise<{ module: M; url: string; init(url: string): Promise<unknown>; version(): number }>): { get(): Promise<M>; readonly name: string } {
  let ready: Promise<M> | null = null;
  return {
    name,
    get: () =>
      (ready ??= load()
        .then(async (m) => {
          await m.init(m.url);
          const v = m.version();
          if (v !== FORMATS_VERSION) throw new Error(`${name} sürüm ${v}, uygulama ${FORMATS_VERSION} bekliyor; modülü yeniden derleyin (pnpm wasm)`);
          return m.module;
        })
        .catch((e: unknown) => {
          ready = null;
          throw e;
        })),
  };
}

const formats = loader<FormatsModule>('Dosya biçimi modülü', async () => {
  const [module, wasm] = await Promise.all([import('./pkg/kentos_formats_wasm.js'), import('./pkg/kentos_formats_wasm_bg.wasm?url')]);
  return { module, url: wasm.default, init: (url) => module.default({ module_or_path: url }), version: () => module.formatsVersion() };
});

const dxf = loader<DxfModule>('DXF modülü', async () => {
  const [module, wasm] = await Promise.all([import('./dxf/pkg/kentos_dxf_wasm.js'), import('./dxf/pkg/kentos_dxf_wasm_bg.wasm?url')]);
  return { module, url: wasm.default, init: (url) => module.default({ module_or_path: url }), version: () => module.dxfVersion() };
});

const ncz = loader<NczModule>('NCZ modülü', async () => {
  const [module, wasm] = await Promise.all([import('./ncz/pkg/kentos_ncz_wasm.js'), import('./ncz/pkg/kentos_ncz_wasm_bg.wasm?url')]);
  return { module, url: wasm.default, init: (url) => module.default({ module_or_path: url }), version: () => module.nczVersion() };
});

/** The bytes wasm-bindgen returned, as a buffer that can be transferred whole. */
const own = (a: Uint8Array): ArrayBuffer => (a.byteOffset === 0 && a.byteLength === a.buffer.byteLength ? a.buffer : a.slice().buffer) as ArrayBuffer;

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Posts a request's progress to the page. */
const progress = (id: number) => (p: KcadProgress) => scope.postMessage({ id, progress: p });

/** A reader's progress (thousandths), posted at most every so often and once at the end. */
const PROGRESS_MS = 50;
function readProgress(id: number): { step(done: number, total: number): void } {
  let last = -Infinity;
  return {
    step(done, total) {
      const now = performance.now();
      if (done < total && now - last < PROGRESS_MS) return;
      last = now;
      scope.postMessage({ id, progress: { stage: 'reading', done, total } });
    },
  };
}

/** What the DXF and NCZ readers give back, as the page takes it. */
interface Imported {
  readonly ok: boolean;
  readonly message: string;
  takeHead(): string;
  takeKinds(): Uint8Array;
  takeUids(): Uint8Array;
  takeInts(): Uint32Array;
  takeFloats(): Float64Array;
  takeText(): Uint16Array;
  takeTextLengths(): Uint32Array;
  free(): void;
}

/** Posts an imported drawing: its head as JSON, its objects as columns whose buffers go to the page. */
function postImported(id: number, r: Imported): void {
  try {
    if (!r.ok) {
      scope.postMessage({ id, ok: false, fatal: false, message: r.message });
      return;
    }
    const columns = { kinds: r.takeKinds(), uids: r.takeUids(), ints: r.takeInts(), floats: r.takeFloats(), text: r.takeText(), textLengths: r.takeTextLengths() };
    scope.postMessage({ id, ok: true, imported: { head: r.takeHead(), columns } }, transferables(columns));
  } finally {
    r.free();
  }
}

/** The module a request needs. */
const moduleFor = (op: FormatsRequest['op']) => (op === 'readDxf' || op === 'writeDxf' ? dxf : op === 'readNcz' ? ncz : formats);

let queue = Promise.resolve();

scope.onmessage = (e) => {
  const m = e.data;
  queue = queue.then(async () => {
    const needed = moduleFor(m.op);
    try {
      await needed.get();
    } catch (err) {
      scope.postMessage({ id: m.id, ok: false, fatal: true, message: `${needed.name} yüklenemedi: ${message(err)}.` });
      return;
    }
    try {
      if (m.op === 'readDxf') {
        postImported(m.id, (await dxf.get()).readDxf(new Uint8Array(m.bytes), JSON.stringify(m.options), readProgress(m.id)));
      } else if (m.op === 'readNcz') {
        postImported(m.id, (await ncz.get()).readNcz(new Uint8Array(m.bytes), JSON.stringify(m.options), readProgress(m.id)));
      } else if (m.op === 'writeDxf') {
        const written = (await dxf.get()).writeDxf(JSON.stringify(m.input));
        try {
          const file = own(written.takeBytes());
          scope.postMessage({ id: m.id, ok: true, file, report: written.report }, [file]);
        } finally {
          written.free();
        }
      } else {
        const f = await formats.get();
        if (m.op === 'readCoords' || m.op === 'readGeoJson' || m.op === 'readFieldCsv') {
          const read = m.op === 'readCoords' ? f.readCoords : m.op === 'readFieldCsv' ? f.readFieldCsv : f.readGeoJson;
          const json = own(read(new Uint8Array(m.bytes), JSON.stringify(m.options)));
          scope.postMessage({ id: m.id, ok: true, json }, [json]);
        } else if (m.op === 'readShapefile') {
          const s = m.files;
          const part = (b?: ArrayBuffer) => (b ? new Uint8Array(b) : undefined);
          const json = own(f.readShapefile(new Uint8Array(s.shp), part(s.shx), part(s.dbf), part(s.prj), part(s.cpg), JSON.stringify(m.options)));
          scope.postMessage({ id: m.id, ok: true, json }, [json]);
        } else if (m.op === 'v1Identities') {
          const json = own(f.v1Identities(m.text));
          scope.postMessage({ id: m.id, ok: true, json }, [json]);
        } else if (m.op === 'encodeKcad') {
          const buffer = own(encodeWith(f, m.drawing, progress(m.id)));
          scope.postMessage({ id: m.id, ok: true, kcad: buffer }, [buffer]);
        } else if (m.op === 'decodeKcad') {
          const drawing = decodeWith(f, new Uint8Array(m.bytes), progress(m.id));
          scope.postMessage({ id: m.id, ok: true, drawing }, transferables(drawing.columns));
        } else {
          const input = JSON.stringify(m.input);
          const written = m.op === 'writeCoords' ? f.writeCoords(input) : f.writeGeoJson(input);
          try {
            const file = own(written.takeBytes());
            scope.postMessage({ id: m.id, ok: true, file, report: written.report }, [file]);
          } finally {
            written.free();
          }
        }
      }
    } catch (err) {
      // A trap is a bug in the module, never the file's fault (it reports bad data as values):
      // the module cannot run again, so the page starts a fresh worker.
      const fatal = err instanceof WebAssembly.RuntimeError;
      const code = err instanceof KcadError ? err.code : undefined;
      scope.postMessage({ id: m.id, ok: false, fatal, ...(code ? { code } : {}), message: fatal ? `${needed.name} beklenmedik biçimde durdu (${message(err)}). Yeniden deneyin; sürerse dosyayla birlikte bildirin.` : message(err) });
    }
  });
};
