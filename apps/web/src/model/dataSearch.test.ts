import { describe, expect, it } from 'vitest';
import type { Entity } from './entities';
import { attributeNames, inScope, recordOf, type SearchIndex } from './dataSearch';
import { fixture } from './ops/dataSearchFixture';

/**
 * What Veride ara reads of an object (docs/adr/0178 §1) against the independent reference in fixtures/search/v1/cases.json
 * (scripts/fixtures/data_search_cases.py): the records the objects give, and the attribute names a drawing's records
 * carry; the cases the desktop runs in crates/native/interaction/tests/all/data_search.rs. An object's attributes are
 * compared by name (the core looks at them in that order).
 */

const byName = <T extends [string, string]>(attrs: T[]) => [...attrs].sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));

describe('Veride ara: the records', () => {
  it('gives each kind of object the words the reference reads', () => {
    expect(fixture.records.length).toBeGreaterThanOrEqual(25);
    const off = fixture.records.flatMap((c) => {
      const got = recordOf(c.entity as Entity, c.layers[(c.entity as Entity).layerId] ?? '', (id) => c.blocks[id]);
      const norm = (r: typeof got) => (r ? JSON.stringify({ ...r, attrs: byName(r.attrs) }) : 'null');
      return norm(got) === norm(c.expected) ? [] : [`${c.name}: ${norm(got)} ≠ ${norm(c.expected)}`];
    });
    expect(off).toEqual([]);
  });

  it('lists the attribute names a drawing carries, each once, in the natural order', () => {
    const off = fixture.names.flatMap((c) => {
      const index: SearchIndex = { ids: [], layerIds: [], records: fixture.drawings[c.drawing] };
      const got = attributeNames(index);
      return JSON.stringify(got) === JSON.stringify(c.expected) ? [] : [`${c.name}: ${got.join(',')} ≠ ${c.expected.join(',')}`];
    });
    expect(off).toEqual([]);
  });
});

describe('Veride ara: the scope', () => {
  const index: SearchIndex = { ids: [10, 11, 12, 13], layerIds: ['a', 'b', 'a', 'c'], records: [] };
  it('takes every object, a layer’s, the selected ones, or the selected ones of a layer, in the drawing’s order', () => {
    expect(inScope(index, { layerId: null, selected: null })).toEqual([0, 1, 2, 3]);
    expect(inScope(index, { layerId: 'a', selected: null })).toEqual([0, 2]);
    expect(inScope(index, { layerId: null, selected: new Set([13, 11]) })).toEqual([1, 3]);
    expect(inScope(index, { layerId: 'a', selected: new Set([12, 11]) })).toEqual([2]);
    expect(inScope(index, { layerId: 'z', selected: null })).toEqual([]);
  });
});
