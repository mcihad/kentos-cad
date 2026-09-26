import { describe, expect, it } from 'vitest';
import type { ExportReport } from '../contracts/generated/ExportReport';
import type { ImportResult } from '../contracts/generated/ImportResult';

/**
 * GeoJSON and Shapefile as the browser reads and writes them (docs/adr/0046):
 * the formats WASM module on the shared fixtures (fixtures/formats/v1/gis),
 * compared with what the independent reader (tools/formats/gis.py) wrote
 * for them, float for float; crates/shared/formats/tests/gis.rs reads the
 * same files natively. The exports in `export/` are written to the
 * committed bytes. Skipped only when the package has not been built.
 */

interface Formats {
  initSync(o: { module: BufferSource }): unknown;
  readGeoJson(bytes: Uint8Array, options: string): Uint8Array;
  readShapefile(shp: Uint8Array, shx: Uint8Array | undefined, dbf: Uint8Array | undefined, prj: Uint8Array | undefined, cpg: Uint8Array | undefined, options: string): Uint8Array;
  writeGeoJson(input: string): { takeBytes(): Uint8Array; readonly report: string; free(): void };
}

const glue = import.meta.glob<Formats>('./pkg/kentos_formats_wasm.js');
const loader = Object.values(glue)[0];
type Fs = { readFileSync(u: URL): Uint8Array<ArrayBuffer>; existsSync(u: URL): boolean; readdirSync(u: URL): string[] };
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): Fs } }).process.getBuiltinModule('node:fs');
const at = (name: string) => new URL(`../../../../fixtures/formats/v1/gis/${name}`, import.meta.url);
const maybe = (name: string) => (fs.existsSync(at(name)) ? fs.readFileSync(at(name)) : undefined);

let loaded: Formats | undefined;
async function load(): Promise<Formats> {
  if (loaded) return loaded;
  const w = await loader!();
  w.initSync({ module: fs.readFileSync(new URL('./pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
  return (loaded = w);
}

const decode = (b: Uint8Array) => JSON.parse(new TextDecoder().decode(b)) as ImportResult;
const xy = (p: { x: number; y: number }) => [p.x, p.y];

/** The reading in the rules' canonical form (docs/adr/0046). */
function canonical(r: ImportResult, encoding?: string): unknown {
  const objects = r.entities.map((e) => {
    const o: Record<string, unknown> = { kind: e.kind, layer: e.layerId };
    if (e.kind === 'point') {
      o.p = xy(e.p);
      if (e.z !== undefined && e.z !== null) o.z = e.z;
    } else if (e.kind === 'line') {
      o.a = xy(e.a);
      o.b = xy(e.b);
    } else if (e.kind === 'polyline' || e.kind === 'polygon') {
      o.pts = e.pts.map(xy);
      if (e.kind === 'polygon' && e.holes?.length) o.holes = e.holes.map((h) => h.pts.map(xy));
    } else throw new Error(`a GIS reader made a ${e.kind}`);
    if (e.label !== undefined && e.label !== null) o.label = e.label;
    o.attrs = e.attrs;
    return o;
  });
  return { declaredSrid: r.declaredCrs?.srid ?? null, ...(encoding ? { encoding } : {}), objects };
}

/** Equal as JSON, numbers by Object.is (−0 is not 0); the first difference's path. */
function diff(got: unknown, want: unknown, path: string): string | null {
  if (typeof got === 'number' && typeof want === 'number') return Object.is(got, want) ? null : `${path}: ${got} ≠ ${want}`;
  if (Array.isArray(got) && Array.isArray(want)) {
    if (got.length !== want.length) return `${path}: ${got.length} öğe ≠ ${want.length}`;
    for (let i = 0; i < got.length; i++) {
      const d = diff(got[i], want[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (got && want && typeof got === 'object' && typeof want === 'object') {
    const g = got as Record<string, unknown>;
    const w = want as Record<string, unknown>;
    for (const k of new Set([...Object.keys(g), ...Object.keys(w)])) {
      if (!(k in g) || !(k in w)) return `${path}.${k}: ${JSON.stringify(g[k])} ≠ ${JSON.stringify(w[k])}`;
      const d = diff(g[k], w[k], `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return got === want ? null : `${path}: ${JSON.stringify(got)} ≠ ${JSON.stringify(want)}`;
}

const names = Object.keys(glue).length ? fs.readdirSync(at('')).filter((n) => n.endsWith('.expected.json')).map((n) => n.slice(0, -'.expected.json'.length)).sort() : [];

async function read(name: string): Promise<{ r: ImportResult; encoding?: string }> {
  const w = await load();
  const geo = maybe(`${name}.geojson`);
  if (geo) return { r: decode(w.readGeoJson(geo, JSON.stringify({ layer: name, maxEntities: 0 }))) };
  const r = decode(w.readShapefile(fs.readFileSync(at(`${name}.shp`)), maybe(`${name}.shx`), maybe(`${name}.dbf`), maybe(`${name}.prj`), maybe(`${name}.cpg`), JSON.stringify({ layer: name, maxEntities: 0 })));
  // The encoding by its usual name: the fact starts with it ("Windows-1254 (Türkçe) (dil sürücüsü 0xCA)").
  const fact = r.report.source.find((f) => f.label === 'Kodlama')?.value ?? '';
  return { r, encoding: fact.split(' (')[0] };
}

describe.skipIf(!loader)('GIS files in the formats WASM module (docs/adr/0046)', () => {
  it('has the fixtures', () => {
    expect(names.length).toBeGreaterThanOrEqual(12);
  });

  it.each(names)('%s reads as the independent reader reads it', async (name) => {
    const want = JSON.parse(new TextDecoder().decode(fs.readFileSync(at(`${name}.expected.json`))));
    const { r, encoding } = await read(name);
    expect(diff(canonical(r, encoding), want, name)).toBeNull();
  });

  it('writes the exports to the committed bytes, and reads them back', async () => {
    const w = await load();
    const exports = fs.readdirSync(at('export/')).filter((n) => n.endsWith('.input.json'));
    expect(exports.length).toBeGreaterThan(0);
    for (const n of exports) {
      const name = n.slice(0, -'.input.json'.length);
      const input = new TextDecoder().decode(fs.readFileSync(at(`export/${n}`)));
      const out = w.writeGeoJson(input);
      const bytes = out.takeBytes();
      const report = JSON.parse(out.report) as ExportReport;
      out.free();
      expect(new TextDecoder().decode(bytes), name).toBe(new TextDecoder().decode(fs.readFileSync(at(`export/${name}.geojson`))));
      expect(Object.values(report.counts).reduce((s, c) => s + (c ?? 0), 0)).toBeGreaterThan(0);
      const back = decode(w.readGeoJson(bytes, JSON.stringify({ layer: name, maxEntities: 0 })));
      expect(back.declaredCrs?.srid).toBe((JSON.parse(input) as { srid: number }).srid);
    }
  });

  it('refuses what is not GeoJSON or not a Shapefile, with the reason', async () => {
    const w = await load();
    expect(() => w.readGeoJson(new TextEncoder().encode('{"type":"Topology"}'), JSON.stringify({ layer: 'x', maxEntities: 0 }))).toThrow(/Topology/);
    expect(() => w.readShapefile(new Uint8Array(100), undefined, undefined, undefined, undefined, JSON.stringify({ layer: 'x', maxEntities: 0 }))).toThrow(/9994/);
  });
});
