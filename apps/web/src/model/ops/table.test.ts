import { describe, expect, it } from 'vitest';
import { entityGrips, moveGrip } from './grips';
import { tableEdit, tableFromRows, tableLayout, tableOneLine, tableRefresh, tableSchedule, tableSizes, type ListedObject, type ScheduleKind, type TableCells, type TableEdit, type TableGeometry, type TableUnits } from './table';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Tablo (docs/adr/0184) through WASM against the shared cases (fixtures/table/v1/cases.json, written by
 * scripts/fixtures/table_cases.py from the ADR, not KentOS code); the core runs them natively
 * (crates/shared/geometry-core/src/ops/table.rs, ops/table_edit.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Changed = { rows: number[]; columns: number[]; cells: string[][]; merges: unknown[]; aligns: string[] | null };
interface File {
  format: string;
  layout: { name: string; table: TableGeometry; font: string; want: { outline: Vec2[]; lines: [Vec2, Vec2][]; frame: Vec2[][]; cells: { row: number; col: number; at: Vec2; width: number; bold: boolean }[] } }[];
  grips: { name: string; table: TableGeometry; grips: Vec2[]; index: number; to: Vec2; want: { p: Vec2; columns: number[] } | null }[];
  sizes: { name: string; table: TableGeometry; font: string; want: { rows: number[]; columns: number[] } }[];
  schedules: { name: string; kind: ScheduleKind; objects: ListedObject[]; units: TableUnits; want: TableCells }[];
  rows: { name: string; rows: string[][]; header: boolean; want: TableCells }[];
  edits: { name: string; table: TableGeometry; edit: TableEdit; want: Changed | { problem: string } }[];
  refresh: { name: string; table: TableGeometry; cells: string[][]; aligns: string[]; font: string; want: Changed }[];
  oneLine: { words: string; want: { words: string; changed: boolean } }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/table/v1/cases.json', import.meta.url), 'utf8')) as File;
const near = (a: number, b: number) => Math.abs(a - b) <= 1e-9 * (1 + Math.abs(b));
const same = (a: Vec2, b: Vec2, name: string) => expect(near(a.x, b.x) && near(a.y, b.y), `${name}: ${JSON.stringify(a)} ≠ ${JSON.stringify(b)}`).toBe(true);
const numbers = (a: readonly number[], b: readonly number[], name: string) => {
  expect(a.length, name).toBe(b.length);
  a.forEach((x, i) => expect(near(x, b[i]), `${name}: ${x} ≠ ${b[i]}`).toBe(true));
};
/** As an entity, for the grips (a table needs an object's fields around its shape). */
const entity = (t: TableGeometry): Entity => ({ id: 1, uid: 'u', layerId: 'L', ...t }) as unknown as Entity;
const changed = (t: TableGeometry, want: Changed, name: string) => {
  numbers(t.rows, want.rows, `${name}: rows`);
  numbers(t.columns, want.columns, `${name}: columns`);
  expect(t.cells, name).toEqual(want.cells);
  expect(t.merges ?? [], name).toEqual(want.merges ?? []);
  expect(t.aligns ?? null, name).toEqual(want.aligns ?? null);
};

describe('Tablo (docs/adr/0184)', () => {
  it('lays out as the shared cases say', () => {
    expect(file.format).toBe('kentos.table-cases');
    for (const c of file.layout) {
      const got = tableLayout(c.table, c.font);
      expect(got, c.name).not.toBeNull();
      got!.outline.forEach((g, i) => same(g, c.want.outline[i], c.name));
      expect(got!.lines.length, `${c.name}: lines`).toBe(c.want.lines.length);
      got!.lines.forEach((g, i) => {
        same(g[0], c.want.lines[i][0], c.name);
        same(g[1], c.want.lines[i][1], c.name);
      });
      expect(got!.frame.length, `${c.name}: frame`).toBe(c.want.frame.length);
      got!.frame.forEach((g, i) => g.forEach((p, k) => same(p, c.want.frame[i][k], c.name)));
      expect(got!.cells.length, `${c.name}: cells`).toBe(c.want.cells.length);
      got!.cells.forEach((g, i) => {
        const w = c.want.cells[i];
        expect([g.row, g.col, g.bold], c.name).toEqual([w.row, w.col, w.bold]);
        same(g.at, w.at, c.name);
        expect(near(g.width, w.width), `${c.name}: ${g.width}`).toBe(true);
      });
    }
  });

  it('moves its grips as the shared cases say', () => {
    for (const c of file.grips) {
      const e = entity(c.table);
      const grips = entityGrips(e);
      expect(grips.length, c.name).toBe(c.grips.length);
      grips.forEach((g, i) => same(g, c.grips[i], c.name));
      const moved = moveGrip(e, c.index, c.to) as (Entity & TableGeometry) | null;
      if (c.want === null) {
        expect(moved, c.name).toBeNull();
        continue;
      }
      same(moved!.p, c.want.p, c.name);
      numbers(moved!.columns, c.want.columns, c.name);
    }
  });

  it('sizes as the shared cases say', () => {
    for (const c of file.sizes) {
      const got = tableSizes(c.table, c.font);
      numbers(got!.rows, c.want.rows, `${c.name}: rows`);
      numbers(got!.columns, c.want.columns, `${c.name}: columns`);
    }
  });

  it('writes schedules and a file’s rows as the shared cases say', () => {
    const asSaid = (got: TableCells, want: TableCells, name: string) => {
      expect(got.problem ?? null, name).toBe(want.problem ?? null);
      expect(got.cells, name).toEqual(want.cells);
      expect(got.aligns, name).toEqual(want.aligns);
      expect(got.fitted, name).toBe(want.fitted);
    };
    for (const c of file.schedules) asSaid(tableSchedule(c.kind, c.objects, c.units), c.want, c.name);
    for (const c of file.rows) asSaid(tableFromRows(c.rows, c.header), c.want, c.name);
    for (const c of file.oneLine) expect(tableOneLine(c.words), c.words).toEqual(c.want);
  });

  it('edits and refreshes as the shared cases say', () => {
    for (const c of file.edits) {
      const got = tableEdit(c.table, c.edit);
      if ('problem' in c.want) {
        expect(got.table, c.name).toBeUndefined();
        expect(got.problem, c.name).toBe(c.want.problem);
      } else {
        expect(got.problem, c.name).toBeUndefined();
        changed(got.table!, c.want, c.name);
      }
    }
    for (const c of file.refresh) {
      const got = tableRefresh(c.table, c.cells, c.aligns, c.font);
      changed(got!, c.want, c.name);
    }
  });
});
