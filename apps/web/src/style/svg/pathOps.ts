import { fitPolyline } from './fitCurve';
import { booleanOp, cutPath, type BoolOp, type FillRule, type RegionInput } from './pathBool';
import { flattenCubic, reverseSubPath, segmentCount, segmentCubic, segmentIsLine, windingOf, flattenSubPathTol, ringSignedArea } from './bezier';
import { offsetRegion, strokeOutline, type Cap, type Join } from './pathStroke';
import type { PathNode, Pt, SubPath } from './pathData';
import { shapeId, toPath, type SvgShape } from './svgModel';

/**
 * The SVG editor's path operations (Inkscape's Path menu) on shapes:
 * booleans (union, difference, intersection, exclusion, division, cut
 * path), combine and break apart, object to path, stroke to path, inset
 * and outset, simplify, reverse, close and open. Booleans keep curves as
 * curves (pathBool.ts); stroke outlines and offsets are fitted with cubics
 * within a ten-thousandth of the drawing (pathStroke.ts). Every operation
 * returns the shapes to put in place of the ones it used, or a message
 * for the user. Pure.
 */

type PathShape = Extract<SvgShape, { kind: 'path' }>;

export type PathOp = BoolOp | 'cut' | 'combine' | 'breakApart' | 'split' | 'toPath' | 'strokeToPath' | 'reverse' | 'close' | 'open';

export type OpResult = { add: SvgShape[]; remove: string[]; note?: string } | { error: string };

/** The fill rule a shape paints with (paths default to even-odd in this editor, SVG shapes to nonzero). */
export const fillRuleOf = (s: SvgShape): FillRule => s.fillRule ?? (s.kind === 'path' ? 'evenodd' : 'nonzero');

const regionOf = (s: SvgShape): RegionInput | null => {
  const p = toPath(s);
  return p.kind === 'path' ? { subs: p.subs, fillRule: fillRuleOf(s) } : null;
};

/** A path shape with the style of `from` (id new unless given). */
function pathLike(from: SvgShape, subs: SubPath[], id = shapeId()): PathShape {
  return { id, kind: 'path', subs, fill: from.fill, stroke: from.stroke, strokeWidth: from.strokeWidth, opacity: from.opacity, dash: from.dash, cap: from.cap, join: from.join, group: from.group, name: from.name };
}

const TEXT_NOTE = 'Yazılar yol işlemlerine katılmaz; seçimde kaldılar.';

/**
 * A boolean on shapes given bottom to top. The result takes the bottom
 * shape's style and place (Inkscape does the same); division gives one
 * shape per piece.
 */
export function booleanShapes(op: BoolOp, shapes: readonly SvgShape[]): OpResult {
  const usable = shapes.filter((s) => s.kind !== 'text');
  const note = usable.length < shapes.length ? TEXT_NOTE : undefined;
  if (usable.length < 2) return { error: op === 'union' ? 'Birleştirmek için en az iki şekil seçin (yazılar katılmaz).' : 'Bu işlem için en az iki şekil seçin: alttaki ve üstündekiler (yazılar katılmaz).' };
  const inputs = usable.map(regionOf).filter((r): r is RegionInput => !!r);
  const results = booleanOp(op, inputs);
  const bottom = usable[0];
  const remove = usable.map((s) => s.id);
  if (op === 'division') {
    const add = results.filter((subs) => subs.length).map((subs) => pathLike(bottom, subs));
    return { add, remove, note: note ?? (add.length ? `${add.length} parça.` : 'Kesişen bir parça çıkmadı.') };
  }
  const subs = results[0] ?? [];
  if (!subs.length) return { add: [], remove, note: note ?? (op === 'intersection' ? 'Ortak alan yok: şekiller silindi (Ctrl+Z geri alır).' : 'Sonuç boş: şekiller silindi (Ctrl+Z geri alır).') };
  return { add: [pathLike(bottom, subs)], remove, note };
}

