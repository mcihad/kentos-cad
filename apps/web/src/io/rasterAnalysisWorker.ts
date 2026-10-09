import type { AnalysisReply, AnalysisRequest } from './rasterAnalysisProtocol';

/**
 * A raster analysis worker (docs/adr/0231 §2, §11; started by io/rasterAnalysis.ts, one a job): the raster core's job
 * (crates/wasm/raster-wasm, loaded here when the job starts) over the raster file's slices (`Blob.slice`), never the
 * whole file: a TIFF's header in the pieces it asks for, then a strip at a time the blocks the job names; a JPEG
 * block or image decoded by the browser (`createImageBitmap`), a PNG by the core. The result's parts are kept in
 * order, its header written over the first at the end, and sent back transferred. Durdur ends the worker: the page
 * terminates it, nothing half-written reaches the drawing.
 */
const scope = self as unknown as {
  onmessage: ((e: MessageEvent<AnalysisRequest>) => void) | null;
  postMessage(m: AnalysisReply, transfer?: Transferable[]): void;
};

type RasterModule = typeof import('./raster/pkg/kentos_raster_wasm.js');
type Analysis = import('./raster/pkg/kentos_raster_wasm.js').Analysis;

async function load(): Promise<RasterModule> {
  const [module, url] = await Promise.all([import('./raster/pkg/kentos_raster_wasm.js'), import('./raster/pkg/kentos_raster_wasm_bg.wasm?url')]);
  await module.default({ module_or_path: url.default });
  return module;
}

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

/** The job over the raster's file: a TIFF read by its slices, a PNG decoded by the core, a JPEG by the browser. */
async function open(m: RasterModule, blob: Blob, spec: string): Promise<Analysis> {
  const head = await bytes(blob, 0, 16);
  if (m.AnalysisOpening.isTiff(head)) {
    const o = new m.AnalysisOpening(blob.size);
    try {
      for (;;) {
        const need = o.need();
        if (!need.length) break;
        o.put(need[0], await bytes(blob, need[0], need[1]));
      }
      return o.analysis(spec);
    } finally {
      o.free();
    }
  }
  if (head[0] === 0x89 && head[1] === 0x50) return m.Analysis.fromPng(new Uint8Array(await blob.arrayBuffer()), spec);
  if (head[0] === 0xff && head[1] === 0xd8) {
    const p = await jpegPixels(blob);
    return m.Analysis.fromPixels(p.width, p.height, 4, p.rgba, spec);
  }
  throw new Error('Dosya GeoTIFF, TIFF, PNG ya da JPEG değil.');
}

async function run(job: AnalysisRequest): Promise<void> {
  const m = await load();
  const a = await open(m, job.blob, job.spec);
  try {
    const parts: Uint8Array[] = [a.header()];
    let told = -1;
    while (!a.done()) {
      const needs = a.needs();
      for (let i = 0; 2 * i < needs.length; i++) {
        const [at, len] = [needs[2 * i], needs[2 * i + 1]];
        if (len > 64 * 1024 * 1024) throw new Error("Rasterin bir bloğu 64 MB'tan büyük.");
        const stream = a.putBlock(i, await bytes(job.blob, at, len));
        if (stream.length) {
          const p = await jpegPixels(stream);
          a.putPixels(i, p.rgba, 4);
        }
      }
      parts.push(a.step());
      const share = a.share();
      if (share - told >= 0.01) {
        told = share;
        scope.postMessage({ type: 'progress', share });
      }
    }
    parts.push(a.finish());
    const bands = a.bands();
    if (bands > 0) {
      const out = new Uint8Array(await new Blob(parts as unknown as BlobPart[]).arrayBuffer());
      // The header written again over the file's start (its directories' places now known).
      out.set(a.head(), 0);
      scope.postMessage({ type: 'done', raster: { bytes: out, bands, sample: a.sample(), style: a.style() } }, [out.buffer]);
      return;
    }
    const lines = { values: a.lineValues(), main: a.lineMain(), sizes: a.lineSizes(), points: a.linePoints(), texts: a.lineTexts() };
    scope.postMessage({ type: 'done', lines }, [lines.values.buffer, lines.main.buffer, lines.sizes.buffer, lines.points.buffer]);
  } finally {
    a.free();
  }
}

scope.onmessage = (e) => {
  run(e.data).catch((err: unknown) => scope.postMessage({ type: 'done', error: err instanceof Error ? err.message : String(err) }));
};
