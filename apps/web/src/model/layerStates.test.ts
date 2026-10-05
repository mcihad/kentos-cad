import { describe, expect, it } from 'vitest';
import type { LayerState } from '../contracts/generated/LayerState';
import type { LayerNode } from './layers';
import { captureLayerState, layerStateChanges, layerStateMatches } from './layerStates';

/**
 * The layer states' rules (docs/adr/0177 §4) against the shared cases (fixtures/layers/v1/states.json, written by
 * scripts/fixtures/layer_state_cases.py from the ADR, not KentOS code). The desktop reads the same file
 * (crates/native/domain/src/layer_states.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Changes {
  visible: [string, boolean][];
  locked: [string, boolean][];
  styles: [string, unknown][];
  missing: number;
}

interface File {
  format: string;
  captures: { name: string; tree: LayerNode[]; id: string; stateName: string; locks: boolean; styles: boolean; state: LayerState }[];
  applies: { name: string; tree: LayerNode[]; state: LayerState; changes: Changes; matches: boolean }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/layers/v1/states.json', import.meta.url), 'utf8')) as File;

describe('Katman durumları (docs/adr/0177 §4)', () => {
  it('keeps the tree as the shared cases say', () => {
    expect(file.format).toBe('kentos.layer-state-cases');
    expect(file.captures.length).toBe(4);
    for (const c of file.captures) expect(captureLayerState(c.tree, c.id, c.stateName, { locks: c.locks, styles: c.styles }), c.name).toEqual(c.state);
  });

  it('changes and matches the tree as the shared cases say', () => {
    expect(file.applies.length).toBeGreaterThanOrEqual(6);
    for (const c of file.applies) {
      expect(layerStateChanges(c.tree, c.state), c.name).toEqual(c.changes);
      expect(layerStateMatches(c.tree, c.state), c.name).toBe(c.matches);
    }
  });

  it('takes two styles alike whatever the order of their keys', () => {
    const tree = file.applies[0].tree;
    const state = structuredClone(file.applies[0].state);
    for (const n of state.nodes) if (n.style) n.style = Object.fromEntries(Object.entries(n.style).reverse()) as typeof n.style;
    expect(layerStateMatches(tree, state)).toBe(true);
  });
});
