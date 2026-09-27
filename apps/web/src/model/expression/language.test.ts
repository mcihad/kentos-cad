import { describe, expect, it } from 'vitest';
import type { Entity } from '../entities';
import { compileExpression, type ExprValue } from './expression';

/**
 * The language since docs/adr/0100 §4 (durum … son, içinde, arasında, gibi,
 * benzer, boş, ^, the new functions) through the app's path: the table the
 * TypeScript builds, the core in WASM, the column read back. The values come
 * from an independent reference (scripts/fixtures/expression_language.py);
 * the Rust core plays the same file (crates/shared/expression/tests/language.rs).
 */

type Encoded = null | ['n', number | '-0'] | ['t', string] | ['b', boolean];
interface Case {
  source: string;
  compiles?: false;
  values?: Encoded[];
  /** A power with an exponent that is not whole: within one unit in the last place. */
  ulp?: number;
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, e: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/expression/v2/language.json', import.meta.url), 'utf8')) as {
  objects: Record<string, string>[];
  cases: Case[];
};

const decode = (v: Encoded): ExprValue => (v === null ? null : v[0] === 'n' ? (v[1] === '-0' ? -0 : v[1]) : v[1]);

/** The distance of two doubles in units in the last place (their bit patterns as integers). */
function ulps(a: number, b: number): number {
  const view = new DataView(new ArrayBuffer(16));
  view.setFloat64(0, a);
  view.setFloat64(8, b);
  const d = view.getBigInt64(0) - view.getBigInt64(8);
  return Number(d < 0n ? -d : d);
}

function same(got: ExprValue, want: ExprValue, ulp: number): boolean {
  if (typeof got === 'number' && typeof want === 'number' && ulp > 0) return Object.is(Math.sign(got), Math.sign(want)) && ulps(got, want) <= ulp;
  return Object.is(got, want);
}

const entities: Entity[] = file.objects.map((attrs, i) => ({ id: i + 1, layerId: '0', attrs, kind: 'point', p: { x: 0, y: 0 } }));

describe('expression language (§4)', () => {
  it(`gives the reference's values for ${file.cases.length} sources`, () => {
    const wrong: string[] = [];
    let values = 0;
    for (const c of file.cases) {
      const r = compileExpression(c.source);
      if (c.compiles === false) {
        if (r.ok) wrong.push(`${JSON.stringify(c.source)}: derlendi, derlenmemeliydi`);
        continue;
      }
      if (!r.ok) {
        wrong.push(`${JSON.stringify(c.source)}: ${r.at}: ${r.error}`);
        continue;
      }
      const col = r.expr.evaluateAll({ entities, layerName: (id) => id }, 'value');
      c.values!.forEach((w, i) => {
        values += 1;
        if (!same(col.value(i), decode(w), c.ulp ?? 0)) wrong.push(`${JSON.stringify(c.source)} ${i + 1}: ${String(col.value(i))} ≠ ${JSON.stringify(w)}`);
      });
    }
    expect(wrong.slice(0, 10), wrong.slice(0, 10).join('\n')).toEqual([]);
    expect(values).toBeGreaterThanOrEqual(500);
  });
});
