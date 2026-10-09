import type { AnalysisReply, AnalysisRequest, OpsRequest, PointRequest } from './rasterAnalysisProtocol';

/**
 * A raster analysis worker (docs/adr/0231 §2, §11; a raster from points, docs/adr/0232; the raster operations over
 * several rasters, docs/adr/0233; started by
 * io/rasterAnalysis.ts, one a job): the raster core's job
 * (crates/wasm/raster-wasm, loaded here when the job starts) over the raster file's slices (`Blob.slice`), never the
 * whole file: a TIFF's header in the pieces it asks for, then a strip at a time the blocks the job names; a JPEG
 * block or image decoded by the browser (`createImageBitmap`), a PNG by the core. The result's parts are kept in
 * order, its header written over the first at the end, and sent back transferred. Durdur ends the worker: the page
 * terminates it, nothing half-written reaches the drawing.
 */
const scope = self as unknown as {
  onmessage: ((e: MessageEvent<AnalysisRequest | PointRequest | OpsRequest>) => void) | null;
  postMessage(m: AnalysisReply, transfer?: Transferable[]): void;
};

type RasterModule = typeof import('./raster/pkg/kentos_raster_wasm.js');
type Analysis = import('./raster/pkg/kentos_raster_wasm.js').Analysis;
type OpsOpening = import('./raster/pkg/kentos_raster_wasm.js').OpsOpening;

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

/** A raster made from points or lines (docs/adr/0232): the job stepped strip by strip, its parts kept in order. */
async function points(job: PointRequest): Promise<void> {
  const m = await load();
  const a = new m.PointAnalysis(job.objects, job.values, job.spec, job.lines);
  try {
    const parts: Uint8Array[] = [a.header()];
    let told = -1;
    while (!a.done()) {
      parts.push(a.step());
      const share = a.share();
      if (share - told >= 0.01) {
        told = share;
        scope.postMessage({ type: 'progress', share });
      }
    }
    parts.push(a.finish());
    const bytes = new Uint8Array(await new Blob(parts as unknown as BlobPart[]).arrayBuffer());
    bytes.set(a.head(), 0);
    const points = {
      bytes,
      grid: Array.from(a.grid()),
      bands: a.bands(),
      sample: a.sample(),
      styles: [a.style(1), a.style(2)] as [string, string],
      notes: a.notes(),
      crossPoint: a.crossPoint(),
      crossObject: a.crossObject(),
      crossValues: a.crossValues(),
    };
    scope.postMessage({ type: 'done', points }, [bytes.buffer, points.crossPoint.buffer, points.crossObject.buffer, points.crossValues.buffer]);
  } finally {
    a.free();
  }
}

/** An operation's input added to the opening: a TIFF read by its slices, a PNG decoded by the core, a JPEG by the browser. */
async function addInput(m: RasterModule, o: OpsOpening, blob: Blob): Promise<void> {
  const head = await bytes(blob, 0, 16);
  if (m.AnalysisOpening.isTiff(head)) {
    const t = new m.AnalysisOpening(blob.size);
    try {
      for (;;) {
        const need = t.need();
        if (!need.length) break;
        t.put(need[0], await bytes(blob, need[0], need[1]));
      }
      o.addTiff(t);
    } finally {
      t.free();
    }
    return;
  }
  if (head[0] === 0x89 && head[1] === 0x50) {
    o.addPng(new Uint8Array(await blob.arrayBuffer()));
    return;
  }
  if (head[0] === 0xff && head[1] === 0xd8) {
    const p = await jpegPixels(blob);
    o.addPixels(p.width, p.height, 4, p.rgba);
    return;
  }
  throw new Error('Dosya GeoTIFF, TIFF, PNG ya da JPEG değil.');
}

/**
 * A raster operation (docs/adr/0233): the inputs it reads (only those Raster hesaplayıcı's expression names) opened in
 * the core's order, then stepped, each block read from its input's file.
 */
async function ops(job: OpsRequest): Promise<void> {
  const m = await load();
  const reads = Array.from(m.opsReads(job.spec));
  const files = reads.map((k) => {
    const s = job.sources[k];
    if (typeof s === 'string') throw new Error(s);
    return s;
  });
  const o = new m.OpsOpening();
  let a: import('./raster/pkg/kentos_raster_wasm.js').OpsAnalysis;
  try {
    for (const blob of files) await addInput(m, o, blob);
    a = o.start(job.spec, job.shapes);
  } finally {
    o.free();
  }
  try {
    const parts: Uint8Array[] = [a.header()];
    let told = -1;
    while (!a.done()) {
      const needs = a.needs();
      for (let i = 0; 3 * i < needs.length; i++) {
        const [input, at, len] = [needs[3 * i], needs[3 * i + 1], needs[3 * i + 2]];
        if (len > 64 * 1024 * 1024) throw new Error("Rasterin bir bloğu 64 MB'tan büyük.");
        const stream = a.putBlock(i, await bytes(files[input], at, len));
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
    const result = { reads, grid: Array.from(a.grid()), bands: a.bands(), sample: a.sample(), style: a.style(), notes: a.notes() };
    if (a.bands() > 0) {
      const out = new Uint8Array(await new Blob(parts as unknown as BlobPart[]).arrayBuffer());
      out.set(a.head(), 0);
      scope.postMessage({ type: 'done', ops: { ...result, bytes: out } }, [out.buffer]);
      return;
    }
    const kind = a.featureKind();
    if (kind) {
      const features = {
        kind: kind as 'polygons' | 'lines' | 'points',
        values: a.featureValues(),
        texts: a.featureTexts(),
        tags: a.featureTags(),
        rings: a.featureRings(),
        sizes: a.featureSizes(),
        xy: a.featureXy(),
        fields: a.featureFields(),
        numbers: a.featureNumbers(),
      };
      const moved = [features.values.buffer, features.tags.buffer, features.rings.buffer, features.sizes.buffer, features.xy.buffer, features.numbers.buffer];
      scope.postMessage({ type: 'done', ops: { ...result, features } }, moved);
      return;
    }
    const zones = a.zones();
    const histogram = a.histogram();
    scope.postMessage({ type: 'done', ops: { ...result, zones, histogram } }, [zones.buffer]);
  } finally {
    a.free();
  }
}

scope.onmessage = (e) => {
  const job = e.data;
  (job.type === 'points' ? points(job) : job.type === 'ops' ? ops(job) : run(job)).catch((err: unknown) =>
    scope.postMessage({ type: 'done', error: err instanceof Error ? err.message : String(err) }),
  );
};
