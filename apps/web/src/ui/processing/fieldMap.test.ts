import { describe, expect, it } from 'vitest';
import { fieldMapProblem } from '../../processing/parameters';
import type { FieldMap, FieldMapRow } from '../../processing/types';
import { fieldMapView, filledMap, withMapRow, withNewMapRow, withoutMapRow } from './fieldPlan';

/**
 * Alan eşleme's rules (docs/adr/0216 §2) against fixtures/processing/v1/field-map.json, written by the independent
 * reference (scripts/fixtures/field_map_cases.py); the desktop's `kentos-processing` plays the same file.
 */

const files = import.meta.glob<string>('../../../../../fixtures/processing/v1/field-map.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  views: { title: string; value: FieldMap; targets: string[]; columns: string[] | null; key: string; expect: unknown }[];
  fills: { title: string; value: FieldMap; columns: string[]; key: string; expect: FieldMap }[];
  problems: { title: string; name: string; value: FieldMap; expect: string | null }[];
  edits: ({ op: 'set'; value: FieldMap; index: number; patch: Partial<FieldMapRow>; expect: FieldMap } | { op: 'remove'; value: FieldMap; index: number; expect: FieldMap } | { op: 'add'; value: FieldMap; expect: FieldMap })[];
};

describe('the field map (fixtures/processing/v1/field-map.json, docs/adr/0216 §2)', () => {
  it('is a v1 file', () => {
    expect([F.format, F.version]).toEqual(['kentos.field-map-cases', 1]);
  });
  for (const c of F.views) it(`shows: ${c.title}`, () => expect(fieldMapView(c.value, c.targets, c.columns ?? undefined, c.key)).toEqual(c.expect));
  for (const c of F.fills) it(`fills: ${c.title}`, () => expect(filledMap(c.value, c.columns, c.key)).toEqual(c.expect));
  for (const c of F.problems) it(`checks: ${c.title}`, () => expect(fieldMapProblem(c.name, c.value)).toBe(c.expect));
  F.edits.forEach((e, i) =>
    it(`edits ${i + 1}: ${e.op}`, () => {
      const got = e.op === 'set' ? withMapRow(e.value, e.index, e.patch) : e.op === 'remove' ? withoutMapRow(e.value, e.index) : withNewMapRow(e.value);
      expect(got).toEqual(e.expect);
    }),
  );
});
