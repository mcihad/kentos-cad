import { describe, expect, it } from 'vitest';
import { analyzeHere, analyzeOpsHere, analyzePointsHere, level0, surfaceModulesBuilt, ulps } from '../processing/surfaceTesting';

/**
 * The raster analyses as the browser runs them (docs/adr/0231): the raster analysis module (crates/wasm/raster-wasm →
 * src/io/raster/pkg, built by `pnpm wasm`) on one thread, as the analysis worker runs it, over every case of the
 * independent references the core plays natively (crates/shared/raster/tests/all): the surface's
 * (scripts/fixtures/terrain_cases.py; a 32-bit sample within one unit in the last place, a byte exact) and the
 * contours' (scripts/fixtures/contour_cases.py; the points bit for bit, the Kot texts), and the interpolations' and
 * densities' (scripts/fixtures/interpolation_cases.py, docs/adr/0232: a 32-bit cell within one unit in the last place,
 * each point's cross-validation within the method's bound), and the raster operations' (scripts/fixtures/raster_ops_cases.py,
 * docs/adr/0233: each sample by its case's rule, the zones' figures, the histograms, the refusals), and raster and vector's
 * (scripts/fixtures/raster_vector_cases.py, docs/adr/0234: the features bit for bit, Rasterleştir's every sample and its
 * notes, the refusals), and hydrology's (scripts/fixtures/hydrology_cases.py, docs/adr/0235: each sample by its case's
 * rule, the objects and their numbers, the notes, the refusals), and distance and cost's (scripts/fixtures/distance_cases.py,
 * docs/adr/0236: each sample by its case's rule, the paths and their numbers, the notes, the refusals), and suitability's
 * (scripts/fixtures/suitability_cases.py, docs/adr/0237: each sample by its case's rule, the weights within their bounds,
 * ROC's figures exactly, the notes, the refusals). Skipped only when the packages have not been built.
 */

type Bytes = Uint8Array<ArrayBuffer>;
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Bytes } } }).process.getBuiltinModule('node:fs');
const read = (rel: string) => fs.readFileSync(new URL(`../../../../fixtures/${rel}`, import.meta.url));
const json = <T>(rel: string) => JSON.parse(new TextDecoder().decode(read(rel))) as T;

interface TerrainCase {
  dem: string;
  name: string;
  spec: unknown;
  sample: string;
  bands: number;
  values?: (number | null)[];
  probe?: [number, number, number | null | (number | null)[]][];
}
interface Line {
  k: number;
  value: number;
  index: boolean;
  pts: [number, number][];
}
interface ContourCase {
  dem: string;
  file: string;
  spec: unknown;
  texts: Record<string, string>;
  lines?: Line[];
  levels?: { k: number; lines: number; vertices: number }[];
  first?: Line[];
}

