import { describe, expect, it } from 'vitest';
import { topologyChanges, topologyEdit, type TopologyChange, type TopologyNeighbour } from './topologyEdit';

/**
 * Topological editing (docs/adr/0160) through the WASM core, against the independent reference in
 * fixtures/topology/v1/edit.json (scripts/fixtures/topology_edit_cases.py, exact fractions, no KentOS code), the cases
 * the core runs natively in crates/shared/geometry-core/src/ops/topology_edit.rs. Nothing is computed: shapes match
 * bit for bit.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Shape = Record<string, unknown>;

interface File {
  format: string;
  cases: { name: string; neighbours: TopologyNeighbour<Shape>[]; changes: TopologyChange[]; points: boolean; expected: { edited: { index: number; shape: Shape }[]; locked: number; invalid: number } }[];
  diffs: { name: string; before: Shape; after: Shape; expected: TopologyChange[] }[];
}

describe('Topolojik düzenleme', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/topology/v1/edit.json', import.meta.url), 'utf8')) as File;

  it('puts every case right as the reference does', () => {
    expect(file.format).toBe('kentos.topology-edit');
    expect(file.cases.length).toBeGreaterThanOrEqual(11);
    for (const c of file.cases) expect(topologyEdit(c.neighbours, c.changes, c.points), c.name).toEqual(c.expected);
  });

  it('finds the changes of every edit as the reference does', () => {
    expect(file.diffs.length).toBeGreaterThanOrEqual(7);
    for (const d of file.diffs) expect(topologyChanges(d.before, d.after), d.name).toEqual(d.expected);
  });
});
