import type { Box, Pt } from '../../style/svg/pathData';
import { regularPolygon, rotation, transformShape, type Paint, type SvgShape } from '../../style/svg/svgModel';

/**
 * The SVG editor's rules that are not the SVG core's geometry and not the
 * page: the shape a drag makes with each drawing tool, a box scaled by a
 * handle, a turn by the knob, the measure's readout, the rulers' steps,
 * a corner's tidy size, a file's name and the starting grid and panel
 * values. Pure functions of their inputs, so the desktop's editor
 * (apps/desktop/src/style/svgedit/) is held to the same answers through
 * fixtures/style/v1/svgedit.json (scripts/fixtures/record-svgedit.test.ts).
 */

export type DragTool = 'rect' | 'ellipse' | 'polygon' | 'text';

/** What a drag needs besides its two points: the tool, the canvas, the polygon's settings, the keys held. */
export interface DragSpec {
  tool: DragTool;
  width: number;
  height: number;
  sides: number;
  star: boolean;
  /** Shift: a square or a circle. */
  shift: boolean;
  /** Alt: from the centre. */
  fromCentre: boolean;
}

/** A new shape's stroke width: a fiftieth of the canvas (a drawn line's a twenty-fifth), at least 1. */
export const shapeStrokeWidth = (width: number) => Math.max(1, width / 50);
export const lineStrokeWidth = (width: number) => Math.max(1, width / 25);

/** The grid a drawing starts with: a twentieth of its width, at least 1. */
export const defaultGrid = (width: number) => Math.max(1, Math.round(width / 20));

/** The panels' starting distance (offset, corner, array gaps): a fiftieth of the width, at least 0.1. */
export const panelUnit = (width: number) => Math.max(0.1, Math.round((width / 50) * 100) / 100);

/** The shape a drag from p0 to p1 makes (null for a drag too short to draw). */
export function shapeFromDrag(spec: DragSpec, p0: Pt, p1: Pt, id: string): SvgShape | null {
  const base = { id, fill: 'fill' as Paint, stroke: 'none' as Paint, strokeWidth: shapeStrokeWidth(spec.width) };
  if (spec.tool === 'text') return { ...base, kind: 'text', x: p0[0], y: p0[1], text: 'Aa', size: spec.height / 5, weight: 700, font: 'sans', anchor: 'start' };
  let dx = p1[0] - p0[0];
  let dy = p1[1] - p0[1];
  if (Math.hypot(dx, dy) < 0.5) return null;
  if (spec.tool === 'polygon') {
    const r = Math.hypot(dx, dy);
    const sp = regularPolygon(p0[0], p0[1], r, Math.max(3, spec.sides), spec.star ? r * 0.45 : undefined);
    // The first corner points at the pointer.
    const turn = Math.atan2(dy, dx) + Math.PI / 2;
    return transformShape({ ...base, kind: 'path', subs: [sp] }, rotation((turn * 180) / Math.PI, p0[0], p0[1]));
  }
  if (spec.shift) {
    const k = Math.max(Math.abs(dx), Math.abs(dy));
    dx = Math.sign(dx || 1) * k;
    dy = Math.sign(dy || 1) * k;
  }
  const x0 = spec.fromCentre ? p0[0] - Math.abs(dx) : Math.min(p0[0], p0[0] + dx);
  const y0 = spec.fromCentre ? p0[1] - Math.abs(dy) : Math.min(p0[1], p0[1] + dy);
  const w = spec.fromCentre ? 2 * Math.abs(dx) : Math.abs(dx);
  const h = spec.fromCentre ? 2 * Math.abs(dy) : Math.abs(dy);
  if (spec.tool === 'rect') return { ...base, kind: 'rect', x: x0, y: y0, w, h };
  return { ...base, kind: 'ellipse', cx: x0 + w / 2, cy: y0 + h / 2, rx: w / 2, ry: h / 2 };
}

/**
 * The selection's box with a handle (0 top left, clockwise to 7 middle
 * left) dragged to q; Shift keeps a corner's proportions. Null when the
 * box would collapse.
 */