describe.skipIf(!surfaceModulesBuilt)('raster analysis module (crates/wasm/raster-wasm)', () => {
  const terrain = json<{ dems: { name: string; file: string; width: number }[]; cases: TerrainCase[] }>('terrain/v1/cases.json');
  for (const c of terrain.cases) {
    it(`${c.dem}: ${c.name}`, async () => {
      const dem = terrain.dems.find((d) => d.name === c.dem)!;
      const out = await analyzeHere(read(`terrain/v1/${dem.file}`), JSON.stringify(c.spec));
      if (!('raster' in out)) throw new Error('a raster result');
      expect([out.raster.bands, out.raster.sample]).toEqual([c.bands, c.sample]);
      const got = await level0(out.raster.bytes);
      const same = (g: number, w: number | null) => (c.sample === 'u8' ? g === w : ulps(g, w ?? NaN) <= 1);
      if (c.values) {
        expect(got.length).toBe(c.values.length);
        const off = got.findIndex((g, k) => !same(g, c.values![k]));
        expect(off, `sample ${off}: ${got[off]} for ${c.values[off]}`).toBe(-1);
      } else {
        for (const [i, j, want] of c.probe!) {
          const at = (j * dem.width + i) * c.bands;
          const wanted = Array.isArray(want) ? want : [want];
          wanted.forEach((w, b) => expect(same(got[at + b], w), `(${i}, ${j}) band ${b}: ${got[at + b]} for ${w}`).toBe(true));
        }
      }
    });
  }

  const contours = json<{ cases: ContourCase[] }>('contours/v1/cases.json');
  contours.cases.forEach((c, n) => {
    it(`contours ${n}: ${c.dem} ${JSON.stringify(c.spec)}`, async () => {
      const out = await analyzeHere(read(c.file), JSON.stringify(c.spec));
      if (!('lines' in out)) throw new Error('lines');
      const l = out.lines;
      const lines: Line[] = [];
      let at = 0;
      for (let i = 0; i < l.sizes.length; i++) {
        const pts: [number, number][] = [];
        for (let k = 0; k < l.sizes[i]; k++) pts.push([l.points[2 * (at + k)], l.points[2 * (at + k) + 1]]);
        at += l.sizes[i];
        lines.push({ k: NaN, value: l.values[i], index: l.main[i] === 1, pts });
      }
      const want = c.lines ?? c.first!;
      if (c.lines) expect(lines.length).toBe(c.lines.length);
      want.forEach((w, i) => {
        expect([lines[i].value, lines[i].index], `line ${i}`).toEqual([w.value, w.index]);
        expect(lines[i].pts, `line ${i}'s points`).toEqual(w.pts);
        // Its Kot as the drawing writes it.
        expect(l.texts[i], `line ${i}'s Kot`).toBe(c.texts[String(w.k)]);
      });
      for (const level of c.levels ?? []) {
        const value = want.find((w) => w.k === level.k)?.value;
        if (value === undefined) continue;
        const of = lines.filter((x) => x.value === value);
        expect([of.length, of.reduce((s, x) => s + x.pts.length, 0)], `level ${level.k}`).toEqual([level.lines, level.vertices]);
      }
    });
  });

  interface PointCase {
    name: string;
    kind: string;
    sources?: unknown[];
    shapes?: unknown[];
    values: (string | null)[] | null;
    tool: { kind: string; variogram?: { fit: string } };
    cell: number;
    grid?: unknown;
    expect: { width: number; height: number; values: (number | null)[]; error?: (number | null)[]; cross?: (number | null | (number | null)[])[] };
  }
  /** The core's bound for a method's cell values (crates/shared/raster/tests/all/interpolation.rs). */
  const bound = (c: PointCase) =>
    ({ idw: 1e-13, tin: 1e-13, naturalNeighbor: 1e-13, spline: 1e-13, kernel: 1e-12, lineDensity: 1e-11 })[c.tool.kind] ??
    (c.tool.variogram?.fit === 'auto' ? 1e-8 : 1e-13);
  const interp = json<{ cases: PointCase[] }>('interpolation/v1/cases.json');
  for (const c of interp.cases.filter((c) => c.kind === 'surface' || c.kind === 'lines')) {
    it(`interpolation: ${c.name}`, async () => {
      const lines = c.kind === 'lines';
      const cross = !!c.expect.cross;
      const spec = JSON.stringify({ tool: c.tool, cell: c.cell, grid: c.grid ?? null, cross });
      const out = await analyzePointsHere(JSON.stringify(lines ? c.shapes : c.sources), JSON.stringify(c.values), spec, lines);
      expect([out.grid[6], out.grid[7]]).toEqual([c.expect.width, c.expect.height]);
      const got = await level0(out.bytes);
      const want = c.expect.error ? c.expect.values.flatMap((v, k) => [v, c.expect.error![k]]) : c.expect.values;
      expect(got.length).toBe(want.length);
      const off = got.findIndex((g, k) => ulps(g, want[k] ?? NaN) > 1);
      expect(off, `cell ${off}: ${got[off]} for ${want[off]}`).toBe(-1);
      if (!cross) return;
      const tol = bound(c);
      c.expect.cross!.forEach((w, k) => {
        const [wv, we] = Array.isArray(w) ? w : [w, null];
        const gv = out.crossValues[5 * k + 3];
        if (wv === null) expect(Number.isNaN(gv), `point ${k}`).toBe(true);
        else expect(Math.abs(gv - wv), `point ${k}: ${gv} for ${wv}`).toBeLessThanOrEqual(tol * Math.max(1, Math.abs(wv)));
        if (we !== null && we !== undefined) expect(Math.abs(out.crossValues[5 * k + 4] - we), `point ${k}'s error`).toBeLessThanOrEqual(tol * Math.max(1, Math.abs(we)));
      });
    });
  }
});

