import { describe, expect, it } from 'vitest';
import formatsRs from '../../../../crates/shared/contracts/src/formats.rs?raw';
import type { CoordRead } from '../contracts/generated/CoordRead';
import type { CoordReadOptions } from '../contracts/generated/CoordReadOptions';
import type { CoordWriteInput } from '../contracts/generated/CoordWriteInput';
import type { ExportReport } from '../contracts/generated/ExportReport';
import type { ImportResult } from '../contracts/generated/ImportResult';
import { FORMATS_VERSION } from './version';

/**
 * The Rust file formats as the browser runs them: the formats WASM module
 * (crates/wasm/formats-wasm → src/io/pkg, built by `pnpm wasm`) on the shared
 * fixtures (fixtures/formats/v1, which crates/shared/formats/tests reads natively
 * too). Skipped only when the package has not been built.
 */

interface Formats {
  initSync(o: { module: BufferSource }): unknown;
  formatsVersion(): number;
  readCoords(bytes: Uint8Array, options: string): Uint8Array;
  readDxf(bytes: Uint8Array, options: string): Uint8Array;
  writeCoords(input: string): { takeBytes(): Uint8Array; readonly report: string; free(): void };
}

const glue = import.meta.glob<Formats>('./pkg/kentos_formats_wasm.js');
const loader = Object.values(glue)[0];
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');
const fixture = (name: string) => fs.readFileSync(new URL(`../../../../fixtures/formats/v1/${name}`, import.meta.url));

let loaded: Formats | undefined;
async function load(): Promise<Formats> {
  if (loaded) return loaded;
  const w = await loader!();
  w.initSync({ module: fs.readFileSync(new URL('./pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
  return (loaded = w);
}

const options = (o: Partial<CoordReadOptions> = {}): string => JSON.stringify({ delimiter: 'auto', decimal: 'auto', header: 'auto', columns: [], previewRows: 10, entities: true, ...o } satisfies CoordReadOptions);
const decode = (b: Uint8Array) => JSON.parse(new TextDecoder().decode(b)) as CoordRead;
const points = (r: CoordRead) => (r.result?.entities ?? []).map((e) => (e.kind === 'point' ? [e.label ?? '', e.p.x, e.p.y, e.z] : null));

describe.skipIf(!loader)('formats WASM module', () => {
  it('speaks the version of the contracts', async () => {
    const w = await load();
    expect(w.formatsVersion()).toBe(FORMATS_VERSION);
    expect(Number(/pub const FORMATS_VERSION: u32 = (\d+);/.exec(formatsRs)?.[1])).toBe(FORMATS_VERSION);
  });

  it('reads a Netcad NCN list with the coordinates exactly as written', async () => {
    const r = decode((await load()).readCoords(fixture('netcad.ncn'), options()));
    expect(r.delimiter).toBe('space');
    expect(r.columns).toEqual(['name', 'y', 'x', 'z']);
    // The same float64 JavaScript reads from the same decimals, bit for bit.
    expect(points(r)).toEqual([
      ['1001', 452345.123, 4412345.678, 105.2],
      ['1002', 452360.5, 4412350.25, 106.75],
      ['P3', 452371.004, 4412339.9, 107],
      ['K12', 452380.1, 4412360.33, 108.125],
    ]);
    expect(r.result?.report.counts).toEqual({ point: 4 });
  });

  it('reads a Turkish spreadsheet (Windows-1254, decimal commas) and names the bad lines', async () => {
    const w = await load();
    const tr = decode(w.readCoords(fixture('excel-tr.csv'), options()));
    expect([tr.encoding, tr.delimiter, tr.decimal, tr.header]).toEqual(['Windows-1254 (Türkçe)', 'semicolon', 'comma', true]);
    expect(points(tr)).toEqual([
      ['Çınar-1', 452345.12, 4412345.5, 12.75],
      ['Şev-2', 452346, 4412346, undefined],
      ['İstasyon', 452350.005, 4412340.125, 13.001],
    ]);
    const bad = decode(w.readCoords(fixture('bad-lines.txt'), options({ entities: false })));
    expect(bad.points).toBe(2);
    expect(bad.errors.map((e) => e.message)).toEqual([
      'Satır 2: Y (sağa) değeri “abc” sayı değil.',
      'Satır 3: 2 alan var; X (yukarı) için en az 3 alan gerekiyor.',
      'Satır 4: Z (kot) değeri “x12” sayı değil.',
    ]);
    expect(bad.result).toBeUndefined();
    expect(bad.preview[1]).toEqual({ line: 2, fields: ['P2', 'abc', '4412345.3', '11'], error: 'Y (sağa) değeri “abc” sayı değil.' });
  });

  it('reads DXF: layers from the table, exploded blocks with exact coordinates, the report', async () => {
    const w = await load();
    const read = (name: string) => JSON.parse(new TextDecoder().decode(w.readDxf(fixture(name), JSON.stringify({ maxEntities: 0 })))) as ImportResult;
    const e = read('entities.dxf');
    expect(e.layers.map((l) => [l.name, l.color, l.visible, l.locked])).toEqual([
      ['0', 'ink', true, false],
      ['PARSEL', '#FF0000', true, false],
      ['Yol ekseni', '#0000FF', false, false],
      ['Kot', '#00FF00', true, true],
      ['Yazı', '#FF00FF', false, false],
    ]);
    const line = e.entities[0];
    expect(line.kind === 'line' && [line.a, line.b]).toEqual([
      { x: 452000.125, y: 4412000.5 },
      { x: 452010.25, y: 4412005.75 },
    ]);
    expect(e.report.skipped.map((s) => s.what)).toEqual(['IMAGE', 'Kâğıt uzayı nesnesi']);
    const b = read('blocks.dxf');
    const circle = b.entities.find((x) => x.kind === 'circle');
    expect(circle?.kind === 'circle' && [circle.c, circle.r, circle.layerId]).toEqual([{ x: 1000, y: 2010 }, 1, 'KAPILAR']);
    const point = b.entities.find((x) => x.kind === 'point');
    expect(point?.kind === 'point' && [point.p, point.z]).toEqual([{ x: 996, y: 2003 }, 101.5]);
    expect(() => w.readDxf(new TextEncoder().encode('AC1032\u0000binary'), '{"maxEntities":0}')).toThrow(/DWG dosyası/);
  });

  it('writes points that read back bit for bit', async () => {
    const w = await load();
    const input: CoordWriteInput = {
      points: [
        { name: '1', p: { x: 452345.123, y: 4412345.678 }, z: 0.1 + 0.2 },
        { name: 'Ağaç 2', p: { x: 1 / 3, y: -2.5e-7 } },
      ],
      delimiter: 'semicolon',
      columns: ['name', 'y', 'x', 'z'],
      header: true,
      encoding: 'utf8',
    };
    const out = w.writeCoords(JSON.stringify(input));
    const bytes = out.takeBytes();
    const report = JSON.parse(out.report) as ExportReport;
    out.free();
    expect(new TextDecoder().decode(bytes)).toBe('Ad;Y;X;Z\r\n1;452345.123;4412345.678;0.30000000000000004\r\nAğaç 2;0.3333333333333333;-0.00000025;\r\n');
    expect(report.counts).toEqual({ point: 2 });
    const back = decode(w.readCoords(bytes, options({ columns: ['name', 'y', 'x', 'z'] })));
    expect(points(back)).toEqual([
      ['1', 452345.123, 4412345.678, 0.1 + 0.2],
      ['Ağaç 2', 1 / 3, -2.5e-7, undefined],
    ]);
  });
});
