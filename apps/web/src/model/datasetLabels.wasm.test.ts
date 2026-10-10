import { describe, expect, it } from 'vitest';
import { dimLabels } from './datasetLabels';

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, e: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Dim {
  name: string;
  values: number[];
  time: boolean;
  units?: string;
  labels: string[];
}

/**
 * The dimensions' labels as the formats core writes them (`dim_labels`) and the independent reference
 * (scripts/fixtures/multidim_cases.py) expects them in every file's info (fixtures/multidim/v1/cases.json).
 */
describe('dimLabels', () => {
  const cases = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/multidim/v1/cases.json', import.meta.url), 'utf8')) as {
    cases: { kind: string; name: string; expect: { grids?: { dims: Dim[] }[]; meshes?: { datasets: { dims: Dim[] }[] }[] } }[];
  };
  const dims = cases.cases
    .filter((c) => c.kind === 'info')
    .flatMap((c) => [...(c.expect.grids ?? []).flatMap((g) => g.dims), ...(c.expect.meshes ?? []).flatMap((m) => m.datasets.flatMap((d) => d.dims))]);

  it('reads every dimension of the reference', () => {
    expect(dims.length).toBeGreaterThan(5);
    for (const d of dims) expect(dimLabels(d.values, d.time, d.units), d.name).toEqual(d.labels);
  });

  it('writes a number out without an exponent', () => {
    expect(dimLabels([1e21, 1e-7, -2.5e-8, 0], false)).toEqual(['1000000000000000000000', '0.0000001', '-0.000000025', '0']);
  });
});
