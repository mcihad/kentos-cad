import type { RasterReply, RasterRequest } from './rasterProtocol';
import { FORMATS_VERSION } from './version';

/**
 * A raster worker (docs/adr/0204 §3, §11; started by render/rasterService.ts,
 * two of them): the formats core's raster reader (crates/wasm/formats-wasm
 * `RasterOpening`, `RasterFile`) over a file's slices (`Blob.slice`), never
 * the whole file; a JPEG block or image decoded by the browser
 * (`createImageBitmap`); a tile's colours sent back as transferred bytes. A
 * large raster without overviews gets its pyramid file once, a band of rows
 * a step between the other requests, kept in IndexedDB `kentos.rasters`
 * (`pyramids`, by the SHA-256 of the file's name, size and change time);
 * until it is made its coarse levels wait. Raster oturt's resampling runs a
 * tile a step the same way. Requests run one at a time, in order: a raster's
 * reader lists the blocks a region wants and takes them by their index, so
 * two requests must not meet in it.
 */
const scope = self as unknown as {
  onmessage: ((e: MessageEvent<RasterRequest>) => void) | null;
  postMessage(m: RasterReply, transfer?: Transferable[]): void;
};

type FormatsModule = typeof import('./pkg/kentos_formats_wasm.js');
type RasterFile = import('./pkg/kentos_formats_wasm.js').RasterFile;

let wasm: Promise<FormatsModule> | null = null;
function formats(): Promise<FormatsModule> {
  return (wasm ??= (async () => {
    const [module, url] = await Promise.all([import('./pkg/kentos_formats_wasm.js'), import('./pkg/kentos_formats_wasm_bg.wasm?url')]);
    await module.default({ module_or_path: url.default });
    const v = module.formatsVersion();
    if (v !== FORMATS_VERSION) throw new Error(`Dosya biçimi modülü sürüm ${v}, uygulama ${FORMATS_VERSION} bekliyor; modülü yeniden derleyin (pnpm wasm)`);
    return module;
  })().catch((e: unknown) => {
    wasm = null;
    throw e;
  }));
}

/** Bytes a raster's decoded blocks may take in this worker. */
const BUDGET = 48 * 1024 * 1024;
/** The first level a pyramid file holds (the core's `PYRAMID_FIRST`). */
const PYRAMID_FIRST = 3;
/** Rows of level 0 a pyramid step reads. */
const BAND = 256;

interface Opened {
  file: RasterFile;
  blob: Blob;
  name: string;
  info: { width: number; height: number; bands: number; sample: string; needsPyramid: boolean };
  pyramid: Blob | null;
  /** Its pyramid is being made: its coarse levels wait. */
  building: boolean;
  /** Its pyramid's pass was stopped or failed: its coarse levels come from level 0. */
  declined: boolean;
  stats: boolean;
}

const opened = new Map<string, Opened>();

async function bytes(blob: Blob, at: number, len: number): Promise<Uint8Array> {
  return new Uint8Array(await blob.slice(at, at + len).arrayBuffer());
}

/** A JPEG's pixels by the browser's decoder: RGBA. */
async function jpegPixels(data: Uint8Array | Blob): Promise<{ width: number; height: number; rgba: Uint8Array }> {
  const blob = data instanceof Blob ? data : new Blob([data as Uint8Array<ArrayBuffer>], { type: 'image/jpeg' });
  const bitmap = await createImageBitmap(blob, { colorSpaceConversion: 'none', premultiplyAlpha: 'none' });
  const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
  const g = canvas.getContext('2d')!;
  g.drawImage(bitmap, 0, 0);
  bitmap.close();
  const img = g.getImageData(0, 0, canvas.width, canvas.height);
  return { width: canvas.width, height: canvas.height, rgba: new Uint8Array(img.data.buffer) };
}

