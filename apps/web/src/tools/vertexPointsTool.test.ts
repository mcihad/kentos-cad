import { beforeEach, describe, expect, it } from 'vitest';
import type { PointEntity } from '../model/entities';
import { SurveyPointTool } from './surveyPointTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';
import { VertexPointsTool } from './vertexPointsTool';

/**
 * Köşelere nokta (docs/adr/0152 §5): selection first, the points shown with their names, Ad and Kod Nokta's, the points
 * written in one step on the active layer and Nokta's Ad moved on. The cases are worked out by hand from the rules; the
 * core itself is checked against the independent reference in model/ops/vertexPoints.test.ts. The desktop walks the
 * same in crates/native/interaction/tests/vertex_points.rs.
 */

beforeEach(() => {
  SurveyPointTool.next = '';
  SurveyPointTool.code = '';
  SurveyPointTool.z = null;
});

const points = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is PointEntity => e.kind === 'point');

/** Two 20 × 15 parcels side by side, the left one's corners with elevations, and a point on a shared corner. */
function parcels(h: ReturnType<typeof toolHarness>) {
  const left = h.add({ kind: 'polygon', pts: [pt(0, 0), pt(20, 0), pt(20, 15), pt(0, 15)], zs: [100, 100.5, null, 101] });
  const right = h.add({ kind: 'polygon', pts: [pt(20, 0), pt(40, 0), pt(40, 15), pt(20, 15)] });
  h.add({ kind: 'point', p: pt(20, 15), label: '7' });
  return [left.id, right.id];
}

describe('Köşelere nokta', () => {
  it('puts a named point at every corner once, past the one a point holds, in one step', () => {
    const h = toolHarness();
    h.ctx.selection.set(parcels(h));
    SurveyPointTool.next = '101';
    SurveyPointTool.code = 'SN';
    const tool = h.use(new VertexPointsTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Köşelere nokta: 5 nokta; 1 köşede zaten nokta var [Ad (A): 101 / Kod (K): SN / Uygula (Enter)]');
    // A click places nothing.
    tool.pointerDown(at(50, 50));
    expect(points(h)).toHaveLength(1);
    tool.pointerMove(at(50, 20));
    const log = canvasLog();
    tool.draw(log.g, log.view);
    expect(log.texts).toEqual(['101', '102', '103', '104', '105', '5 nokta', '1 köşede zaten nokta var', 'Enter: uygula']);
    expect(log.arcs.slice(0, 5)).toEqual([[0, 0], [20, 0], [0, 15], [40, 0], [40, 15]]);
    tool.confirm();
    expect(points(h).slice(1).map((e) => [e.p, e.label, e.attrs, e.z ?? null, e.layerId])).toEqual([
      [pt(0, 0), '101', { Kod: 'SN' }, 100, 'cizim'],
      [pt(20, 0), '102', { Kod: 'SN' }, 100.5, 'cizim'],
      [pt(0, 15), '103', { Kod: 'SN' }, 101, 'cizim'],
      [pt(40, 0), '104', { Kod: 'SN' }, null, 'cizim'],
      [pt(40, 15), '105', { Kod: 'SN' }, null, 'cizim'],
    ]);
    expect(h.said().at(-1)).toBe('Köşelere nokta: 5 nokta eklendi (101 – 105); 1 köşede zaten nokta vardı.');
    // Nokta goes on past the last name.
    expect(SurveyPointTool.next).toBe('106');
    expect(h.state.exited).toBe(1);
    expect(h.doc.undo()).toBe('Köşelere nokta');
    expect(points(h)).toHaveLength(1);
  });

  it('picks first when nothing is selected, and takes Ad and Kod as Nokta does', async () => {
    const h = toolHarness();
    const [left] = parcels(h);
    const tool = h.use(new VertexPointsTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Köşelere nokta: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili)');
    h.state.hit = h.doc.get(left)!;
    tool.pointerDown(at(1, 1));
    tool.pointerUp(at(1, 1));
    tool.confirm();
    expect(tool.prompt.value).toBe('Köşelere nokta: 3 nokta; 1 köşede zaten nokta var [Ad (A): — / Kod (K): — / Uygula (Enter)]');
    expect(tool.input('a')).toBe(true);
    expect(tool.prompt.value).toBe('Köşelere nokta: değeri yazın');
    await Promise.resolve();
    h.state.textInputs.at(-1)!.commit('P9');
    expect(tool.prompt.value).toBe('Köşelere nokta: 3 nokta; 1 köşede zaten nokta var [Ad (A): P9 / Kod (K): — / Uygula (Enter)]');
    tool.confirm();
    expect(points(h).slice(1).map((e) => [e.label, e.attrs])).toEqual([
      ['P9', {}],
      ['P10', {}],
      ['P11', {}],
    ]);
    expect(h.said().at(-1)).toBe('Köşelere nokta: 3 nokta eklendi (P9 – P11); 1 köşede zaten nokta vardı.');
    expect(SurveyPointTool.next).toBe('P12');
  });

  it('writes unnamed points without a name, and leaves Nokta’s empty Ad as it is', () => {
    const h = toolHarness();
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    h.ctx.selection.set([line.id]);
    const tool = h.use(new VertexPointsTool(h.ctx));
    tool.activate();
    tool.confirm();
    expect(points(h).map((e) => [e.p, e.label])).toEqual([
      [pt(0, 0), undefined],
      [pt(10, 0), undefined],
    ]);
    expect(h.said().at(-1)).toBe('Köşelere nokta: 2 nokta eklendi.');
    expect(SurveyPointTool.next).toBe('');
  });

  it('says when every corner has a point already, and when the selection has no corners', async () => {
    const h = toolHarness();
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    h.add({ kind: 'point', p: pt(0, 0) });
    h.add({ kind: 'point', p: pt(10, 0) });
    h.ctx.selection.set([line.id]);
    const tool = h.use(new VertexPointsTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Köşelere nokta: yazılacak nokta yok; 2 köşede zaten nokta var [Ad (A): — / Kod (K): — / Uygula (Enter)]');
    tool.confirm();
    expect(h.said().at(-1)).toBe('Köşelere nokta: yazılacak nokta yok; 2 köşede zaten nokta var.');
    expect(points(h)).toHaveLength(2);
    const circle = h.add({ kind: 'circle', c: pt(0, 0), r: 5 });
    h.ctx.selection.set([circle.id]);
    const again = h.use(new VertexPointsTool(h.ctx));
    again.activate();
    expect(h.said().at(-1)).toBe('Köşelere nokta: seçimde çizgi, çoklu çizgi ya da alan yok.');
    await Promise.resolve();
    expect(h.state.exited).toBe(2);
  });

  it('is refused on a locked layer, and stays', () => {
    const h = toolHarness();
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    h.ctx.selection.set([line.id]);
    h.doc.layers.active.set('kilitli');
    const tool = h.use(new VertexPointsTool(h.ctx));
    tool.activate();
    tool.confirm();
    expect(points(h)).toHaveLength(0);
    expect(h.said().at(-1)).toMatch(/kilitli/);
    expect(h.state.exited).toBe(0);
  });
});
