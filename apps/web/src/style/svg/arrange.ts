import { apply, multiply, type Box, type Matrix, type Pt } from './pathData';
import { rotation, shapeBox, shapeId, translate, transformShape, type SvgShape } from './svgModel';

/**
 * Arranging shapes in the SVG editor: align and distribute (Inkscape's
 * Align and Distribute), the numeric transforms (move, scale, rotate,
 * skew, matrix; together or each separately) and arrays for pattern and
 * symbol design (rectangular, polar, mirror copy, as a CAD array). A
 * group moves as one. Everything here gives matrices or new shapes;
 * the editor applies them as one undo step. Pure.
 */

/** Things that move together: a group, or a shape on its own; in the order they were chosen. */
export interface Unit {
  ids: string[];
  box: Box;
}

const unionBox = (a: Box, b: Box): Box => ({ minX: Math.min(a.minX, b.minX), minY: Math.min(a.minY, b.minY), maxX: Math.max(a.maxX, b.maxX), maxY: Math.max(a.maxY, b.maxY) });
const area = (b: Box) => (b.maxX - b.minX) * (b.maxY - b.minY);

/** The chosen shapes as units, in the order of `order` (the selection's order). */
export function unitsOf(shapes: readonly SvgShape[], order: readonly string[]): Unit[] {
  const byId = new Map(shapes.map((s) => [s.id, s]));
  const units = new Map<string, Unit>();
  for (const id of order) {
    const s = byId.get(id);
    if (!s) continue;
    const key = s.group ? `g:${s.group}` : `s:${s.id}`;
    const b = shapeBox(s);
    const u = units.get(key);
    if (u) {
      u.ids.push(id);
      u.box = unionBox(u.box, b);
    } else units.set(key, { ids: [id], box: b });
  }
  return [...units.values()];
}

export type AlignTo = 'selection' | 'first' | 'last' | 'biggest' | 'smallest' | 'canvas';
export type AlignSide = 'left' | 'hcenter' | 'right' | 'top' | 'vcenter' | 'bottom';

/** The box the units align against. One unit alone aligns to the canvas. */
export function alignReference(units: readonly Unit[], to: AlignTo, page: { width: number; height: number }): Box {
  const canvas = { minX: 0, minY: 0, maxX: page.width, maxY: page.height };
  if (to === 'canvas' || units.length < 2) return canvas;
  if (to === 'first') return units[0].box;
  if (to === 'last') return units[units.length - 1].box;
  if (to === 'biggest') return units.reduce((a, b) => (area(b.box) > area(a.box) ? b : a)).box;
  if (to === 'smallest') return units.reduce((a, b) => (area(b.box) < area(a.box) ? b : a)).box;
  return units.map((u) => u.box).reduce(unionBox);
}

function alignShift(b: Box, ref: Box, side: AlignSide): Pt {
  switch (side) {
    case 'left':
      return [ref.minX - b.minX, 0];
    case 'right':
      return [ref.maxX - b.maxX, 0];
    case 'hcenter':
      return [(ref.minX + ref.maxX - b.minX - b.maxX) / 2, 0];
    case 'top':
      return [0, ref.minY - b.minY];
    case 'bottom':
      return [0, ref.maxY - b.maxY];
    case 'vcenter':
      return [0, (ref.minY + ref.maxY - b.minY - b.maxY) / 2];
  }
}

/**
 * One translation per unit putting its edge or centre on the reference's.
 * `asOne` moves the whole selection as one block (useful against the canvas).
 */
export function alignMoves(units: readonly Unit[], side: AlignSide, to: AlignTo, page: { width: number; height: number }, asOne = false): Matrix[] {
  const ref = alignReference(units, to, page);
  if (asOne && units.length) {
    const [dx, dy] = alignShift(units.map((u) => u.box).reduce(unionBox), ref, side);
    return units.map(() => translate(dx, dy));
  }
  return units.map((u) => {
    const [dx, dy] = alignShift(u.box, ref, side);
    return translate(dx, dy);
  });
}

export type Distribute = 'left' | 'hcenter' | 'right' | 'hgap' | 'top' | 'vcenter' | 'bottom' | 'vgap';

/**
 * Even spacing between the outermost units (they stay): of edges or
 * centres, or equal gaps between neighbouring boxes.
 */