/** The bottom shape's outline cut where the others cross it (open pieces, no fill). */
export function cutShapes(shapes: readonly SvgShape[]): OpResult {
  const usable = shapes.filter((s) => s.kind !== 'text');
  if (usable.length < 2) return { error: 'Yolu kesmek için alttaki şekli ve üstünde onu kesen en az bir şekil seçin.' };
  const [bottom, ...rest] = usable.map((s) => toPath(s) as PathShape);
  const pieces = cutPath(
    bottom.subs,
    rest.map((s) => s.subs),
  );
  const stroke = bottom.stroke !== 'none' ? bottom.stroke : bottom.fill !== 'none' ? bottom.fill : 'fill';
  const add = pieces.map((sp) => ({ ...pathLike(bottom, [sp]), fill: 'none', stroke, strokeWidth: bottom.stroke !== 'none' ? bottom.strokeWidth : Math.max(bottom.strokeWidth, 1) }));
  return { add, remove: usable.map((s) => s.id), note: `${add.length} parça.` };
}

/** Every selected shape's sub-paths in one path (the bottom shape's style). */
export function combineShapes(shapes: readonly SvgShape[]): OpResult {
  const usable = shapes.filter((s) => s.kind !== 'text');
  if (usable.length < 2) return { error: 'Tek yolda toplamak için en az iki şekil seçin.' };
  const subs = usable.flatMap((s) => (toPath(s) as PathShape).subs.map(copySub));
  return { add: [pathLike(usable[0], subs)], remove: usable.map((s) => s.id) };
}

const copySub = (sp: SubPath): SubPath => ({ closed: sp.closed, nodes: sp.nodes.map((n) => ({ ...n })) });

/**
 * Sub-paths as separate shapes. With `keepHoles` a hole stays with the
 * sub-path around it (the letter O stays one shape).
 */
export function breakApart(shape: SvgShape, keepHoles: boolean): OpResult {
  const p = toPath(shape);
  if (p.kind !== 'path') return { error: 'Yazı parçalara ayrılmaz.' };
  if (p.subs.length < 2) return { error: 'Bu yol tek parça.' };
  const groups = keepHoles ? holeGroups(p.subs) : p.subs.map((sp) => [sp]);
  const group = shapeId();
  return { add: groups.map((subs) => ({ ...pathLike(p, subs.map(copySub)), group: shape.group ?? (groups.length > 1 ? group : undefined) })), remove: [shape.id] };
}

/** Sub-paths grouped with the ones they contain at odd depth (holes). */
function holeGroups(subs: readonly SubPath[]): SubPath[][] {
  const rings = subs.map((sp) => flattenSubPathTol(sp, 0.01));
  const area = rings.map((r) => Math.abs(ringSignedArea(r)));
  const inside = (i: number, j: number) => i !== j && rings[i].length > 0 && rings[j].length > 2 && windingOf(rings[j], rings[i][0]) !== 0;
  const depth = rings.map((_, i) => rings.reduce((d, _r, j) => (inside(i, j) ? d + 1 : d), 0));
  const out = new Map<number, SubPath[]>();
  subs.forEach((sp, i) => {
    if (depth[i] % 2 === 0) {
      out.set(i, [...(out.get(i) ?? []), sp]);
      return;
    }
    // A hole joins the smallest even-depth ring around it.
    let host = -1;
    rings.forEach((_, j) => {
      if (depth[j] % 2 === 0 && inside(i, j) && (host < 0 || area[j] < area[host])) host = j;
    });
    if (host < 0) out.set(i, [sp]);
    else out.set(host, [...(out.get(host) ?? []), sp]);
  });
  return [...out.values()];
}

/** Rectangles and ellipses as editable paths (texts cannot: there are no letter outlines here). */
export function shapesToPath(shapes: readonly SvgShape[]): OpResult {
  const conv = shapes.filter((s) => s.kind === 'rect' || s.kind === 'ellipse');
  if (!conv.length) return { error: shapes.some((s) => s.kind === 'text') ? 'Yazı yola çevrilemez (harf çizimleri yok).' : 'Seçilenler zaten yol.' };
  return { add: conv.map((s) => toPath(s)), remove: [], note: shapes.some((s) => s.kind === 'text') ? 'Yazılar yola çevrilemez; yerinde kaldılar.' : undefined };
}

/** Default stroke end and corner of a shape (as elementOf writes them). */
const capOf = (s: SvgShape): Cap => s.cap ?? (s.kind === 'path' ? 'round' : 'butt');
const joinOf = (s: SvgShape): Join => s.join ?? (s.kind === 'path' ? 'round' : 'miter');