/** A raster read from its file's slices: a TIFF's header, a PNG decoded here, a JPEG by the browser. */
async function openBlob(m: FormatsModule, blob: Blob, world: string | null): Promise<RasterFile> {
  const head = await bytes(blob, 0, 16);
  if (m.RasterOpening.isTiff(head)) {
    const o = new m.RasterOpening(blob.size);
    try {
      for (;;) {
        const need = o.need();
        if (!need.length) break;
        o.put(need[0], await bytes(blob, need[0], need[1]));
      }
      return o.open(world ?? undefined, BUDGET);
    } finally {
      o.free();
    }
  }
  if (head[0] === 0x89 && head[1] === 0x50) return m.RasterFile.fromPng(new Uint8Array(await blob.arrayBuffer()), world ?? undefined, BUDGET);
  if (head[0] === 0xff && head[1] === 0xd8) {
    const p = await jpegPixels(blob);
    return m.RasterFile.fromPixels(p.width, p.height, 4, p.rgba, world ?? undefined, BUDGET);
  }
  throw new Error('Dosya GeoTIFF, TIFF, PNG ya da JPEG değil.');
}

async function open(key: string, blob: Blob, name: string): Promise<Opened> {
  const kept = opened.get(key);
  if (kept) return kept;
  const m = await formats();
  const file = await openBlob(m, blob, null);
  const info = JSON.parse(file.info()) as Opened['info'];
  const o: Opened = { file, blob, name, info, pyramid: null, building: false, declined: false, stats: false };
  opened.set(key, o);
  return o;
}

/** Every block a region of `level` wants read and kept (a JPEG's by the browser). */
async function fill(o: Opened, level: number, x: number, y: number, w: number, h: number): Promise<void> {
  for (let round = 0; round < 3; round++) {
    const needs = o.file.needs(level, x, y, w, h);
    if (!needs.length) return;
    for (let i = 0; 3 * i < needs.length; i++) {
      const [which, at, len] = [needs[3 * i], needs[3 * i + 1], needs[3 * i + 2]];
      const from = which === 1 ? o.pyramid : o.blob;
      if (!from) throw new Error('Önizleme piramidi henüz yok.');
      if (len > 64 * 1024 * 1024) throw new Error("Rasterin bir bloğu 64 MB'tan büyük.");
      const stream = o.file.putBlock(i, await bytes(from, at, len));
      if (stream.length) {
        const p = await jpegPixels(stream);
        o.file.putPixels(i, p.rgba, 4);
      }
    }
  }
}

/** Whether a look stretches by the raster's statistics. */
function wantsStats(look: { render: string; stretch?: string }, sample: string): boolean {
  if (look.render === 'hillshade' || look.render === 'palette') return false;
  const stretch = look.stretch ?? 'none';
  if (stretch === 'manual') return false;
  if (stretch === 'minMax' || stretch === 'percent') return true;
  return sample !== 'u8';
}

async function statsOf(o: Opened): Promise<string | null> {
  const [level, w, h] = o.file.statsRegion();
  await fill(o, level, 0, 0, w, h);
  const s = o.file.takeStats() ?? null;
  o.stats = s !== null;
  return s;
}

// ── The pyramid file (docs/adr/0204 §3) ─────────────────────────────────

let db: Promise<IDBDatabase> | null = null;
function store(): Promise<IDBDatabase> {
  return (db ??= new Promise((resolve, reject) => {
    const r = indexedDB.open('kentos.rasters', 1);
    r.onupgradeneeded = () => r.result.createObjectStore('pyramids');
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  }));
}

async function idb<T>(mode: IDBTransactionMode, run: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const d = await store();
  return new Promise((resolve, reject) => {
    const r = run(d.transaction('pyramids', mode).objectStore('pyramids'));
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });
}

async function pyramidKey(o: Opened): Promise<string> {
  const f = o.blob as Blob & { lastModified?: number };
  const id = new TextEncoder().encode(`${o.name}|${o.blob.size}|${f.lastModified ?? 0}`);
  const sum = new Uint8Array(await crypto.subtle.digest('SHA-256', id));
  return Array.from(sum, (b) => b.toString(16).padStart(2, '0')).join('');
}

async function attach(m: FormatsModule, o: Opened, pyramid: Blob): Promise<void> {
  const r = new m.RasterOpening(pyramid.size);
  try {
    for (;;) {
      const need = r.need();
      if (!need.length) break;
      r.put(need[0], await bytes(pyramid, need[0], need[1]));
    }
    r.attachTo(o.file);
    o.pyramid = pyramid;
  } finally {
    r.free();
  }
}

/** A pass under way: its raster, its parts so far, its next row, whether stopped. */
interface Pass {
  key: string;
  o: Opened;
  id: string;
  parts: Uint8Array[];
  y: number;
  stop: boolean;
}
const passes = new Map<string, Pass>();

