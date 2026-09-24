import { describe, expect, it } from 'vitest';
import type { Entity } from '../entities';
import { compileExpression, type ExprAs, type ExprValue } from './expression';

/**
 * The frozen answers of the expression language (fixtures/expression/v1/cases.json,
 * scripts/fixtures/record-expression.test.ts) through the app's path: the
 * table the TypeScript builds, the core in WASM, the column read back.
 * The Rust core checks the same file natively (crates/shared/style-core/tests/cases.rs).
 */

type Num = number | '-0' | 'Infinity' | '-Infinity';
type Encoded = null | ['n', Num] | ['t', string] | ['b', boolean];
interface FixtureObject {
  attrs: Record<string, string>;
  label: string | null;
  layer: string;
  kind: string;
  id: number;
  vertices: number | null;
  measures: Num[];
}
interface Case {
  source: string;
  error?: { message: string; at: number };
  fields?: string[];
  plotScale?: number | null;
  objects?: FixtureObject[];
  results?: Record<ExprAs, Encoded[]>;
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, e: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/expression/v1/cases.json', import.meta.url), 'utf8')) as { cases: Case[] };

/** "-0", "Infinity", "-Infinity": the numbers JSON cannot keep. */
const num = (x: Num) => (typeof x === 'number' ? x : x === '-0' ? -0 : Number(x));
const decode = (v: Encoded): ExprValue => (v === null ? null : v[0] === 'n' ? num(v[1]) : v[1]);

/** An object that reads as the fixture's: its kind and layer names come from the lookup below. */
const KINDS: Record<string, Entity['kind']> = { 'Kapalı alan': 'polygon', 'Çoklu çizgi': 'polyline', Çizgi: 'line', Nokta: 'point', Daire: 'circle', Eğri: 'spline' };

function entity(o: FixtureObject): Entity {
  const base = { id: o.id, layerId: o.layer, attrs: o.attrs, ...(o.label !== null ? { label: o.label } : {}) };
  const p = { x: 0, y: 0 };
  // Only what $köşe counts matters: as many corners as the fixture says.
  const pts = Array.from({ length: o.vertices ?? 0 }, () => p);
  switch (KINDS[o.kind]) {
    case 'polygon':
      return { ...base, kind: 'polygon', pts };
    case 'polyline':
      return { ...base, kind: 'polyline', pts };
    case 'spline':
      return { ...base, kind: 'spline', pts, closed: false };
    case 'line':
      return { ...base, kind: 'line', a: p, b: p };
    case 'point':
      return { ...base, kind: 'point', p };
    default:
      return { ...base, kind: 'circle', c: p, r: 1 };
  }
}

describe('expression fixture', () => {
  it(`gives the frozen answers to ${file.cases.length} sources`, () => {
    const wrong: string[] = [];
    for (const c of file.cases) {
      const r = compileExpression(c.source);
      if (c.error) {
        if (r.ok || r.error !== c.error.message || r.at !== c.error.at) wrong.push(`${JSON.stringify(c.source)}: ${r.ok ? 'derlendi' : `${r.at}: ${r.error}`}`);
        continue;
      }
      if (!r.ok) {
        wrong.push(`${JSON.stringify(c.source)}: ${r.error}`);
        continue;
      }
      if (JSON.stringify(r.expr.fields) !== JSON.stringify(c.fields)) wrong.push(`${JSON.stringify(c.source)}: alanlar`);
      const objects = c.objects!;
      const entities = objects.map(entity);
      const measures = Float64Array.from(objects.flatMap((o) => o.measures.map(num)));
      for (const [as, want] of Object.entries(c.results!) as [ExprAs, Encoded[]][]) {
        const col = r.expr.evaluateAll({ entities, layerName: (id) => id, plotScale: c.plotScale ?? undefined, measures: () => measures }, as);
        want.forEach((w, i) => {
          if (!Object.is(col.value(i), decode(w))) wrong.push(`${JSON.stringify(c.source)} [${as}] ${i + 1}: ${String(col.value(i))} ≠ ${JSON.stringify(w)}`);
        });
      }
    }
    expect(wrong.slice(0, 10), wrong.slice(0, 10).join('\n')).toEqual([]);
  });
});
