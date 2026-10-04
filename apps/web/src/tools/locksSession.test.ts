import { describe, expect, it } from 'vitest';
import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import type { PolylineEntity } from '../model/entities';
import { clearLocks, keepLocks, lockLength, lockToward, NO_LOCKS, typedLock, type LockState } from './locks';
import { PathTool } from './pathTool';
import { at, toolHarness } from './toolHarness';

/**
 * The digitizing locks with a running tool (docs/adr/0166 §1–§2, §6), on Çoklu çizgi over the harness's drawing (a CBS
 * project in grads), the desktop's crates/native/interaction/tests/all/locks.rs: a locked length and a typed direction hold
 * the next point, one-shot locks go once it is placed and kept ones follow, Sapma turns from the last edge, Kilitleri
 * kaldır lets them go.
 */
function polyline() {
  const h = toolHarness();
  const ctx = h.ctx as AppContext & { settings: { locks: Signal<LockState> }; tools: Record<string, unknown> };
  (ctx.settings as unknown as Record<string, unknown>).locks = new Signal<LockState>(NO_LOCKS);
  const tool = new PathTool(ctx, { id: 'polyline', label: 'Çoklu çizgi', closed: false });
  Object.assign(ctx.tools, { active: tool, activeId: new Signal('polyline') });
  tool.activate();
  const click = (x: number, y: number) => tool.pointerDown(at(x, y));
  const newest = () => [...h.doc.all()].at(-1) as PolylineEntity;
  const near = (p: { x: number; y: number }, x: number, y: number) => Math.abs(p.x - x) < 1e-6 && Math.abs(p.y - y) < 1e-6;
  return { ...h, ctx, tool, click, newest, near, locks: () => ctx.settings.locks.value };
}

describe('Sayısallaştırma kilitleri araçta (docs/adr/0166)', () => {
  it('holds the next point at a locked length and lets it go once placed', () => {
    const t = polyline();
    t.click(0, 0);
    expect(lockLength(t.ctx, 12)).toBe(true);
    expect(t.said().at(-1)).toBe('Kilit: Uzunluk 12.000 m.');
    t.click(30, 40);
    expect(t.locks().length, 'a one-shot lock goes with the point').toBeNull();
    t.click(7.2, 20);
    t.tool.confirm();
    const pts = t.newest().pts;
    expect(t.near(pts[1], 7.2, 9.6), JSON.stringify(pts)).toBe(true);
    expect(t.near(pts[2], 7.2, 20), 'the next point is free').toBe(true);
  });

  it('locks a typed direction one way; a bare distance follows it', () => {
    const t = polyline();
    t.click(0, 0);
    expect(typedLock(t.ctx, '<100')).toBe(true);
    expect(t.locks().toward).toEqual({ kind: 'angle', value: 100 });
    expect(t.said().at(-1)).toBe('Kilit: Semt 100.0000 g.');
    t.tool.pointerMove(at(20, 7));
    expect(t.tool.input('15')).toBe(true);
    t.tool.confirm();
    expect(t.near(t.newest().pts[1], 15, 0), JSON.stringify(t.newest().pts)).toBe(true);
  });

  it('gives the reference for a cursor behind a one-way direction', () => {
    const t = polyline();
    t.click(0, 0);
    typedLock(t.ctx, '<100');
    t.click(-20, 3);
    expect(t.tool.pointCount).toBe(1);
    expect(t.locks().toward, 'the lock waits for its point').not.toBeNull();
  });

  it('keeps kept locks for the following points', () => {
    const t = polyline();
    t.click(0, 0);
    keepLocks(t.ctx, true);
    expect(lockLength(t.ctx, 5)).toBe(true);
    t.click(0, 30);
    t.click(30, 5);
    expect(t.locks().length).toBe(5);
    t.tool.confirm();
    const pts = t.newest().pts;
    expect(t.near(pts[1], 0, 5) && t.near(pts[2], 5, 5), JSON.stringify(pts)).toBe(true);
    // The command waits for a new path: no reference, so no lock; Kalıcı stays with the command.
    expect(t.locks().length).toBeNull();
    expect(t.locks().keep).toBe(true);
  });

  it('turns a deflection from the last edge', () => {
    const t = polyline();
    t.click(0, 0);
    expect(lockToward(t.ctx, { kind: 'deflection', value: 100 })).toBe(false);
    expect(t.said().at(-1)).toBe('Sapma için önce bir kenar çizin: sapma önceki kenarın doğrultusundan ölçülür.');
    t.click(10, 0);
    // A CBS project turns clockwise: 100 grads right of east is south.
    expect(lockToward(t.ctx, { kind: 'deflection', value: 100 })).toBe(true);
    t.click(13, -20);
    t.tool.confirm();
    expect(t.near(t.newest().pts[2], 10, -20), JSON.stringify(t.newest().pts)).toBe(true);
  });

  it('lets the locks go on Kilitleri kaldır', () => {
    const t = polyline();
    t.click(0, 0);
    lockLength(t.ctx, 12);
    expect(clearLocks(t.ctx)).toBe(true);
    expect(t.locks().length).toBeNull();
    expect(t.said().at(-1)).toBe('Kilitler kaldırıldı.');
    expect(t.tool.pointCount).toBe(1);
  });

  it('locks nothing before the first point', () => {
    const t = polyline();
    expect(lockLength(t.ctx, 12)).toBe(false);
    expect(typedLock(t.ctx, '<100'), 'lock text is understood, and refused').toBe(true);
    const why = 'Kilit için önce bir nokta verin: uzunluk ve doğrultu son noktadan ölçülür.';
    expect(t.said().slice(-2)).toEqual([why, why]);
    expect(t.locks()).toEqual(NO_LOCKS);
  });
});
