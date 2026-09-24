import { describe, expect, it } from 'vitest';
import type { Entity } from '../model/entities';
import { CoreStore } from './core';
import { packEntities } from './pack';
import { Gen } from './calls/harness';
import { entity } from './calls/sets/p5-entities';

/**
 * Packed objects (./pack.ts → crates/shared/geometry-core/src/store/pack.rs) must
 * build the very objects their JSON builds: every kind, with bulges, holes,
 * optional fields present and absent, labels and layers.
 */
describe('packEntities', () => {
  it('gives the store the same objects as JSON', () => {
    const g = new Gen(1_2026);
    const list: Entity[] = [];
    for (let i = 1; i <= 3000; i++) {
      g.frame();
      list.push({ ...entity(g), id: i });
    }
    list.push(
      { id: 5001, layerId: 'a', attrs: {}, kind: 'point', p: { x: 1, y: 2 }, z: 850.5 },
      { id: 5002, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, c: { x: 1, y: 1 }, offset: 2, height: 0.5, style: 'angular', text: '' },
      { id: 5003, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, offset: -2, height: 0.5, style: 'linear', angle: 90, text: 'Çıkma' },
      { id: 5004, layerId: 'b', attrs: {}, kind: 'hatch', ring: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }], holes: [[{ x: 1, y: 1 }, { x: 2, y: 1 }, { x: 2, y: 2 }]], pattern: { type: 'cross', angle: 30, spacing: 0.5 } },
      { id: 5005, layerId: 'b', attrs: {}, kind: 'polygon', pts: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }], bulges: [0.2, 0, 0], holes: [{ pts: [{ x: 1, y: 1 }, { x: 2, y: 1 }, { x: 2, y: 2 }], bulges: [0, 0.3, 0] }], label: '12' },
      { id: 5006, layerId: 'b', attrs: {}, kind: 'polyline', pts: [] },
      { id: 5007, layerId: 'b', attrs: {}, kind: 'text', p: { x: 1, y: 1 }, text: 'Ada 104 😀', height: 2, rotation: -30 },
    );
    const packed = new CoreStore();
    const p = packEntities(list);
    packed.putPacked(p.nums, p.strings);
    const json = new CoreStore();
    json.put(JSON.stringify(list));
    expect(packed.size).toBe(list.length);
    const diff = list.map((e) => e.id).filter((id) => packed.itemJson(id) !== json.itemJson(id));
    expect(diff.slice(0, 3).map((id) => `${packed.itemJson(id)}\n≠ ${json.itemJson(id)}`).join('\n')).toBe('');
    expect(Array.from(packed.ids())).toEqual(Array.from(json.ids()));
    packed.dispose();
    json.dispose();
  });

  it('keeps −0 and NaN that JSON would lose', () => {
    const s = new CoreStore();
    const p = packEntities([{ id: 1, layerId: 'a', attrs: {}, kind: 'line', a: { x: -0, y: NaN }, b: { x: 1, y: 1 } }]);
    s.putPacked(p.nums, p.strings);
    expect(s.itemJson(1)).toBe('{"id":1,"layerId":"a","label":false,"kind":"line","a":{"x":-0,"y":"#NaN"},"b":{"x":1,"y":1}}');
    expect(() => s.putPacked(packEntities([{ id: 2, layerId: 'a', kind: 'daire' }]).nums, '[]')).toThrow();
    s.dispose();
  });
});
