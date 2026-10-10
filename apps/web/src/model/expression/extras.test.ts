import { describe, expect, it } from 'vitest';
import { CoreStore } from '../../wasm/core';
import type { Entity } from '../entities';
import { compileExpression, type ExprValue, type ExprVariable } from './expression';
import { exprLayers } from './layers';

/**
 * The language's additions of docs/adr/0214 (dates, `@` values, regular
 * expressions, arrays and maps, aggregates, spatial relations, a value from
 * another layer) through the app's path: the context compiled in, the table
 * the TypeScript builds, the layers as one table, the core in WASM, the
 * column read back. The values come from an independent reference
 * (scripts/fixtures/expression_extras.py); the Rust core plays the same file
 * (crates/shared/expression/tests/language.rs).
 */

type Encoded = null | ['n', number | '-0'] | ['t', string] | ['b', boolean];
interface Case {
  source: string;
  compiles?: false;
  values?: Encoded[];
  unknown?: string[];
  /** Distances and overlaps: within 1e-9 relative. */
  near?: boolean;
}

interface Layer {
  id: string;
  name: string;
  objects: (Entity & { attrs: Record<string, string> })[];
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, e: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/expression/v2/extras.json', import.meta.url), 'utf8')) as {
  variables: { name: string; value: string | number | boolean | null }[];
  evaluated: string;
  layers: Layer[];
  cases: Case[];
};

const decode = (v: Encoded): ExprValue => (v === null ? null : v[0] === 'n' ? (v[1] === '-0' ? -0 : v[1]) : v[1]);

function same(got: ExprValue, want: ExprValue, near: boolean): boolean {
  if (near && typeof got === 'number' && typeof want === 'number') return Math.abs(got - want) <= 1e-9 * Math.max(1, Math.abs(got), Math.abs(want));
  return Object.is(got, want);
}

describe('expression language (docs/adr/0214)', () => {
  it(`gives the reference's values for ${file.cases.length} sources`, () => {
    const store = new CoreStore();
    store.put(JSON.stringify(file.layers.flatMap((l) => l.objects)));
    const variables: ExprVariable[] = file.variables.map((v) => ({ name: v.name, value: v.value }));
    const names = new Map(file.layers.map((l) => [l.id, l.name]));
    const layers = exprLayers(
      file.layers.map((l) => [l.id, l.name] as const),
      (id) => file.layers.find((l) => l.id === id)?.objects ?? [],
    );
    const entities = file.layers.find((l) => l.id === file.evaluated)!.objects;
    const wrong: string[] = [];
    let values = 0;
    for (const c of file.cases) {
      const r = compileExpression(c.source, { variables, world: true });
      if (c.compiles === false) {
        if (r.ok) wrong.push(`${JSON.stringify(c.source)}: derlendi, derlenmemeliydi`);
        continue;
      }
      if (!r.ok) {
        wrong.push(`${JSON.stringify(c.source)}: ${r.at}: ${r.error}`);
        continue;
      }
      if (JSON.stringify(r.expr.unknown) !== JSON.stringify(c.unknown ?? [])) wrong.push(`${JSON.stringify(c.source)}: bilinmeyenler ${JSON.stringify(r.expr.unknown)}`);
      const col = r.expr.evaluateAll({ entities, layerName: (id) => names.get(id) ?? id, geometry: store, layers }, 'value');
      c.values!.forEach((w, i) => {
        values += 1;
        if (!same(col.value(i), decode(w), c.near === true)) wrong.push(`${JSON.stringify(c.source)} ${i + 1}: ${JSON.stringify(col.value(i))} ≠ ${JSON.stringify(w)}`);
      });
    }
    store.dispose();
    expect(wrong.slice(0, 10), wrong.slice(0, 10).join('\n')).toEqual([]);
    expect(values).toBeGreaterThanOrEqual(900);
  });
});