/**
 * The stroke as a filled outline (its paint becomes the fill). A shape that
 * also has a fill keeps it as a shape below the outline, grouped with it.
 */
export function strokeToPath(shapes: readonly SvgShape[]): OpResult {
  const add: SvgShape[] = [];
  const remove: string[] = [];
  let skipped = 0;
  for (const s of shapes) {
    if (s.kind === 'text' || s.stroke === 'none' || !(s.strokeWidth > 0)) {
      skipped++;
      continue;
    }
    const p = toPath(s) as PathShape;
    const outline = strokeOutline(p.subs, { width: s.strokeWidth, cap: capOf(s), join: joinOf(s), dash: s.dash });
    if (!outline.length) {
      skipped++;
      continue;
    }
    const group = s.fill !== 'none' ? (s.group ?? shapeId()) : s.group;
    if (s.fill !== 'none') add.push({ ...s, stroke: 'none', group });
    add.push({ ...pathLike(s, outline), fill: s.stroke, stroke: 'none', dash: undefined, cap: undefined, join: undefined, fillRule: 'nonzero', group });
    remove.push(s.id);
  }
  if (!add.length) return { error: 'Çizgisi olan bir şekil seçin (yazılar çevrilmez).' };
  return { add, remove, note: skipped ? `${skipped} şeklin çizgisi yok, olduğu gibi kaldı.` : undefined };
}

/** Each shape's fill grown (d > 0) or shrunk (d < 0) by |d| drawing units. */
export function offsetShapes(shapes: readonly SvgShape[], d: number, join: Join = 'round'): OpResult {
  const usable = shapes.filter((s) => s.kind !== 'text');
  if (!usable.length) return { error: 'Büyütmek ya da küçültmek için bir şekil seçin (yazılar katılmaz).' };
  const add: SvgShape[] = [];
  const remove: string[] = [];
  let vanished = 0;
  for (const s of usable) {
    const subs = offsetRegion(regionOf(s)!, d, join);
    remove.push(s.id);
    if (!subs.length) vanished++;
    else add.push({ ...pathLike(s, subs, s.id), fillRule: 'nonzero' });
  }
  return { add, remove, note: vanished ? `${vanished} şekil bu kadar küçültülünce kayboldu.` : undefined };
}

/**
 * Fewer nodes within `tol`: every sub-path flattened and fitted again with
 * cubics; corners sharper than `cornerDeg` stay corners.
 */
export function simplifySubPath(sp: SubPath, tol: number, cornerDeg = 35): SubPath {
  const n = segmentCount(sp);
  if (n < 1) return copySub(sp);
  const pts: Pt[] = [[sp.nodes[0].x, sp.nodes[0].y]];
  const corners: number[] = [];
  const turn = (i: number) => {
    const node = sp.nodes[i];
    const m = sp.nodes.length;
    const prev = sp.nodes[(i - 1 + m) % m];
    const next = sp.nodes[(i + 1) % m];
    const a = node.in ?? (prev ? [prev.x, prev.y] : null);
    const b = node.out ?? (next ? [next.x, next.y] : null);
    if (!a || !b) return 180;
    const u = [node.x - a[0], node.y - a[1]];
    const w = [b[0] - node.x, b[1] - node.y];
    return (Math.abs(Math.atan2(u[0] * w[1] - u[1] * w[0], u[0] * w[0] + u[1] * w[1])) * 180) / Math.PI;
  };
  if (!sp.closed || turn(0) > cornerDeg) corners.push(0);
  for (let i = 0; i < n; i++) {
    const c = segmentCubic(sp, i);
    if (segmentIsLine(sp, i)) pts.push(c[3]);
    else pts.push(...flattenCubic(c, tol / 10).pts.slice(1));
    const end = (i + 1) % sp.nodes.length;
    if (end !== 0 && (turn(end) > cornerDeg || (!sp.closed && end === sp.nodes.length - 1))) corners.push(pts.length - 1);
  }
  if (sp.closed) pts.pop();
  return fitPolyline(pts, tol, { closed: sp.closed, corners, cornerDeg: 180 });
}

