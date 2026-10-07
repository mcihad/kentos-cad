import { describe, expect, it } from 'vitest';
import { topologyCatalog } from './ops/topologyRules';
import { nextRuleId, sanitizeTopology, TOPOLOGY_KINDS, toleranceOf } from './topologyRules';

/** The project's topology settings as it keeps them (docs/adr/0202 §1; the contract's `TopologySettings::sanitized`). */
describe('topology settings', () => {
  it('know every kind the core knows, between layers and with values alike', () => {
    const core = topologyCatalog().kinds;
    expect(Object.keys(TOPOLOGY_KINDS)).toEqual(core.map((k) => k.key));
    for (const k of core) expect([k.key, TOPOLOGY_KINDS[k.key].between, TOPOLOGY_KINDS[k.key].value]).toEqual([k.key, k.between, k.value ?? null]);
  });

  it('keep the rules that hold, the exceptions of rules kept, and nothing when nothing is left', () => {
    expect(sanitizeTopology(null)).toBeNull();
    expect(sanitizeTopology({})).toBeNull();
    const kept = sanitizeTopology({
      tolerance: 5,
      rules: [
        { id: 'a', kind: 'mustNotOverlap', layer: 'p' },
        { id: 'a', kind: 'mustNotHaveGaps', layer: 'p' },
        { id: 'b', kind: 'mustBeCoveredBy', layer: 'b' },
        { id: 'c', kind: 'mustBeCoveredBy', layer: 'b', other: 'b' },
        { id: 'd', kind: 'mustBeCoveredBy', layer: 'b', other: 'p' },
        { id: 'e', kind: 'mustNotHaveSmallAngles', layer: 'p', value: 2 },
        { id: 'f', kind: 'mustNotHaveShortEdges', layer: 'p', value: 0.2 },
        { id: '', kind: 'mustBeValid', layer: 'p' },
      ],
      exceptions: [
        { rule: 'a', objects: ['0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5e01'], at: { x: 1, y: 2 } },
        { rule: 'b', objects: ['0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5e01'], at: { x: 1, y: 2 } },
        { rule: 'd', objects: [], at: { x: 1, y: 2 } },
      ],
    });
    expect(kept).toEqual({
      rules: [
        { id: 'a', kind: 'mustNotOverlap', layer: 'p' },
        { id: 'd', kind: 'mustBeCoveredBy', layer: 'b', other: 'p' },
        { id: 'e', kind: 'mustNotHaveSmallAngles', layer: 'p' },
        { id: 'f', kind: 'mustNotHaveShortEdges', layer: 'p', value: 0.2 },
      ],
      exceptions: [{ rule: 'a', objects: ['0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5e01'], at: { x: 1, y: 2 } }],
    });
    expect(toleranceOf(kept)).toBe(0.001);
    expect(toleranceOf({ tolerance: 0.01 })).toBe(0.01);
  });

  it('give a new rule the least free id', () => {
    expect(nextRuleId([])).toBe('kural-1');
    expect(nextRuleId([{ id: 'kural-1', kind: 'mustBeValid', layer: 'p' }, { id: 'kural-3', kind: 'mustBeValid', layer: 'p' }])).toBe('kural-2');
  });
});