interface OpsInput {
  affine: number[];
  width: number;
  height: number;
  bands: number;
  sample: string;
  nodata: 'nan' | number | null;
  alpha: boolean;
  values: (number | null)[];
  name: string;
}
interface OpsCase {
  name: string;
  tool: unknown;
  inputs: OpsInput[];
  shapes: unknown[];
  expect: {
    refused?: string;
    raster?: { width: number; height: number; bands: number; affine: number[]; rule: string; values: (number | null)[] };
    zones?: { n: number; value: number | null; rule: string }[];
    histogram?: { lo: number; hi: number; counts: number[]; below: number; above: number; valid: number; empty: number };
  };
}

/**
 * A reference input as an uncompressed TIFF in one strip: a 32-bit float band or a 16-bit signed one (its nodata the
 * GDAL_NODATA tag) or bytes (three bands RGB). The core reads its own tiled files (crates/shared/raster/tests/all/raster_ops.rs)
 * and these alike; the place is the run's, from the settings.
 */
function tiffOf(r: OpsInput): Uint8Array {
  const one = r.bands === 1;
  if (r.alpha || !((r.sample === 'f32' && one) || (r.sample === 'f64' && one) || (r.sample === 'i16' && one) || (r.sample === 'u8' && (one || r.bands === 3))))
    throw new Error(`${r.name}: not an input this writer makes`);
  const float = r.sample === 'f32' || r.sample === 'f64';
  const size = r.sample === 'f64' ? 8 : r.sample === 'f32' ? 4 : r.sample === 'i16' ? 2 : 1;
  const strip = new Uint8Array(r.width * r.height * r.bands * size);
  const sv = new DataView(strip.buffer);
  const empty = typeof r.nodata === 'number' ? r.nodata : 0;
  r.values.forEach((v, k) =>
    size === 8 ? sv.setFloat64(8 * k, v ?? NaN, true) : size === 4 ? sv.setFloat32(4 * k, v ?? NaN, true) : size === 2 ? sv.setInt16(2 * k, v ?? empty, true) : (strip[k] = v ?? empty),
  );
  const nodata = r.nodata === null ? null : String(r.nodata);
  // [tag, type (2 ASCII, 3 SHORT, 4 LONG), values]; the strip's offset is filled in below.
  const tags: [number, 2 | 3 | 4, number[] | string][] = [
    [256, 4, [r.width]],
    [257, 4, [r.height]],
    [258, 3, Array<number>(r.bands).fill(8 * size)],
    [259, 3, [1]],
    [262, 3, [r.bands === 3 ? 2 : 1]],
    [273, 4, [0]],
    [277, 3, [r.bands]],
    [278, 4, [r.height]],
    [279, 4, [strip.length]],
    [284, 3, [1]],
    [339, 3, Array<number>(r.bands).fill(float ? 3 : size === 2 ? 2 : 1)],
    ...(nodata === null ? [] : [[42113, 2, `${nodata}\0`] as [number, 2, string]]),
  ];
  const width = (t: 2 | 3 | 4) => (t === 2 ? 1 : t === 3 ? 2 : 4);
  const ifdEnd = 8 + 2 + 12 * tags.length + 4;
  let extra = ifdEnd;
  const places = tags.map(([, t, v]) => {
    const n = v.length * width(t);
    if (n <= 4) return -1;
    const at = extra;
    extra += n + (n & 1);
    return at;
  });
  const data = extra;
  const out = new Uint8Array(data + strip.length);
  const dv = new DataView(out.buffer);
  out.set([0x49, 0x49, 42, 0, 8, 0, 0, 0]);
  dv.setUint16(8, tags.length, true);
  const put = (at: number, t: 2 | 3 | 4, v: number[] | string) => {
    if (typeof v === 'string') for (let k = 0; k < v.length; k++) out[at + k] = v.charCodeAt(k);
    else v.forEach((x, k) => (t === 3 ? dv.setUint16(at + 2 * k, x, true) : dv.setUint32(at + 4 * k, x, true)));
  };
  tags.forEach(([tag, t, v], k) => {
    const e = 10 + 12 * k;
    dv.setUint16(e, tag, true);
    dv.setUint16(e + 2, t, true);
    dv.setUint32(e + 4, v.length, true);
    put(places[k] < 0 ? e + 8 : places[k], t, tag === 273 ? [data] : v);
    if (places[k] >= 0) dv.setUint32(e + 8, places[k], true);
  });
  out.set(strip, data);
  return out;
}

