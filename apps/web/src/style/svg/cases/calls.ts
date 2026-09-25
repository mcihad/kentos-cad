import type { Gen } from '../../../wasm/calls/harness';
import { unitsOf } from '../arrange';
import { flattenSubPathTol } from '../bezier';
import type { Pt, SubPath } from '../pathData';
import { inkMask, traceContours, type TraceOptions } from '../trace';
import { colorText, cssText, drawing, flatTree, importOptions, lengthText, svgTree, transformText } from './files';
import { cubic, matrix, pathData, pt, shape, shapes, subPath, subs } from './geometry';

/**
 * The arguments of every operation of the SVG core's table (`CALLS`), as
 * the page sends them, which the frozen cases (fixtures/svg/v1/cases.json,
 * scripts/fixtures/record-svg.test.ts) are recorded from; and the inputs of
 * the typed entries: pictures to trace, trace options, snap indexes with
 * their queries.
 */

const t01 = (g: Gen) => g.pick([0, 1, 0.5, g.num(0, 1), g.num(-0.2, 1.2)]);
const tolOf = (g: Gen) => g.pick([1e-3, 0.01, 0.05, 0.5, 5, g.num(0.001, 2)]);
const region = (g: Gen) => ({ subs: subs(g), fillRule: g.pick(['nonzero', 'evenodd'] as const) });
export const polyline = (g: Gen): Pt[] => (g.chance(0.5) ? flattenSubPathTol(subPath(g), 0.2) : Array.from({ length: g.int(1, 12) }, () => pt(g)));
const strokeStyle = (g: Gen) => ({
  width: g.pick([0, 0.5, 2, 6, g.num(0.1, 10)]),
  cap: g.pick(['butt', 'round', 'square']),
  join: g.pick(['miter', 'round', 'bevel']),
  ...(g.chance(0.3) ? { miterLimit: g.pick([1, 2, 10]) } : {}),
  ...(g.chance(0.3) ? { dash: g.pick([[3, 2], [1], [0, 2], [4, -1], [2, 1, 0.5]]) } : {}),
});
export const box = (g: Gen) => {
  const a = pt(g);
  const b = pt(g);
  return { minX: Math.min(a[0], b[0]), minY: Math.min(a[1], b[1]), maxX: Math.max(a[0], b[0]), maxY: Math.max(a[1], b[1]) };
};
/** Nodes of the sub-paths to act on: mostly real ones, now and then twice or past the end. */
function refs(g: Gen, sps: readonly SubPath[]): { sub: number; index: number }[] {
  const out: { sub: number; index: number }[] = [];
  const k = g.int(0, 5);
  for (let i = 0; i < k && sps.length; i++) {
    const sub = g.int(0, sps.length - 1);
    const n = sps[sub].nodes.length;
    out.push({ sub, index: g.chance(0.05) ? n + 1 : g.int(0, Math.max(0, n - 1)) });
  }
  if (out.length && g.chance(0.15)) out.push({ ...out[0] });
  // Ends of open sub-paths, for joins.
  if (g.chance(0.3)) sps.forEach((sp, sub) => !sp.closed && out.push({ sub, index: g.chance(0.5) ? 0 : sp.nodes.length - 1 }));
  return out;
}
export const nodeCase = (g: Gen): [SubPath[], { sub: number; index: number }[]] => {
  const sps = subs(g);
  return [sps, refs(g, sps)];
};
export const units = (g: Gen) => {
  const list = shapes(g, g.int(1, 5));
  return unitsOf(list, list.map((x) => x.id).filter(() => g.chance(0.85)));
};
export const PAGE = { width: 100, height: 80 };
const ANCHORS = ['tl', 't', 'tr', 'l', 'c', 'r', 'bl', 'b', 'br'] as const;
const SIDES = ['left', 'hcenter', 'right', 'top', 'vcenter', 'bottom'] as const;
const TARGETS = ['selection', 'first', 'last', 'biggest', 'smallest', 'canvas'] as const;
const OPS = ['union', 'difference', 'intersection', 'exclusion', 'division'] as const;
/** A paint as the page shows it: the file's parameters, the editor's colours, or a plain value. */
const shown = (g: Gen, p: string) => (p === 'fill' ? g.pick(['currentColor', '#123456']) : p === 'stroke' ? g.pick(['param(stroke) #000000', '#654321']) : p);

