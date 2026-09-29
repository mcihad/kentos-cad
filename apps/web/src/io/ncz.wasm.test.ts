import { describe, expect, it } from 'vitest';
import type { Entity } from '../contracts/generated/Entity';
import type { ImportResult } from '../contracts/generated/ImportResult';
import { ColumnsReader } from './columns';
import { FORMATS_VERSION } from './version';

/**
 * Netcad NCZ as the browser reads it (docs/adr/0138): the NCZ WASM module
 * (crates/wasm/ncz-wasm → src/io/ncz/pkg, built by `pnpm wasm`; the formats
 * worker loads it the first time an NCZ is imported) on the fixtures that
 * scripts/fixtures/ncz_reference.py writes block by block, with the values
 * worked out from that script; crates/shared/ncz/tests/fixtures.rs reads the
 * same files natively and expects the same. Skipped only when the package
 * has not been built.
 */

interface Imported {
  readonly ok: boolean;
  readonly message: string;
  takeHead(): string;
  takeKinds(): Uint8Array;
  takeUids(): Uint8Array;
  takeInts(): Uint32Array;
  takeFloats(): Float64Array;
  takeText(): Uint16Array;
  takeTextLengths(): Uint32Array;
  free(): void;
}

interface Ncz {
  initSync(o: { module: BufferSource }): unknown;
  nczVersion(): number;
  readNcz(bytes: Uint8Array, options: string, progress: { step(done: number, total: number): void }): Imported;
}

const glue = import.meta.glob<Ncz>('./ncz/pkg/kentos_ncz_wasm.js');
const loader = Object.values(glue)[0];
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');
const fixture = (name: string) => fs.readFileSync(new URL(`../../../../fixtures/formats/v1/ncz/${name}`, import.meta.url));

let loaded: Ncz | undefined;
async function load(): Promise<Ncz> {
  if (loaded) return loaded;
  const w = await loader!();
  w.initSync({ module: fs.readFileSync(new URL('./ncz/pkg/kentos_ncz_wasm_bg.wasm', import.meta.url)) });
  return (loaded = w);
}

const options = JSON.stringify({ maxEntities: 0, drawingFont: '' });
const quiet = { step: () => {} };
const N0 = 4_448_000;
const E0 = 421_000;

/** A read's result: the head, and the objects read back from the columns. */
function imported(r: Imported): ImportResult {
  try {
    if (!r.ok) throw new Error(r.message);
    const columns = { kinds: r.takeKinds(), uids: r.takeUids(), ints: r.takeInts(), floats: r.takeFloats(), text: r.takeText(), textLengths: r.takeTextLengths() };
    const head = JSON.parse(r.takeHead()) as ImportResult;
    const reader = new ColumnsReader(columns);
    const entities: Entity[] = [];
    for (let i = 0; i < reader.count; i++) {
      const { uid: _placeholder, ...e } = reader.next();
      entities.push(e as Entity);
    }
    expect(reader.done).toBe(true);
    return { ...head, entities };
  } finally {
    r.free();
  }
}

const read = async (name: string) => imported((await load()).readNcz(fixture(name), options, quiet));
const layers = (r: ImportResult) => r.layers.map((l) => [l.name, l.color, l.count]);

