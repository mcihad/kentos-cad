import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { AdjoinTool, NO_REGION } from './adjoinTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * Bitişik alan (docs/adr/0162 §3): an open path whose ends lie in or on the neighbouring areas; the region it closes
 * with them is written as one area. The scene is fixtures/interaction/v1/adjoin.kcad's: parcels 101 (−30..−10) and
 * 103 (10..30) between y 0 and 24 and a 4 m transformer lot (−2..2, 10..14) between them on the active layer, a road
 * below them on Yol (y −8..0). Expected values worked out by hand; the desktop's are
 * crates/native/interaction/tests/adjoin.rs, the shared trace fixtures/interaction/v1/adjoin.json.
 */
const rect = (x0: number, y0: number, x1: number, y1: number): Vec2[] => [pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)];

function scene() {
  const h = toolHarness();
  h.add({ kind: 'polygon', pts: rect(-30, 0, -10, 24) });
  h.add({ kind: 'polygon', pts: rect(10, 0, 30, 24) });
  h.add({ kind: 'polygon', pts: rect(-2, 10, 2, 14) });
  h.add({ kind: 'polygon', layerId: 'yol', pts: rect(-40, -8, 40, 0) });
  return h;
}

type Harness = ReturnType<typeof scene>;

/** Parcels and the road: what Seçili katmanlarda önle closes against here. */
function withTheRoad(h: Harness): void {
  h.ctx.settings.overlap.set('layers');
  h.ctx.settings.overlapLayers.set(new Set(['cizim', 'yol']));
}

/** A ring turned counter-clockwise to start at its lowest corner (x, then y): the core starts and turns a ring where it likes. */
function lowestFirst(ring: readonly Vec2[]): Vec2[] {
  let twice = 0;
  ring.forEach((a, i) => {
    const b = ring[(i + 1) % ring.length];
    twice += a.x * b.y - b.x * a.y;
  });
  const out = twice < 0 ? [...ring].reverse() : [...ring];
  let k = 0;
  out.forEach((p, i) => {
    if (p.x < out[k].x || (p.x === out[k].x && p.y < out[k].y)) k = i;
  });
  return [...out.slice(k), ...out.slice(0, k)];
}

const newest = (h: Harness) => [...h.doc.all()].at(-1) as PolylineEntity;
const count = (h: Harness) => [...h.doc.all()].length;

function start(h: Harness): AdjoinTool {
  const tool = h.use(new AdjoinTool(h.ctx));
  tool.activate();
  return tool;
}

function draw(h: Harness, ...corners: [number, number][]): AdjoinTool {
  const tool = start(h);
  for (const [x, y] of corners) tool.pointerDown(at(x, y));
  tool.confirm();
  return tool;
}

const ACROSS: [number, number][] = [
  [-15, 20],
  [15, 20],
];

