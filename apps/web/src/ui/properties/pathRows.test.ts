import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../../model/entities';
import { cornerRows, holeRows } from './pathRows';

/** Öznitelikler' counting rows of a path or an area (docs/adr/0143): a multi-part area's are every part's. */
const p = (x: number, y: number) => ({ x, y });
const square = (x: number, y: number, size: number) => ({ pts: [p(x, y), p(x + size, y), p(x + size, y + size), p(x, y + size)] });
const own = { ...square(0, 0, 10) } satisfies Pick<PolylineEntity, 'pts'>;
const cells = (rows: { label: string; value: string }[]) => rows.map((r) => [r.label, r.value]);

describe('Köşe sayısı and Parça sayısı', () => {
  it('a one-part area or a polyline: its corners only, no Parça sayısı', () => {
    expect(cells(cornerRows(own))).toEqual([['Köşe sayısı', '4']]);
    expect(cells(cornerRows({ ...own, parts: [] }))).toEqual([['Köşe sayısı', '4']]);
    expect(cells(cornerRows({ pts: [p(0, 0), p(3, 0), p(3, 4)] }))).toEqual([['Köşe sayısı', '3']]);
  });

  it('a multi-part area: the sum of every part’s corners, then the number of parts', () => {
    const rows = cornerRows({ ...own, parts: [square(20, 0, 10), { pts: [p(40, 0), p(50, 0), p(45, 8)] }] });
    expect(cells(rows)).toEqual([
      ['Köşe sayısı', '11'],
      ['Parça sayısı', '3'],
    ]);
    expect(rows.every((r) => r.numeric)).toBe(true);
  });

  it('the holes’ vertices are not corners', () => {
    expect(cells(cornerRows({ ...own, holes: [square(2, 2, 2)], parts: [{ ...square(20, 0, 10), holes: [square(22, 2, 2)] }] }))).toEqual([
      ['Köşe sayısı', '8'],
      ['Parça sayısı', '2'],
    ]);
  });
});

describe('Ada (delik)', () => {
  it('none without a hole in any part', () => {
    expect(holeRows(own)).toEqual([]);
    expect(holeRows({ ...own, holes: [], parts: [square(20, 0, 10)] })).toEqual([]);
  });

  it('counts the area’s own holes and every part’s', () => {
    expect(cells(holeRows({ ...own, holes: [square(2, 2, 2)] }))).toEqual([['Ada (delik)', '1']]);
    const rows = holeRows({ ...own, holes: [square(2, 2, 2)], parts: [{ ...square(20, 0, 10), holes: [square(22, 2, 2), square(26, 6, 2)] }, square(40, 0, 10)] });
    expect(cells(rows)).toEqual([['Ada (delik)', '3']]);
    expect(rows[0].numeric).toBe(true);
  });

  it('shows the islands of a later part when the area’s own ring has none', () => {
    expect(cells(holeRows({ ...own, parts: [{ ...square(20, 0, 10), holes: [square(22, 2, 2)] }] }))).toEqual([['Ada (delik)', '1']]);
  });
});