/** The units in the last place between two doubles (sign apart: as far as they come). */
function ulps64(a: number, b: number): number {
  const key = (v: number) => {
    const i = new DataView(new Float64Array([v]).buffer).getBigInt64(0, true);
    return i < 0n ? -9223372036854775808n - i : i;
  };
  const d = key(a) - key(b);
  return Number(d < 0n ? -d : d);
}

/** A sample against the reference's by its rule (`exact`: bit for bit; `sum`: one unit in the last place of the result's type, two in float64, one in an integer). */
function meets(got: number, want: number, rule: string, sample: string): boolean {
  if (Number.isNaN(got) || Number.isNaN(want)) return Number.isNaN(got) && Number.isNaN(want);
  if (rule === 'exact') return Object.is(got, want) || got === want;
  if (sample === 'f32') return ulps(got, want) <= 1;
  if (sample === 'f64') return ulps64(got, want) <= 2;
  return Math.abs(got - want) <= 1;
}

describe.skipIf(!surfaceModulesBuilt)('raster operations in the module (crates/wasm/raster-wasm, docs/adr/0233)', () => {
  const ops = json<{ cases: OpsCase[] }>('raster-ops/v1/cases.json');

  it('reads every case', () => {
    expect(ops.cases.length).toBeGreaterThanOrEqual(70);
  });

  for (const c of ops.cases) {
    it(`raster operation: ${c.name}`, async () => {
      const spec = JSON.stringify({ tool: c.tool, inputs: c.inputs.map((r) => ({ affine: r.affine, name: r.name })) });
      const run = analyzeOpsHere(c.inputs.map(tiffOf), spec, JSON.stringify(c.shapes));
      if (c.expect.refused) {
        await expect(run).rejects.toThrow(c.expect.refused);
        return;
      }
      const out = await run;
      const raster = c.expect.raster;
      if (raster) {
        expect([out.grid[6], out.grid[7], out.bands]).toEqual([raster.width, raster.height, raster.bands]);
        expect(out.grid.slice(0, 6)).toEqual(raster.affine);
        const got = await level0(out.bytes!);
        expect(got.length).toBe(raster.values.length);
        const off = got.findIndex((g, k) => !meets(g, raster.values[k] ?? NaN, raster.rule, out.sample));
        expect(off, `sample ${off}: ${got[off]} for ${raster.values[off]}`).toBe(-1);
      }
      if (c.expect.zones) {
        const z = out.zones!;
        expect(z.length).toBe(7 * c.expect.zones.length);
        c.expect.zones.forEach((w, k) => {
          expect(z[7 * k], `zone ${k}'s count`).toBe(w.n);
          expect(meets(z[7 * k + 1], w.value ?? NaN, w.rule, 'f64'), `zone ${k}: ${z[7 * k + 1]} for ${w.value}`).toBe(true);
        });
      }
      if (c.expect.histogram) expect(JSON.parse(out.histogram!)).toMatchObject(c.expect.histogram);
    });
  }
});

interface VectorFeatures {
  kind: string;
  values: (number | null)[];
  texts: string[];
  tags: number[];
  rings: number[];
  sizes: number[];
  xy: number[];
}
interface VectorCase {
  name: string;
  tool: { kind: string };
  inputs?: OpsInput[];
  shapes?: unknown[];
  values?: (string | null)[] | null;
  cell?: number;
  grid?: unknown;
  expect: {
    refused?: string;
    features?: VectorFeatures;
    raster?: { width: number; height: number; values: (number | null)[] };
    notes?: { taken: number; unread: number; outside: number; empty: number };
  };
}

/** Two lists of numbers alike bit for bit, NaN only where the case says none. */
const alike = (got: ArrayLike<number>, want: (number | null)[]) =>
  got.length === want.length && Array.from(got).every((g, k) => (want[k] === null ? Number.isNaN(g) : Object.is(g, want[k]) || g === want[k]));

