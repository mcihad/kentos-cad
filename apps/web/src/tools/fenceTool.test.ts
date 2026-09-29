import { describe, expect, it } from 'vitest';
import type { LineEntity, PolylineEntity } from '../model/entities';
import { ExtendTool, TrimTool } from './edgeTools';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * The Çit method of Buda and Uzat (docs/adr/0140): a fence of clicked points; everything
 * it crosses is trimmed where it crosses (or extended at the end nearest the crossing),
 * all as one cad.entities.edit step.
 */
const lines = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is LineEntity => e.kind === 'line');
const sorted = (ls: LineEntity[]) => ls.map((l) => [l.a.x, l.a.y, l.b.x, l.b.y].map(Math.round).join(',')).sort();

/** Two vertical boundaries at x = 10 and 20 and, across them, horizontal lines at the given heights. */
function crossing(...heights: number[]) {
  const h = toolHarness();
  h.add({ kind: 'line', a: pt(10, -5), b: pt(10, 20) });
  h.add({ kind: 'line', a: pt(20, -5), b: pt(20, 20) });
  const across = heights.map((y) => h.add({ kind: 'line', a: pt(0, y), b: pt(30, y), color: '#E5484D', attrs: { Ad: `y${y}` } }));
  return { h, across };
}

function draw(tool: TrimTool | ExtendTool, ...points: [number, number][]) {
  for (const [x, y] of points) tool.pointerDown(at(x, y));
}

