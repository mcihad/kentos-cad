import { describe, expect, it } from 'vitest';
import { CoordinateReadTool } from './coordinateTool';
import { MeasureAngleTool } from './angleTool';
import { at, recorder, toolHarness } from './toolHarness';

/** Açı ölç and Koordinat oku (docs/adr/0140): both only say things; nothing is written to the drawing. */
describe('Açı ölç', () => {
  it('measures the angle and its explement in the project unit, says it, and asks again', () => {
    const h = toolHarness();
    const tool = h.use(new MeasureAngleTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('açının tepe noktasını belirtin');
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toContain('birinci kolun');
    tool.pointerDown(at(10, 0));
    expect(tool.prompt.value).toContain('ikinci kolun');
    const r = recorder();
    tool.pointerMove(at(0, 10));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    expect(r.calls).toContain('fillText');
    tool.pointerDown(at(0, 10));
    expect(h.said().at(-1)).toBe('Açı 100.0000 g; dış açı 300.0000 g.');
    // The drawing is untouched, and the tool asks again.
    expect([...h.doc.all()]).toHaveLength(0);
    expect(h.doc.canUndo.value).toBe(false);
    expect(tool.prompt.value).toContain('açının tepe noktasını belirtin');
    // The measurement stays on the drawing until the next one begins; Esc takes it off, the next Esc leaves.
    const kept = recorder();
    tool.draw(kept.g, kept.view);
    expect(kept.calls).toContain('fillText');
    expect(tool.cancel()).toBe(true);
    const gone = recorder();
    tool.draw(gone.g, gone.view);
    expect(gone.calls).not.toContain('stroke');
    expect(tool.cancel()).toBe(false);
  });

  it('the angle is the smaller of the two whichever arm comes first', () => {
    const h = toolHarness();
    const tool = h.use(new MeasureAngleTool(h.ctx));
    tool.activate();
    tool.input('0,0');
    tool.input('0,10');
    tool.input('10,0');
    expect(h.said().at(-1)).toBe('Açı 100.0000 g; dış açı 300.0000 g.');
    tool.input('0,0');
    tool.input('10,0');
    tool.input('-10,10');
    expect(h.said().at(-1)).toBe('Açı 150.0000 g; dış açı 250.0000 g.');
  });

  it('a point on the vertex is refused; Esc steps back a point, then leaves', () => {
    const h = toolHarness();
    const tool = h.use(new MeasureAngleTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(2, 2));
    tool.pointerDown(at(2, 2));
    expect(h.said().at(-1)).toBe('Nokta tepe noktasıyla çakışıyor; birinci kolun başka bir yerini gösterin.');
    tool.pointerDown(at(8, 2));
    tool.pointerDown(at(2, 2));
    expect(h.said().at(-1)).toBe('Nokta tepe noktasıyla çakışıyor; ikinci kolun başka bir yerini gösterin.');
    expect(tool.cancel()).toBe(true);
    expect(tool.cancel()).toBe(true);
    expect(tool.cancel()).toBe(false);
  });

  it('says degrees when the project counts in degrees', () => {
    const h = toolHarness();
    (h.ctx.format as unknown as { prefs: { angleUnit: { set(v: string): void } } }).prefs.angleUnit.set('deg');
    const tool = h.use(new MeasureAngleTool(h.ctx));
    tool.activate();
    tool.input('0,0');
    tool.input('5,0');
    tool.input('5,5');
    expect(h.said().at(-1)).toBe('Açı 45.0000°; dış açı 315.0000°.');
  });
});

describe('Koordinat oku', () => {
  it('says the snapped click’s Y and X, and its Z when the snapped object is a point that has one', () => {
    const h = toolHarness();
    const spot = h.add({ kind: 'point', p: { x: 100.5, y: 200.25 }, z: 12.5 });
    const tool = h.use(new CoordinateReadTool(h.ctx));
    expect(tool.prompt.value).toContain('Koordinat oku');
    tool.pointerDown(at(10, 20));
    expect(h.said().at(-1)).toBe('Y=10.000, X=20.000');
    tool.pointerDown(at(100.5, 200.25, { snap: { kind: 'endpoint', point: { x: 100.5, y: 200.25 }, entityId: spot.id } as never }));
    expect(h.said().at(-1)).toBe('Y=100.500, X=200.250, Z=12.500');
    // A snap to an object without a Z says none.
    const line = h.add({ kind: 'line', a: { x: 0, y: 0 }, b: { x: 5, y: 0 } });
    tool.pointerDown(at(5, 0, { snap: { kind: 'endpoint', point: { x: 5, y: 0 }, entityId: line.id } as never }));
    expect(h.said().at(-1)).toBe('Y=5.000, X=0.000');
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('fillText');
    expect(h.doc.canUndo.value).toBe(true);
    // Nothing more was written than the two objects added above.
    expect([...h.doc.all()]).toHaveLength(2);
  });

  it('reads a typed coordinate too; Enter ends the tool', () => {
    const h = toolHarness();
    const tool = h.use(new CoordinateReadTool(h.ctx));
    expect(tool.input('486512.34,4420118.9')).toBe(true);
    expect(h.said().at(-1)).toBe('Y=486512.340, X=4420118.900');
    expect(tool.input('bir şey')).toBe(false);
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });
});