describe.skipIf(!surfaceModulesBuilt)('raster and vector in the module (crates/wasm/raster-wasm, docs/adr/0234)', () => {
  const vector = json<{ cases: VectorCase[] }>('raster-vector/v1/cases.json');

  it('reads every case', () => {
    expect(vector.cases.length).toBeGreaterThanOrEqual(39);
  });

  for (const c of vector.cases) {
    it(`raster and vector: ${c.name}`, async () => {
      if (c.tool.kind === 'rasterize') {
        const spec = JSON.stringify({ tool: c.tool, cell: c.cell ?? 0, grid: c.grid ?? null });
        const run = analyzePointsHere(JSON.stringify(c.shapes), JSON.stringify(c.values ?? null), spec, true);
        if (c.expect.refused) {
          await expect(run).rejects.toThrow(c.expect.refused);
          return;
        }
        const out = await run;
        const raster = c.expect.raster!;
        expect([out.grid[6], out.grid[7]]).toEqual([raster.width, raster.height]);
        const got = await level0(out.bytes);
        const off = got.findIndex((g, k) => !alike([g], [raster.values[k]]));
        expect(got.length).toBe(raster.values.length);
        expect(off, `sample ${off}: ${got[off]} for ${raster.values[off]}`).toBe(-1);
        expect(JSON.parse(out.notes)).toMatchObject(c.expect.notes!);
        return;
      }
      const inputs = c.inputs!.map((r) => ({ ...r, name: 'A' }));
      const spec = JSON.stringify({ tool: c.tool, inputs: inputs.map((r) => ({ affine: r.affine, name: r.name })) });
      const run = analyzeOpsHere(inputs.map(tiffOf), spec, '[]');
      if (c.expect.refused) {
        await expect(run).rejects.toThrow(c.expect.refused);
        return;
      }
      const f = (await run).features!;
      const want = c.expect.features!;
      expect(f.kind).toBe(want.kind);
      expect(alike(f.values, want.values), `values ${Array.from(f.values)}`).toBe(true);
      expect(f.texts).toEqual(want.texts);
      expect(Array.from(f.tags)).toEqual(want.tags);
      expect(Array.from(f.rings)).toEqual(want.rings);
      expect(Array.from(f.sizes)).toEqual(want.sizes);
      expect(alike(f.xy, want.xy), `xy ${Array.from(f.xy).slice(0, 16)}`).toBe(true);
    });
  }
});

interface HydroCase {
  name: string;
  input: OpsInput;
  geographic: boolean;
  tool: { kind: string };
  shapes: unknown[];
  expect: {
    refused?: string;
    rule?: string;
    raster?: { width: number; height: number; rule: string; values: (number | null)[] };
    features?: { kind: string; fields: string[]; numbers: number[]; rings: number[]; sizes: number[]; xy: number[] };
    notes?: Record<string, number | number[]>;
  };
}

/** A sample against the reference's (null: none) by its rule (`exact` bit for bit, `f32ulp` one unit in float32). */
const hydroMeets = (got: number, want: number | null, rule: string) =>
  want === null ? Number.isNaN(got) : rule === 'f32ulp' ? ulps(got, want) <= 1 : Object.is(got, want) || got === want;

