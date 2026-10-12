import { describe, expect, it } from 'vitest';
import { ChartData, type ChartSpec, type DataColumn } from './charts';

/**
 * Grafikler ve alan istatistikleri in WASM (docs/adr/0227) against the independent reference in
 * fixtures/charts/v1/cases.json (scripts/fixtures/chart_cases.py, no KentOS code), as the core's own test reads it
 * (crates/shared/geometry-core/tests/all/charts.rs): every field's summary, statistics and values with their rows, every
 * chart's words, elements and table; and what a chart starts with (the core's `specs`).
 */

type FixtureCell = null | { v: string; s?: string } | { invalid: string };
interface FixtureData {
  rows: number;
  columns: { key: string; label: string; kind: DataColumn['kind']; labelled?: boolean; cells: FixtureCell[] }[];
}
interface File {
  format: string;
  data: Record<string, FixtureData>;
  fields: { name: string; data: string | FixtureData; column: number; rows: number[]; summary: Record<string, unknown>; detail: { stats: [string, string][]; values: { text: string; count: number; share: string; mark: string; rows: number[] }[]; more: number } }[];
  charts: { name: string; data: string; spec: ChartSpec; rows: number[]; want: { kind: string; xTitle: string; yTitle: string; notes: string[]; elements: number[][]; table: { columns: string[]; rows: string[][] } } }[];
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/charts/v1/cases.json', import.meta.url), 'utf8')) as File;

const dataOf = (d: FixtureData): ChartData =>
  new ChartData(
    d.rows,
    d.columns.map((c) => ({ key: c.key, label: c.label, kind: c.kind, labelled: !!c.labelled, cells: c.cells.map((cell) => (cell === null ? null : 'invalid' in cell ? { invalid: cell.invalid } : cell.s !== undefined ? { v: cell.v, s: cell.s } : { v: cell.v })) })),
  );

describe('charts against the reference (docs/adr/0227)', () => {
  it('reads the reference file', () => {
    expect(file.format).toBe('kentos.chart-cases');
    expect(file.fields.length).toBeGreaterThanOrEqual(25);
    expect(file.charts.length).toBeGreaterThanOrEqual(25);
  });

  it('gives every field its statistics and values', () => {
    for (const c of file.fields) {
      const data = dataOf(typeof c.data === 'string' ? file.data[c.data] : c.data);
      const rows = Uint32Array.from(c.rows);
      const s = data.summary(c.column, rows) as unknown as Record<string, unknown>;
      const got = Object.fromEntries(Object.entries(s).filter(([, v]) => v !== null && v !== undefined));
      expect(got, `${c.name}: summary`).toEqual(c.summary);
      const d = data.detail(c.column, rows);
      expect(d.stats.map((x) => [x.label, x.value]), `${c.name}: stats`).toEqual(c.detail.stats);
      expect(
        d.values.map((v, i) => ({ text: v.text, count: v.count, share: v.share, mark: v.mark, rows: [...data.valueRows(i)] })),
        `${c.name}: values`,
      ).toEqual(c.detail.values);
      expect(d.more, `${c.name}: more`).toBe(c.detail.more);
      data.free();
    }
  });

  it('gives every chart its words, elements and table', () => {
    for (const c of file.charts) {
      const data = dataOf(file.data[c.data]);
      const info = data.chart(c.spec, Uint32Array.from(c.rows));
      const w = c.want;
      expect([info.kind, info.xTitle, info.yTitle], `${c.name}: words`).toEqual([w.kind, w.xTitle, w.yTitle]);
      expect(info.notes, `${c.name}: notes`).toEqual(w.notes);
      expect(
        Array.from({ length: info.elements }, (_, e) => [...data.elementRows(e)]),
        `${c.name}: elements`,
      ).toEqual(w.elements);
      expect(info.table, `${c.name}: table`).toEqual(w.table);
      data.free();
    }
  });

  it('starts a chart as the core says: made whole, a column’s, a tool’s table’s', () => {
    const data = new ChartData(2, [
      { key: 'Eşik', label: 'Eşik', kind: 'number', labelled: false, cells: [{ v: '0.5' }, { v: '0.1' }] },
      { key: 'Doğru pozitif oranı (%)', label: 'Doğru pozitif oranı (%)', kind: 'number', labelled: false, cells: [{ v: '70' }, { v: '100' }] },
      { key: 'Alan oranı (%)', label: 'Alan oranı (%)', kind: 'number', labelled: false, cells: [{ v: '20' }, { v: '100' }] },
    ]);
    expect(data.forTable('suitability.roc')).toMatchObject({ kind: 'line', x: 'Alan oranı (%)', y: ['Doğru pozitif oranı (%)'], aggregate: 'none' });
    expect(data.forTable('başka.araç')).toMatchObject({ kind: 'bar', category: 'Eşik', value: 'Doğru pozitif oranı (%)', aggregate: 'sum' });
    expect(data.forColumn('Eşik')).toMatchObject({ kind: 'histogram', value: 'Eşik' });
    expect(data.complete({ kind: 'scatter' })).toMatchObject({ kind: 'scatter', x: 'Eşik', y: ['Doğru pozitif oranı (%)'], trend: true });
    expect(data.complete({ kind: 'line', aggregate: 'mean' })).toMatchObject({ kind: 'line', aggregate: 'mean', x: 'Eşik' });
    data.free();
  });

  it('reads Alan and Uzunluk in the project’s unit: m² and m, or mm² and mm in a project in millimetres', () => {
    // The store's measure records (six numbers a row: flags, length, area …): a 2 × 1.5 area and a line 4 long.
    const records = Float64Array.from([3, 7, 3, 0, 0, 0, 1, 4, 0, 0, 0, 0]);
    for (const [perMetre, area, length, sum] of [
      [1, 'Alan (m²)', 'Uzunluk (m)', '3.000'],
      [1000, 'Alan (mm²)', 'Uzunluk (mm)', '3000000.000'],
    ] as const) {
      const data = new ChartData(2, [], records, perMetre);
      expect(data.columns.map((c) => [c.key, c.label])).toEqual([
        ['$alan', area],
        ['$uzunluk', length],
      ]);
      expect(data.summary(0, Uint32Array.from([0, 1])).sum).toBe(sum);
      data.free();
    }
  });
});
