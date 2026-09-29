import { describe, expect, it } from 'vitest';
import { parsePrompt } from '../ui/promptOptions';
import { ON_LINE, readStation, StationOffsetTool, stationLine, stationTag } from './stationOffsetTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * Dik ayak ölç (docs/adr/0141): the base line by two clicks, each point measured against it. The
 * worked example of the ADR, the sides, both ends, the steps of Esc, and that nothing is written.
 */

/** A tool with its line A(0,0)–B(100,0) clicked. */
function withLine() {
  const h = toolHarness();
  const tool = h.use(new StationOffsetTool(h.ctx));
  tool.activate();
  tool.pointerDown(at(0, 0));
  tool.pointerDown(at(100, 0));
  return { h, tool };
}
const format = { length: (m: number) => `${m.toFixed(3)} m` };
const said = (h: ReturnType<typeof toolHarness>) => h.said().filter((t) => t.startsWith('Dik ayak'));

describe('the reading', () => {
  it('the worked example: A(0,0), B(100,0), P(30,5) is a foot of 30 and an offset of 5 to the left', () => {
    const r = readStation(pt(0, 0), pt(100, 0), pt(30, 5))!;
    expect(r.foot).toBeCloseTo(30, 12);
    expect(r.offset).toBeCloseTo(-5, 12);
    expect(r.side).toBe('left');
    expect(r.footPoint.x).toBeCloseTo(30, 12);
    expect(r.footPoint.y).toBeCloseTo(0, 12);
    expect(stationLine(format, r)).toBe('Dik ayak 30.000 m, dik boy 5.000 m (solda)');
    // The tag's offset is signed: − (U+2212) for the left.
    expect(stationTag(format, r)).toEqual(['Ayak 30.000 m', 'Boy −5.000 m']);
  });

  it('the right of the line is positive', () => {
    const r = readStation(pt(0, 0), pt(100, 0), pt(30, -5))!;
    expect(r.offset).toBeCloseTo(5, 12);
    expect(r.side).toBe('right');
    expect(stationLine(format, r)).toBe('Dik ayak 30.000 m, dik boy 5.000 m (sağda)');
    expect(stationTag(format, r)).toEqual(['Ayak 30.000 m', 'Boy +5.000 m']);
  });

  it('agrees with Nokta hesapla’s yan nokta: a line running north has its right in the east', () => {
    const r = readStation(pt(0, 0), pt(0, 100), pt(5, 30))!;
    expect(r.foot).toBeCloseTo(30, 12);
    expect(r.offset).toBeCloseTo(5, 12);
    expect(r.side).toBe('right');
  });

  it('a point on the line has no side and no sign', () => {
    const r = readStation(pt(0, 0), pt(100, 0), pt(30, 0))!;
    expect(r.side).toBe('on');
    expect(stationLine(format, r)).toBe('Dik ayak 30.000 m, dik boy 0.000 m (hat üzerinde)');
    expect(stationTag(format, r)).toEqual(['Ayak 30.000 m', 'Boy 0.000 m']);
  });

  it('a point less than half a millimetre off is on the line', () => {
    expect(readStation(pt(0, 0), pt(100, 0), pt(30, ON_LINE * 0.9))!.side).toBe('on');
    expect(readStation(pt(0, 0), pt(100, 0), pt(30, -ON_LINE * 0.9))!.side).toBe('on');
    // Just past it, it has a side: the line heads east, so north of it is its left.
    expect(readStation(pt(0, 0), pt(100, 0), pt(30, ON_LINE * 1.1))!.side).toBe('left');
    expect(readStation(pt(0, 0), pt(100, 0), pt(30, -ON_LINE * 1.1))!.side).toBe('right');
  });

  it('the foot runs past the ends: behind A and beyond B', () => {
    const behind = readStation(pt(0, 0), pt(100, 0), pt(-12.5, 4))!;
    expect(behind.foot).toBeCloseTo(-12.5, 12);
    expect(behind.footPoint.x).toBeCloseTo(-12.5, 12);
    const beyond = readStation(pt(0, 0), pt(100, 0), pt(130, 4))!;
    expect(beyond.foot).toBeCloseTo(130, 12);
    expect(stationLine(format, beyond)).toBe('Dik ayak 130.000 m, dik boy 4.000 m (solda)');
  });

  it('works along any direction', () => {
    // A slanted line: 3-4-5. P is 5 m to the right of it, 10 m along.
    const a = pt(1000, 2000);
    const b = pt(1030, 2040);
    const along = { x: 0.6, y: 0.8 };
    const right = { x: 0.8, y: -0.6 };
    const p = pt(a.x + along.x * 10 + right.x * 5, a.y + along.y * 10 + right.y * 5);
    const r = readStation(a, b, p)!;
    expect(r.foot).toBeCloseTo(10, 9);
    expect(r.offset).toBeCloseTo(5, 9);
    expect(r.side).toBe('right');
  });

  it('has nothing to read against a line of no length', () => {
    expect(readStation(pt(3, 3), pt(3, 3), pt(5, 5))).toBeNull();
  });

  it('never shows “-0.000”', () => {
    const r = readStation(pt(0, 0), pt(100, 0), pt(-1e-9, 4))!;
    expect(stationTag(format, r)[0]).toBe('Ayak 0.000 m');
    expect(stationLine(format, r)).toContain('Dik ayak 0.000 m');
  });
});