describe.skipIf(!loader)('NCZ WASM module', () => {
  it('speaks the version of the contracts', async () => {
    expect((await load()).nczVersion()).toBe(FORMATS_VERSION);
  });

  it('reads every record type onto its layer, with the system the file declares', async () => {
    const r = await read('01-her-tur.ncz');
    expect(r.report.source.find((f) => f.label === 'Sürüm')?.value).toBe('Netcad 5.2.0.1035N');
    expect([r.declaredCrs?.srid, r.declaredCrs?.source]).toEqual([5257, 'ncz']);
    expect(r.report.counts).toEqual({ arc: 2, circle: 1, line: 2, point: 5, polygon: 4, polyline: 2, text: 2 });
    expect(layers(r)).toEqual([
      ['PARSEL', '#FF0000', 5],
      ['YAZI', 'ink', 5],
      ['İMAR_ŞERİT', '#008000', 4],
      ['PAFTA', 'ink', 3],
      ['KATMAN_6', '#28323C', 1],
    ]);
    const p = r.entities[0];
    expect(p.kind === 'point' && [p.p.x, p.p.y, p.z, p.label]).toEqual([E0, N0, 1088.5, '1284']);
    const arc = r.entities[5];
    expect(arc.kind === 'arc' && [arc.r, arc.a0, arc.a1]).toEqual([5, (30 * Math.PI) / 180, (120 * Math.PI) / 180]);
    // What each layer holds, without walking its objects.
    expect(r.layers[0].kinds).toEqual({ line: 2, point: 3 });
    expect(r.layers[0].bounds).toEqual({ minX: E0, minY: N0, maxX: E0 + 702, maxY: N0 + 702 });
  });

  it('draws smart objects as their symbols, on the layers that come first', async () => {
    const r = await read('07-akilli-nesneler.ncz');
    expect(layers(r)).toEqual([
      ['SM_YERLESIM', '#C80000', 16],
      ['SM_YAPILASMA', '#0000C8', 10],
      ['SM_YOL', '#007800', 9],
      ['SM_NOT', '#505050', 5],
      ['SM_FONKADI', '#780078', 1],
    ]);
    const settlement = r.entities[0];
    expect(settlement.kind === 'circle' && [settlement.c.x, settlement.c.y, settlement.r]).toEqual([E0 + 1000, N0 + 1000, 5]);
    expect(settlement.attrs).toEqual({ 'Akıllı nesne': 'Yerleşim', Kat: '3', Nizam: 'AYRIK', 'Yan bahçe': '3', 'Ön bahçe': '5' });
    const road = r.entities.find((e) => e.attrs['Akıllı nesne'] === 'Yol');
    expect(road?.kind === 'circle' && [road.r, road.attrs['Genişlik']]).toEqual([10 * Math.fround(1.275), '17']);
    expect(r.view).toEqual(r.bounds);
  });

  it('draws a map sheet as its cell, turned in the zone the file names', async () => {
    const r = await read('08-pafta.ncz');
    // Each corner in millimetres, (easting, northing), south-west first.
    const frame = (name: string) => {
      const e = r.entities.find((x) => x.attrs['Pafta'] === name);
      return e?.kind === 'polygon' ? e.pts.map((p) => [Math.round(p.x * 1000), Math.round(p.y * 1000)]) : e;
    };
    expect(frame('GB')).toEqual([
      [421_758_834, 4_450_753_176],
      [422_291_094, 4_450_747_687],
      [422_298_227, 4_451_441_691],
      [421_766_016, 4_451_447_181],
    ]);
    expect(frame('KD')?.[0]).toEqual([422_298_227, 4_451_441_691]);
    // A sheet of round local coordinates is no grid cell's: it keeps the box, and the report says why.
    expect(frame('YEREL-1')).toEqual([
      [421_400_000, 4_448_400_000],
      [421_400_000, 4_449_100_000],
      [421_940_000, 4_449_100_000],
      [421_940_000, 4_448_400_000],
    ]);
    expect(r.report.notes.find((n) => n.what === 'Pafta çerçevesi')?.count).toBe(4);
    expect(r.report.skipped.find((s) => s.what === 'Pafta çerçevesinin gerçek biçimi')?.count).toBe(1);
  });

  it('smart layers first even for the frames of older smart objects', async () => {
    const r = await read('02-akilli-nesne.ncz');
    expect(layers(r)).toEqual([
      ['AKILLI', 'ink', 3],
      ['0', 'ink', 1],
    ]);
  });

  it('says how far a read is, and refuses what is not an NCZ with the reason', async () => {
    const w = await load();
    const heard: [number, number][] = [];
    imported(w.readNcz(fixture('01-her-tur.ncz'), options, { step: (done, total) => heard.push([done, total]) }));
    expect(heard.at(-1)).toEqual([1000, 1000]);
    const r = w.readNcz(new TextEncoder().encode('  0\r\nSECTION\r\n  2\r\nENTITIES\r\n  0\r\nEOF\r\n'), options, quiet);
    expect([r.ok, r.message]).toEqual([false, expect.stringMatching(/^Bu dosyada Netcad NCZ çizimi bulunamadı/)]);
    r.free();
  });
});