describe.skipIf(!surfaceModulesBuilt)('hydrology in the module (crates/wasm/raster-wasm, docs/adr/0235)', () => {
  const hydro = json<{ cases: HydroCase[] }>('hydrology/v1/cases.json');

  it('reads every case', () => {
    expect(hydro.cases.length).toBeGreaterThanOrEqual(130);
  });

  for (const c of hydro.cases) {
    it(`hydrology: ${c.name}`, async () => {
      const input = { ...c.input, name: 'A' };
      const spec = JSON.stringify({ tool: c.tool, inputs: [{ affine: input.affine, name: 'A' }], geographic: c.geographic });
      const run = analyzeOpsHere([tiffOf(input)], spec, JSON.stringify(c.shapes));
      if (c.expect.refused) {
        await expect(run).rejects.toThrow(c.expect.refused);
        return;
      }
      const out = await run;
      const notes = (JSON.parse(out.notes) as { hydro: Record<string, number | number[]> }).hydro;
      if (c.expect.raster) {
        const r = c.expect.raster;
        expect([out.grid[6], out.grid[7]]).toEqual([r.width, r.height]);
        const got = await level0(out.bytes!);
        expect(got.length).toBe(r.values.length);
        const off = got.findIndex((g, k) => !hydroMeets(g, r.values[k], r.rule));
        expect(off, `sample ${off}: ${got[off]} for ${r.values[off]}`).toBe(-1);
      } else {
        const f = out.features!;
        const want = c.expect.features!;
        expect(f.kind).toBe(want.kind);
        expect(f.fields).toEqual(want.fields);
        const km = want.fields.indexOf('Km');
        const stride = want.fields.length;
        expect(f.numbers.length).toBe(want.numbers.length);
        want.numbers.forEach((w, k) => {
          const g = f.numbers[k];
          if (c.expect.rule === 'km' && k % stride === km) expect(Math.abs(g - w), `number ${k}`).toBeLessThanOrEqual(1e-9);
          else expect(Object.is(g, w) || g === w, `number ${k}: ${g} for ${w}`).toBe(true);
        });
        expect(Array.from(f.rings)).toEqual(want.rings);
        expect(Array.from(f.sizes)).toEqual(want.sizes);
        expect(Array.from(f.xy)).toEqual(want.xy);
      }
      // Hydrology's notes in the module's words: the reference's `empty` list is `emptyPoints`.
      for (const [key, value] of Object.entries(c.expect.notes ?? {})) {
        const k = key === 'empty' && Array.isArray(value) ? 'emptyPoints' : key;
        expect(notes[k], `note ${key}`).toEqual(value);
      }
    });
  }
});

interface DistanceCase {
  name: string;
  job: 'ops' | 'points';
  input?: OpsInput;
  surface?: OpsInput | null;
  geographic?: boolean;
  tool: { kind: string };
  spec?: Record<string, unknown>;
  shapes: unknown[];
  expect: {
    refused?: string;
    raster?: { width: number; height: number; rule: string; affine?: number[]; values: (number | null)[] };
    features?: { kind: string; fields: string[]; numbers: number[]; rings: number[]; sizes: number[]; xy: number[] };
    notes?: Record<string, number | number[] | null>;
  };
}

describe.skipIf(!surfaceModulesBuilt)('distance and cost in the module (crates/wasm/raster-wasm, docs/adr/0236)', () => {
  const distance = json<{ cases: DistanceCase[] }>('distance/v1/cases.json');

  it('reads every case', () => {
    expect(distance.cases.length).toBeGreaterThanOrEqual(39);
  });

  for (const c of distance.cases) {
    it(`distance: ${c.name}`, async () => {
      if (c.job === 'points') {
        // Uzaklık yüzeyi from objects: the point job on the objects' box with its margin, or a raster's grid.
        const run = analyzePointsHere(JSON.stringify(c.shapes), 'null', JSON.stringify(c.spec), true);
        if (c.expect.refused) {
          await expect(run).rejects.toThrow(c.expect.refused);
          return;
        }
        const out = await run;
        const r = c.expect.raster!;
        expect(Array.from(out.grid.slice(0, 6))).toEqual(r.affine);
        const got = await level0(out.bytes);
        const off = got.findIndex((g, k) => !hydroMeets(g, r.values[k], r.rule));
        expect(off, `sample ${off}: ${got[off]} for ${r.values[off]}`).toBe(-1);
        expect(JSON.parse(out.notes)).toMatchObject(c.expect.notes!);
        return;
      }
      // The cost raster first, the surface second.
      const inputs = [{ ...c.input!, name: 'A' }, ...(c.surface ? [{ ...c.surface, name: 'B' }] : [])];
      const spec = JSON.stringify({ tool: c.tool, inputs: inputs.map((r) => ({ affine: r.affine, name: r.name })), geographic: c.geographic ?? false });
      const run = analyzeOpsHere(inputs.map(tiffOf), spec, JSON.stringify(c.shapes));
      if (c.expect.refused) {
        await expect(run).rejects.toThrow(c.expect.refused);
        return;
      }
      const out = await run;
      const notes = (JSON.parse(out.notes) as { distance: Record<string, number | number[] | null> }).distance;
      const r = c.expect.raster;
      if (r) {
        expect([out.grid[6], out.grid[7]]).toEqual([r.width, r.height]);
        const got = await level0(out.bytes!);
        expect(got.length).toBe(r.values.length);
        const off = got.findIndex((g, k) => !hydroMeets(g, r.values[k], r.rule));
        expect(off, `sample ${off}: ${got[off]} for ${r.values[off]}`).toBe(-1);
      } else {
        const f = out.features!;
        const want = c.expect.features!;
        expect(f.kind).toBe(want.kind);
        expect(f.fields).toEqual(want.fields);
        expect(Array.from(f.numbers)).toEqual(want.numbers);
        expect(Array.from(f.sizes)).toEqual(want.sizes);
        expect(Array.from(f.xy)).toEqual(want.xy);
      }
      // The notes in the module's words (least and most null when no cell has a value), by the raster's rule.
      for (const [key, value] of Object.entries(c.expect.notes ?? {})) {
        const got = notes[key];
        if (r?.rule === 'f32ulp' && typeof value === 'number' && typeof got === 'number') expect(ulps(got, value), `note ${key}`).toBeLessThanOrEqual(1);
        else expect(got, `note ${key}`).toEqual(value);
      }
    });
  }
});

