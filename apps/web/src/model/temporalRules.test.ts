import { describe, expect, it } from 'vitest';
import type { LayerTime } from '../contracts/generated/LayerTime';
import type { ScenarioInfo } from '../contracts/generated/ScenarioInfo';
import type { LayerNode } from './layers';
import { layerTimeProblem, scenarioPairs, scenarioProblem, scenariosProblem } from './temporalRules';

/**
 * The rules of temporal layers and scenarios (docs/adr/0210 §2) against the shared cases
 * (fixtures/temporal/v1/rules.json, written by scripts/fixtures/temporal_rules_cases.py from the ADR, its verdicts
 * checked with tools/kcad/kcad.py's own copy of the rules): the words are the contract's, which Rust gives too
 * (crates/shared/contracts/tests/all/temporal_rules.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case<T> {
  name: string;
  value: T;
  problem: string | null;
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/temporal/v1/rules.json', import.meta.url), 'utf8')) as {
  format: string;
  times: Case<LayerTime>[];
  scenarios: Case<ScenarioInfo>[];
  trees: Case<LayerNode[]>[];
};

describe('temporal rules (docs/adr/0210 §2)', () => {
  it('reads the shared cases', () => {
    expect(file.format).toBe('kentos.temporal-rules');
    expect(file.times.length + file.scenarios.length + file.trees.length).toBeGreaterThan(30);
  });
  for (const c of file.times) it(`time: ${c.name}`, () => expect(layerTimeProblem(c.value)).toBe(c.problem));
  for (const c of file.scenarios) it(`scenario: ${c.name}`, () => expect(scenarioProblem(c.value)).toBe(c.problem));
  for (const c of file.trees) it(`tree: ${c.name}`, () => expect(scenariosProblem(c.value)).toBe(c.problem));

  it('pairs a scenario layer with the base layer it stands for, a base layer gone with none', () => {
    const tree = file.trees.find((c) => c.name === 'senaryo ve ana katman')!.value;
    expect(scenarioPairs(tree, tree[0])).toEqual([['yol', 'yol-a']]);
    const gone = file.trees.find((c) => c.name === 'ağaçta olmayan katman')!.value;
    expect(scenarioPairs(gone, gone[0])).toEqual([]);
  });
});
