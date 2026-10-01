import { describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { LayerStore } from '../model/layers';
import { buildSceneLayer } from '../render/sceneBuilder';
import { parseHex, type CanvasPalette } from '../render/color';
import { PickIndex } from '../viewport/picking';
import { DrawnReader } from './geometry';

/**
 * A leader's drawn record (`MIXED`, docs/adr/0146 §5): its line on to its landing's end and an open arrowhead's sides,
 * then its filled arrowhead's or dot's area; the highlight outlines the area and fills it. By hand, h = 3, the leader
 * from (0, 0) up to (0, 4): the arrowhead's base 3 m up, 1 m wide; the landing 6 m east from (0, 4).
 */
const v = (x: number, y: number): Vec2 => ({ x, y });
const leader = (extra: Partial<Extract<NewEntity, { kind: 'leader' }>> = {}): NewEntity => ({
  layerId: 'a',
  attrs: {},
  kind: 'leader',
  pts: [v(0, 0), v(0, 4)],
  text: 'Not',
  height: 3,
  rotation: 0,
  ...extra,
});

function drawing(list: NewEntity[]): { index: PickIndex; entities: Entity[] } {
  const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: v(0, 0) });
  const entities = list.map((e) => doc.add(e));
  return { index: new PickIndex(doc), entities };
}

describe('the drawn record of a leader', () => {
  it('reads as its lines and its arrowhead, the reader in step with the records around it', () => {
    const { index, entities } = drawing([
      leader(),
      leader({ arrow: 'open' }),
      leader({ arrow: 'dot', text: undefined }),
      { layerId: 'a', attrs: {}, kind: 'point', p: v(9, 9) },
    ]);
    const reader = new DrawnReader(index.drawn(entities.map((e) => e.id), true));
    const [filled, open, dot, point] = entities.map((e) => reader.read(e));
    index.dispose();
    expect(filled).toEqual({
      cls: 'mixed',
      paths: [{ pts: [v(0, 0), v(0, 4), v(6, 4)], closed: false }],
      // Turned counter-clockwise for the style engine: a base corner, the other, the tip.
      rings: [[v(0.5, 3), v(-0.5, 3), v(0, 0)]],
    });
    expect(open).toEqual({
      cls: 'mixed',
      paths: [
        { pts: [v(0, 0), v(0, 4), v(6, 4)], closed: false },
        { pts: [v(-0.5, 3), v(0, 0), v(0.5, 3)], closed: false },
      ],
      rings: [],
    });
    // No note, no landing; the dot a full turn's 72 points about the tip, 0.75 m round.
    expect(dot?.cls === 'mixed' && dot.paths[0].pts).toEqual([v(0, 0), v(0, 4)]);
    expect(dot?.cls === 'mixed' && dot.rings[0].length).toBe(72);
    expect(dot?.cls === 'mixed' && dot.rings[0][0]).toEqual(v(0.75, 0));
    expect(point).toEqual({ cls: 'marker', point: v(9, 9) });
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

describe('a highlighted leader', () => {
  const style = { color: '#FFFFFF', lineType: 'continuous' as const, lineWeight: 0.25 };

  it('draws its line, its landing and its arrowhead outlined and filled in the highlight', () => {
    const { index, entities } = drawing([leader()]);
    const accent = parseHex('#F2B632');
    const layer = buildSceneLayer('__sel', entities, style, { origin: v(0, 0), palette, geometry: index, overrideColor: accent, overrideFill: [1, 0.7, 0.2, 0.2] });
    index.dispose();
    // The line's 2 segments and the triangle's 3 sides, four numbers each.
    expect(layer.lines[0].positions.length).toBe(5 * 4);
    expect(layer.lines[0].color).toEqual(accent);
    // The triangle, 1 m wide and 3 m long: 1.5 m².
    expect(layer.fills).toHaveLength(1);
    const p = Array.from(layer.fills[0].positions);
    expect(Math.abs((p[2] - p[0]) * (p[5] - p[1]) - (p[4] - p[0]) * (p[3] - p[1])) / 2).toBeCloseTo(1.5, 9);
  });
});
