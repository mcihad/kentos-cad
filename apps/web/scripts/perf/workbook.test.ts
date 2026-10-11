// Tablo alışverişi on the web (docs/adr/0223 §8), the desktop's `workbook_timing`
// (crates/shared/formats/tests/all/workbook_timing.rs) value for value: a table of 100 000 rows and 10 columns (three
// texts, four numbers, a date, a yes or no and a column of no kind) written as XLSX, ODS and CSV; a sheet of 250 000
// rows, İşlemler's most, written once in each workbook and read back within İşlemler's limits; through the formats
// WASM as its worker calls it (the input's JSON made here too); the best of three after a warm-up against the ADR's
// budgets. Runs only on purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm:formats; WORKBOOK_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/workbook.test.ts --disable-console-intercept; rm apps/web/src/io/pkg/.stamp
import { expect, it } from 'vitest';
import type { TableFileRead } from '../../src/contracts/generated/TableFileRead';
import type { WorkbookFormat } from '../../src/contracts/generated/WorkbookFormat';
import type { WorkbookSheet } from '../../src/contracts/generated/WorkbookSheet';

interface Written {
  takeBytes(): Uint8Array;
  free(): void;
}

interface Formats {
  initSync(o: { module: BufferSource }): unknown;
  readTableFile(bytes: Uint8Array, processing?: boolean): Uint8Array;
  writeWorkbook(input: string): Written;
}

type Fs = { readFileSync(u: URL): Uint8Array<ArrayBuffer> };
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): Fs } }).process.getBuiltinModule('node:fs');
const glue = import.meta.glob<Formats>('../../src/io/pkg/kentos_formats_wasm.js');
const loader = Object.values(glue)[0];

const PLACES = ['Kızılay', 'Çankaya', 'Güvenevler', 'Öveçler'];
const pad = (n: number, w: number) => String(n).padStart(w, '0');

/** The desktop's `sheet`, value for value: a parcel register's rows. */
function sheet(rows: number): WorkbookSheet {
  const column = (name: string, kind: WorkbookSheet['columns'][number]['kind'], decimals?: number) => ({ name, kind, ...(decimals !== undefined && { decimals }) });
  const out: string[][] = [];
  for (let i = 0; i < rows; i++) {
    out.push([
      `P${i}`,
      PLACES[i % 4],
      `Ayşe Öztürk ${i % 997}`,
      String(100 + Math.floor(i / 50)),
      String(1 + (i % 50)),
      `${300 + (i % 900)}.${pad(i % 100, 2)}`,
      `${487_000 + (i % 4000)}.${pad(i % 1000, 3)}`,
      `20${pad(10 + (i % 15), 2)}-${pad(1 + (i % 12), 2)}-${pad(1 + (i % 28), 2)}`,
      i % 3 === 0 ? 'true' : 'false',
      i % 5 === 0 ? 'köşe' : String(i % 7),
    ]);
  }
  return {
    name: 'Parseller',
    columns: [
      column('Ad', 'text'),
      column('Mahalle', 'text'),
      column('Malik', 'text'),
      column('Ada', 'number', 0),
      column('Parsel', 'number', 0),
      column('Alan', 'number', 2),
      column('Doğu', 'number', 3),
      column('Tarih', 'date'),
      column('Ruhsat', 'boolean'),
      column('Not', 'auto'),
    ],
    rows: out,
  };
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

it.skipIf(!process.env.WORKBOOK_BENCH || !loader)('writes and reads workbooks within their budgets (docs/adr/0223 §8)', async () => {
  const w = await loader!();
  w.initSync({ module: fs.readFileSync(new URL('../../src/io/pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
  const write = (s: WorkbookSheet, format: WorkbookFormat) => taken(w.writeWorkbook(JSON.stringify({ format, sheets: [s] })));
  const table = sheet(100_000);
  for (const [format, name, budget] of [
    ['xlsx', 'XLSX', 900],
    ['ods', 'ODS', 1100],
    ['csv', 'CSV', 250],
  ] as const) {
    const bytes = time(`${name} yaz (100 000 satır, 10 sütun)`, budget, () => write(table, format));
    console.log(`  ${name}: ${(bytes.length / 1e6).toFixed(1)} MB`);
  }
  const most = sheet(250_000);
  for (const [format, name, budget] of [
    ['xlsx', 'XLSX', 2200],
    ['ods', 'ODS', 3000],
  ] as const) {
    const bytes = write(most, format);
    const read = time(`${name} oku (250 000 satır, 10 sütun; İşlemler'in sınırıyla)`, budget, () => JSON.parse(new TextDecoder().decode(w.readTableFile(bytes, true))) as TableFileRead);
    expect(read.problem).toBeUndefined();
    expect(read.sheets[0].rows.length).toBe(250_001);
  }
}, 900_000);
