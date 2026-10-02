import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../model/entities';
import { AreaMeasureTool, DistanceTool } from './measureTools';
import { PathTool } from './pathTool';
import { at, pt, toolHarness } from './toolHarness';

/**
 * İzle in the path tools (docs/adr/0161 §1): off at first; a click near a line goes onto it while it is on; a segment
 * between points on connected line work runs along it, and closing on the first corner goes back along it; not for
 * Sabit ilk nokta's rays nor İçine tıkla. Two parcels side by side: (0, 0), (100, 0), (100, 50), (0, 60) and
 * (100, 0)–(200, 50); the screen is the world, so the 8 px snap aperture reaches 8 m. Expected values are worked out
 * by hand; the desktop's are crates/native/interaction/tests/trace_draw.rs, the shared trace
 * fixtures/interaction/v1/trace-draw.json. İzle is kept for the session (static), so each test sets it as it needs.
 */
function scene() {
  const h = toolHarness();
  h.add({ kind: 'polygon', pts: [pt(0, 0), pt(100, 0), pt(100, 50), pt(0, 60)] });
  h.add({ kind: 'polygon', pts: [pt(100, 0), pt(200, 0), pt(200, 50), pt(100, 50)] });
  return h;
}

const on = (tool: { prompt: { value: string } }) => tool.prompt.value.includes('İzle (İ): açık');

/** Sets İzle for the session, through a polyline's chip (offered from its first point on). */
function setTrace(h: ReturnType<typeof scene>, want: boolean): void {
  const tool = new PathTool(h.ctx, { id: 'polyline', label: 'Çoklu çizgi', closed: false });
  tool.activate();
  tool.pointerDown(at(500, 500));
  if (on(tool) !== want) expect(tool.input('İ')).toBe(true);
  expect(on(tool)).toBe(want);
  tool.deactivate();
}

const newest = (h: ReturnType<typeof scene>) => [...h.doc.all()].at(-1) as PolylineEntity;

function polyline(h: ReturnType<typeof scene>, closed = false): PathTool {
  const tool = h.use(new PathTool(h.ctx, { id: closed ? 'polygon' : 'polyline', label: closed ? 'Kapalı alan' : 'Çoklu çizgi', closed }));
  tool.activate();
  return tool;
}

