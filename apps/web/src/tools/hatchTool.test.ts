import { describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { PickIndex } from '../viewport/picking';
import { HatchTool } from './hatchTool';
import { at, pt, toolHarness } from './toolHarness';

/**
 * Tarama by a closed object on a multi-part area (docs/adr/0143): the region is the part the click is in, and the
 * islands take every part of every other closed object, still only those smaller than the region. The desktop's rule
 * is `crates/native/interaction/src/hatch.rs` `region`.
 */
const square = (x: number, y: number, side: number): Vec2[] => [pt(x, y), pt(x + side, y), pt(x + side, y + side), pt(x, y + side)];

/** A parcel of two parts (20 m squares 20 m apart), with a building inside the first. */
function scene() {
  const h = toolHarness();
  const parcel = h.add({ kind: 'polygon', pts: square(0, 0, 20), parts: [{ pts: square(40, 0, 20) }], attrs: { Parsel: '5' } }) as PolylineEntity;
  const building = h.add({ kind: 'polygon', layerId: 'yol', pts: square(5, 5, 5), attrs: {} }) as PolylineEntity;
  const index = new PickIndex(h.doc);
  Object.assign(h.ctx.view, { enclosingRing: (p: Vec2) => index.enclosing(p) });
  const tool = h.use(new HatchTool(h.ctx));
  const hatches = () => [...h.doc.all()].filter((e): e is Extract<Entity, { kind: 'hatch' }> => e.kind === 'hatch');
  /** Clicks at (x, y); the hatch it wrote, or null. */
  const click = (x: number, y: number) => {
    const before = hatches().length;
    tool.pointerDown(at(x, y));
    const all = hatches();
    return all.length > before ? all[all.length - 1] : null;
  };
  return { h, parcel, building, tool, click, index, hatches };
}

/** A ring's corners as a set: the hatch's ring starts where the core's area does, which is not the object's first corner. */
const corners = (ring: readonly Vec2[]) => new Set(ring.map((q) => `${q.x},${q.y}`));
const sameCorners = (ring: readonly Vec2[], want: readonly Vec2[]) => expect(corners(ring)).toEqual(corners(want));

describe('Tarama by a closed object on a multi-part area', () => {
  it('a click in the second part fills that part alone', () => {
    const { click, index } = scene();
    const hatch = click(50, 10)!;
    expect(hatch).toBeTruthy();
    sameCorners(hatch.ring, square(40, 0, 20));
    expect(hatch.holes).toBeUndefined();
    index.dispose();
  });

  it('a click in the first part still leaves the building out', () => {
    const { click, index } = scene();
    const hatch = click(15, 15)!;
    sameCorners(hatch.ring, square(0, 0, 20));
    expect(hatch.holes).toHaveLength(1);
    sameCorners(hatch.holes![0], square(5, 5, 5));
    index.dispose();
  });

  it('the part is kept with the object: the cursor moved in the first part, a click in the second fills the second', () => {
    const { tool, click, index } = scene();
    tool.pointerMove(at(15, 15));
    const hatch = click(50, 10)!;
    sameCorners(hatch.ring, square(40, 0, 20));
    expect(hatch.holes).toBeUndefined();
    // And the other way round: the second part last, then the first.
    tool.pointerMove(at(50, 10));
    sameCorners(click(15, 15)!.ring, square(0, 0, 20));
    index.dispose();
  });

  it('a click between the parts, in no part, hatches nothing and says why', () => {
    const { h, click, hatches, index } = scene();
    expect(click(30, 10)).toBeNull();
    expect(hatches()).toHaveLength(0);
    expect(h.said().at(-1)).toContain('Tıklanan noktayı çevreleyen kapalı bir alan');
    index.dispose();
  });

  it('every part of another multi-part object is an island of its own: the one in each part is left out of that part only', () => {
    const { h, click, index } = scene();
    // Two buildings as one object: one in the first part, one in the second, each 4 m wide.
    h.add({ kind: 'polygon', layerId: 'yol', pts: square(12, 12, 4), parts: [{ pts: square(45, 5, 4) }], attrs: {} });
    const first = click(2, 2)!;
    sameCorners(first.ring, square(0, 0, 20));
    // The two buildings of the first part (the one drawn apart and the multi-part object's first part); not the second one.
    expect(first.holes).toHaveLength(2);
    const second = click(58, 18)!;
    sameCorners(second.ring, square(40, 0, 20));
    expect(second.holes).toHaveLength(1);
    sameCorners(second.holes![0], square(45, 5, 4));
    index.dispose();
  });

  it('a bigger multi-part object around the region is no island: its parts, larger than the region, stay out of it', () => {
    const { h, click, index } = scene();
    // A block of two 25 m parts around the parcel's first part: each part is larger than the region, so none is an island.
    h.add({ kind: 'polygon', layerId: 'yol', pts: square(-2, -2, 25), parts: [{ pts: square(100, 0, 25) }], attrs: {} });
    const hatch = click(2, 2)!;
    // The block encloses the point too, but the smallest closed area around a click is the parcel's part.
    sameCorners(hatch.ring, square(0, 0, 20));
    expect(hatch.holes).toHaveLength(1);
    sameCorners(hatch.holes![0], square(5, 5, 5));
    index.dispose();
  });

  it('a hatch made inside a closed object follows it: its tie names the object and its islands by their persistent ids (docs/adr/0186 §6)', () => {
    const { h, parcel, building, click, index } = scene();
    const hatch = click(15, 15)!;
    expect(hatch.assoc).toEqual({ outer: h.doc.uidOf(parcel.id), islands: [h.doc.uidOf(building.id)], seed: pt(15, 15) });
    index.dispose();
  });

  it('a one-part area is hatched as it was: its region, its islands', () => {
    const h = toolHarness();
    h.add({ kind: 'polygon', pts: square(0, 0, 20), attrs: {} });
    h.add({ kind: 'polygon', layerId: 'yol', pts: square(5, 5, 5), attrs: {} });
    const index = new PickIndex(h.doc);
    Object.assign(h.ctx.view, { enclosingRing: (p: Vec2) => index.enclosing(p) });
    const tool = h.use(new HatchTool(h.ctx));
    tool.pointerDown(at(15, 15));
    const hatch = [...h.doc.all()].find((e) => e.kind === 'hatch') as Extract<Entity, { kind: 'hatch' }>;
    sameCorners(hatch.ring, square(0, 0, 20));
    expect(hatch.holes).toHaveLength(1);
    index.dispose();
  });
});
