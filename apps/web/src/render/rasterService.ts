import { Signal } from '../core/signal';
import type { BandStats, RasterInspected, RasterReply, RasterRequest } from '../io/rasterProtocol';

/**
 * The rasters' tiles on the web (docs/adr/0204 §3, §5, §11), twin of the
 * desktop's `rasters/tiles.rs`: what the raster pass asks the atlas for
 * (`AtlasSource.rasterTile`) made in two raster workers (io/rasterWorker.ts),
 * a raster always in the same one so that its reader and blocks are read
 * once. A tile asked for waits on the page's queue, nearest the view's
 * centre first (the pass asks so); at most two a worker at a time, so a
 * tile no longer asked for by the next frame is dropped before it is made
 * (the frames' generation). Finished tiles are kept, the least recently
 * used let go first, within a budget; each arrival asks the views for a
 * frame. A linked raster's file is the session's: the browser never knows a
 * path, so Raster ekle (and Kaynağı yeniden seç) give the file by its name;
 * an embedded raster's bytes come from the project's library.
 */

/** Bytes the finished tiles may take (about 240 tiles). */
const TILE_BUDGET = 64 * 1024 * 1024;
/** Requests a worker holds at a time. */
const IN_FLIGHT = 2;
/** The first level a pyramid file holds: such tiles wait while it is made. */
const PYRAMID_FIRST = 3;

interface Want {
  tileKey: string;
  raster: string;
  url: string | null;
  look: string;
  affine: number[];
  level: number;
  tx: number;
  ty: number;
  generation: number;
}

interface Pending {
  resolve(v: { value?: unknown; bytes?: Uint8Array; wait?: boolean }): void;
  reject(e: Error): void;
  progress?(share: number): void;
}

/** A request without its id (the service gives it). */
type Asked = RasterRequest extends infer R ? (R extends { id: number } ? Omit<R, 'id'> : never) : never;

/** A pyramid's pass the panel shows. */
export interface PyramidProgress {
  key: string;
  name: string;
  done: number;
}

/** A data address's bytes. */
function dataBlob(url: string): Blob | null {
  const comma = url.indexOf(',');
  if (comma < 0 || !url.slice(0, comma).endsWith(';base64')) return null;
  try {
    const bin = atob(url.slice(comma + 1));
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return new Blob([out]);
  } catch {
    return null;
  }
}

/** A string's hash, to keep a raster in one worker. */
function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return h >>> 0;
}

export class RasterService {
  /** The session's linked files by the name the raster keeps. */
  private readonly files = new Map<string, File>();
  /** Embedded rasters' bytes by their key, decoded once. */
  private readonly blobs = new Map<string, Blob>();
  private readonly tiles = new Map<string, Uint8Array>();
  private tileBytes = 0;
  private readonly queue: Want[] = [];
  private readonly wanted = new Map<string, Want>();
  private readonly failed = new Set<string>();
  private readonly building = new Set<string>();
  private workers: Worker[] = [];
  private busy: number[] = [];
  private generation = 0;
  private next = 1;
  private readonly pending = new Map<number, Pending>();
  private readonly listeners = new Set<() => void>();
  /** The pyramids being made, for the panel. */
  readonly pyramids = new Signal<readonly PyramidProgress[]>([]);
  /** Raster oturt's resampling, for the panel: its raster's name and share. */
  readonly warping = new Signal<{ name: string; done: number; id: number; stop(): void } | null>(null);