/** Starts the raster's pyramid unless it is made, under way or declined: a file kept before is taken at once. */
async function startPyramid(key: string, o: Opened): Promise<void> {
  if (!o.info.needsPyramid || o.building || o.declined || o.pyramid) return;
  o.building = true;
  const m = await formats();
  const id = await pyramidKey(o).catch(() => '');
  const kept = id ? await idb<Blob | undefined>('readonly', (s) => s.get(id) as IDBRequest<Blob | undefined>).catch(() => undefined) : undefined;
  if (kept) {
    try {
      await attach(m, o, kept);
      o.building = false;
      scope.postMessage({ type: 'refresh' });
      return;
    } catch {
      // A broken copy is made again.
    }
  }
  passes.set(key, { key, o, id, parts: [o.file.pyramidStart()], y: 0, stop: false });
  scope.postMessage({ type: 'pyramid', key, name: o.name, done: 0 });
  queue.push({ type: 'pyramidStep', key });
}

async function pyramidStep(key: string): Promise<void> {
  const p = passes.get(key);
  if (!p) return;
  const { o } = p;
  const end = (made: boolean) => {
    passes.delete(key);
    o.building = false;
    if (!made) o.declined = true;
    scope.postMessage({ type: 'pyramid', key, name: o.name, done: null });
    scope.postMessage({ type: 'refresh' });
  };
  if (p.stop) return end(false);
  try {
    const h = o.info.height;
    const n = Math.min(BAND, h - p.y);
    await fill(o, 0, 0, p.y, o.info.width, n);
    p.parts.push(o.file.pyramidPush(p.y, n));
    p.y += n;
    if (p.y < h) {
      scope.postMessage({ type: 'pyramid', key, name: o.name, done: p.y / h });
      queue.push({ type: 'pyramidStep', key });
      return;
    }
    p.parts.push(o.file.pyramidFinish());
    p.parts[0] = o.file.pyramidHead();
    const pyramid = new Blob(p.parts as unknown as BlobPart[], { type: 'image/tiff' });
    await attach(await formats(), o, pyramid);
    if (p.id) await idb('readwrite', (s) => s.put(pyramid, p.id)).catch(() => undefined);
    end(true);
  } catch {
    end(false);
  }
}

// ── Raster oturt's resampling (docs/adr/0204 §6) ────────────────────────

interface Warp {
  id: number;
  o: Opened;
  parts: Uint8Array[];
  stop: boolean;
}
const warps = new Map<number, Warp>();

async function warpStep(id: number): Promise<void> {
  const w = warps.get(id);
  if (!w) return;
  const fail = (error: string) => {
    warps.delete(id);
    scope.postMessage({ type: 'done', id, error });
  };
  if (w.stop) return fail('durduruldu');
  try {
    // A few tiles a step, so other requests are not kept waiting long.
    for (let k = 0; k < 4; k++) {
      const region = w.o.file.warpRegion();
      if (region.length === 1) {
        // The grid before the job ends: finishing lets it go.
        const grid = Array.from(w.o.file.warpGrid());
        w.parts.push(w.o.file.warpFinish());
        w.parts[0] = w.o.file.warpHead();
        const out = new Uint8Array(await new Blob(w.parts as unknown as BlobPart[]).arrayBuffer());
        warps.delete(id);
        scope.postMessage({ type: 'done', id, value: { grid }, bytes: out }, [out.buffer]);
        return;
      }
      if (region.length === 4) await fill(w.o, 0, region[0], region[1], region[2], region[3]);
      w.parts.push(w.o.file.warpTile());
    }
    scope.postMessage({ type: 'progress', id, share: w.o.file.warpShare() });
    queue.push({ type: 'warpStep', id });
  } catch (e) {
    fail(e instanceof Error ? e.message : String(e));
  }
}

// ── Requests, one at a time ─────────────────────────────────────────────

type Job = RasterRequest | { type: 'pyramidStep'; key: string } | { type: 'warpStep'; id: number };
const queue: Job[] = [];
let running = false;

const err = (e: unknown) => (e instanceof Error ? e.message : String(e));

