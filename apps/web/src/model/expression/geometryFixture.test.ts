import { describe, expect, it } from 'vitest';
import { CoreStore } from '../../wasm/core';
import type { Entity } from '../entities';
import { compileExpression, expressionError } from './expression';

/**
 * The geometry values expressions read of an object ($uzunluk, $alan, $merkez_y … $yükseklik, $köşe; docs/adr/0100 §3)
 * against the independent reference (fixtures/expression/v2/geometry.json, scripts/fixtures/expression_geometry.py),
 * through the app's path: the table the TypeScript builds (the corner count is its own, `vertexCount`), the geometry
 * store, the column read back. The Rust engine checks the same file (crates/shared/expression/tests/typed.rs), a
 * multi-part area among the cases (docs/adr/0143).
 */
interface Case {
  name: string;
  entity: Entity;
  values: Record<string, number | null>;
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, e: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/expression/v2/geometry.json', import.meta.url), 'utf8')) as { cases: Case[] };

describe('geometry values against the reference (fixtures/expression/v2/geometry.json)', () => {
  it('has a case for an area of several parts', () => {
    const many = file.cases.filter((c) => c.entity.kind === 'polygon' && (c.entity.parts?.length ?? 0) > 0);
    expect(many.map((c) => c.name)).toContain('iki parçalı alan (büyüğü delikli)');
  });

  it('reads every value of every object as the reference has it, within 0.1 µm', () => {
    let checked = 0;
    for (const c of file.cases) {
      const store = new CoreStore();
      try {
        store.put(JSON.stringify([c.entity]));
        for (const [name, want] of Object.entries(c.values)) {
          const r = compileExpression(`$${name}`);
          if (!r.ok) throw new Error(`${c.name} $${name}: ${expressionError(r)}`);
          const got = r.expr.evaluateAll({ entities: [c.entity], layerName: () => 'a', geometry: store }).value(0);
          // 0.1 µm: far below a survey's precision, far above float noise at projected coordinates.
          if (want === null) expect(got, `${c.name} $${name}`).toBeNull();
          else expect(Math.abs(Number(got) - want), `${c.name} $${name}: ${String(got)} ≠ ${want}`).toBeLessThanOrEqual(1e-7);
          checked++;
        }
      } finally {
        store.dispose();
      }
    }
    expect(checked).toBeGreaterThanOrEqual(100);
  });

  it('counts every part’s ring and holes in $köşe: 12 for two parts, the first with a hole', () => {
    const c = file.cases.find((x) => x.name === 'iki parçalı alan (büyüğü delikli)')!;
    const r = compileExpression('$köşe');
    if (!r.ok) throw new Error(expressionError(r));
    // Without a store too (the corner count is the table's own).
    expect(r.expr.evaluateAll({ entities: [c.entity], layerName: () => 'a' }).value(0)).toBe(12);
    expect(c.values.köşe).toBe(12);
  });
});