export function distributeMoves(units: readonly Unit[], how: Distribute): Matrix[] {
  const out = units.map(() => translate(0, 0));
  if (units.length < 3) return out;
  const horizontal = how === 'left' || how === 'hcenter' || how === 'right' || how === 'hgap';
  const lo = (b: Box) => (horizontal ? b.minX : b.minY);
  const hi = (b: Box) => (horizontal ? b.maxX : b.maxY);
  const key = (b: Box) => (how === 'left' || how === 'top' || how === 'hgap' || how === 'vgap' ? lo(b) : how === 'right' || how === 'bottom' ? hi(b) : (lo(b) + hi(b)) / 2);
  const order = units.map((_u, i) => i).sort((a, b) => key(units[a].box) - key(units[b].box));
  const move = (i: number, d: number) => (out[i] = horizontal ? translate(d, 0) : translate(0, d));
  if (how === 'hgap' || how === 'vgap') {
    const first = units[order[0]].box;
    const last = units[order[order.length - 1]].box;
    const sizes = order.reduce((s, i) => s + hi(units[i].box) - lo(units[i].box), 0);
    const gap = (hi(last) - lo(first) - sizes) / (order.length - 1);
    let at = lo(first);
    for (const i of order) {
      move(i, at - lo(units[i].box));
      at += hi(units[i].box) - lo(units[i].box) + gap;
    }
    return out;
  }
  const a = key(units[order[0]].box);
  const step = (key(units[order[order.length - 1]].box) - a) / (order.length - 1);
  order.forEach((i, k) => move(i, a + step * k - key(units[i].box)));
  return out;
}

// ── Transforms ─────────────────────────────────────────────────────────

/** A box point: corners, edge middles, centre. */
export type Anchor = 'tl' | 't' | 'tr' | 'l' | 'c' | 'r' | 'bl' | 'b' | 'br';

export function anchorPoint(b: Box, a: Anchor): Pt {
  const x = a.endsWith('l') ? b.minX : a.endsWith('r') ? b.maxX : (b.minX + b.maxX) / 2;
  const y = a.startsWith('t') ? b.minY : a.startsWith('b') ? b.maxY : (b.minY + b.maxY) / 2;
  return [x, y];
}

export const scaleAbout = (sx: number, sy: number, p: Pt): Matrix => [sx, 0, 0, sy, p[0] - sx * p[0], p[1] - sy * p[1]];

/** Skew by angles (degrees): x leans with y by `ax`, y with x by `ay`, about p. */
export function skewAbout(ax: number, ay: number, p: Pt): Matrix {
  const tx = Math.tan((ax * Math.PI) / 180);
  const ty = Math.tan((ay * Math.PI) / 180);
  return [1, ty, tx, 1, -tx * p[1], -ty * p[0]];
}

/** Rotation by `deg`, counter-clockwise on screen when `ccw` (the drawing's y runs down). */
export const rotateAbout = (deg: number, p: Pt, ccw: boolean): Matrix => rotation(ccw ? -deg : deg, p[0], p[1]);

export type TransformSpec =
  | { kind: 'move'; x: number; y: number; relative: boolean }
  | { kind: 'scale'; sx: number; sy: number; anchor: Anchor }
  | { kind: 'rotate'; deg: number; ccw: boolean; about: Anchor | Pt }
  | { kind: 'skew'; ax: number; ay: number; anchor: Anchor }
  | { kind: 'matrix'; m: Matrix };

/**
 * The matrix of every unit. Together, the selection's box is the frame;
 * separately, each unit's own box (a relative move then steps each unit
 * one move further than the one before, spreading them out).
 */
export function transformMoves(units: readonly Unit[], spec: TransformSpec, separately: boolean): Matrix[] {
  if (!units.length) return [];
  const all = units.map((u) => u.box).reduce(unionBox);
  const one = (b: Box, k: number): Matrix => {
    switch (spec.kind) {
      case 'move':
        return spec.relative ? translate(spec.x * (separately ? k + 1 : 1), spec.y * (separately ? k + 1 : 1)) : translate(spec.x - b.minX, spec.y - b.minY);
      case 'scale':
        return scaleAbout(spec.sx / 100, spec.sy / 100, anchorPoint(b, spec.anchor));
      case 'rotate':
        return rotateAbout(spec.deg, typeof spec.about === 'string' ? anchorPoint(b, spec.about) : spec.about, spec.ccw);
      case 'skew':
        return skewAbout(spec.ax, spec.ay, anchorPoint(b, spec.anchor));
      case 'matrix':
        // Separately: about each box's top-left corner, as if it were the origin.
        return separately ? multiply(translate(b.minX, b.minY), multiply(spec.m, translate(-b.minX, -b.minY))) : spec.m;
    }
  };
  return units.map((u, k) => one(separately ? u.box : all, k));
}

