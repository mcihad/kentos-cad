import type { AppContext } from '../app/context';
import type { Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { op } from '../wasm/core';
import { parsePointInput } from './coordinateInput';
import { drawTag } from './preview';
import type { ToolPointer } from './Tool';

/** A polar-tracking ray the cursor is currently locked to. */
export interface Tracking {
  origin: Vec2;
  /** Degrees, CCW from east. */
  angle: number;
}

const CAPTURE_PX = 10;

/**
 * The effective cursor for point input from `from` (Rust core,
 * tools/point_input.rs): an exact point (object snap or tracking) stays,
 * ortho keeps the larger of the two moves, polar tracking (`polarStep`
 * degrees; null when off) locks onto the nearest ray within `tol`.
 */
export const constrainCursor =
  op<(from: Vec2 | null, world: Vec2, exact: boolean, ortho: boolean, polarStep: number | null, tol: number) => { point: Vec2; tracking: Tracking | null }>('constrainCursor');

/**
 * Resolves the effective cursor for point input: object snap wins, then
 * ortho (Shift inverts it), then polar tracking to the nearest increment
 * within 10 px.
 */
export function constrainPoint(ctx: AppContext, from: Vec2 | null, p: ToolPointer): { point: Vec2; tracking: Tracking | null } {
  // Object snaps and object tracking are exact; ortho and polar never move them (the core
  // answers the same; this saves the call on every snapped move).
  if (!from || p.snap || p.track) return { point: p.world, tracking: null };
  const polarStep = ctx.settings.polar.value ? ctx.prefs.polarIncrement.value : null;
  return constrainCursor(from, p.world, false, ctx.settings.ortho.value !== p.shift, polarStep, ctx.view.worldTolerance(CAPTURE_PX));
}

/** Dashed tracking ray across the viewport plus a small angle tag. */
export function drawTracking(g: CanvasRenderingContext2D, view: ViewTransform, t: Tracking, at: Vec2, color: string, bg: string): void {
  const o = view.worldToScreen(t.origin);
  const rad = (t.angle * Math.PI) / 180;
  const far = { x: o.x + Math.cos(rad) * 1e4, y: o.y - Math.sin(rad) * 1e4 };
  g.save();
  g.strokeStyle = color;
  g.globalAlpha = 0.7;
  g.setLineDash([2, 4]);
  g.beginPath();
  g.moveTo(o.x, o.y);
  g.lineTo(far.x, far.y);
  g.stroke();
  g.restore();
  const s = view.worldToScreen(at);
  drawTag(g, { x: s.x - 8, y: s.y - 44 }, [`Kutupsal ${t.angle.toFixed(0)}°`], color, bg);
}

/**
 * Typed point input for tools. Like parsePointInput, but while object
 * tracking holds the cursor on an alignment a bare number is the distance
 * from the tracked point along that line.
 */
export function pointFromText(ctx: AppContext, text: string, last: Vec2 | null, cursor: Vec2 | null): Vec2 | null {
  return parsePointInput(text, last, cursor, (d) => ctx.view.trackAlong(d));
}
