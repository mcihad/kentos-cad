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
 * docs/adr/0233: each sample by its case's rule, the zones' figures, the histograms, the refusals). Skipped only when the
 * packages have not been built.
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
 * A reference input as an uncompressed TIFF in one strip: a 32-bit float band (its nodata the GDAL_NODATA tag) or bytes
 * (three bands RGB). The core reads its own tiled files (crates/shared/raster/tests/all/raster_ops.rs) and these alike;
 * the place is the run's, from the settings.
 */
function tiffOf(r: OpsInput): Uint8Array {
  if (r.alpha || !((r.sample === 'f32' && r.bands === 1) || (r.sample === 'u8' && (r.bands === 1 || r.bands === 3)))) throw new Error(`${r.name}: not an input this writer makes`);
  const float = r.sample === 'f32';
  const strip = new Uint8Array(r.width * r.height * r.bands * (float ? 4 : 1));
  const sv = new DataView(strip.buffer);
  r.values.forEach((v, k) => (float ? sv.setFloat32(4 * k, v ?? NaN, true) : (strip[k] = v ?? 0)));
  const nodata = r.nodata === null ? null : String(r.nodata);
  // [tag, type (2 ASCII, 3 SHORT, 4 LONG), values]; the strip's offset is filled in below.
  const tags: [number, 2 | 3 | 4, number[] | string][] = [
    [256, 4, [r.width]],
    [257, 4, [r.height]],
    [258, 3, Array<number>(r.bands).fill(float ? 32 : 8)],
    [259, 3, [1]],
    [262, 3, [r.bands === 3 ? 2 : 1]],
    [273, 4, [0]],
    [277, 3, [r.bands]],
    [278, 4, [r.height]],
    [279, 4, [strip.length]],
    [284, 3, [1]],
    [339, 3, Array<number>(r.bands).fill(float ? 3 : 1)],
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
