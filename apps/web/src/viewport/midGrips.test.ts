import { describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { LayerStore } from '../model/layers';
import type { Camera } from './Camera';
import { midGripVisible } from './overlay';
import { PickIndex } from './picking';
import { readGrips } from './storeRecords';

/**
 * The mid grips of an area's parts (docs/adr/0143): the store counts a mid grip's segment within its own part, so
 * whether one is shown (its segment 28 px long on the screen) is judged on that part's vertices, not the first
 * part's. The grips of an area of one part, and of a path, are read as they were.
 */
const v = (x: number, y: number): Vec2 => ({ x, y });
const square = (x: number, y: number, side: number) => [v(x, y), v(x + side, y), v(x + side, y + side), v(x, y + side)];
/** Two screen pixels to the metre: a segment is shown from 14 m. */
const camera = { worldToScreen: (p: Vec2) => v(p.x * 2, p.y * 2) } as unknown as Camera;

function gripsOf(entity: NewEntity) {
  const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: v(0, 0) });
  const e = doc.add(entity);
  const index = new PickIndex(doc);
  const [set] = index.grips([e.id]);
  index.dispose();
  return set;
}

/** Which mid grips are shown, by the index of the segment they belong to, ring by ring. */
const shown = (set: ReturnType<typeof gripsOf>) => set.segments.flatMap((seg, i) => (seg >= 0 ? [[i, seg, midGripVisible(set, i, camera)] as const] : []));

describe('the mid grips of an area of several parts', () => {
  it('are judged on the vertices of their own part: a long second part shows its mid grips beside a short first one', () => {
    const set = gripsOf({ layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0, 10), parts: [{ pts: square(100, 0, 100) }] });
    // 12 grips for the first part's 4 corners and 4 mid grips, then the second part's the same.
    expect(set.points).toHaveLength(16);
    expect(set.vertices).toBe(4);
    expect(shown(set).map(([, , on]) => on)).toEqual([false, false, false, false, true, true, true, true]);
    expect(set.rings?.[12]).toEqual({ from: 8, count: 4 });
    expect(set.rings?.[15]).toEqual({ from: 8, count: 4 });
    // The first part's, and every grip that is no mid grip, are read as they were.
    expect([0, 4, 7, 8, 11].map((i) => set.rings?.[i])).toEqual([undefined, undefined, undefined, undefined, undefined]);
  });

  it('and the other way round: a short second part hides its mid grips beside a long first one', () => {
    const set = gripsOf({ layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0, 100), parts: [{ pts: square(200, 0, 10) }] });
    expect(shown(set).map(([, , on]) => on)).toEqual([true, true, true, true, false, false, false, false]);
  });

  it('follow the holes of the parts before them, and the last edge closes on its own part’s first vertex', () => {
    const set = gripsOf({
      layerId: 'a',
      attrs: {},
      kind: 'polygon',
      pts: square(0, 0, 100),
      holes: [{ pts: square(10, 10, 5) }],
      // The second part is a long, thin rectangle: 100 m by 4 m, so only its long edges are shown.
      parts: [{ pts: [v(200, 0), v(300, 0), v(300, 4), v(200, 4)], holes: [{ pts: square(210, 1, 2) }] }, { pts: square(400, 0, 100) }],
    });
    const on = shown(set);
    // The first part: 4 + 4 + 4 grips; the second: 4 + 4 + 4; the third: 4 + 4.
    expect(set.points).toHaveLength(12 + 12 + 8);
    expect(on.slice(0, 4).map(([, , v1]) => v1)).toEqual([true, true, true, true]);
    // The thin part: the bottom (0) and the top (2) are 100 m, the sides 4 m.
    expect(on.slice(4, 8).map(([, , v1]) => v1)).toEqual([true, false, true, false]);
    expect(on.slice(8).map(([, , v1]) => v1)).toEqual([true, true, true, true]);
    expect(set.rings?.[16]).toEqual({ from: 12, count: 4 });
    expect(set.rings?.[28]).toEqual({ from: 24, count: 4 });
  });
});

describe('the mid grips of anything else', () => {
  it('are read as they were: no rings, the first vertices', () => {
    const one = gripsOf({ layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0, 100), holes: [{ pts: square(10, 10, 20) }] });
    expect(one.rings).toBeUndefined();
    expect(shown(one).map(([, , on]) => on)).toEqual([true, true, true, true]);
    const path = gripsOf({ layerId: 'a', attrs: {}, kind: 'polyline', pts: [v(0, 0), v(100, 0), v(100, 5)] });
    expect(path.rings).toBeUndefined();
    expect(shown(path).map(([, , on]) => on)).toEqual([true, false]);
    // A record read by hand: a circle's grips have no mid grip at all.
    const [circle] = readGrips(Float64Array.of(7, 5, 0, 0, 0, -1, 1, 0, -1, 0, 1, -1, -1, 0, -1, 0, -1, -1));
    expect(circle.rings).toBeUndefined();
  });
});
