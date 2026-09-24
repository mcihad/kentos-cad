import type { CoordRead } from '../contracts/generated/CoordRead';
import type { CoordReadOptions } from '../contracts/generated/CoordReadOptions';
import type { CoordWriteInput } from '../contracts/generated/CoordWriteInput';
import type { ExportReport } from '../contracts/generated/ExportReport';
import type { FormatsReply, FormatsRequest } from './protocol';

/**
 * The page's side of the formats worker. The worker, and the Rust formats
 * module in it, start with the first request (never at start-up, CLAUDE.md
 * §20) and stop after a quiet half minute, so the memory a large file took
 * is given back. A trap in the module or a worker that fails to load fails
 * the waiting requests with a Turkish message; the next request starts a
 * fresh worker.
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

const decoder = new TextDecoder();

export class FormatsClient {
  private worker: WorkerLike | null = null;
  private seq = 0;
  private readonly pending = new Map<number, { resolve(r: Ok): void; reject(e: Error): void }>();
  private idle: ReturnType<typeof setTimeout> | null = null;
  private readonly spawn: () => WorkerLike;

  constructor(spawn: () => WorkerLike = spawnWorker) {
    this.spawn = spawn;
  }

  /** Reads a coordinate list; the caller keeps `bytes` (the worker gets a copy). */
  async readCoords(bytes: Uint8Array, options: CoordReadOptions): Promise<CoordRead> {
    const copy = bytes.slice().buffer;
    return json<CoordRead>(await this.request({ op: 'readCoords', bytes: copy, options }, [copy]));
  }

  async writeCoords(input: CoordWriteInput): Promise<WrittenFile> {
    return written(await this.request({ op: 'writeCoords', input }, []));
  }

  /** Stops the worker now (a dialog closed while its file was being read). */
  cancel(): void {
    this.stop('İşlem durduruldu.');
  }

  private request(m: Request, transfer: Transferable[]): Promise<Ok> {
    return new Promise((resolve, reject) => {
      const id = ++this.seq;
      this.pending.set(id, { resolve, reject });
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
      this.pending.delete(r.id);
      if (r.ok) p.resolve(r);
      else {
        p.reject(new Error(r.message));
        if (r.fatal) return this.stop(r.message);
      }
      if (!this.pending.size && this.worker) this.idle = setTimeout(() => this.stop(null), IDLE_MS);
    };
    w.onerror = () => this.stop('Dosya biçimi modülü yüklenemedi ya da beklenmedik biçimde durdu. Bağlantınızı denetleyip yeniden deneyin.');
    this.worker = w;
    return w;
  }

  private stop(message: string | null): void {
    this.worker?.terminate();
    this.worker = null;
    if (this.idle) clearTimeout(this.idle);
    this.idle = null;
    for (const p of this.pending.values()) p.reject(new Error(message ?? 'İşlem durduruldu.'));
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
