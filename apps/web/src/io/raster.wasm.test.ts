import { describe, expect, it } from 'vitest';
import { analyzeHere, level0, surfaceModulesBuilt, ulps } from '../processing/surfaceTesting';

/**
 * The raster analyses as the browser runs them (docs/adr/0231): the raster analysis module (crates/wasm/raster-wasm →
 * src/io/raster/pkg, built by `pnpm wasm`) on one thread, as the analysis worker runs it, over every case of the
 * independent references the core plays natively (crates/shared/raster/tests/all): the surface's
 * (scripts/fixtures/terrain_cases.py; a 32-bit sample within one unit in the last place, a byte exact) and the
 * contours' (scripts/fixtures/contour_cases.py; the points bit for bit, the Kot texts). Skipped only when the
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
});