export function scaledBox(box: Box, handle: number, q: Pt, shift: boolean): Box | null {
  const b = { ...box };
  const h = handle;
  if (h === 0 || h === 6 || h === 7) b.minX = q[0];
  if (h === 2 || h === 3 || h === 4) b.maxX = q[0];
  if (h === 0 || h === 1 || h === 2) b.minY = q[1];
  if (h === 4 || h === 5 || h === 6) b.maxY = q[1];
  if (shift && h % 2 === 0) {
    const k = Math.max((b.maxX - b.minX) / (box.maxX - box.minX), (b.maxY - b.minY) / (box.maxY - box.minY));
    const w = (box.maxX - box.minX) * k;
    const hh = (box.maxY - box.minY) * k;
    if (h === 0 || h === 6) b.minX = b.maxX - w;
    else b.maxX = b.minX + w;
    if (h === 0 || h === 2) b.minY = b.maxY - hh;
    else b.maxY = b.minY + hh;
  }
  if (Math.abs(b.maxX - b.minX) < 1e-6 || Math.abs(b.maxY - b.minY) < 1e-6) return null;
  return b;
}

/** The turn of a drag on the rotation knob about the box's centre, in degrees; Shift in 15° steps. */
export function knobTurn(box: Box, p0: Pt, p: Pt, shift: boolean): number {
  const cx = (box.minX + box.maxX) / 2;
  const cy = (box.minY + box.maxY) / 2;
  const deg = ((Math.atan2(p[1] - cy, p[0] - cx) - Math.atan2(p0[1] - cy, p0[0] - cx)) * 180) / Math.PI;
  return shift ? Math.round(deg / 15) * 15 : deg;
}

/** A number for labels: at most `digits` decimals, no trailing zeros. */
export const fmtNum = (v: number, digits = 3) => {
  const s = (Math.round(v * 10 ** digits) / 10 ** digits).toString();
  return s === '-0' ? '0' : s;
};

/** A length at the symbol's size, or nothing when the drawing has no size in mm. */
export const mmOf = (units: number, width: number, sizeMm: number | undefined) => (sizeMm ? `${fmtNum((units * sizeMm) / width, 2)} mm` : '');

/**
 * The measure from a to b: the distance (in mm too at the symbol's size)
 * and the angle, then ΔX and ΔY. The angle is the drawing's own: y runs
 * down, so it grows clockwise from the right, from −180° to 180°, as ΔY,
 * the rulers and a shape's Döndürme read (it used to be counter-clockwise
 * from 0° to 360° beside a ΔY that runs down).
 */
export function measureReadout(a: Pt, b: Pt, width: number, sizeMm: number | undefined): { main: string; more: string } {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const d = Math.hypot(dx, dy);
  const ang = (Math.atan2(dy, dx) * 180) / Math.PI;
  const mm = mmOf(d, width, sizeMm);
  return { main: `${fmtNum(d)} birim${mm ? ` · ${mm}` : ''} · açı ${fmtNum(ang, 2)}°`, more: `ΔX ${fmtNum(dx)}, ΔY ${fmtNum(dy)}` };
}

/** The smallest 1-2-5 step not below `v` (the rulers' majors). */
export function niceStep(v: number): number {
  const e = 10 ** Math.floor(Math.log10(v));
  const m = v / e;
  return (m <= 1 ? 1 : m <= 2 ? 2 : m <= 5 ? 5 : 10) * e;
}

/** A size rounded to a 1-2-5 step near `unit` (what a pixel is worth), so dragging gives tidy values. */
export function niceRound(v: number, unit: number): number {
  const e = 10 ** Math.floor(Math.log10(unit));
  const step = unit / e < 2 ? e : unit / e < 5 ? 2 * e : 5 * e;
  return Math.round(v / step) * step;
}

/** A file's name from a drawing's name: Turkish letters plain, anything else a dash. */
export const fileSlug = (s: string) =>
  s
    .toLocaleLowerCase('tr')
    .replace(/[çğıöşü]/g, (c) => ({ ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u' })[c] ?? c)
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'cizim';
