import { describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { LayerStore } from '../model/layers';
import { buildSceneLayer } from '../render/sceneBuilder';
import { parseHex, type CanvasPalette } from '../render/color';
import { PickIndex } from '../viewport/picking';
import { DrawnReader, styledGeometry } from './geometry';

/**
 * A multi-part area's drawn record (`FILLS`, docs/adr/0143): each part a fill of its own with its holes, every ring
 * outlined. The plain scene builder triangulates each part apart, so a hole is a hole of its own part only and
 * another part inside it (an island in a hole) is filled.
 */
const v = (x: number, y: number): Vec2 => ({ x, y });
const square = (x: number, y: number, side: number) => [v(x, y), v(x + side, y), v(x + side, y + side), v(x, y + side)];
const signed = (r: readonly Vec2[]) => r.reduce((s, p, i) => s + (p.x * r[(i + 1) % r.length].y - r[(i + 1) % r.length].x * p.y), 0) / 2;

/** A 10 m square with a 2 m hole (drawn clockwise: the store must turn it), and a 4 m square 20 m east with a 1 m hole. */
const area = (): NewEntity => ({
  layerId: 'a',
  attrs: {},
  kind: 'polygon',
  pts: [v(0, 0), v(0, 10), v(10, 10), v(10, 0)],
  holes: [{ pts: square(4, 4, 2) }],
  parts: [{ pts: square(20, 0, 4), holes: [{ pts: square(21, 1, 1) }] }],
});

function drawing(list: NewEntity[]): { doc: CadDocument; index: PickIndex; entities: Entity[] } {
  const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: v(0, 0) });
  const entities = list.map((e) => doc.add(e));
  return { doc, index: new PickIndex(doc), entities };
}

describe('the drawn record of a multi-part area', () => {
  it('reads as one fill with the rings of every part, and the parts each apart; a one-part area has no parts', () => {
    const { index, entities } = drawing([area(), { layerId: 'a', attrs: {}, kind: 'polygon', pts: square(50, 0, 3) }]);
    const reader = new DrawnReader(index.drawn(entities.map((e) => e.id), true));
    const multi = reader.read(entities[0]);
    const single = reader.read(entities[1]);
    index.dispose();
    expect(multi?.cls).toBe('fill');
    if (multi?.cls !== 'fill' || single?.cls !== 'fill') return;
    expect(multi.parts?.map((p) => p.length)).toEqual([2, 2]);
    expect(multi.rings).toHaveLength(4);
    expect(multi.rings).toEqual(multi.parts!.flat());
    // Oriented for the style engine: each outer ring counter-clockwise, each hole clockwise.
    expect(multi.parts!.map((p) => p.map((r) => Math.sign(signed(r))))).toEqual([[1, -1], [1, -1]]);
    expect(multi.parts![1][0]).toEqual(square(20, 0, 4));
    expect(single.parts).toBeUndefined();
    expect(single.rings).toEqual([square(50, 0, 3)]);
  });

  it('is read whole even among other records: the reader stays in step', () => {
    const { index, entities } = drawing([
      { layerId: 'a', attrs: {}, kind: 'line', a: v(0, 0), b: v(5, 5) },
      area(),
      { layerId: 'a', attrs: {}, kind: 'polygon', pts: square(50, 0, 3), holes: [{ pts: square(51, 1, 1) }] },
      { layerId: 'a', attrs: {}, kind: 'point', p: v(9, 9) },
    ]);
    const reader = new DrawnReader(index.drawn(entities.map((e) => e.id), false));
    const got = entities.map((e) => reader.read(e));
    index.dispose();
    expect(got.map((g) => g?.cls)).toEqual(['line', 'fill', 'fill', 'marker']);
    const holed = got[2];
    expect(holed?.cls === 'fill' && holed.rings.map((r) => r.length)).toEqual([4, 4]);
    expect(holed?.cls === 'fill' && holed.parts).toBeUndefined();
    expect(got[3]).toEqual({ cls: 'marker', point: v(9, 9) });
  });

  it('is the object’s geometry for one object too (symbol previews and tests)', () => {
    const g = styledGeometry({ ...area(), id: 1 } as Entity);
    expect(g?.cls === 'fill' && g.parts?.length).toBe(2);
  });
});

const palette: CanvasPalette = {
  background: [0, 0, 0, 1],
  fg: '#FFFFFF',
  fgDim: '#AAAAAA',
  ink: '#FFFFFF',
  paper: '#000000',
  gridMinor: [1, 1, 1, 0.05],
  gridMajor: [1, 1, 1, 0.1],
  accent: '#F2B632',
  snap: '#6FD08C',
  danger: '#EF6B61',
  label: '#C7D0DA',
  labelHalo: '#151B22',
  font: 'sans-serif',
  drawingFont: 'sans-serif',
};

/** The triangles of a fill batch as their total area (the batch holds x, y of three corners each, relative to the origin). */
function filled(positions: Float32Array): number {
  let sum = 0;
  for (let t = 0; t + 5 < positions.length; t += 6) {
    const [ax, ay, bx, by, cx, cy] = Array.from(positions.subarray(t, t + 6));
    sum += Math.abs((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)) / 2;
  }
  return sum;
}

describe('the plain scene builder draws every part of an area', () => {
  const style = { color: '#FFFFFF', lineType: 'continuous' as const, lineWeight: 0.25, fill: '#7FB2E540' };

  it('outlines every ring and fills each part with its holes off: 96 m² and 15 m², nothing in the holes', () => {
    const { index, entities } = drawing([area()]);
    const layer = buildSceneLayer('a', entities, style, { origin: v(0, 0), palette, geometry: index });
    index.dispose();
    // 4 rings of 4 edges: 16 segments, four numbers each.
    expect(layer.lines).toHaveLength(1);
    expect(layer.lines[0].positions.length).toBe(16 * 4);
    expect(layer.fills).toHaveLength(1);
    expect(filled(layer.fills[0].positions)).toBeCloseTo(96 + 15, 4);
    expect(layer.fills[0].color).toEqual(parseHex('#7FB2E540'));
  });

  it('a part in the hole of another is filled: the parts are not one polygon', () => {
    const island = {
      layerId: 'a',
      attrs: {},
      kind: 'polygon' as const,
      pts: square(0, 0, 20),
      holes: [{ pts: square(5, 5, 10) }],
      parts: [{ pts: square(8, 8, 4) }],
    };
    const { index, entities } = drawing([island]);
    const layer = buildSceneLayer('a', entities, style, { origin: v(0, 0), palette, geometry: index });
    index.dispose();
    expect(filled(layer.fills[0].positions)).toBeCloseTo(400 - 100 + 16, 4);
  });

  it('a one-part area is drawn as it was: its rings, its fill', () => {
    const { index, entities } = drawing([{ layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0, 10), holes: [{ pts: square(4, 4, 2) }] }]);
    const layer = buildSceneLayer('a', entities, style, { origin: v(0, 0), palette, geometry: index });
    index.dispose();
    expect(layer.lines[0].positions.length).toBe(8 * 4);
    expect(filled(layer.fills[0].positions)).toBeCloseTo(96, 4);
  });
});
