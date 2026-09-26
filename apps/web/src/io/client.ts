import type { CoordRead } from '../contracts/generated/CoordRead';
import type { CoordReadOptions } from '../contracts/generated/CoordReadOptions';
import type { CoordWriteInput } from '../contracts/generated/CoordWriteInput';
import type { DxfReadOptions } from '../contracts/generated/DxfReadOptions';
import type { DxfWriteInput } from '../contracts/generated/DxfWriteInput';
import type { ImportResult } from '../contracts/generated/ImportResult';
import type { ExportReport } from '../contracts/generated/ExportReport';
import type { V1Identities } from '../contracts/generated/V1Identities';
import type { PackedDrawing } from './columns';
import { KcadError, transferables, type KcadProgress } from './kcad';
import type { FormatsReply, FormatsRequest } from './protocol';

/**
 * The page's side of the formats worker. The worker, and the Rust formats
 * module in it, start with the first request (an import, an export, a
 * drawing opened; never at start-up, CLAUDE.md §20) and stop after a quiet
 * half minute, so the memory a large file took is given back; after a large
 * drawing was read or written they stop at once (the module's memory never
 * shrinks while it lives, docs/adr/0030). A trap in the module or a worker
 * that fails to load fails the waiting requests with a Turkish message; the
 * next request starts a fresh worker. `cancel` ends the worker and fails
 * what waits with the code `cancelled`.
 */

/** The part of a Worker the client uses (a fake in tests). */
export interface WorkerLike {
  postMessage(message: FormatsRequest, transfer: Transferable[]): void;
  terminate(): void;
  onmessage: ((e: { data: FormatsReply }) => void) | null;
  onerror: ((e: unknown) => void) | null;
}

/** A written file and what the writer did. */
export interface WrittenFile {
  bytes: Uint8Array;
  report: ExportReport;
}

type Ok = Extract<FormatsReply, { ok: true }>;
type Request = FormatsRequest extends infer R ? (R extends FormatsRequest ? Omit<R, 'id'> : never) : never;

/** An idle worker is stopped after this long. */
const IDLE_MS = 30_000;
/** A drawing this large (bytes) was read or written: the worker stops as soon as it is idle. */
const LARGE = 16 << 20;

const decoder = new TextDecoder();

export class FormatsClient {
  private worker: WorkerLike | null = null;
  private seq = 0;
  private readonly pending = new Map<number, { resolve(r: Ok): void; reject(e: Error): void; progress?: (p: KcadProgress) => void }>();
  private idle: ReturnType<typeof setTimeout> | null = null;
  /** A large drawing went through since the worker was last idle. */
  private large = false;
  private readonly spawn: () => WorkerLike;

  constructor(spawn: () => WorkerLike = spawnWorker) {
    this.spawn = spawn;
  }

  /** Reads a coordinate list; the caller keeps `bytes` (the worker gets a copy). */
  async readCoords(bytes: Uint8Array, options: CoordReadOptions): Promise<CoordRead> {
    const copy = bytes.slice().buffer;
    return json<CoordRead>(await this.request({ op: 'readCoords', bytes: copy, options }, [copy]));
  }

  /**
   * Reads a DXF file. A large file is not copied: when `bytes` spans its
   * whole buffer, the buffer is handed over to the worker and `bytes` is
   * left empty (the caller no longer needs it); a view into a larger buffer
   * sends a copy of its own bytes.
   */
  async readDxf(bytes: Uint8Array, options: DxfReadOptions): Promise<ImportResult> {
    const whole = bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength && bytes.buffer instanceof ArrayBuffer;
    const buffer = whole ? (bytes.buffer as ArrayBuffer) : bytes.slice().buffer;
    return json<ImportResult>(await this.request({ op: 'readDxf', bytes: buffer, options }, [buffer]));
  }

  async writeCoords(input: CoordWriteInput): Promise<WrittenFile> {
    return written(await this.request({ op: 'writeCoords', input }, []));
  }

  /** Writes a DXF (AutoCAD 2007) of the objects and their layers; the input goes as a structured copy. */
  async writeDxf(input: DxfWriteInput): Promise<WrittenFile> {
    return written(await this.request({ op: 'writeDxf', input }, []));
  }

  /**
   * The persistent ids of a v1 drawing's objects (docs/adr/0014), from its
   * text: the Rust contracts read it and derive them, the same wherever the
   * drawing is opened. The text goes as a copy; the page reads its own
   * meanwhile. A drawing the contract cannot read rejects with the reason.
   */
  async v1Identities(text: string): Promise<V1Identities> {
    return json<V1Identities>(await this.request({ op: 'v1Identities', text }, []));
  }

