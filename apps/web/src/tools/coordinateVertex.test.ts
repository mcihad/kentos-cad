import { describe, expect, it } from 'vitest';
import { CoordinateReadTool } from './coordinateTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Koordinat oku on a vertex that has an elevation (docs/adr/0142): the reading gains its Z, in the format a
 * point's elevation has had since docs/adr/0140; a vertex without one, or a snap that is no vertex, reads as before.
 */
function scene() {
  const h = toolHarness();
  const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0), za: 118.5, zb: 0 });
  const path = h.add({ kind: 'polyline', pts: [pt(0, 5), pt(5, 5), pt(9, 9)], zs: [98.5, null, 105.25] });
  const area = h.add({ kind: 'polygon', pts: [pt(20, 20), pt(30, 20), pt(30, 30), pt(20, 30)], holes: [{ pts: [pt(22, 22), pt(24, 22), pt(24, 24)], zs: [7, 8, 9] }] });
  const tool = h.use(new CoordinateReadTool(h.ctx));
  /** A click snapped to `kind` at (x, y) on `entity`. */
  const click = (entity: { id: number }, x: number, y: number, kind = 'endpoint') => {
    tool.pointerDown(at(x, y, { snap: { kind, point: pt(x, y), entityId: entity.id } as never }));
    return h.said().at(-1);
  };
  return { h, tool, line, path, area, click };
}

describe('Koordinat oku on a vertex with an elevation', () => {
  it('says the Z of a line’s end, of a polyline’s vertex and of a hole’s', () => {
    const { click, line, path, area } = scene();
    expect(click(line, 0, 0)).toBe('Y=0.000, X=0.000, Z=118.500');
    expect(click(path, 9, 9)).toBe('Y=9.000, X=9.000, Z=105.250');
    expect(click(path, 0, 5)).toBe('Y=0.000, X=5.000, Z=98.500');
    expect(click(area, 24, 22)).toBe('Y=24.000, X=22.000, Z=8.000');
  });

  it('0 is an elevation and is said', () => {
    const { click, line } = scene();
    expect(click(line, 10, 0)).toBe('Y=10.000, X=0.000, Z=0.000');
  });

  it('reads as before for a vertex without one, a vertex of an object without any, and a snap that is no vertex', () => {
    const { h, click, line, path, area } = scene();
    expect(click(path, 5, 5)).toBe('Y=5.000, X=5.000');
    // The outer ring of the area has none.
    expect(click(area, 30, 20)).toBe('Y=30.000, X=20.000');
    // The middle of an elevated line is no vertex: the elevation there is not read off the edge.
    expect(click(line, 5, 0, 'midpoint')).toBe('Y=5.000, X=0.000');
    // An object that takes no elevation.
    const circle = h.add({ kind: 'circle', c: pt(50, 50), r: 5 });
    expect(click(circle, 55, 50, 'quadrant')).toBe('Y=55.000, X=50.000');
  });

  it('a click that snaps to nothing, and a typed coordinate, have no Z', () => {
    const { h, tool } = scene();
    tool.pointerDown(at(0, 0));
    expect(h.said().at(-1)).toBe('Y=0.000, X=0.000');
    expect(tool.input('0,0')).toBe(true);
    expect(h.said().at(-1)).toBe('Y=0.000, X=0.000');
  });

  it('the tag beside the point says Z on a line of its own, as it does for a point', () => {
    const h = toolHarness();
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0), za: 118.5 });
    const tool = h.use(new CoordinateReadTool(h.ctx));
    tool.pointerDown(at(0, 0, { snap: { kind: 'endpoint', point: pt(0, 0), entityId: line.id } as never }));
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('fillText');
    // The three lines of the tag, by what the canvas was asked to draw.
    const texts: string[] = [];
    const g = new Proxy({}, { get: (_t, k: string) => (k === 'measureText' ? () => ({ width: 40 }) : k === 'fillText' ? (t: string) => void texts.push(t) : () => {}), set: () => true }) as unknown as CanvasRenderingContext2D;
    tool.draw(g, r.view);
    expect(texts).toEqual(['Y 0.000', 'X 0.000', 'Z 118.500']);
  });
});
