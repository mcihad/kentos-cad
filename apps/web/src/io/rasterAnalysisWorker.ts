import type { AnalysisReply, AnalysisRequest, MultidimRequest, OpsRequest, PointRequest } from './rasterAnalysisProtocol';

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
  onmessage: ((e: MessageEvent<AnalysisRequest | PointRequest | OpsRequest | MultidimRequest>) => void) | null;
  postMessage(m: AnalysisReply, transfer?: Transferable[]): void;
};

type RasterModule = typeof import('./raster/pkg/kentos_raster_wasm.js');
type Analysis = import('./raster/pkg/kentos_raster_wasm.js').Analysis;
type OpsOpening = import('./raster/pkg/kentos_raster_wasm.js').OpsOpening;
type CubeOpening = import('./raster/pkg/kentos_raster_wasm.js').CubeOpening;

/** A NetCDF raster's file opened for its slice `part`: its header, then what the slice needs, read by its slices. */
async function cubeOpening(m: RasterModule, blob: Blob, part: string): Promise<CubeOpening> {
  const c = new m.CubeOpening(blob.size, part);
  try {
    for (;;) {
      const need = c.need();
      if (!need.length) return c;
      if (need[1] > 1024 * 1024 * 1024) throw new Error("NetCDF'in bir değişkeni 1 GB'tan büyük; okunmuyor.");
      c.put(need[0], await bytes(blob, need[0], need[1]));
    }
  } catch (e) {
    c.free();
    throw e;
  }
}

async function load(): Promise<RasterModule> {
  const [module, url] = await Promise.all([import('./raster/pkg/kentos_raster_wasm.js'), import('./raster/pkg/kentos_raster_wasm_bg.wasm?url')]);
  await module.default({ module_or_path: url.default });
  return module;
}

async function bytes(blob: Blob, at: number, len: number): Promise<Uint8Array> {
  return new Uint8Array(await blob.slice(at, at + len).arrayBuffer());
}

/**
 * Runs `[at, len]` pairs read from `blob` and handed to `put` in their order. A browser's read of a file costs a round
 * trip of some 0.1 ms whatever its size, about what reading 128 KB more costs: runs closer than that are read as one
 * span (at most 16 MB) and cut apart, and up to 256 spans or 32 MB are read at once. A time series asks for tens of
 * thousands of small runs (docs/adr/0243 §12).
 */
