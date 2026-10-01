import { describe, expect, it } from 'vitest';
import type { PointEntity } from '../../model/entities';
import { clickPick, nextSort, POINT_COLUMNS, rowOf } from './PointTable';

/**
 * Noktalar's rules (docs/adr/0153 §2) apart from the DOM: a header click's sort, what a click on a row selects, and a
 * point as the table reads it. The order itself is the core's (model/ops/pointEditor.test.ts). The desktop holds its
 * table to the same cases in apps/desktop/src/points/tests.rs.
 */

describe('Noktalar', () => {
  it('has the columns of the ADR, Sıra the drawing’s order', () => {
    expect(POINT_COLUMNS.map((c) => [c.label, c.sort])).toEqual([
      ['Sıra', null],
      ['Ad', 'name'],
      ['Y (sağa)', 'east'],
      ['X (yukarı)', 'north'],
      ['Z (kot)', 'z'],
      ['Kod', 'code'],
      ['Katman', 'layer'],
    ]);
  });

  it('sorts a column ascending, then descending, then in the drawing’s order', () => {
    let q = { sort: null, descending: false } as Parameters<typeof nextSort>[0];
    q = nextSort(q, 'name');
    expect(q).toEqual({ sort: 'name', descending: false });
    q = nextSort(q, 'name');
    expect(q).toEqual({ sort: 'name', descending: true });
    q = nextSort(q, 'name');
    expect(q).toEqual({ sort: null, descending: false });
    // Another column starts ascending; Sıra is the drawing's order.
    expect(nextSort({ sort: 'name', descending: true }, 'z')).toEqual({ sort: 'z', descending: false });
    expect(nextSort({ sort: 'z', descending: false }, null)).toEqual({ sort: null, descending: false });
  });

  it('selects the row clicked, turns one over with Ctrl and takes a run with Shift', () => {
    const shown = [11, 12, 13, 14, 15];
    const none = { ctrl: false, shift: false };
    expect(clickPick(new Set([99]), shown, 2, null, none)).toEqual([13]);
    expect(clickPick(new Set([99, 13]), shown, 2, 0, { ctrl: true, shift: false })).toEqual([99]);
    expect(clickPick(new Set([99]), shown, 2, 0, { ctrl: true, shift: false })).toEqual([99, 13]);
    // The run from the last click, either way, in the order shown.
    expect(clickPick(new Set([12]), shown, 3, 1, { ctrl: false, shift: true })).toEqual([12, 13, 14]);
    expect(clickPick(new Set([15]), shown, 1, 4, { ctrl: false, shift: true })).toEqual([12, 13, 14, 15]);
    // Without a last click, Shift is a plain click.
    expect(clickPick(new Set(), shown, 3, null, { ctrl: false, shift: true })).toEqual([14]);
  });

  it('reads a point: its label, place, elevation, Kod, layer and whether it is selected', () => {
    const p = { id: 4, kind: 'point', layerId: 'nokta', attrs: { Kod: 'SN', Tür: 'x' }, label: '101', p: { x: 487001.5, y: 4420002.25 }, z: 100.5 } as unknown as PointEntity;
    expect(rowOf(p, 'Nokta', true)).toEqual({ name: '101', east: 487001.5, north: 4420002.25, z: 100.5, code: 'SN', layer: 'Nokta', selected: true });
    const bare = { id: 5, kind: 'point', layerId: 'nokta', attrs: {}, p: { x: 1, y: 2 } } as unknown as PointEntity;
    expect(rowOf(bare, 'Nokta', false)).toEqual({ name: null, east: 1, north: 2, z: null, code: null, layer: 'Nokta', selected: false });
  });
});