  /**
   * A packed drawing's `.kcad` v2 bytes (docs/specs/kcad-v2.md), made and
   * checked in the worker (io/kcad.ts): read back to the same drawing before
   * they come here. The drawing's buffers go to the worker (they are its now).
   * Rejects with the reason (and a KCAD code) when the drawing cannot be
   * written; `progress` hears the objects written and the reading back.
   */
  async encodeKcad(drawing: PackedDrawing, progress?: (p: KcadProgress) => void): Promise<Uint8Array<ArrayBuffer>> {
    if (drawing.columns.floats.byteLength + drawing.columns.text.byteLength > LARGE) this.large = true;
    const r = await this.request({ op: 'encodeKcad', drawing }, transferables(drawing.columns), progress);
    if (!('kcad' in r)) throw new Error('Dosya biçimi modülü beklenmeyen bir yanıt verdi.');
    return new Uint8Array(r.kcad);
  }

  /**
   * The drawing in a `.kcad` v2 file, packed (io/columns.ts). A large file is
   * not copied: when `bytes` spans its whole buffer, the buffer goes to the
   * worker and `bytes` is left empty. A file the reader refuses rejects with
   * a KcadError (the specification's code and a Turkish message); `progress`
   * hears the integrity check, the project and the objects as they are read.
   */
  async decodeKcad(bytes: Uint8Array, progress?: (p: KcadProgress) => void): Promise<PackedDrawing> {
    if (bytes.byteLength > LARGE) this.large = true;
    const whole = bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength && bytes.buffer instanceof ArrayBuffer;
    const buffer = whole ? (bytes.buffer as ArrayBuffer) : bytes.slice().buffer;
    const r = await this.request({ op: 'decodeKcad', bytes: buffer }, [buffer], progress);
    if (!('drawing' in r)) throw new Error('Dosya biçimi modülü beklenmeyen bir yanıt verdi.');
    return r.drawing;
  }

  /**
   * Stops the worker now (a dialog closed while its file was being read, an
   * open cancelled): what waits fails with the code `cancelled`, and the
   * memory the worker took goes back.
   */
  cancel(): void {
    this.stop('İşlem durduruldu.', 'cancelled');
  }

  private request(m: Request, transfer: Transferable[], progress?: (p: KcadProgress) => void): Promise<Ok> {
    return new Promise((resolve, reject) => {
      const id = ++this.seq;
      this.pending.set(id, { resolve, reject, progress });
      if (this.idle) clearTimeout(this.idle);
      this.idle = null;
      try {
        this.ensure().postMessage({ ...m, id } as FormatsRequest, transfer);
      } catch (err) {
        this.pending.delete(id);
        reject(new Error(`Dosya biçimi modülüne gönderilemedi: ${(err as Error).message}`));
      }
    });
  }

  private ensure(): WorkerLike {
    if (this.worker) return this.worker;
    const w = this.spawn();
    w.onmessage = (e) => {
      const r = e.data;
      const p = this.pending.get(r.id);
      if (!p) return;
      if ('progress' in r) return p.progress?.(r.progress);
      this.pending.delete(r.id);
      if (r.ok) p.resolve(r);
      else {
        p.reject(r.code ? new KcadError(r.code, r.message) : new Error(r.message));
        if (r.fatal) return this.stop(r.message);
      }
      if (this.pending.size || !this.worker) return;
      // A large drawing left the module's memory at its high-water mark: give it back now.
      if (this.large) return this.stop(null);
      this.idle = setTimeout(() => this.stop(null), IDLE_MS);
    };
    w.onerror = () => this.stop('Dosya biçimi modülü yüklenemedi ya da beklenmedik biçimde durdu. Bağlantınızı denetleyip yeniden deneyin.');
    this.worker = w;
    return w;
  }

  private stop(message: string | null, code?: string): void {
    this.worker?.terminate();
    this.worker = null;
    this.large = false;
    if (this.idle) clearTimeout(this.idle);
    this.idle = null;
    const text = message ?? 'İşlem durduruldu.';
    for (const p of this.pending.values()) p.reject(code ? new KcadError(code, text) : new Error(text));
    this.pending.clear();
  }
}

function json<T>(r: Ok): T {
  if (!('json' in r)) throw new Error('Dosya biçimi modülü beklenmeyen bir yanıt verdi.');
  return JSON.parse(decoder.decode(r.json)) as T;
}

function written(r: Ok): WrittenFile {
  if (!('file' in r)) throw new Error('Dosya biçimi modülü beklenmeyen bir yanıt verdi.');
  return { bytes: new Uint8Array(r.file), report: JSON.parse(r.report) as ExportReport };
}

function spawnWorker(): WorkerLike {
  return new Worker(new URL('./formatsWorker.ts', import.meta.url), { type: 'module', name: 'KentOS dosya biçimleri' }) as unknown as WorkerLike;
}

let shared: FormatsClient | null = null;

/** The page's formats client: one worker for every import and export. */
export const formats = (): FormatsClient => (shared ??= new FormatsClient());
