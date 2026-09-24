import { describe, expect, it } from 'vitest';
import type { Entity } from '../model/entities';
import { COORD_ORDERS, coordPoints, orderColumns, orderOf } from './coords';

/**
 * What the coordinate list windows offer (io/coords.ts): column orders laid
 * over a file, the order a column choice follows, and a drawing's points as
 * rows to write.
 */

describe('coordinate list columns', () => {
  it('lays an order over the file and pads the columns it does not name', () => {
    expect(orderColumns(['name', 'y', 'x', 'z'], 5)).toEqual(['name', 'y', 'x', 'z', 'skip']);
    // A file with fewer columns than the order still gets the whole order.
    expect(orderColumns(['y', 'x', 'z'], 2)).toEqual(['y', 'x', 'z']);
  });

  it('names the order a choice follows, and none for a choice of its own', () => {
    expect(orderOf(['name', 'y', 'x', 'z'])).toBe('nyxz');
    expect(orderOf(['name', 'y', 'x', 'z', 'skip'])).toBe('nyxz');
    expect(orderOf(['x', 'y', 'z'])).toBe('xyz');
    expect(orderOf(['skip', 'y', 'x', 'z'])).toBeNull();
    expect(orderOf(['name', 'y', 'x', 'code'])).toBeNull();
    // Every offered order is recognised as itself.
    for (const o of COORD_ORDERS) expect(orderOf(o.columns)).toBe(o.id);
  });
});

describe('coordPoints', () => {
  const base = { layerId: 'a', attrs: {} as Record<string, string> };
  it('writes point objects only: the label or the Ad attribute as the name, Z and Kod when there', () => {
    const list: Entity[] = [
      { ...base, id: 1, kind: 'point', p: { x: 452345.123, y: 4412345.678 }, z: 105.2, label: 'P1', attrs: { Ad: 'eski', Kod: 'ST' } },
      { ...base, id: 2, kind: 'point', p: { x: 1, y: 2 }, attrs: { Ad: 'A-2' } },
      { ...base, id: 3, kind: 'point', p: { x: 3, y: 4 } },
      { ...base, id: 4, kind: 'line', a: { x: 0, y: 0 }, b: { x: 1, y: 1 } },
    ];
    expect(coordPoints(list)).toEqual([
      { name: 'P1', p: { x: 452345.123, y: 4412345.678 }, z: 105.2, code: 'ST' },
      { name: 'A-2', p: { x: 1, y: 2 } },
      { name: '', p: { x: 3, y: 4 } },
    ]);
  });
});
