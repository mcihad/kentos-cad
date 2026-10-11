import { describe, expect, it } from 'vitest';
import type { WorkbookSheet } from '../contracts/generated/WorkbookSheet';
import type { Entity } from './entities';
import { featureSheet, featureTableModel } from './featureTable';
import type { LayerField } from './layerFields';

/**
 * Öznitelik tablosu's Dışa aktar (docs/adr/0223 §3): the table as a workbook's sheet, as
 * fixtures/feature-table/v1/export.json says (scripts/fixtures/feature_export_cases.py, from the rule and the layer
 * fields' reference). The desktop gives the same (crates/native/interaction/tests/all/feature_export.rs).
 */
interface Case {
  title: string;
  layer: string;
  fields: LayerField[];
  objects: { kind: 'point' | 'polygon'; attrs: Record<string, string> }[];
  expect: WorkbookSheet;
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/feature-table/v1/export.json', import.meta.url), 'utf8')) as { cases: Case[] };

const entity = (o: Case['objects'][number], id: number): Entity =>
  (o.kind === 'polygon'
    ? { id, kind: 'polygon', layerId: 'katman', attrs: o.attrs, pts: [{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }] }
    : { id, kind: 'point', layerId: 'katman', attrs: o.attrs, p: { x: 0, y: 0 } }) as unknown as Entity;

describe('the attribute table as a workbook sheet (fixtures/feature-table/v1/export.json)', () => {
  it('has the cases', () => expect(file.cases.length).toBeGreaterThanOrEqual(2));
  for (const c of file.cases) {
    it(c.title, () => {
      const entities = c.objects.map((o, i) => entity(o, i + 1));
      const model = featureTableModel(c.fields, entities, () => false, () => true, null);
      expect(featureSheet(model, entities.map((_, i) => i), c.layer)).toEqual(c.expect);
    });
  }
});
