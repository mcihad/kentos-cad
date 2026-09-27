// Records the SVG editor's rules into fixtures/style/v1/svgedit.json (ui/svgedit/svgEditModel.ts): the shape
// a drag makes with each drawing tool (Shift, Alt, the polygon's sides and star), a box scaled by each handle
// (Shift at the corners, a collapse), a turn by the knob (Shift's 15° steps), the measure's readout (the
// drawing's clockwise angle, mm at the symbol's size), the rulers' steps, a corner's tidy size, file names
// and the starting grid, stroke widths and panel distance. Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-svgedit.test.ts
// The answers are the web's and were read when recorded; rewriting them is a deliberate change, to be read in
// the diff. src/ui/svgedit/svgEditFixture.test.ts keeps checking them; the desktop's editor
// (apps/desktop/src/style/svgedit/fixture.rs) checks the same file.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import type { Box, Pt } from '../../src/style/svg/pathData';
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
  type DragTool,
} from '../../src/ui/svgedit/svgEditModel';

const OUT = new URL('../../../../fixtures/style/v1/svgedit.json', import.meta.url);

const spec = (tool: DragTool, over: Partial<DragSpec> = {}): DragSpec => ({ tool, width: 100, height: 100, sides: 6, star: false, shift: false, fromCentre: false, ...over });

const DRAGS: { spec: DragSpec; p0: Pt; p1: Pt }[] = [
  { spec: spec('rect'), p0: [10, 10], p1: [45, 40] },
  { spec: spec('rect'), p0: [45, 40], p1: [10, 10] },
  { spec: spec('rect', { shift: true }), p0: [10, 10], p1: [45, 30] },
  { spec: spec('rect', { shift: true }), p0: [50, 50], p1: [30, 20] },
  { spec: spec('rect', { fromCentre: true }), p0: [50, 50], p1: [60, 45] },
  { spec: spec('rect', { shift: true, fromCentre: true, width: 64, height: 32 }), p0: [32, 16], p1: [40, 18] },
  { spec: spec('ellipse'), p0: [55, 10], p1: [90, 40] },
  { spec: spec('ellipse', { shift: true }), p0: [20, 20], p1: [30, 60] },
  { spec: spec('ellipse', { fromCentre: true }), p0: [50, 50], p1: [70, 35] },
  { spec: spec('ellipse', { width: 24, height: 24 }), p0: [2, 2], p1: [2.3, 2.3] },
  { spec: spec('polygon'), p0: [50, 72], p1: [50, 50] },
  { spec: spec('polygon', { sides: 5 }), p0: [50, 50], p1: [80, 60] },
  { spec: spec('polygon', { sides: 5, star: true }), p0: [50, 50], p1: [50, 20] },
  { spec: spec('polygon', { sides: 2 }), p0: [10, 10], p1: [20, 10] },
  { spec: spec('text'), p0: [15, 55], p1: [15, 55] },
  { spec: spec('text', { width: 400, height: 250 }), p0: [3.5, 7.25], p1: [90, 90] },
];

const BOX: Box = { minX: 10, minY: 20, maxX: 50, maxY: 40 };
const SCALED: { box: Box; handle: number; q: Pt; shift: boolean }[] = [
  ...[0, 1, 2, 3, 4, 5, 6, 7].map((handle) => ({ box: BOX, handle, q: [60, 70] as Pt, shift: false })),
  ...[0, 2, 4, 6].map((handle) => ({ box: BOX, handle, q: [0, 50] as Pt, shift: true })),
  { box: BOX, handle: 4, q: [10, 60], shift: false },
  { box: BOX, handle: 3, q: [10.0000001, 30], shift: false },
];

const TURNS: { box: Box; p0: Pt; p: Pt; shift: boolean }[] = [
  { box: BOX, p0: [30, 4], p: [60, 30], shift: false },
  { box: BOX, p0: [30, 4], p: [60, 30], shift: true },
  { box: BOX, p0: [30, 4], p: [0, 25], shift: true },
  { box: BOX, p0: [30, 4], p: [31, 60], shift: false },
];

const MEASURES: { a: Pt; b: Pt; width: number; sizeMm?: number }[] = [
  { a: [0, 0], b: [30, -40], width: 100, sizeMm: 24 },
  { a: [0, 0], b: [0, 10], width: 100 },
  { a: [10, 40], b: [90, 10], width: 100 },
  { a: [5, 5], b: [-5, 5], width: 64, sizeMm: 5 },
  { a: [1.23456, 2], b: [1.23456, 2], width: 10, sizeMm: 3 },
];

it('records the SVG editor rules', () => {
  if (!process.env.GOLDEN_WRITE) return;
  const data = {
    about: 'The SVG editor rules (ui/svgedit/svgEditModel.ts): recorded by scripts/fixtures/record-svgedit.test.ts; the desktop editor (apps/desktop/src/style/svgedit/fixture.rs) is held to the same answers.',
    drags: DRAGS.map((d) => ({ ...d, shape: shapeFromDrag(d.spec, d.p0, d.p1, 'id') })),
    scaled: SCALED.map((c) => ({ ...c, out: scaledBox(c.box, c.handle, c.q, c.shift) })),
    turns: TURNS.map((c) => ({ ...c, deg: knobTurn(c.box, c.p0, c.p, c.shift) })),
    measures: MEASURES.map((m) => ({ ...m, ...measureReadout(m.a, m.b, m.width, m.sizeMm) })),
    niceStep: [0.37, 1, 1.5, 3.2, 6, 11.2, 56 / 4, 56 / 580, 56 / 0.2, 1000].map((v) => [v, niceStep(v)]),
    niceRound: [
      [2.37, 0.5],
      [2.37, 0.03],
      [13.33, 3],
      [0.0442, 0.007],
      [7.5, 1],
    ].map(([v, unit]) => [v, unit, niceRound(v, unit)]),
    slugs: ['Yeni çizim', 'Şişli İğne Ölçüsü', '  Ağaç (kopya) ', '!!!', 'ISPARTA ılık'].map((s) => [s, fileSlug(s)]),
    defaults: [100, 64, 24, 12, 7, 400, 0.5].map((width) => ({ width, grid: defaultGrid(width), unit: panelUnit(width), shapeStroke: shapeStrokeWidth(width), lineStroke: lineStrokeWidth(width) })),
  };
  writeFileSync(OUT, `${JSON.stringify(data, null, 2)}\n`);
});
