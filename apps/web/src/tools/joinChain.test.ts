import { describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import { JoinTool } from './editTools';
import { at, pt, toolHarness } from './toolHarness';

/**
 * Birleştir's Zincir (docs/adr/0161 §2), on the shared trace's drawing in small (fixtures/interaction/v1/join-chain.kcad):
 * lines 1 and 2, the arc 3 and the junction's lines 4 and 5; line 6 on a locked layer at line 1's western end. Expected
 * values are worked out by hand; the desktop's are crates/native/interaction/tests/all/join_chain.rs. Zincir is kept for the
 * session (static), so each test sets it as it needs.
 */
function scene() {
  const h = toolHarness();
  const ids = [
    h.add({ kind: 'line', layerId: 'yol', a: pt(-25, -5), b: pt(-15, -5) }),
    h.add({ kind: 'line', layerId: 'yol', a: pt(-15, -5), b: pt(-5, 0) }),
    h.add({ kind: 'arc', layerId: 'yol', c: pt(0, -5), r: Math.hypot(5, 5), a0: Math.PI / 4, a1: (3 * Math.PI) / 4 }),
    h.add({ kind: 'line', layerId: 'yol', a: pt(5, 0), b: pt(15, -5) }),
    h.add({ kind: 'line', layerId: 'yol', a: pt(5, 0), b: pt(15, 5) }),
    h.add({ kind: 'line', layerId: 'kilitli', a: pt(-28, -8), b: pt(-25, -5) }),
  ].map((e) => e.id);
  const tool = h.use(new JoinTool(h.ctx));
  return { h, ids, tool };
}

const chainOn = (tool: JoinTool) => tool.prompt.value.includes('Zincir (Z): açık');

describe('Zincir', () => {
  it('joins the chain of the object clicked and says where it stopped', () => {
    const { h, ids, tool } = scene();
    tool.activate();
    if (!chainOn(tool)) expect(tool.input('Z')).toBe(true);
    expect(tool.prompt.value).toBe('Birleştir: zincirin bir nesnesine tıklayın [uç boşluğu toleransı 0.001 m; değiştirmek için sayı yazın; Zincir (Z): açık]');
    h.state.hit = h.doc.get(ids[1]) as Entity;
    const before = h.said().length;
    tool.pointerDown(at(-10, -2.5));
    tool.pointerUp(at(-10, -2.5));
    expect([...h.doc.all()].map((e) => e.id)).toEqual([ids[1], ids[3], ids[4], ids[5]]);
    const p = h.doc.get(ids[1]) as PolylineEntity;
    expect(p.kind).toBe('polyline');
    const want = [pt(-25, -5), pt(-15, -5), pt(-5, 0), pt(5, 0)];
    p.pts.forEach((q, i) => expect(Math.hypot(q.x - want[i].x, q.y - want[i].y)).toBeLessThan(1e-9));
    expect(Math.abs(p.bulges![2] + 0.414213562373095)).toBeLessThan(1e-9);
    expect(h.said().slice(before).slice(-2)).toEqual(['3 nesne birleştirildi: çoklu çizgi.', 'Zincir kilitli katmandaki bir nesnede durdu.']);
    expect(h.doc.undo()).toBe('Birleştir');
    expect([...h.doc.all()].map((e) => e.id)).toEqual(ids);
  });

  it('says a locked or lonely object and waits for another click', () => {
    const { h, ids, tool } = scene();
    tool.activate();
    if (!chainOn(tool)) expect(tool.input('Z')).toBe(true);
    h.state.hit = h.doc.get(ids[5]) as Entity;
    const before = h.said().length;
    tool.pointerDown(at(-26.5, -6.5));
    tool.pointerUp(at(-26.5, -6.5));
    expect(h.said().slice(before)).toEqual(['Kilitli katmandaki nesne birleştirilemez.']);
    expect(h.state.exited).toBe(0);
    // Off: a click selects as usual.
    expect(tool.input('Z')).toBe(true);
    h.state.hit = h.doc.get(ids[3]) as Entity;
    tool.pointerDown(at(10, -2.5));
    tool.pointerUp(at(10, -2.5));
    expect([...h.ctx.selection.ids.value]).toEqual([ids[3]]);
  });
});