  /** Told when tiles arrive (a view redraws). */
  listen(fn: () => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private changed(): void {
    for (const fn of this.listeners) fn();
  }

  /** The session's file of a linked raster, by the name it keeps. */
  setFile(name: string, file: File): void {
    this.files.set(name, file);
    const key = `file:${name}`;
    this.failed.delete(key);
    this.forgetTiles();
    if (this.workers.length) this.post(hash(key) % 2, { type: 'drop', key });
    this.changed();
  }

  hasFile(name: string): boolean {
    return this.files.has(name);
  }

  /** A raster's bytes: the session's file or the library's data; none for a linked file not given this session. */
  source(raster: string, url: string | null): { blob: Blob; name: string } | null {
    if (raster.startsWith('file:')) {
      const name = raster.slice(5);
      const f = this.files.get(name);
      return f ? { blob: f, name } : null;
    }
    let blob = this.blobs.get(raster) ?? null;
    if (!blob && url) {
      blob = dataBlob(url);
      if (blob) this.blobs.set(raster, blob);
    }
    return blob ? { blob, name: raster.slice(raster.indexOf(':') + 1) } : null;
  }

  private worker(i: number): Worker {
    if (!this.workers.length) {
      for (let k = 0; k < 2; k++) {
        const w = new Worker(new URL('../io/rasterWorker.ts', import.meta.url), { type: 'module', name: `KentOS raster ${k + 1}` });
        w.onmessage = (e: MessageEvent<RasterReply>) => this.heard(e.data);
        this.workers.push(w);
        this.busy.push(0);
      }
    }
    return this.workers[i];
  }

  private post(i: number, m: RasterRequest): void {
    this.worker(i).postMessage(m);
  }

  private heard(r: RasterReply): void {
    if (r.type === 'refresh') {
      this.changed();
      return;
    }
    if (r.type === 'pyramid') {
      const list = this.pyramids.value.filter((p) => p.key !== r.key);
      if (r.done === null) this.building.delete(r.key);
      else {
        this.building.add(r.key);
        list.push({ key: r.key, name: r.name, done: r.done });
      }
      this.pyramids.set(list);
      return;
    }
    const p = this.pending.get(r.id);
    if (r.type === 'progress') {
      p?.progress?.(r.share);
      return;
    }
    this.pending.delete(r.id);
    if (!p) return;
    if (r.error !== undefined) p.reject(new Error(r.error));
    else p.resolve({ value: r.value, bytes: r.bytes, wait: r.wait });
  }

  /** A request to the raster's worker and its answer. */
  private ask(raster: string, m: Asked, progress?: (share: number) => void): Promise<{ value?: unknown; bytes?: Uint8Array; wait?: boolean }> {
    const id = this.next++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject, progress });
      this.post(hash(raster) % 2, { ...m, id } as RasterRequest);
    });
  }

  /** A frame begins: what is not asked for again by its end may be dropped. */
  frame(): void {
    this.generation++;
  }

  /** A tile when made; otherwise it is asked for (null). */
  tile(raster: string, url: string | null, look: string, affine: readonly number[], level: number, tx: number, ty: number): Uint8Array | null {
    const tileKey = `${raster}|${look}|${affine[1]},${affine[2]},${affine[4]},${affine[5]}|${level}|${tx}|${ty}`;
    const kept = this.tiles.get(tileKey);
    if (kept) {
      // Most recently used last.
      this.tiles.delete(tileKey);
      this.tiles.set(tileKey, kept);
      return kept;
    }
    if (this.failed.has(raster) || (level >= PYRAMID_FIRST && this.building.has(raster))) return null;
    const w = this.wanted.get(tileKey);
    if (w) w.generation = this.generation;
    else {
      const want: Want = { tileKey, raster, url, look, affine: [...affine], level, tx, ty, generation: this.generation };
      this.wanted.set(tileKey, want);
      this.queue.push(want);
      this.pump();
    }
    return null;
  }

  /** Sends what the workers can take, the stale left out. */
  private pump(): void {
    for (let i = 0; i < this.queue.length; ) {
      const want = this.queue[i];
      if (want.generation + 1 < this.generation) {
        this.queue.splice(i, 1);
        this.wanted.delete(want.tileKey);
        continue;
      }
      const k = hash(want.raster) % 2;
      this.worker(k);
      if (this.busy[k] >= IN_FLIGHT) {
        i++;
        continue;
      }
      this.queue.splice(i, 1);
      const src = this.source(want.raster, want.url);
      if (!src) {
        this.wanted.delete(want.tileKey);
        continue;
      }
      this.busy[k]++;
      this.ask(want.raster, { type: 'tile', key: want.raster, blob: src.blob, name: src.name, look: want.look, affine: want.affine, level: want.level, tx: want.tx, ty: want.ty })
        .then((r) => {
          if (r.bytes) this.keep(want.tileKey, r.bytes);
          if (r.wait) this.building.add(want.raster);
        })
        .catch(() => this.failed.add(want.raster))
        .finally(() => {
          this.busy[k]--;
          this.wanted.delete(want.tileKey);
          this.pump();
          this.changed();
        });
    }
  }

  private keep(tileKey: string, rgba: Uint8Array): void {
    const old = this.tiles.get(tileKey);
    if (old) this.tileBytes -= old.byteLength;
    this.tiles.set(tileKey, rgba);
    this.tileBytes += rgba.byteLength;
    for (const [k, t] of this.tiles) {
      if (this.tileBytes <= TILE_BUDGET) break;
      this.tiles.delete(k);
      this.tileBytes -= t.byteLength;
    }
  }

  /** Lets every tile go (a raster's look or file changed). */
  forgetTiles(): void {
    this.tiles.clear();
    this.tileBytes = 0;
  }

  /** Raster ekle: the chosen files read (the image and its world file) for the window. */
  async inspect(files: File[], srid: number, confirmed: boolean, view: readonly number[]): Promise<RasterInspected> {
    const r = await this.ask(files[0]?.name ?? '', { type: 'inspect', files, srid, confirmed, view: [...view] });
    return r.value as RasterInspected;
  }

  /** The bands' statistics of a raster (Raster stili); null when it cannot be read. */
  async stats(raster: string, url: string | null): Promise<BandStats[] | null> {
    const src = this.source(raster, url);
    if (!src) return null;
    const r = await this.ask(raster, { type: 'stats', key: raster, blob: src.blob, name: src.name });
    return (r.value as BandStats[] | null) ?? null;
  }

  /** The bands' values at pixel (i, j) (Koordinat oku); null outside or unread. */
  async values(raster: string, url: string | null, i: number, j: number): Promise<number[] | null> {
    const src = this.source(raster, url);
    if (!src) return null;
    const r = await this.ask(raster, { type: 'values', key: raster, blob: src.blob, name: src.name, i, j });
    return (r.value as number[] | null) ?? null;
  }

  /** Raster oturt's resampling: the GeoTIFF's bytes and its grid; `stop` ends it. */
  warp(
    raster: string,
    url: string | null,
    name: string,
    job: { points: number[]; method: string; pixel: number; nearest: boolean; epsg: number },
  ): { done: Promise<{ bytes: Uint8Array; grid: number[] }>; stop(): void } {
    const src = this.source(raster, url);
    if (!src) return { done: Promise.reject(new Error('Rasterin dosyası bu oturumda yok: Kaynağı yeniden seçin.')), stop: () => {} };
    const id = this.next;
    const stop = () => this.post(hash(raster) % 2, { type: 'stopWarp', id });
    this.warping.set({ name, done: 0, id, stop });
    const done = this.ask(raster, { type: 'warp', key: raster, blob: src.blob, name: src.name, ...job }, (share) => this.warping.set({ name, done: share, id, stop }))
      .then((r) => ({ bytes: r.bytes!, grid: (r.value as { grid: number[] }).grid }))
      .finally(() => this.warping.set(null));
    return { done, stop };
  }

  /** Durdur on a pyramid: its raster's coarse levels come from level 0. */
  stopPyramid(key: string): void {
    this.post(hash(key) % 2, { type: 'stopPyramid', key });
  }

  /** Lets every raster but those named go (another drawing opened). */
  keepOnly(keys: readonly string[]): void {
    if (!this.workers.length) return;
    for (let k = 0; k < this.workers.length; k++) this.post(k, { type: 'forget', keep: [...keys] });
  }
}

let shared: RasterService | null = null;

/** The page's raster service. */
export function rasterService(): RasterService {
  return (shared ??= new RasterService());
}