async function readRuns(blob: Blob, needs: ArrayLike<number>, put: (k: number, b: Uint8Array) => Promise<void> | void): Promise<void> {
  const GAP = 128 << 10;
  const SPAN = 16 << 20;
  // Spans: their first run, the run after their last, where they begin and end.
  const spans: [number, number, number, number][] = [];
  for (let k = 0; 2 * k < needs.length; k++) {
    const at = needs[2 * k];
    const end = at + needs[2 * k + 1];
    const last = spans[spans.length - 1];
    if (last && at >= last[3] && at - last[3] <= GAP && end - last[2] <= SPAN) {
      last[1] = k + 1;
      last[3] = end;
    } else spans.push([k, k + 1, at, end]);
  }
  for (let s = 0; s < spans.length; ) {
    let e = s;
    let held = 0;
    while (e < spans.length && e - s < 256 && (e === s || held + spans[e][3] - spans[e][2] <= 32 << 20)) held += spans[e][3] - spans[e++][2];
    const got = await Promise.all(spans.slice(s, e).map(([, , a, z]) => bytes(blob, a, z - a)));
    for (let q = 0; q < got.length; q++) {
      const [k0, k1, a] = spans[s + q];
      for (let k = k0; k < k1; k++) await put(k, got[q].subarray(needs[2 * k] - a, needs[2 * k] - a + needs[2 * k + 1]));
    }
    s = e;
  }
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

/** The job over the raster's file: a TIFF read by its slices, a PNG decoded by the core, a JPEG by the browser, a NetCDF's slice. */
async function open(m: RasterModule, blob: Blob, spec: string, part?: string | null): Promise<Analysis> {
  if (part) {
    const c = await cubeOpening(m, blob, part);
    try {
      return c.analysis(spec);
    } finally {
      c.free();
    }
  }
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
  const a = await open(m, job.blob, job.spec, job.part);
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

/** An operation's input added to the opening: a TIFF read by its slices, a PNG decoded by the core, a JPEG by the browser, a NetCDF's slice. */
async function addInput(m: RasterModule, o: OpsOpening, blob: Blob, part?: string | null): Promise<void> {
  if (part) {
    const c = await cubeOpening(m, blob, part);
    try {
      o.addCube(c);
    } finally {
      c.free();
    }
    return;
  }
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
    for (const [k, blob] of files.entries()) await addInput(m, o, blob, job.parts?.[reads[k]]);
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
    const roc = a.roc();
    scope.postMessage({ type: 'done', ops: { ...result, zones, histogram, roc } }, [zones.buffer]);
  } finally {
    a.free();
  }
}

/**
 * A Çok boyutlu veri tool (docs/adr/0243 §8–§10): the raster opened (any raster for Kesit and bands, a NetCDF slice's cube
 * for time steps and Mesh hesaplayıcı), the job stepped, each run read from the file; Mesh hesaplayıcı's file sent back.
 */
async function multidim(job: MultidimRequest): Promise<void> {
  const m = await load();
  const affine = Float64Array.from(job.affine);
  const nodata = job.nodata ?? undefined;
  let a: import('./raster/pkg/kentos_raster_wasm.js').MultidimAnalysis;
  if (job.kind === 'timeSeries' || job.kind === 'meshCalc') {
    if (!job.part) throw new Error('Bu raster NetCDF veri seti göstermiyor.');
    const c = await cubeOpening(m, job.blob, job.part);
    try {
      a = job.kind === 'timeSeries' ? m.MultidimAnalysis.timeSeries(c, affine, nodata, job.objects) : m.MultidimAnalysis.meshCalc(c, job.spec ?? '{}');
    } finally {
      c.free();
    }
  } else {
    const o = new m.OpsOpening();
    try {
      await addInput(m, o, job.blob, job.part);
      a =
        job.kind === 'profile'
          ? m.MultidimAnalysis.profile(o, affine, nodata, job.objects, job.step ?? 1, job.band ?? 1, job.axes ?? 'Y,X')
          : m.MultidimAnalysis.bandSeries(o, affine, nodata, job.objects);
    } finally {
      o.free();
    }
  }
  try {
    let told = -1;
    while (!a.done()) {
      const needs = a.needs();
      for (let i = 0; 2 * i < needs.length; i++) if (needs[2 * i + 1] > 1024 * 1024 * 1024) throw new Error("Rasterin bir parçası 1 GB'tan büyük.");
      await readRuns(job.blob, needs, async (i, got) => {
        const stream = a.putBlock(i, got);
        if (stream.length) {
          const p = await jpegPixels(stream);
          a.putPixels(i, p.rgba, 4);
        }
      });
      a.step();
      const share = a.share();
      if (share - told >= 0.01) {
        told = share;
        scope.postMessage({ type: 'progress', share });
      }
    }
    const result = a.finish();
    const file = a.file();
    const output = a.output();
    scope.postMessage({ type: 'done', multidim: { result, file: file.length ? file : undefined, output: output || undefined } }, file.length ? [file.buffer] : []);
  } finally {
    a.free();
  }
}

scope.onmessage = (e) => {
  const job = e.data;
  (job.type === 'points' ? points(job) : job.type === 'ops' ? ops(job) : job.type === 'multidim' ? multidim(job) : run(job)).catch((err: unknown) =>
    scope.postMessage({ type: 'done', error: err instanceof Error ? err.message : String(err) }),
  );
};