describe('Bitişik alan', () => {
  it('Serbest closes against its own layer only, and the path stays', () => {
    const h = scene();
    const tool = start(h);
    expect(tool.prompt.value).toBe('Bitişik alan: ilk noktayı komşu alanın içinde ya da sınırında belirtin');
    tool.pointerDown(at(-15, 20));
    tool.pointerDown(at(15, 20));
    tool.confirm();
    // The road is on Yol: below the path the gap stays open.
    expect(h.said().at(-1)).toBe(NO_REGION);
    expect(count(h)).toBe(4);
    expect(tool.pointCount).toBe(2);
    tool.deactivate();
  });

  it('with the road chosen the gap closes and the transformer lot is its hole', () => {
    const h = scene();
    withTheRoad(h);
    const tool = draw(h, ...ACROSS);
    const made = newest(h);
    expect(made.layerId).toBe('cizim');
    expect(lowestFirst(made.pts)).toEqual(rect(-10, 0, 10, 20));
    expect(made.holes?.length).toBe(1);
    expect(lowestFirst(made.holes![0].pts)).toEqual(rect(-2, 10, 2, 14));
    expect(made.parts).toBeUndefined();
    expect(h.said().at(-1)).toBe('Bitişik alan eklendi: 384.00 m²');
    expect(tool.pointCount).toBe(0);
    // One undo step, by the tool's name.
    expect(h.doc.undo()).toBe('Bitişik alan');
    expect(count(h)).toBe(4);
    tool.deactivate();
  });

  it('Geri takes back an end that closed nothing', () => {
    const h = scene();
    withTheRoad(h);
    const tool = start(h);
    tool.pointerDown(at(-15, 20));
    // North of everything: the path hangs loose.
    tool.pointerDown(at(0, 40));
    tool.confirm();
    expect(h.said().at(-1)).toBe(NO_REGION);
    expect(tool.pointCount).toBe(2);
    expect(tool.input('G')).toBe(true);
    expect(tool.pointCount).toBe(1);
    tool.pointerDown(at(15, 20));
    tool.confirm();
    expect(h.said().at(-1)).toBe('Bitişik alan eklendi: 384.00 m²');
    expect(count(h)).toBe(5);
    tool.deactivate();
  });

  it('a gap closed all round fills on both sides of the path', () => {
    const h = scene();
    withTheRoad(h);
    h.add({ kind: 'polygon', pts: rect(-10, 24, 10, 34) });
    draw(h, ...ACROSS).deactivate();
    expect(lowestFirst(newest(h).pts)).toEqual(rect(-10, 0, 10, 24));
    expect(h.said().at(-1)).toBe('Bitişik alan eklendi: 464.00 m²');
  });

  it('a path through a neighbour makes one area of two parts', () => {
    const h = scene();
    withTheRoad(h);
    // Parcel 102 between the two: the path crosses it, a gap on each side.
    h.add({ kind: 'polygon', pts: rect(-4, 0, 4, 24) });
    draw(h, ...ACROSS).deactivate();
    const made = newest(h);
    const rings = [made.pts, ...(made.parts ?? []).map((p) => p.pts)].map(lowestFirst).sort((a, b) => a[0].x - b[0].x);
    expect(rings).toEqual([rect(-10, 0, -4, 20), rect(4, 0, 10, 20)]);
    expect(h.said().at(-1)).toBe('Bitişik alan eklendi: 240.00 m² (2 parça)');
  });

  it('the preview fills the region the cursor would close', () => {
    const h = scene();
    withTheRoad(h);
    const tool = start(h);
    tool.pointerDown(at(-15, 20));
    tool.pointerMove(at(15, 20));
    let c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts.slice(-2)).toEqual(['Yol 30.000 m', 'Alan 384.00 m²']);
    // The cursor leaves the parcels: nothing would close.
    tool.pointerMove(at(15, 40));
    c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts.some((t) => t.startsWith('Alan'))).toBe(false);
    tool.deactivate();
  });

  it('many neighbour edges leave the region to the clicks', () => {
    const h = scene();
    withTheRoad(h);
    // A ring of 2 400 corners in view, away from the gap: over the preview's budget.
    const round = Array.from({ length: 2400 }, (_, i) => {
      const a = (i * 2 * Math.PI) / 2400;
      return pt(-44 + 3 * Math.cos(a), -30 + 3 * Math.sin(a));
    });
    h.add({ kind: 'polygon', pts: round });
    const tool = start(h);
    tool.pointerDown(at(-15, 20));
    tool.pointerMove(at(15, 20));
    let c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts.some((t) => t.startsWith('Alan')), 'no region at a move').toBe(false);
    tool.pointerDown(at(15, 20));
    c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts.at(-1), 'the region at the click').toBe('Alan 384.00 m²');
    tool.confirm();
    expect(h.said().at(-1)).toBe('Bitişik alan eklendi: 384.00 m²');
    tool.deactivate();
  });

  it('a hidden layer’s areas are no neighbours', () => {
    const h = scene();
    withTheRoad(h);
    h.doc.layers.setVisible('yol', false);
    draw(h, ...ACROSS).deactivate();
    expect(h.said().at(-1)).toBe(NO_REGION);
  });
});
