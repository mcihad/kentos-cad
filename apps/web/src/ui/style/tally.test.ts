import { describe, expect, it } from 'vitest';
import type { Entity } from '../../model/entities';
import type { LayerRenderer, Rule } from '../../model/style';
import { withObjects } from '../../processing/geometry';
import { valuesOf } from '../../style/classify';
import { layerDocument } from '../../style/cases';
import { fieldToken } from '../processing/fieldPlan';
import { categoryTally, classTally, drawable, numbersPerObject, ruleCounts, shadowed } from './tally';

/**
 * The layer style window's counts as fixtures/style/v1/tally.json holds them
 * (written by hand): what each category, class and rule takes of the drawn
 * objects, which categories are shadowed, and how a field is written in an
 * expression. The desktop's window checks the same file
 * (crates/native/style/tests/tally.rs).
 */

type Categories = Extract<LayerRenderer, { type: 'categorized' }>['categories'];
type Classes = Extract<LayerRenderer, { type: 'graduated' }>['classes'];

const files = import.meta.glob<string>('../../../../../fixtures/style/v1/tally.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  layerName: string;
  entities: Entity[];
  categories: { id: string; expr: string; categories: Categories; expect: { counts: number[]; rest: number; shadowed: boolean[] } }[];
  classes: { id: string; expr: string; classes: Classes; expect: { counts: number[]; rest: number } }[];
  rules: { id: string; rules: Rule[]; expect: ({ path: number[]; count: number } | { path: number[]; error: string })[] }[];
  fieldTokens: { name: string; token: string }[];
};

const doc = layerDocument(F.layerName, { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, F.entities);
const entities = [...doc.all()];
const scope = { layerName: (id: string) => doc.layers.get(id)?.name ?? id, measures: (list: readonly Entity[]) => withObjects(list, (s) => s.measures(list.map((e) => e.id))) };
const drawn = drawable(entities);

describe('layer style counts (fixtures/style/v1/tally.json)', () => {
  it('is a v1 tally file', () => expect([F.format, F.version]).toEqual(['kentos.style-tally', 1]));

  for (const c of F.categories)
    it(`categories: ${c.id}`, () => {
      const values = valuesOf(entities, c.expr, scope).values;
      expect({ ...categoryTally(values, drawn, c.categories), shadowed: c.categories.map((_, i) => shadowed(c.categories, i)) }).toEqual(c.expect);
    });

  for (const c of F.classes)
    it(`classes: ${c.id}`, () => {
      const numbers = numbersPerObject(entities, c.expr, scope).values;
      expect(classTally(numbers, drawn, c.classes)).toEqual(c.expect);
    });

  for (const r of F.rules)
    it(`rules: ${r.id}`, () => {
      const counts = ruleCounts(r.rules, entities, scope);
      const got = r.expect.map((e) => {
        const c = counts.get(e.path.join('/'));
        return c && 'error' in c ? { path: e.path, error: c.error } : { path: e.path, count: c?.n };
      });
      expect(got).toEqual(r.expect);
      expect(counts.size).toBe(r.expect.length);
    });

  it('writes a field bare or in brackets', () => {
    for (const t of F.fieldTokens) expect(fieldToken(t.name), t.name).toBe(t.token);
  });
});
