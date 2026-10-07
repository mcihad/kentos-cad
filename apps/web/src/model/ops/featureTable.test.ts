import { describe, expect, it } from 'vitest';
import { featureTable, type TableColumn, type TableQuery, type TableRow } from './featureTable';

/**
 * The attribute table's rows (docs/adr/0199 §4) through the WASM core, against the independent reference in
 * fixtures/feature-table/v1/cases.json (scripts/fixtures/feature_table_cases.py, no KentOS code), the queries the core
 * runs natively in crates/shared/geometry-core/tests/all/feature_table.rs.
 */

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Cases {
  format: string;
  tables: { name: string; columns: TableColumn[]; rows: TableRow[]; queries: { name: string; query: TableQuery; want: number[] }[] }[];
}

const cases = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/feature-table/v1/cases.json', import.meta.url), 'utf8')) as Cases;

describe('Öznitelik tablosu: rows', () => {
  it('shows every query’s rows in the reference’s order', () => {
    expect(cases.format).toBe('kentos.feature-table-cases');
    const off = cases.tables.flatMap((t) =>
      t.queries.flatMap((q) => {
        const got = featureTable(t.columns, t.rows, q.query);
        return JSON.stringify(got) === JSON.stringify(q.want) ? [] : [`${t.name} › ${q.name}: ${got.join(',')} ≠ ${q.want.join(',')}`];
      }),
    );
    expect(off).toEqual([]);
    expect(cases.tables.reduce((n, t) => n + t.queries.length, 0)).toBeGreaterThanOrEqual(60);
  });
});
