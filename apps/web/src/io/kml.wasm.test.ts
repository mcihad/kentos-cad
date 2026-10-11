import { describe, expect, it } from 'vitest';
import type { ImportResult } from '../contracts/generated/ImportResult';

/**
 * KML and KMZ as the browser reads and writes them (docs/adr/0222): the formats WASM module on the shared fixtures
 * (fixtures/formats/v1/kml, scripts/fixtures/kml_cases.py: OGC KML 2.2 and the ADR's rules with Python's expat and
 * PROJ, no KentOS code), each sample read float for float as the reference reads it, each reading moved into a
 * project's system within the reference's tolerance, the writer's files as committed; the GPX and RFC 7946 GeoJSON
 * writers' files likewise (fixtures/formats/v1/wgs84). crates/shared/formats/tests/all/kml.rs and wgs84_write.rs read
 * the same files natively. Skipped only when the package has not been built.
 */

interface Written {
  takeBytes(): Uint8Array;
  readonly report: string;
  free(): void;
}

interface Formats {
  initSync(o: { module: BufferSource }): unknown;
  readKml(bytes: Uint8Array, max: number): Uint8Array;
  reprojectImport(result: string, from: string, to: string, choices: string): Uint8Array;
  writeKml(input: string, from: string, choices: string): Written;
  writeGpx(input: string, from: string, choices: string): Written;
  writeGeoJsonWgs84(input: string, from: string, choices: string): Written;
}

const glue = import.meta.glob<Formats>('./pkg/kentos_formats_wasm.js');
const loader = Object.values(glue)[0];
type Fs = { readFileSync(u: URL, enc?: 'utf8'): Uint8Array<ArrayBuffer> & string; readdirSync(u: URL): string[] };
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): Fs } }).process.getBuiltinModule('node:fs');
const kml = (name: string) => new URL(`../../../../fixtures/formats/v1/kml/${name}`, import.meta.url);
const wgs84 = (name: string) => new URL(`../../../../fixtures/formats/v1/wgs84/${name}`, import.meta.url);