describe('Buda: Çit', () => {
  it('trims every object the fence crosses where it crosses, in one step', () => {
    const { h, across } = crossing(0, 3);
    const tool = h.use(new TrimTool(h.ctx));
    tool.activate();
    // Çit is a method of the catalog, not an option of the first prompt.
    expect(tool.prompt.value).not.toContain('Çit (C)');
    expect(tool.input('C')).toBe(true);
    expect(tool.prompt.value).toContain('çitin ilk noktasına tıklayın: kestiği her parça budanır');
    expect(tool.prompt.value).toContain('Tıklayarak (K)');
    const uid = h.doc.uidOf(across[0].id);
    draw(tool, [15, -2], [15, 5]);
    expect(tool.prompt.value).toContain('çitin sonraki noktasına tıklayın; sağ tık: uygula (2 nesne budanacak)');
    // The preview shows what goes and what stays; nothing is written yet.
    tool.pointerMove(at(15, 5));
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    expect(lines(h)).toHaveLength(4);
    tool.confirm();
    // The middle of each goes; what remains of each keeps its layer and colour (as after a click), the first piece its place and id.
    expect(sorted(lines(h))).toEqual(['0,0,10,0', '0,3,10,3', '10,-5,10,20', '20,-5,20,20', '20,0,30,0', '20,3,30,3'].sort());
    expect(h.doc.uidOf(across[0].id)).toBe(uid);
    expect(lines(h).filter((l) => l.color === '#E5484D')).toHaveLength(4);
    expect(h.said().at(-1)).toBe('Çit: 2 nesne budandı.');
    expect(h.doc.undo()).toBe('Buda');
    expect(lines(h)).toHaveLength(4);
    // The tool stays with a fresh fence.
    expect(tool.prompt.value).toContain('çitin ilk noktasına tıklayın');
    expect(h.state.exited).toBe(0);
  });

  it('an object crossed twice is worked on its new geometry: the second pick trims what the first left', () => {
    const { h, across } = crossing(0);
    const tool = h.use(new TrimTool(h.ctx));
    tool.activate();
    tool.input('C');
    // Crosses the line at x = 5 (before the first boundary) and at x = 25 (after the second).
    draw(tool, [5, -2], [5, 2], [25, 2], [25, -2]);
    tool.confirm();
    // The first pick took [0, 10], the second what was left of [20, 30]: [10, 20] remains.
    expect(sorted(lines(h).filter((l) => l.id === across[0].id))).toEqual(['10,0,20,0']);
    // A single piece keeps everything, its attributes too.
    expect(h.doc.get(across[0].id)?.attrs).toEqual({ Ad: 'y0' });
    expect(h.doc.undo()).toBe('Buda');
    expect(h.doc.get(across[0].id)).toMatchObject({ a: pt(0, 0), b: pt(30, 0) });
  });

  it('a crossing in a part an earlier pick took away is nothing to do', () => {
    const { h, across } = crossing(0);
    const tool = h.use(new TrimTool(h.ctx));
    tool.activate();
    tool.input('C');
    draw(tool, [15, -2], [15, 2], [16, 2], [16, -2]);
    tool.confirm();
    const left = lines(h).filter((l) => l.color === '#E5484D');
    expect(sorted(left)).toEqual(['0,0,10,0', '20,0,30,0']);
    expect(h.said().at(-1)).toBe('Çit: 1 nesne budandı.');
    expect(across).toHaveLength(1);
  });

  it('says so when the fence crosses nothing, and writes nothing', () => {
    const { h } = crossing(0);
    const tool = h.use(new TrimTool(h.ctx));
    tool.activate();
    tool.input('C');
    draw(tool, [50, 50], [60, 50]);
    tool.confirm();
    expect(h.said().at(-1)).toBe('Çit hiçbir düzenlenebilir nesneyi kesmiyor; çiti nesnelerin üzerinden geçirin.');
    expect(h.doc.canUndo.value).toBe(true);
    expect(lines(h)).toHaveLength(3);
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('leaves the objects on a locked layer alone and says so', () => {
    const { h } = crossing(0);
    h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 3), b: pt(30, 3) });
    const tool = h.use(new TrimTool(h.ctx));
    tool.activate();
    tool.input('C');
    draw(tool, [15, -2], [15, 5]);
    tool.confirm();
    expect(h.said()).toContain('1 nesne kilitli katmanda olduğu için atlandı.');
    expect(lines(h).find((l) => l.layerId === 'kilitli')).toMatchObject({ a: pt(0, 3), b: pt(30, 3) });
    expect(h.said().at(-1)).toBe('Çit: 1 nesne budandı.');
  });

  it('an area with holes is not trimmed, as a click refuses it', () => {
    const { h } = crossing();
    const ring = h.add({ kind: 'polygon', pts: [pt(0, 0), pt(30, 0), pt(30, 10), pt(0, 10)], holes: [{ pts: [pt(12, 2), pt(18, 2), pt(18, 8), pt(12, 8)] }] });
    const tool = h.use(new TrimTool(h.ctx));
    tool.activate();
    tool.input('C');
    draw(tool, [15, -2], [15, 12]);
    tool.confirm();
    expect(h.said().at(-1)).toMatch(/^Çitin kestiği 1 nesne budanamadı/);
    expect(h.doc.get(ring.id)?.kind).toBe('polygon');
  });

  it('Esc steps back: the fence, then the method, then it leaves; K and G work as options', () => {
    const { h } = crossing(0);
    const tool = h.use(new TrimTool(h.ctx));
    tool.activate();
    tool.input('C');
    expect(tool.snaps).toBe(true);
    draw(tool, [15, -2], [15, 2]);
    expect(tool.input('G')).toBe(true);
    expect(tool.prompt.value).toContain('(1 nokta)');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('çitin ilk noktasına tıklayın');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('silinecek parçaya tıklayın');
    expect(tool.snaps).toBe(false);
    expect(tool.cancel()).toBe(false);
    tool.input('C');
    expect(tool.input('K')).toBe(true);
    expect(tool.prompt.value).toContain('silinecek parçaya tıklayın');
    // Enter with nothing drawn leaves the tool from the fence method.
    tool.input('C');
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('one point of a fence is let go by Enter, not applied', () => {
    const { h } = crossing(0);
    const tool = h.use(new TrimTool(h.ctx));
    tool.activate();
    tool.input('C');
    draw(tool, [15, -2]);
    tool.confirm();
    expect(tool.prompt.value).toContain('çitin ilk noktasına tıklayın');
    expect(lines(h)).toHaveLength(3);
  });
});

describe('Uzat: Çit', () => {
  /** A boundary at x = 20 and two short lines that stop short of it. */
  function shortOnes() {
    const h = toolHarness();
    h.add({ kind: 'line', a: pt(20, -5), b: pt(20, 20) });
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0), color: '#E5484D' });
    const b = h.add({ kind: 'line', a: pt(0, 2), b: pt(12, 2) });
    return { h, a, b };
  }

  it('extends the end of every object the fence crosses that lies nearest the crossing, in one step', () => {
    const { h, a, b } = shortOnes();
    const tool = h.use(new ExtendTool(h.ctx));
    tool.activate();
    // Çit is a method of the catalog, not an option of the first prompt.
    expect(tool.prompt.value).not.toContain('Çit (C)');
    tool.input('C');
    expect(tool.prompt.value).toContain('çitin ilk noktasına tıklayın: kestiği her nesnenin ucu uzatılır');
    // The fence crosses both lines nearer their east ends.
    draw(tool, [8, -1], [8, 3]);
    expect(tool.prompt.value).toContain('2 nesne uzatılacak');
    const r = recorder();
    tool.pointerMove(at(8, 3));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    tool.confirm();
    expect(h.doc.get(a.id)).toMatchObject({ a: pt(0, 0), b: pt(20, 0), color: '#E5484D' });
    expect(h.doc.get(b.id)).toMatchObject({ a: pt(0, 2), b: pt(20, 2) });
    expect(h.said().at(-1)).toBe('Çit: 2 nesne uzatıldı.');
    expect(h.doc.undo()).toBe('Uzat');
    expect(h.doc.get(a.id)).toMatchObject({ b: pt(10, 0) });
  });

  it('the end nearest the crossing: a fence near the start extends the start', () => {
    const h = toolHarness();
    h.add({ kind: 'line', a: pt(-10, -5), b: pt(-10, 20) });
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0) });
    const tool = h.use(new ExtendTool(h.ctx));
    tool.activate();
    tool.input('C');
    draw(tool, [3, -2], [3, 2]);
    tool.confirm();
    expect(h.doc.get(a.id)).toMatchObject({ a: pt(-10, 0), b: pt(30, 0) });
  });

  it('extends each end at most once, however often the fence crosses near it; Ç works as C', () => {
    const h = toolHarness();
    h.add({ kind: 'line', a: pt(20, -5), b: pt(20, 20) });
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new ExtendTool(h.ctx));
    tool.activate();
    expect(tool.input('Ç')).toBe(true);
    draw(tool, [8, -1], [8, 1], [9, 1], [9, -1]);
    tool.confirm();
    expect(h.doc.get(a.id)).toMatchObject({ a: pt(0, 0), b: pt(20, 0) });
    expect(h.said().at(-1)).toBe('Çit: 1 nesne uzatıldı.');
  });

  it('an object with nothing ahead of it is counted as not done', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new ExtendTool(h.ctx));
    tool.activate();
    tool.input('C');
    draw(tool, [8, -1], [8, 1]);
    tool.confirm();
    expect(h.said().at(-1)).toMatch(/^Çitin kestiği 1 nesne uzatılamadı/);
    expect(h.doc.get(a.id)).toMatchObject({ b: pt(10, 0) });
  });

  it('extends a polyline at its end too', () => {
    const h = toolHarness();
    h.add({ kind: 'line', a: pt(20, -5), b: pt(20, 20) });
    const p = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 5), pt(10, 5)] });
    const tool = h.use(new ExtendTool(h.ctx));
    tool.activate();
    tool.input('C');
    draw(tool, [8, 3], [8, 8]);
    tool.confirm();
    expect((h.doc.get(p.id) as PolylineEntity).pts.at(-1)).toEqual(pt(20, 5));
  });
});
