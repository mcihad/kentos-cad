// KML, GPX and GeoJSON in WGS 84 on the web (docs/adr/0222 §9), the desktop's `kml_timing`
// (crates/shared/formats/tests/all/kml_timing.rs) value for value: 100 000 objects of a TUREF / TM36 drawing (40 000
// five-corner parcels, 30 000 three-corner paths, 30 000 points with elevations; 350 000 vertices) written as KML and
// KMZ (each vertex moved into WGS 84), read back, the reading moved into the project's system, GPX and RFC 7946 GeoJSON
// written; through the formats WASM as its worker calls it (the input's JSON made here too); the best of three after a
// warm-up against the ADR's budgets. Runs only on purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm:formats; KML_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/kml.test.ts --disable-console-intercept; rm apps/web/src/io/pkg/.stamp
import { expect, it } from 'vitest';

interface Written {
  takeBytes(): Uint8Array;
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

type Fs = { readFileSync(u: URL): Uint8Array<ArrayBuffer> };
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): Fs } }).process.getBuiltinModule('node:fs');
const glue = import.meta.glob<Formats>('../../src/io/pkg/kentos_formats_wasm.js');
const loader = Object.values(glue)[0];

const TM36 = JSON.stringify({ kind: 'tm', datum: 'TUREF', centralMeridian: 36, scaleFactor: 1, falseEasting: 500000, falseNorthing: 0 });
const WGS84 = JSON.stringify({ kind: 'geographic', datum: 'WGS84' });

/** The desktop's scene, object for object. */
function scene(): unknown[] {
  const [e0, n0] = [480_000, 4_410_000];
  const base = (id: number, layerId: string) => ({ id, layerId, attrs: { Ad: String(id) } });
  const out: unknown[] = [];
  for (let i = 0; i < 40_000; i++) {
    const [x, y] = [e0 + (i % 200) * 50, n0 + Math.floor(i / 200) * 50];
    out.push({ kind: 'polygon', ...base(i + 1, 'parsel'), pts: [{ x, y }, { x: x + 40, y }, { x: x + 45, y: y + 20 }, { x: x + 40, y: y + 40 }, { x, y: y + 40 }] });
  }
  for (let i = 0; i < 30_000; i++) {
    const [x, y] = [e0 + (i % 150) * 66, n0 + Math.floor(i / 150) * 50 + 5];
    out.push({ kind: 'polyline', ...base(40_001 + i, 'yol'), pts: [{ x, y }, { x: x + 30, y: y + 3 }, { x: x + 60, y }] });
  }
  for (let i = 0; i < 30_000; i++) {
    const [x, y] = [e0 + (i % 150) * 66 + 3, n0 + Math.floor(i / 150) * 50 + 25];
    out.push({ kind: 'point', ...base(70_001 + i, 'nokta'), p: { x, y }, z: 850 + (i % 97) });
  }
  return out;
}

function time<T>(what: string, budget: number, f: () => T): T {
  f();
  let best = Infinity;
  let out: T | undefined;
  for (let i = 0; i < 3; i++) {
    const t = performance.now();
    out = f();
    best = Math.min(best, performance.now() - t);
  }
  console.log(`${what}: ${best.toFixed(0)} ms (bütçe ${budget} ms)`);
  expect(best, what).toBeLessThanOrEqual(budget);
  return out!;
}

const taken = (w: Written): Uint8Array => {
  try {
    return w.takeBytes();
  } finally {
    w.free();
  }
};

it.skipIf(!process.env.KML_BENCH || !loader)('KML, KMZ, GPX and GeoJSON in WGS 84 within their budgets (docs/adr/0222 §9)', async () => {
  const w = await loader!();
  w.initSync({ module: fs.readFileSync(new URL('../../src/io/pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
  const entities = scene();
  const layers = ['parsel', 'yol', 'nokta'].map((id) => ({ id, name: id, path: ['Ölçüm'], color: '#336699', lineWeight: 0.25 }));
  const kml = time("KML yaz (100 000 nesne, 350 000 köşe WGS 84'e)", 2500, () =>
    taken(w.writeKml(JSON.stringify({ entities, layers, name: 'Ölçüm', kmz: false }), TM36, '[]')),
  );
  const kmz = time('KMZ yaz', 4000, () => taken(w.writeKml(JSON.stringify({ entities, layers, name: 'Ölçüm', kmz: true }), TM36, '[]')));
  const read = time('KML oku', 2500, () => w.readKml(kml, 0));
  time('KMZ oku', 3000, () => w.readKml(kmz, 0));
  const text = new TextDecoder().decode(read);
  time('Projenin sistemine çevir (100 000 nesne)', 2500, () => w.reprojectImport(text, WGS84, TM36, '[]'));
  time('GPX yaz', 2500, () => taken(w.writeGpx(JSON.stringify({ entities, name: 'Ölçüm', routes: false }), TM36, '[]')));
  time("GeoJSON yaz (WGS 84'e)", 2500, () =>
    taken(w.writeGeoJsonWgs84(JSON.stringify({ entities, layers: layers.map(({ id, name }) => ({ id, name })), srid: 5256, name: 'Ölçüm' }), TM36, '[]')),
  );
}, 600_000);
