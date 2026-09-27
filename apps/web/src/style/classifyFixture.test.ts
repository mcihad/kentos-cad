import { describe, expect, it } from 'vitest';
import type { Entity } from '../model/entities';
import { withObjects } from '../processing/geometry';
import { layerDocument } from './cases';
import {
  categoriesOf,
  categoryCounts,
  CLASS_COUNT,
  classCount,
  classesPresent,
  CLASSIFY_TEXTS,
  classLabel,
  countIn,
  DEFAULT_RAMP,
  graduatedOf,
  newCategory,
  numbersOf,
  OTHER_COLOR,
  plainSymbols,
  QUALITATIVE,
  RAMPS,
  rampColors,
  uniqueValues,
  valuesOf,
  type Category,
} from './classify';
import type { GeometryClass } from './geometry';

/**
 * The layer style window's classes as fixtures/style/v1/classify.json holds
 * them (scripts/fixtures/record-classify.test.ts): values and categories,
 * numeric classes and their counts, ramps, plain symbols, labels, the class
 * count field and the texts. The desktop's layer style checks the same file.
 */

type Present = Record<GeometryClass, number>;

const files = import.meta.glob<string>('../../../../fixtures/style/v1/classify.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  layerName: string;
  entities: Entity[];
  present: Present;
  qualitative: string[];
  ramps: unknown;
  defaults: unknown;
  texts: Record<string, unknown>;
  values: { expr: string; values: (string | null)[]; unique: unknown; error: string | null }[];
  categories: { id: string; expr: string; old: Category[]; expect: unknown }[];
  newCategory: { count: number; category: unknown }[];
  graduated: { expr: string; method: 'interval' | 'count'; n: number; ramp: string; expect: unknown }[];
  rampColors: { ramp: string; n: number; colors: string[] }[];
  plain: { present: Present; color: string; symbols: unknown }[];
  labels: { min: number; max: number; digits?: number; label: string }[];
  classCount: { typed: string; count: number }[];
};

const doc = layerDocument(F.layerName, { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, F.entities);
const entities = [...doc.all()];
const scope = { layerName: (id: string) => doc.layers.get(id)?.name ?? id, measures: (list: readonly Entity[]) => withObjects(list, (s) => s.measures(list.map((e) => e.id))) };

describe('layer style classes (fixtures/style/v1/classify.json)', () => {
  it('is a v1 classify file with the constants and texts the window uses', () => {
    expect([F.format, F.version]).toEqual(['kentos.style-classify', 1]);
    expect(classesPresent(entities)).toEqual(F.present);
    expect(QUALITATIVE).toEqual(F.qualitative);
    expect(RAMPS).toEqual(F.ramps);
    expect({ classCount: CLASS_COUNT, ramp: DEFAULT_RAMP, method: 'interval', otherColor: OTHER_COLOR }).toEqual(F.defaults);
    expect({
      ...CLASSIFY_TEXTS,
      found: { n: 7, text: CLASSIFY_TEXTS.found(7) },
      classified: { classes: 5, values: 6, text: CLASSIFY_TEXTS.classified(5, 6) },
    }).toEqual(F.texts);
  });

  for (const v of F.values)
    it(`values of ${v.expr}`, () => {
      const r = valuesOf(entities, v.expr, scope);
      expect({ values: r.values, unique: uniqueValues(r.values), error: r.error ?? null }).toEqual({ values: v.values, unique: v.unique, error: v.error });
    });

  for (const c of F.categories)
    it(`categories: ${c.id}`, () => {
      const values = valuesOf(entities, c.expr, scope).values;
      const made = categoriesOf(uniqueValues(values), F.present, c.old);
      expect({ categories: made, counts: categoryCounts(values, made) }).toEqual(c.expect);
    });

  it('a new category takes the next colour in turn', () => {
    for (const n of F.newCategory) expect(newCategory(n.count, F.present), String(n.count)).toEqual(n.category);
  });

  for (const g of F.graduated)
    it(`classes of ${g.expr} by ${g.method}, ${g.n}, ${g.ramp}`, () => {
      const numbers = numbersOf(entities, g.expr, scope).values;
      const classes = graduatedOf(numbers, g.method, g.n, g.ramp, F.present);
      expect({ numbers, classes, counts: classes.map((c, i) => countIn(numbers, c.min, c.max, i === classes.length - 1)) }).toEqual(g.expect);
    });

  it('ramps, plain symbols, labels and the class count field', () => {
    for (const r of F.rampColors) expect(rampColors((RAMPS as Record<string, { stops: readonly string[] }>)[r.ramp].stops, r.n), `${r.ramp} ${r.n}`).toEqual(r.colors);
    for (const p of F.plain) expect(plainSymbols(p.color, p.present), JSON.stringify(p.present)).toEqual(p.symbols);
    for (const l of F.labels) expect(classLabel(l, l.digits), `${l.min} ${l.max}`).toBe(l.label);
    for (const c of F.classCount) expect(classCount(c.typed), JSON.stringify(c.typed)).toBe(c.count);
  });
});