/** A small picture: blobs, rings and lines of ink, grey and see-through pixels. */
export function bitmap(g: Gen): { width: number; height: number; data: Uint8ClampedArray | number[] } {
  const w = g.int(1, 36);
  const h = g.int(1, 36);
  const data = new Uint8ClampedArray(w * h * 4).fill(255);
  const blobs = Array.from({ length: g.int(0, 4) }, () => ({ x: g.num(0, w), y: g.num(0, h), r: g.num(1, 12), hole: g.chance(0.3), grey: g.int(0, 200), a: g.pick([255, 255, 128, 0]) }));
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      for (const b of blobs) {
        const d = Math.hypot(x - b.x, y - b.y);
        if (d < b.r && !(b.hole && d < b.r * 0.45)) {
          const i = (y * w + x) * 4;
          data[i] = data[i + 1] = data[i + 2] = b.grey;
          data[i + 3] = b.a;
        }
      }
      if (g.chance(0.02)) data[(y * w + x) * 4] = 0;
    }
  return { width: w, height: h, data: g.chance(0.1) ? [...data] : data };
}

export const traceOptions = (g: Gen): Partial<TraceOptions> => ({
  ...(g.chance(0.5) ? { threshold: g.pick([64, 128, 200]) } : {}),
  ...(g.chance(0.2) ? { invert: true } : {}),
  ...(g.chance(0.5) ? { speckle: g.pick([0, 2, 6, 30]) } : {}),
  ...(g.chance(0.5) ? { tolerance: g.pick([0.2, 0.5, 1, 2, 4]) } : {}),
  ...(g.chance(0.5) ? { corner: g.pick([20, 45, 60, 120]) } : {}),
  ...(g.chance(0.5) ? { smooth: g.pick([0, 0.3, 1, 1.5]) } : {}),
});

const rings = (g: Gen) => {
  const img = bitmap(g);
  return traceContours(inkMask(img, 128), img.width, img.height);
};

/** A snap index's source (as the core reads it: skipped nodes listed) and queries (point, radius, start point or none). */
export function snapCase(g: Gen): [Record<string, unknown>, [Pt, number, Pt | null][]] {
  const list = shapes(g, g.int(0, 5));
  const kinds = ['cusp', 'smooth', 'mid', 'intersection', 'bboxCorner', 'bboxMid', 'bboxCentre', 'centre', 'perpendicular', 'tangent', 'guide', 'page'].filter(() => g.chance(0.75));
  const guides = Array.from({ length: g.int(0, 3) }, (_, i) => ({ id: `k${i}`, x: g.num(0, 100), y: g.num(0, 80), angle: g.pick([0, 90, 45, g.num(0, 180)]) }));
  const exclude = list.filter(() => g.chance(0.15)).map((x) => x.id);
  const moving = list.find((x) => x.kind === 'path' && g.chance(0.5));
  const skip = moving ? [[moving.id, 0, g.int(0, 2)]] : [];
  const queries: [Pt, number, Pt | null][] = Array.from({ length: 12 }, () => [pt(g), g.pick([0.5, 2, 5, 40]), g.chance(0.4) ? pt(g) : null]);
  return [{ shapes: list, guides, page: PAGE, kinds, exclude, skip }, queries];
}

/**
 * Arguments of every operation of the core's table, as the page sends
 * them (facades that reshape their arguments are given the core's shape:
 * a file's tree flat, sets as lists, paints already shown).
 */