/** A matrix is usable: finite and not flattening everything to a line. */
export const invertible = (m: Matrix) => m.every(Number.isFinite) && Math.abs(m[0] * m[3] - m[1] * m[2]) > 1e-12;

// ── Arrays ─────────────────────────────────────────────────────────────

export interface RectArraySpec {
  rows: number;
  cols: number;
  /** Column and row spacing: centre to centre (`step`) or the gap between boxes (`gap`). */
  dx: number;
  dy: number;
  mode: 'step' | 'gap';
}

/** Offsets of every copy of a rectangular array (the original, row 0 column 0, is left out). */
export function rectArray(box: Box, spec: RectArraySpec): Matrix[] {
  const sx = spec.mode === 'gap' ? box.maxX - box.minX + spec.dx : spec.dx;
  const sy = spec.mode === 'gap' ? box.maxY - box.minY + spec.dy : spec.dy;
  const out: Matrix[] = [];
  for (let r = 0; r < Math.max(1, Math.round(spec.rows)); r++) for (let c = 0; c < Math.max(1, Math.round(spec.cols)); c++) if (r || c) out.push(translate(c * sx, r * sy));
  return out;
}

export interface PolarArraySpec {
  count: number;
  /** Angle the copies fill (360: a full turn, evenly spaced). */
  angle: number;
  centre: Pt;
  /** Copies turn with their place (as a wheel); off, they only move. */
  rotate: boolean;
  ccw: boolean;
}

export function polarArray(box: Box, spec: PolarArraySpec): Matrix[] {
  const n = Math.max(1, Math.round(spec.count));
  const full = Math.abs(spec.angle) >= 360 - 1e-9;
  const step = n > 1 ? spec.angle / (full ? n : n - 1) : 0;
  const mid: Pt = [(box.minX + box.maxX) / 2, (box.minY + box.maxY) / 2];
  const out: Matrix[] = [];
  for (let k = 1; k < n; k++) {
    const r = rotateAbout(step * k, spec.centre, spec.ccw);
    if (spec.rotate) out.push(r);
    else {
      const [x, y] = apply(r, mid[0], mid[1]);
      out.push(translate(x - mid[0], y - mid[1]));
    }
  }
  return out;
}

/** Reflection across a line through p: vertical, horizontal, or at `deg` from the x axis. */
export function mirrorMatrix(axis: 'v' | 'h' | 'angle', p: Pt, deg = 0): Matrix {
  const a = axis === 'v' ? Math.PI / 2 : axis === 'h' ? 0 : (deg * Math.PI) / 180;
  const c = Math.cos(2 * a);
  const s = Math.sin(2 * a);
  // x' = p + R(x − p), R the reflection across the direction a.
  return [c, s, s, -c, p[0] - c * p[0] - s * p[1], p[1] - s * p[0] + c * p[1]];
}

/** Copies of the shapes under each matrix: new ids, and each copy's groups its own. */
export function copiesOf(shapes: readonly SvgShape[], matrices: readonly Matrix[]): SvgShape[] {
  const out: SvgShape[] = [];
  for (const m of matrices) {
    const groups = new Map<string, string>();
    for (const s of shapes) {
      const g = s.group ? (groups.get(s.group) ?? groups.set(s.group, shapeId()).get(s.group)) : undefined;
      out.push({ ...transformShape(structuredClone(s), m), id: shapeId(), group: g });
    }
  }
  return out;
}

// ── Stacking order ─────────────────────────────────────────────────────

/** The chosen shapes one step up or down past a neighbour, or to the top or bottom (list is back to front). */
export function restack(list: readonly SvgShape[], ids: ReadonlySet<string>, op: 'raise' | 'lower' | 'top' | 'bottom'): SvgShape[] {
  const sel = list.filter((s) => ids.has(s.id));
  const rest = list.filter((s) => !ids.has(s.id));
  if (op === 'top') return [...rest, ...sel];
  if (op === 'bottom') return [...sel, ...rest];
  const out = [...list];
  const order = op === 'raise' ? [...out.keys()].reverse() : [...out.keys()];
  for (const i of order) {
    const j = op === 'raise' ? i + 1 : i - 1;
    if (ids.has(out[i].id) && j >= 0 && j < out.length && !ids.has(out[j].id)) [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}