async function run(job: Job): Promise<void> {
  switch (job.type) {
    case 'pyramidStep':
      return pyramidStep(job.key);
    case 'warpStep':
      return warpStep(job.id);
    case 'stopPyramid': {
      const p = passes.get(job.key);
      if (p) p.stop = true;
      return;
    }
    case 'stopWarp': {
      const w = warps.get(job.id);
      if (w) w.stop = true;
      return;
    }
    case 'forget':
      for (const [key, o] of opened)
        if (!job.keep.includes(key)) {
          o.file.free();
          opened.delete(key);
        }
      return;
    case 'drop': {
      opened.get(job.key)?.file.free();
      opened.delete(job.key);
      return;
    }
    case 'inspect': {
      try {
        const m = await formats();
        const exts = (n: string) => n.slice(n.lastIndexOf('.') + 1).toLowerCase();
        const image = job.files.find((f) => ['tif', 'tiff', 'png', 'jpg', 'jpeg'].includes(exts(f.name))) ?? job.files[0];
        const wanted = new Set(m.rasterWorldExtensions(exts(image.name)));
        const worldFile = job.files.find((f) => f !== image && wanted.has(exts(f.name)));
        const world = worldFile ? await worldFile.text() : null;
        const file = await openBlob(m, image, world);
        try {
          const value = {
            name: image.name,
            world: worldFile?.name ?? null,
            info: JSON.parse(file.info()),
            style: JSON.parse(file.defaultStyle()),
            placement: JSON.parse(file.placement(job.srid, job.confirmed, Float64Array.from(job.view))),
            confirmedPlacement: JSON.parse(file.placement(job.srid, true, Float64Array.from(job.view))),
          };
          scope.postMessage({ type: 'done', id: job.id, value });
        } finally {
          file.free();
        }
      } catch (e) {
        scope.postMessage({ type: 'done', id: job.id, error: err(e) });
      }
      return;
    }
    case 'tile': {
      try {
        const o = await open(job.key, job.blob, job.name);
        if (o.info.needsPyramid && !o.declined && !o.pyramid) await startPyramid(job.key, o);
        if (job.level >= PYRAMID_FIRST && o.building) {
          scope.postMessage({ type: 'done', id: job.id, wait: true });
          return;
        }
        const look = JSON.parse(job.look) as { render: string; stretch?: string };
        if (!o.stats && wantsStats(look, o.info.sample)) await statsOf(o);
        await fill(o, job.level, job.tx * 256 - 2, job.ty * 256 - 2, 260, 260);
        const rgba = o.file.renderTile(job.level, job.tx, job.ty, job.look, Float64Array.from(job.affine));
        if (!rgba.length) throw new Error('Karonun blokları okunamadı.');
        scope.postMessage({ type: 'done', id: job.id, bytes: rgba }, [rgba.buffer]);
      } catch (e) {
        scope.postMessage({ type: 'done', id: job.id, error: err(e) });
      }
      return;
    }
    case 'stats': {
      try {
        const o = await open(job.key, job.blob, job.name);
        const s = await statsOf(o);
        scope.postMessage({ type: 'done', id: job.id, value: s ? JSON.parse(s) : null });
      } catch (e) {
        scope.postMessage({ type: 'done', id: job.id, error: err(e) });
      }
      return;
    }
    case 'values': {
      try {
        const o = await open(job.key, job.blob, job.name);
        const [i, j] = [Math.floor(job.i), Math.floor(job.j)];
        await fill(o, 0, i, j, 1, 1);
        const v = o.file.sample(i, j);
        scope.postMessage({ type: 'done', id: job.id, value: v ? Array.from(v) : null });
      } catch (e) {
        scope.postMessage({ type: 'done', id: job.id, error: err(e) });
      }
      return;
    }
    case 'warp': {
      try {
        const o = await open(job.key, job.blob, job.name);
        const header = o.file.warpStart(Float64Array.from(job.points), job.method, job.pixel, job.nearest, job.epsg);
        warps.set(job.id, { id: job.id, o, parts: [header], stop: false });
        queue.push({ type: 'warpStep', id: job.id });
      } catch (e) {
        scope.postMessage({ type: 'done', id: job.id, error: err(e) });
      }
      return;
    }
  }
}

async function drain(): Promise<void> {
  if (running) return;
  running = true;
  try {
    for (let job = queue.shift(); job; job = queue.shift()) await run(job);
  } finally {
    running = false;
  }
}

scope.onmessage = (e) => {
  const job = e.data;
  // Stops go first: a long pass stops at its next step.
  if (job.type === 'stopPyramid' || job.type === 'stopWarp') void run(job);
  else queue.push(job);
  void drain();
};