describe('Dik ayak ölç', () => {
  it('asks for A, then B, then the points', () => {
    const h = toolHarness();
    const tool = h.use(new StationOffsetTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Dik ayak ölç: hattın başına tıklayın (A)');
    expect(parsePrompt(tool.prompt.value).options).toEqual([]);
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toBe('Dik ayak ölç: hattın sonuna tıklayın (B)');
    expect(parsePrompt(tool.prompt.value).options).toEqual([]);
    tool.pointerDown(at(100, 0));
    expect(tool.prompt.value).toBe('Dik ayak ölç: ölçülecek noktaya tıklayın [Başka hat (H) / Bitir (Enter)]');
    expect(parsePrompt(tool.prompt.value).options.map((o) => o.key)).toEqual(['H', 'Enter']);
  });

  it('writes each click’s reading to the log: the worked example', () => {
    const { h, tool } = withLine();
    tool.pointerDown(at(30, 5));
    expect(said(h)).toEqual(['Dik ayak 30.000 m, dik boy 5.000 m (solda)']);
    tool.pointerDown(at(70, -2));
    tool.pointerDown(at(50, 0));
    expect(said(h)).toEqual([
      'Dik ayak 30.000 m, dik boy 5.000 m (solda)',
      'Dik ayak 70.000 m, dik boy 2.000 m (sağda)',
      'Dik ayak 50.000 m, dik boy 0.000 m (hat üzerinde)',
    ]);
  });

  it('shows the foot and the signed offset in the tag by the cursor', () => {
    const { tool } = withLine();
    tool.pointerMove(at(30, 5));
    const left = canvasLog();
    tool.draw(left.g, left.view);
    expect(left.texts).toContain('Ayak 30.000 m');
    expect(left.texts).toContain('Boy −5.000 m');
    tool.pointerMove(at(30, -5));
    const right = canvasLog();
    tool.draw(right.g, right.view);
    expect(right.texts).toContain('Boy +5.000 m');
  });

  it('previews the line extended across the view, the foot, and the dashed perpendicular', () => {
    const { tool } = withLine();
    tool.pointerMove(at(30, 5));
    const { g, view, paths, arcs } = canvasLog();
    tool.draw(g, view);
    const dashed = paths.filter((p) => p.dashed).map((p) => p.points);
    // The line beyond both ends: from far behind A to A, and from B to far past B.
    expect(dashed.some((p) => p[0][0] < -100 && p[1][0] === 0 && p[0][1] === 0)).toBe(true);
    expect(dashed.some((p) => p[0][0] === 100 && p[1][0] > 200 && p[1][1] === 0)).toBe(true);
    // The perpendicular runs from the foot (30, 0) to the cursor.
    expect(dashed).toContainEqual([[30, 0], [30, 5]]);
    // The line itself is solid.
    expect(paths.some((p) => !p.dashed && p.points.length === 2 && p.points[0][0] === 0 && p.points[1][0] === 100)).toBe(true);
    // The line's ends are ringed; the foot is marked by a dot.
    expect(arcs).toContainEqual([0, 0]);
    expect(arcs).toContainEqual([100, 0]);
    expect(arcs).toContainEqual([30, 0]);
    // The square corner at the foot: a solid path of three points with 9 px arms (this canvas's y is the world's, so the arm towards the cursor is drawn the way it would be on a y-down screen).
    expect(paths.some((p) => !p.dashed && p.points.length === 3 && p.points[1][0] === 39 && p.points[1][1] === -9)).toBe(true);
  });

  it('marks the points measured, until the line changes', () => {
    const { tool } = withLine();
    tool.pointerDown(at(30, 5));
    tool.pointerDown(at(60, -8));
    tool.pointerMove(at(80, 3));
    const kept = canvasLog();
    tool.draw(kept.g, kept.view);
    const dashed = kept.paths.filter((p) => p.dashed).map((p) => p.points);
    expect(dashed).toContainEqual([[30, 0], [30, 5]]);
    expect(dashed).toContainEqual([[60, 0], [60, -8]]);
    tool.input('H');
    tool.pointerDown(at(0, 10));
    tool.pointerDown(at(0, 60));
    tool.pointerMove(at(5, 30));
    const fresh = canvasLog();
    tool.draw(fresh.g, fresh.view);
    expect(fresh.paths.filter((p) => p.dashed).map((p) => p.points)).not.toContainEqual([[30, 0], [30, 5]]);
  });

  it('draws the rubber line and its length while B is chosen', () => {
    const h = toolHarness();
    const tool = h.use(new StationOffsetTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.pointerMove(at(30, 40));
    const { g, view, paths, texts } = canvasLog();
    tool.draw(g, view);
    expect(paths.map((p) => p.points)).toContainEqual([[0, 0], [30, 40]]);
    expect(texts).toContain('50.000 m');
    expect(texts).toContain('A');
  });

  it('“Başka hat” asks for a new line', () => {
    const { h, tool } = withLine();
    tool.pointerDown(at(30, 5));
    expect(tool.input('H')).toBe(true);
    expect(tool.prompt.value).toBe('Dik ayak ölç: hattın başına tıklayın (A)');
    expect(tool.pointCount).toBe(0);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(0, 10));
    tool.pointerDown(at(3, 4));
    expect(said(h).at(-1)).toBe('Dik ayak 4.000 m, dik boy 3.000 m (sağda)');
  });

  it('takes “Başka hat” only where there is a line', () => {
    const h = toolHarness();
    const tool = h.use(new StationOffsetTool(h.ctx));
    tool.activate();
    expect(tool.input('H')).toBe(false);
    tool.pointerDown(at(0, 0));
    expect(tool.input('H')).toBe(false);
  });

  it('refuses a B on A and asks again', () => {
    const h = toolHarness();
    const tool = h.use(new StationOffsetTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(5, 5));
    tool.pointerDown(at(5, 5));
    expect(tool.pointCount).toBe(1);
    expect(h.log.entries.value.at(-1)).toMatchObject({ level: 'warn', text: 'B noktası A ile çakışıyor; hattın sonu için başka bir nokta gösterin.' });
    expect(tool.prompt.value).toContain('hattın sonuna');
  });

  it('Esc steps back: the points to a new line, B to A, A leaves', () => {
    const { h, tool } = withLine();
    tool.pointerDown(at(30, 5));
    // From the points: back to A, a new line.
    h.ctx.tools.exit();
    expect(h.state.exited).toBe(0);
    expect(tool.prompt.value).toContain('hattın başına');
    // From B: back to A.
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toContain('hattın sonuna');
    h.ctx.tools.exit();
    expect(h.state.exited).toBe(0);
    expect(tool.prompt.value).toContain('hattın başına');
    // From A: leave.
    h.ctx.tools.exit();
    expect(h.state.exited).toBe(1);
  });

  it('Enter or a right click ends the tool at any step', () => {
    const { h, tool } = withLine();
    tool.pointerDown(at(30, 5));
    tool.confirm();
    expect(h.state.exited).toBe(1);
    const again = h.use(new StationOffsetTool(h.ctx));
    again.activate();
    again.confirm();
    expect(h.state.exited).toBe(2);
    const half = h.use(new StationOffsetTool(h.ctx));
    half.activate();
    half.pointerDown(at(0, 0));
    half.confirm();
    expect(h.state.exited).toBe(3);
  });

  it('writes nothing to the drawing', () => {
    const { h, tool } = withLine();
    const revision = h.doc.revision;
    tool.pointerDown(at(30, 5));
    tool.pointerDown(at(60, -8));
    tool.input('H');
    expect(h.doc.size).toBe(0);
    expect(h.doc.revision).toBe(revision);
    expect(h.doc.canUndo.value).toBe(false);
  });

  it('takes typed points too: absolute for the line, then a point to measure', () => {
    const h = toolHarness();
    const tool = h.use(new StationOffsetTool(h.ctx));
    tool.activate();
    expect(tool.input('0,0')).toBe(true);
    expect(tool.input('100,0')).toBe(true);
    expect(tool.input('30,5')).toBe(true);
    expect(said(h)).toEqual(['Dik ayak 30.000 m, dik boy 5.000 m (solda)']);
  });

  it('takes the points to measure as they are: ortho and polar do not bend them', () => {
    const { h, tool } = withLine();
    h.ctx.settings.ortho.set(true);
    tool.pointerDown(at(30, 5));
    // Ortho against B(100, 0) would have made this (30, 0): the point is where it was clicked.
    expect(said(h)).toEqual(['Dik ayak 30.000 m, dik boy 5.000 m (solda)']);
    h.ctx.settings.ortho.set(false);
  });

  it('keeps perpendicular snaps to the line’s own ends', () => {
    const h = toolHarness();
    const tool = h.use(new StationOffsetTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    expect(tool.snapFrom()).toEqual(pt(0, 0));
    tool.pointerDown(at(100, 0));
    expect(tool.snapFrom()).toBeNull();
  });
});