describe('İzle', () => {
  it('is off at first: a click near a line stays where it is', () => {
    const h = scene();
    const tool = polyline(h);
    tool.pointerDown(at(50, 2));
    expect(on(tool)).toBe(false);
    tool.pointerDown(at(50, 30));
    tool.confirm();
    expect(newest(h).pts).toEqual([pt(50, 2), pt(50, 30)]);
    tool.deactivate();
  });

  it('closes a new parcel along its neighbours’ boundary', () => {
    const h = scene();
    const tool = polyline(h, true);
    tool.pointerDown(at(200, 50));
    if (!on(tool)) expect(tool.input('İ')).toBe(true);
    expect(tool.prompt.value).toBe('Kapalı alan: sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / İzle (İ): açık / Akış (A) / Geri (G)]');
    // Free corners: no line within the aperture, straight edges.
    tool.pointerDown(at(200, 100));
    tool.pointerDown(at(0, 120));
    tool.pointerDown(at(0, 60));
    // The first corner again: the last edge goes back along the parcels' northern boundary.
    tool.pointerDown(at(200, 50));
    expect(newest(h).pts).toEqual([pt(200, 50), pt(200, 100), pt(0, 120), pt(0, 60), pt(100, 50)]);
    tool.deactivate();
  });

  it('puts a click near a line onto it, and the way follows the boundary', () => {
    const h = scene();
    setTrace(h, true);
    const tool = polyline(h);
    // 4 m off the southern edge: onto it.
    tool.pointerDown(at(50, -4));
    // 6 m off the shared edge: onto it, by the way the boundary goes.
    tool.pointerDown(at(106, 30));
    tool.confirm();
    expect(newest(h).pts).toEqual([pt(50, 0), pt(100, 0), pt(100, 30)]);
    tool.deactivate();
  });

  it('follows nothing for Sabit ilk nokta’s rays', () => {
    const h = scene();
    setTrace(h, true);
    const tool = h.use(new DistanceTool(h.ctx));
    tool.activate();
    if (!tool.prompt.value.includes('Sabit ilk nokta (S): açık')) expect(tool.input('S')).toBe(true);
    tool.pointerDown(at(50, 5));
    const before = h.said().length;
    tool.pointerDown(at(50, 30));
    // The first point was not put on the edge 5 m below: 25 m, not 30 (the echo says where the click went).
    expect(h.said().slice(before)).toEqual(['  Y 50.000  X 30.000', '1: 25.000 m, semt 0.0000 g']);
    expect(tool.input('S')).toBe(true);
    tool.deactivate();
  });

  it('leaves İçine tıkla’s click inside the region it measures', () => {
    const h = scene();
    setTrace(h, true);
    const tool = h.use(new AreaMeasureTool(h.ctx));
    tool.activate();
    if (!tool.prompt.value.includes('İçine tıkla (I): açık')) expect(tool.input('I')).toBe(true);
    const before = h.said().length;
    // 5 m inside the southern edge: the click stays inside, the parcel is measured.
    tool.pointerDown(at(50, 5));
    expect(h.said().slice(before)).toEqual(['  Y 50.000  X 5.000', expect.stringMatching(/^Alan 5500\.00 m²/)]);
    expect(tool.input('I')).toBe(true);
    setTrace(h, false);
    tool.deactivate();
  });
});

describe('Akış', () => {
  it('leaves a vertex every step the pointer goes; B asks for the step; Sabit ilk nokta streams nothing', () => {
    const h = toolHarness();
    const tool = polyline(h as ReturnType<typeof scene>);
    tool.pointerDown(at(-20, 0));
    expect(tool.input('A')).toBe(true);
    expect(tool.input('B')).toBe(true);
    expect(tool.prompt.value).toBe('Çoklu çizgi: akışın adım boyunu yazın [Geri (G)]');
    expect(tool.input('0')).toBe(true);
    expect(h.said().at(-1)).toBe('Adım boyu sıfırdan büyük olmalı.');
    expect(tool.input('2')).toBe(true);
    const steps: [[number, number], number][] = [
      [[-19, 0], 1],
      [[-17.5, 0], 2],
      [[-16, 0.5], 2],
      [[-15, 1], 3],
      [[-13, 2], 4],
    ];
    for (const [[x, y], n] of steps) {
      tool.pointerMove(at(x, y));
      expect(tool.pointCount, `${x}, ${y}`).toBe(n);
    }
    // The step back to 1 m and Akış off, as the session began.
    expect(tool.input('B')).toBe(true);
    expect(tool.input('1')).toBe(true);
    tool.confirm();
    expect(newest(h as ReturnType<typeof scene>).pts).toEqual([pt(-20, 0), pt(-17.5, 0), pt(-15, 1), pt(-13, 2)]);
    tool.deactivate();
    const measure = h.use(new DistanceTool(h.ctx));
    measure.activate();
    if (!measure.prompt.value.includes('Sabit ilk nokta (S): açık')) expect(measure.input('S')).toBe(true);
    measure.pointerDown(at(0, 0));
    measure.pointerMove(at(10, 0));
    expect(measure.pointCount).toBe(1);
    expect(measure.input('S')).toBe(true);
    measure.deactivate();
    // Akış off for the session again.
    const off = polyline(h as ReturnType<typeof scene>);
    off.pointerDown(at(50, 50));
    expect(off.input('A')).toBe(true);
    expect(off.prompt.value).not.toContain('Akış (A): açık');
    off.deactivate();
  });
});