export const CALLS: Record<string, (g: Gen) => unknown[]> = {
  // pathData.ts
  parsePathData: (g) => [pathData(g)],
  arcToCubics: (g) => {
    const a = pt(g);
    const b = g.chance(0.1) ? a : pt(g);
    return [a[0], a[1], g.pick([0, 1, 5, 40, g.num(-60, 60)]), g.pick([0, 2, 30, g.num(-60, 60)]), g.pick([0, 30, 90, g.num(-360, 360)]), g.chance(0.5), g.chance(0.5), b[0], b[1]];
  },
  pathDataOf: (g) => [subs(g), ...(g.chance(0.5) ? [g.pick([0, 1, 3, 6])] : [])],
  transformSubPaths: (g) => [subs(g), matrix(g)],
  multiply: (g) => [matrix(g), matrix(g)],
  apply: (g) => [matrix(g), g.num(-100, 100), g.num(-100, 100)],
  flattenSubPath: (g) => [subPath(g), ...(g.chance(0.5) ? [g.pick([1, 4, 16, 33])] : [])],
  subPathsBox: (g) => [subs(g)],
  // bezier.ts
  bez: (g) => [cubic(g), t01(g)],
  bezDeriv: (g) => [cubic(g), t01(g)],
  bezTangent: (g) => [cubic(g), t01(g)],
  splitCubic: (g) => [cubic(g), t01(g)],
  flattenCubic: (g) => [cubic(g), g.pick([0, -1, 1e-4, tolOf(g)])],
  flattenSubPathTol: (g) => [subPath(g), tolOf(g)],
  cubicLength: (g) => [cubic(g), ...(g.chance(0.5) ? [t01(g), t01(g)] : [])],
  nearestOnCubic: (g) => [cubic(g), pt(g)],
  paramAtDistance: (g) => [cubic(g), g.num(0, 80), g.chance(0.5)],
  ringSignedArea: (g) => [polyline(g)],
  windingOf: (g) => [polyline(g), pt(g)],
  // fitCurve.ts
  fitRun: (g) => [polyline(g), tolOf(g), ...(g.chance(0.3) ? [[0.6, 0.8], [-1, 0]] : [])],
  fitOne: (g) => [polyline(g), [0.6, 0.8], [-1, 0]],
  // pathBool.ts
  booleanOp: (g) => [g.pick(OPS), Array.from({ length: g.int(1, 3) }, () => region(g))],
  cutPath: (g) => [subs(g), Array.from({ length: g.int(1, 2) }, () => subs(g))],
  // pathStroke.ts
  strokeOutline: (g) => [subs(g), strokeStyle(g)],
  offsetRegion: (g) => [region(g), g.pick([0, 1, -1, 3, -4, g.num(-5, 5)]), ...(g.chance(0.6) ? [g.pick(['round', 'miter', 'bevel'])] : [])],
  // pathOps.ts
  booleanShapes: (g) => [g.pick(OPS), shapes(g)],
  cutShapes: (g) => [shapes(g)],
  combineShapes: (g) => [shapes(g)],
  breakApart: (g) => [shape(g, 0), g.chance(0.5)],
  shapesToPath: (g) => [shapes(g)],
  strokeToPath: (g) => [shapes(g)],
  offsetShapes: (g) => [shapes(g), g.pick([1, -1, 2.5, -3, g.num(-4, 4)]), ...(g.chance(0.5) ? [g.pick(['round', 'miter', 'bevel'])] : [])],
  simplifySubPath: (g) => [subPath(g), tolOf(g), ...(g.chance(0.4) ? [g.pick([10, 35, 90])] : [])],
  simplifyShapes: (g) => [shapes(g), tolOf(g)],
  reverseShapes: (g) => [shapes(g)],
  closeSubPath: (g) => [subPath(g)],
  openSubPath: (g) => [subPath(g)],
  closeShapes: (g) => [shapes(g)],
  openShapes: (g) => [shapes(g)],
  // svgModel.ts
  elementOf: (g) => {
    const s = shape(g, 0);
    return [s, shown(g, s.fill), shown(g, s.stroke)];
  },
  serializeDoc: (g) => [{ width: g.pick([100, 64, g.num(1, 300)]), height: g.pick([100, 48, g.num(1, 300)]), shapes: shapes(g, g.int(0, 6)) }],
  toPath: (g) => [shape(g, 0)],
  shapeBox: (g) => [shape(g, 0)],
  shapesBox: (g) => [shapes(g, g.int(0, 4))],
  transformShape: (g) => [shape(g, 0), matrix(g)],
  transformShapes: (g) => [shapes(g, g.int(0, 4)), matrix(g)],
  boxToBox: (g) => [box(g), box(g)],
  rotation: (g) => [g.pick([0, 90, 45, g.num(-360, 360)]), g.num(-50, 50), g.num(-50, 50)],
  regularPolygon: (g) => [g.num(0, 100), g.num(0, 100), g.num(1, 50), g.pick([3, 5, 6, 12, 4.5]), ...(g.chance(0.5) ? [g.pick([0, 5, g.num(1, 20)])] : [])],
  // nodeOps.ts
  nodeTypeOf: (g) => {
    const sp = subPath(g);
    return [sp, g.int(0, sp.nodes.length - 1)];
  },
  refreshAuto: (g) => [subs(g).map((sp) => ({ ...sp, nodes: sp.nodes.map((n) => (g.chance(0.4) ? { ...n, type: 'auto' as const } : n)) }))],
  setNodeType: (g) => [...nodeCase(g), g.pick(['cusp', 'smooth', 'symmetric', 'auto'])],
  moveNodes: (g) => [...nodeCase(g), g.num(-20, 20), g.num(-20, 20)],
  insertMidNodes: (g) => nodeCase(g),
  deleteNodes: (g) => [...nodeCase(g), ...(g.chance(0.5) ? [g.chance(0.5)] : [])],
  joinEnds: (g) => [...nodeCase(g), g.chance(0.5)],
  breakAtNodes: (g) => nodeCase(g),
  deleteSegments: (g) => nodeCase(g),
  segmentsTo: (g) => [...nodeCase(g), g.pick(['line', 'curve'])],
  cornerAt: (g) => {
    const sp = subPath(g);
    return [sp, g.int(0, sp.nodes.length - 1)];
  },
  filletDistance: (g) => [{ angle: g.pick([Math.PI / 2, g.num(0, Math.PI)]), back: [1, 0], ahead: [0, 1], max: 5 }, g.num(0, 10)],
  filletRadius: (g) => [{ angle: g.num(0, Math.PI), back: [1, 0], ahead: [0, 1], max: 5 }, g.num(0, 10)],
  // In range only: the TypeScript read past the end (a TypeError) where the core reports the node as no corner.
  cornerNodes: (g) => {
    const [sps, rs] = nodeCase(g);
    return [sps, rs.filter((r) => r.index < sps[r.sub].nodes.length), g.pick(['fillet', 'chamfer']), g.pick([0, 0.5, 2, 6, 30, g.num(0.1, 10)])];
  },
  alignNodes: (g) => [...nodeCase(g), g.pick(['x', 'y']), g.pick(['min', 'mid', 'max'])],
  distributeNodes: (g) => [...nodeCase(g), g.pick(['x', 'y'])],
  // arrange.ts
  unitsOf: (g) => {
    const list = shapes(g, g.int(1, 5));
    return [list, list.map((x) => x.id).filter(() => g.chance(0.85)).concat(g.chance(0.1) ? ['yok'] : [])];
  },
  alignReference: (g) => [units(g), g.pick(TARGETS), PAGE],
  alignMoves: (g) => [units(g), g.pick(SIDES), g.pick(TARGETS), PAGE, g.chance(0.3)],
  distributeMoves: (g) => [units(g), g.pick(['left', 'hcenter', 'right', 'hgap', 'top', 'vcenter', 'bottom', 'vgap'])],
  anchorPoint: (g) => [box(g), g.pick(ANCHORS)],
  scaleAbout: (g) => [g.pick([1, 2, -1, 0, g.num(-3, 3)]), g.pick([1, 0.5, g.num(-3, 3)]), pt(g)],
  skewAbout: (g) => [g.pick([0, 30, 45, 90, g.num(-89, 89)]), g.pick([0, -20, g.num(-89, 89)]), pt(g)],
  rotateAbout: (g) => [g.pick([0, 90, 45, -30, g.num(-360, 360)]), pt(g), g.chance(0.5)],
  transformMoves: (g) => {
    const anchor = g.pick(ANCHORS);
    const spec = g.pick([
      { kind: 'move', x: g.num(-20, 20), y: g.num(-20, 20), relative: g.chance(0.5) },
      { kind: 'scale', sx: g.pick([100, 50, 200, g.num(10, 300)]), sy: g.pick([100, 150, g.num(10, 300)]), anchor },
      { kind: 'rotate', deg: g.pick([90, 45, g.num(-180, 180)]), ccw: g.chance(0.5), about: g.chance(0.7) ? anchor : pt(g) },
      { kind: 'skew', ax: g.num(-40, 40), ay: g.num(-40, 40), anchor },
      { kind: 'matrix', m: matrix(g) },
    ]);
    return [units(g), spec, g.chance(0.5)];
  },
  invertible: (g) => [g.chance(0.2) ? [1, 2, 2, 4, 0, 0] : matrix(g)],
  rectArray: (g) => [{ minX: 10, minY: 5, maxX: 30, maxY: 25 }, { rows: g.pick([1, 2, 3, 2.5]), cols: g.pick([1, 4, 0]), dx: g.num(-10, 30), dy: g.num(-10, 30), mode: g.pick(['step', 'gap']) }],
  polarArray: (g) => [{ minX: 10, minY: 5, maxX: 30, maxY: 25 }, { count: g.pick([1, 2, 6, 12, 3.5]), angle: g.pick([360, 180, 90, -360, g.num(-400, 400)]), centre: pt(g), rotate: g.chance(0.5), ccw: g.chance(0.5) }],
  mirrorMatrix: (g) => [g.pick(['v', 'h', 'angle']), pt(g), ...(g.chance(0.7) ? [g.num(-180, 180)] : [])],
  copiesOf: (g) => [shapes(g, g.int(1, 4)).map((x, i) => (i && g.chance(0.5) ? { ...x, group: g.pick(['g1', 'g2']) } : x)), Array.from({ length: g.int(0, 3) }, () => matrix(g))],
  // trace.ts (the picture's bytes cross typed: inkMask, traceContours, traceBitmap)
  ringArea: (g) => [g.pick(rings(g)) ?? polyline(g)],
  nestRings: (g) => [rings(g), g.pick([0, 2, 6, 40])],
  simplifyRing: (g) => [rings(g)[0] ?? [[0, 0], [1, 0], [1, 1]], g.pick([0.2, 0.5, 1, 3])],
  // svgValues.ts (an argument the page leaves out is left out: JSON would make `undefined` null)
  readColor: (g) => [colorText(g)],
  readPaint: (g) => (g.chance(0.05) ? [] : [colorText(g)]),
  readTransform: (g) => (g.chance(0.05) ? [] : [transformText(g)]),
  viewBoxTransform: (g) => [[g.num(-20, 20), g.num(-20, 20), g.num(0.5, 100), g.num(0.5, 100)], g.num(1, 200), g.num(1, 200), ...(g.chance(0.7) ? [g.pick(['', 'none', 'xMinYMin', 'xMaxYMax slice', 'xMidYMax meet', ' defer  xMaxYMid ', 'foo'])] : [])],
  viewBoxOf: (g) => (g.chance(0.05) ? [] : [g.pick(['0 0 100 100', '0,0,24,24', '0 0 0 10', '1 2 3', '-5 -5 10 10', '1e1 2 3 4', transformText(g)])]),
  parseLength: (g) => (g.chance(0.05) ? [] : [lengthText(g)]),
  toUser: (g) => [lengthText(g), g.pick([100, 0, 64, g.num(0, 500)]), g.pick([16, 12, 0, 7.5])],
  parseCss: (g) => [cssText(g), ...(g.chance(0.3) ? [g.int(0, 5)] : [])],
  isNearBlack: (g) => [g.pick(['#000000', '#1D1D1B', '#303031', '#313131', '#-1-1-1', '#0x0000', '# 10 10', '#é10000', '#1', '', 'fill', colorText(g)])],
  withAlpha: (g) => [g.pick(['#AA3300', '#000000']), g.pick([1, 0.999, 0.5, 0, g.num(0, 1), 1e-9])],
  // importSvg.ts
  docFromSvgTree: (g) => [flatTree(svgTree(g)), importOptions(g)],
  colorUsage: (g) => [drawing(g)],
  mapColors: (g) => [drawing(g), g.pick(['black', 'dominant', '#AA3300', '#aa3300', null, '']), g.pick([null, '#AA3300', '#000000', ''])],
  // exportSvg.ts (the PNG crosses typed: crc32, withPngDpi)
  exportBox: (g) => [drawing(g), ...(g.chance(0.6) ? [['s0', 's2', 'yok'].filter(() => g.chance(0.6))] : [])],
  svgText: (g) => {
    const o: Record<string, unknown> = {};
    if (g.chance(0.4)) o.colors = { ink: g.pick(['#000000', '#1A2B3C']), second: g.pick(['#FF0000', '#00000080']) };
    if (g.chance(0.3)) o.only = ['s0', 's1', 's3'].filter(() => g.chance(0.6));
    if (g.chance(0.4)) o.pretty = true;
    if (g.chance(0.3)) o.reference = g.chance(0.2) ? null : { href: 'data:image/png;base64,AA"<&>', x: g.num(-5, 5), y: 2, width: 40, height: g.num(1, 30), opacity: 0.5, locked: g.chance(0.5), name: 'Altlık "1"' };
    return [drawing(g), o];
  },
  sourceText: (g) => [drawing(g)],
  pngSize: (g) => [g.num(0, 300), g.pick([0, g.num(1, 300)]), g.pick([{ px: g.pick([1, 800, 10000, g.num(0, 2000)]) }, { dpi: g.pick([72, 96, 300, 1200]) }, { dpi: 300, widthMm: g.num(1, 50) }])],
};