let loaded: Formats | undefined;
async function load(): Promise<Formats> {
  if (loaded) return loaded;
  const w = await loader!();
  w.initSync({ module: fs.readFileSync(new URL('./pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
  return (loaded = w);
}

const decode = <T>(b: Uint8Array) => JSON.parse(new TextDecoder().decode(b)) as T;
const xy = (p: { x: number; y: number }) => [p.x, p.y];

type Ring = { pts: { x: number; y: number }[]; zs?: (number | null)[] | null };

function area(o: Record<string, unknown>, a: Ring & { holes?: Ring[] | null }): Record<string, unknown> {
  o.pts = a.pts.map(xy);
  if (a.zs) o.zs = a.zs;
  if (a.holes?.length) {
    o.holes = a.holes.map((h) => h.pts.map(xy));
    if (a.holes.some((h) => h.zs)) o.holeZs = a.holes.map((h) => h.zs ?? null);
  }
  return o;
}

/** An object in the reference's form: its geometry, layer, attributes, colour and line weight. */
function object(e: ImportResult['entities'][number]): Record<string, unknown> {
  const o: Record<string, unknown> = { kind: e.kind };
  if (e.kind === 'point') {
    o.p = xy(e.p);
    if (e.z != null) o.z = e.z;
    if (e.parts?.length) o.parts = e.parts.map((q) => ({ p: xy(q.p), ...(q.z != null && { z: q.z }) }));
  } else if (e.kind === 'line') {
    o.a = xy(e.a);
    o.b = xy(e.b);
    if (e.za != null) o.za = e.za;
    if (e.zb != null) o.zb = e.zb;
  } else if (e.kind === 'polyline') {
    o.pts = e.pts.map(xy);
    if (e.zs) o.zs = e.zs;
    if (e.parts?.length) o.parts = e.parts.map((q) => ({ pts: q.pts.map(xy), ...(q.zs && { zs: q.zs }) }));
  } else if (e.kind === 'polygon') {
    area(o, e);
    if (e.parts?.length) o.parts = e.parts.map((q) => area({}, q));
  }
  o.layer = e.layerId;
  o.attrs = e.attrs;
  if (e.color) o.color = e.color;
  if (e.lineWeight !== undefined && e.lineWeight !== null) o.lineWeight = e.lineWeight;
  return o;
}

function canonical(r: ImportResult): unknown {
  return {
    objects: r.entities.map(object),
    layers: r.layers.map((l) => ({ name: l.name, color: l.color, visible: l.visible, count: l.count })),
    counts: r.report.counts,
    skipped: r.report.skipped.map((i) => ({ what: i.what, reason: i.reason, count: i.count, lines: i.lines })),
    notes: r.report.notes.map((i) => ({ what: i.what, reason: i.reason, count: i.count, lines: i.lines })),
    source: r.report.source.map((f) => [f.label, f.value]),
    declared: r.declaredCrs,
  };
}

/** Numbers within `tol` (0: equal), everything else equal; the first difference's path. */
function differ(got: unknown, want: unknown, path: string, tol: number): string | null {
  if (typeof got === 'number' && typeof want === 'number') return (tol ? Math.abs(got - want) <= tol : Object.is(got, want)) ? null : `${path}: ${got} ≠ ${want}`;
  if (Array.isArray(got) && Array.isArray(want)) {
    if (got.length !== want.length) return `${path}: ${got.length} öğe ≠ ${want.length}`;
    for (let i = 0; i < got.length; i++) {
      const d = differ(got[i], want[i], `${path}[${i}]`, tol);
      if (d) return d;
    }
    return null;
  }
  if (got && want && typeof got === 'object' && typeof want === 'object') {
    const keys = new Set([...Object.keys(got), ...Object.keys(want)]);
    for (const k of [...keys].sort()) {
      const d = differ((got as Record<string, unknown>)[k], (want as Record<string, unknown>)[k], `${path}.${k}`, tol);
      if (d) return d;
    }
    return null;
  }
  return got === want ? null : `${path}: ${JSON.stringify(got)} ≠ ${JSON.stringify(want)}`;
}

interface Cases {
  tolerance: { metres: number };
  reads: { file: string; expect: { error?: string } & Record<string, unknown> }[];
  moves: { file: string; srid: number; system: unknown; objects: unknown[]; moved: Record<string, unknown> }[];
}

const cases = JSON.parse(fs.readFileSync(kml('cases.json'), 'utf8')) as Cases;
const WGS84 = JSON.stringify({ kind: 'geographic', datum: 'WGS84' });

describe.skipIf(!loader)('KML, KMZ, GPX and GeoJSON in WGS 84 through WASM (docs/adr/0222)', () => {
  it('reads every sample as the reference reads it', async () => {
    const w = await load();
    for (const c of cases.reads) {
      let got: unknown;
      try {
        got = canonical(decode<ImportResult>(w.readKml(fs.readFileSync(kml(c.file)), 0)));
      } catch (e) {
        expect((e as Error).message, c.file).toBe(c.expect.error);
        continue;
      }
      expect(c.expect.error, c.file).toBeUndefined();
      expect(differ(got, c.expect, c.file, 0)).toBeNull();
    }
  });

  it('moves a reading into a project system as PROJ does', async () => {
    const w = await load();
    for (const m of cases.moves) {
      const read = new TextDecoder().decode(w.readKml(fs.readFileSync(kml(m.file)), 0));
      const out = decode<{ result: ImportResult; moved: Record<string, unknown> }>(w.reprojectImport(read, WGS84, JSON.stringify(m.system), '[]'));
      expect([out.moved.objects, out.moved.vertices, out.moved.accuracy, out.moved.via, out.moved.unofficial], m.file).toEqual([
        m.moved.objects,
        m.moved.vertices,
        m.moved.accuracy,
        m.moved.via,
        m.moved.unofficial,
      ]);
      expect(differ(out.result.entities.map(object), m.objects, `${m.file} → ${m.srid}`, cases.tolerance.metres)).toBeNull();
    }
  });

  /** Byte for byte (the test runs without Node's types, so no Buffer). */
  const same = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((v, i) => v === b[i]);

  it('writes the committed KML, KMZ, GPX and GeoJSON files', async () => {
    const w = await load();
    const check = (dir: (name: string) => URL, name: string, ext: string, written: Written) => {
      try {
        const bytes = written.takeBytes();
        expect(same(bytes, fs.readFileSync(dir(`${name}.${ext}`))), `${name}.${ext}`).toBe(true);
      } finally {
        written.free();
      }
    };
    const kmlCases = fs.readdirSync(kml('write/')).filter((f) => f.endsWith('.case.json'));
    expect(kmlCases.length).toBeGreaterThanOrEqual(2);
    for (const f of kmlCases) {
      const c = JSON.parse(fs.readFileSync(kml(`write/${f}`), 'utf8')) as { system: unknown; choices: unknown[]; input: { kmz?: boolean } };
      const name = f.replace('.case.json', '');
      check((n) => kml(`write/${n}`), name, c.input.kmz ? 'kmz' : 'kml', w.writeKml(JSON.stringify(c.input), JSON.stringify(c.system), JSON.stringify(c.choices)));
    }
    for (const f of fs.readdirSync(wgs84('')).filter((f) => f.endsWith('.case.json'))) {
      const c = JSON.parse(fs.readFileSync(wgs84(f), 'utf8')) as { format: 'gpx' | 'geojson'; system: unknown; choices: unknown[]; input: unknown };
      const name = f.replace('.case.json', '');
      const args = [JSON.stringify(c.input), JSON.stringify(c.system), JSON.stringify(c.choices)] as const;
      check(wgs84, name, c.format, c.format === 'gpx' ? w.writeGpx(...args) : w.writeGeoJsonWgs84(...args));
    }
  });
});
