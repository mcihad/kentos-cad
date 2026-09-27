import { describe, expect, it } from 'vitest';
import type { Box, Pt } from '../../style/svg/pathData';
import {
  defaultGrid,
  fileSlug,
  knobTurn,
  lineStrokeWidth,
  measureReadout,
  niceRound,
  niceStep,
  panelUnit,
  scaledBox,
  shapeFromDrag,
  shapeStrokeWidth,
  type DragSpec,
} from './svgEditModel';

// fixtures/style/v1/svgedit.json holds the SVG editor's rules as the web answered them; the desktop's editor
// checks the same file (apps/desktop/src/style/svgedit/fixture.rs). Rewritten only on purpose by
// scripts/fixtures/record-svgedit.test.ts.
const files = import.meta.glob('../../../../../fixtures/style/v1/svgedit.json', { eager: true, import: 'default' });
const data = Object.values(files)[0] as {
  drags: { spec: DragSpec; p0: Pt; p1: Pt; shape: unknown }[];
  scaled: { box: Box; handle: number; q: Pt; shift: boolean; out: Box | null }[];
  turns: { box: Box; p0: Pt; p: Pt; shift: boolean; deg: number }[];
  measures: { a: Pt; b: Pt; width: number; sizeMm?: number; main: string; more: string }[];
  niceStep: [number, number][];
  niceRound: [number, number, number][];
  slugs: [string, string][];
  defaults: { width: number; grid: number; unit: number; shapeStroke: number; lineStroke: number }[];
};

describe('the SVG editor rules the desktop shares (svgedit.json)', () => {
  it('draws the same shapes', () => {
    for (const d of data.drags) expect(shapeFromDrag(d.spec, d.p0, d.p1, 'id')).toEqual(d.shape);
  });
  it('scales and turns the same way', () => {
    for (const c of data.scaled) expect(scaledBox(c.box, c.handle, c.q, c.shift)).toEqual(c.out);
    for (const c of data.turns) expect(knobTurn(c.box, c.p0, c.p, c.shift)).toBe(c.deg);
  });
  it('measures, steps and names the same way', () => {
    for (const m of data.measures) expect(measureReadout(m.a, m.b, m.width, m.sizeMm)).toEqual({ main: m.main, more: m.more });
    for (const [v, out] of data.niceStep) expect(niceStep(v)).toBe(out);
    for (const [v, unit, out] of data.niceRound) expect(niceRound(v, unit)).toBe(out);
    for (const [s, slug] of data.slugs) expect(fileSlug(s)).toBe(slug);
    for (const d of data.defaults) expect([defaultGrid(d.width), panelUnit(d.width), shapeStrokeWidth(d.width), lineStrokeWidth(d.width)]).toEqual([d.grid, d.unit, d.shapeStroke, d.lineStroke]);
  });
});