export function simplifyShapes(shapes: readonly SvgShape[], tol: number): OpResult {
  const usable = shapes.filter((s) => s.kind !== 'text');
  if (!usable.length) return { error: 'Sadeleştirmek için bir yol seçin.' };
  let before = 0;
  let after = 0;
  const add = usable.map((s) => {
    const p = toPath(s) as PathShape;
    // A sub-path the fit cannot shorten stays as it was.
    const subs = p.subs.map((sp) => {
      const out = simplifySubPath(sp, tol);
      return out.nodes.length < sp.nodes.length ? out : copySub(sp);
    });
    before += p.subs.reduce((k, sp) => k + sp.nodes.length, 0);
    after += subs.reduce((k, sp) => k + sp.nodes.length, 0);
    return { ...p, id: s.id, subs };
  });
  return { add, remove: usable.map((s) => s.id), note: `${before} düğüm → ${after} düğüm.` };
}

/** A per-sub-path change applied to every path among the shapes. */
function mapSubs(shapes: readonly SvgShape[], fn: (sp: SubPath) => SubPath, empty: string): OpResult {
  const usable = shapes.filter((s) => s.kind === 'path' || s.kind === 'rect' || s.kind === 'ellipse');
  if (!usable.length) return { error: empty };
  return { add: usable.map((s) => ({ ...(toPath(s) as PathShape), id: s.id, subs: (toPath(s) as PathShape).subs.map(fn) })), remove: usable.map((s) => s.id) };
}

export const reverseShapes = (shapes: readonly SvgShape[]): OpResult => mapSubs(shapes, reverseSubPath, 'Yönünü çevirmek için bir yol seçin.');

/** Open sub-paths closed (an end on the start merges into it). */
export function closeSubPath(sp: SubPath): SubPath {
  if (sp.closed || sp.nodes.length < 2) return copySub(sp);
  const nodes: PathNode[] = sp.nodes.map((n) => ({ ...n }));
  const a = nodes[0];
  const b = nodes[nodes.length - 1];
  if (nodes.length > 2 && Math.hypot(a.x - b.x, a.y - b.y) < 1e-9) {
    nodes.pop();
    if (b.in) a.in = b.in;
  }
  return { closed: true, nodes };
}

/** Closed sub-paths opened at their first node, keeping the shape (the closing segment stays). */
export function openSubPath(sp: SubPath): SubPath {
  if (!sp.closed || sp.nodes.length < 2) return copySub(sp);
  const nodes: PathNode[] = sp.nodes.map((n) => ({ ...n }));
  const first = nodes[0];
  const end: PathNode = { x: first.x, y: first.y };
  if (first.in) end.in = first.in;
  delete first.in;
  nodes.push(end);
  return { closed: false, nodes };
}

export const closeShapes = (shapes: readonly SvgShape[]): OpResult => mapSubs(shapes, closeSubPath, 'Kapatmak için bir yol seçin.');
export const openShapes = (shapes: readonly SvgShape[]): OpResult => mapSubs(shapes, openSubPath, 'Açmak için kapalı bir yol seçin.');

/** Puts an operation's result into the list: new shapes where the bottom-most removed one was. */
export function applyResult(list: readonly SvgShape[], r: Extract<OpResult, { add: SvgShape[] }>): SvgShape[] {
  const gone = new Set(r.remove);
  const replaced = new Map(r.add.filter((s) => gone.has(s.id)).map((s) => [s.id, s]));
  const fresh = r.add.filter((s) => !gone.has(s.id) || !replaced.has(s.id));
  // Shapes changed in place keep their place; new ones go where the bottom-most removed shape was.
  const firstGone = list.findIndex((s) => gone.has(s.id));
  const out: SvgShape[] = [];
  list.forEach((s, i) => {
    if (i === firstGone) out.push(...fresh.filter((f) => !replaced.has(f.id)));
    if (gone.has(s.id)) {
      const k = replaced.get(s.id);
      if (k) out.push(k);
    } else out.push(s);
  });
  if (firstGone < 0) {
    // Nothing removed (object to path): same-id shapes swap in place, the rest go on top.
    const byId = new Map(r.add.map((s) => [s.id, s]));
    return [...list.map((s) => byId.get(s.id) ?? s), ...r.add.filter((s) => !list.some((x) => x.id === s.id))];
  }
  return out;
}