interface SuitCase {
  name: string;
  tool: { kind: string };
  inputs: OpsInput[];
  names: string[];
  shapes: unknown[];
  expect: {
    refused?: string;
    raster?: { width: number; height: number; rule: string; values: (number | null)[] };
    notes?: Record<string, number>;
    pairwise?: { weights: number[]; lambda: number; ci: number; ri: number; cr: number };
    roc?: Record<string, unknown>;
  };
}

/** A sample against the reference's by its rule: also `f64ulp`, one unit in float64 (libm's exp and pow). */
const suitMeets = (got: number, want: number | null, rule: string) =>
  want === null ? Number.isNaN(got) : rule === 'f64ulp' ? ulps64(got, want) <= 1 : hydroMeets(got, want, rule);

describe.skipIf(!surfaceModulesBuilt)('suitability in the module (crates/wasm/raster-wasm, docs/adr/0237)', () => {
  const suit = json<{ cases: SuitCase[] }>('suitability/v1/cases.json');

  it('reads every case', () => {
    expect(suit.cases.length).toBeGreaterThanOrEqual(50);
  });

  for (const c of suit.cases) {
    it(`suitability: ${c.name}`, async () => {
      const spec = JSON.stringify({ tool: c.tool, inputs: c.inputs.map((r, k) => ({ affine: r.affine, name: c.names[k] })) });
      const run = analyzeOpsHere(c.inputs.map(tiffOf), spec, JSON.stringify(c.shapes));
      if (c.expect.refused) {
        await expect(run).rejects.toThrow(c.expect.refused);
        return;
      }
      const out = await run;
      const notes = (JSON.parse(out.notes) as { suit: Record<string, unknown> & { pairwise: SuitCase['expect']['pairwise'] | null } }).suit;
      const r = c.expect.raster;
      if (r) {
        expect([out.grid[6], out.grid[7]]).toEqual([r.width, r.height]);
        // Ağırlıklı çakıştırma's 32-bit integer nodata is no value.
        const got = (await level0(out.bytes!)).map((v) => (v === -2147483648 ? NaN : v));
        expect(got.length).toBe(r.values.length);
        const off = got.findIndex((g, k) => !suitMeets(g, r.values[k], r.rule));
        expect(off, `sample ${off}: ${got[off]} for ${r.values[off]}`).toBe(-1);
      }
      if (c.expect.roc) expect(JSON.parse(out.roc!)).toEqual(c.expect.roc);
      for (const [key, value] of Object.entries(c.expect.notes ?? {})) expect(notes[key], `note ${key}`).toBe(value);
      const p = c.expect.pairwise;
      if (p) {
        const got = notes.pairwise!;
        expect(got.weights.length).toBe(p.weights.length);
        got.weights.forEach((w, k) => expect(Math.abs(w - p.weights[k]), `weight ${k}`).toBeLessThanOrEqual(1e-12));
        for (const k of ['lambda', 'ci', 'ri', 'cr'] as const) expect(Math.abs(got[k] - p[k]), k).toBeLessThanOrEqual(1e-10);
      }
    });
  }
});
