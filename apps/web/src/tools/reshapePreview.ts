import { entityGeometry, entityOutline, polygonRing, type Entity, type EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { strokePath } from './preview';

/** Preview drawing of the reshaping tools (docs/adr/0140): direction arrows, marked vertices, the pieces of a split. */

/**
 * The paths of an object's outline, in the order it runs: an area's ring and its holes, each closed, then each
 * other part's ring and holes (docs/adr/0143).
 */
export function outlinesOf(geom: EntityGeometry): { pts: Vec2[]; closed: boolean }[] {
  if (geom.kind === 'point' || geom.kind === 'text' || geom.kind === 'dimension') return [];
  if (geom.kind === 'polygon') {
    const ring = (r: { pts: Vec2[]; bulges?: number[] }) => ({ pts: polygonRing(r), closed: true });
    return [
      { pts: entityOutline(geom, 64), closed: true },
      ...(geom.holes ?? []).map(ring),
      ...(geom.parts ?? []).flatMap((part) => [ring(part), ...(part.holes ?? []).map(ring)]),
    ];
  }
  return [{ pts: entityOutline(geom, 64), closed: geom.kind === 'circle' }];
}

/**
 * The rings of a path or an area as lists of points: a polyline's points; an area's outer ring, its holes, then each
 * other part's ring and holes (docs/adr/0143). Nothing for another kind.
 */
export function pathRings(e: Entity): Vec2[][] {
  if (e.kind === 'polyline') return [e.pts];
  if (e.kind !== 'polygon') return [];
  return [e.pts, ...(e.holes ?? []).map((h) => h.pts), ...(e.parts ?? []).flatMap((part) => [part.pts, ...(part.holes ?? []).map((h) => h.pts)])];
}

/** Arrowheads along a path of the drawing, one every `gap` screen pixels, pointing the way it runs; a dot marks its start. */
export function strokeDirection(g: CanvasRenderingContext2D, view: ViewTransform, pts: readonly Vec2[], closed: boolean, color: string, halo: string, gap = 64): void {
  const s = pts.map((p) => view.worldToScreen(p));
  if (closed && s.length) s.push(s[0]);
  if (s.length < 2) return;
  g.save();
  g.lineJoin = 'round';
  let next = gap / 2;
  let walked = 0;
  let heads = 0;
  for (let i = 0; i + 1 < s.length && heads < 400; i++) {
    const [a, b] = [s[i], s[i + 1]];
    const len = Math.hypot(b.x - a.x, b.y - a.y);
    if (len < 1e-6) continue;
    const ux = (b.x - a.x) / len;
    const uy = (b.y - a.y) / len;
    while (next <= walked + len && heads < 400) {
      const t = next - walked;
      const cx = a.x + ux * t;
      const cy = a.y + uy * t;
      g.beginPath();
      g.moveTo(cx + ux * 8, cy + uy * 8);
      g.lineTo(cx - ux * 6 - uy * 6, cy - uy * 6 + ux * 6);
      g.lineTo(cx - ux * 6 + uy * 6, cy - uy * 6 - ux * 6);
      g.closePath();
      g.fillStyle = color;
      g.strokeStyle = halo;
      g.lineWidth = 2.5;
      g.stroke();
      g.fill();
      next += gap;
      heads++;
    }
    walked += len;
  }
  // The start: a filled dot with a ring.
  g.beginPath();
  g.arc(s[0].x, s[0].y, 4, 0, Math.PI * 2);
  g.fillStyle = color;
  g.strokeStyle = halo;
  g.lineWidth = 2;
  g.stroke();
  g.fill();
  g.restore();
}

/** The directions an object's outline runs, for every path of it. */
export function strokeDirections(g: CanvasRenderingContext2D, view: ViewTransform, e: Entity, color: string, halo: string): void {
  for (const o of outlinesOf(entityGeometry(e))) strokeDirection(g, view, o.pts, o.closed, color, halo);
}

/** Vertices marked with a ring; `cross` strikes them through (a vertex to go). */
export function markVertices(g: CanvasRenderingContext2D, view: ViewTransform, pts: readonly Vec2[], color: string, cross: boolean): void {
  g.save();
  g.strokeStyle = color;
  g.lineWidth = 1.75;
  g.setLineDash([]);
  for (const p of pts) {
    const s = view.worldToScreen(p);
    g.beginPath();
    g.arc(s.x, s.y, 5, 0, Math.PI * 2);
    g.stroke();
    if (cross) {
      g.beginPath();
      g.moveTo(s.x - 3.5, s.y - 3.5);
      g.lineTo(s.x + 3.5, s.y + 3.5);
      g.moveTo(s.x + 3.5, s.y - 3.5);
      g.lineTo(s.x - 3.5, s.y + 3.5);
      g.stroke();
    }
  }
  g.restore();
}

/** The first point of an object that runs from one to another (a line, a polyline, an arc). */
export function startOf(e: Entity): Vec2 | null {
  switch (e.kind) {
    case 'line':
      return e.a;
    case 'polyline':
      return e.pts[0] ?? null;
    case 'arc':
      return { x: e.c.x + e.r * Math.cos(e.a0), y: e.c.y + e.r * Math.sin(e.a0) };
    default:
      return null;
  }
}

/**
 * The pieces of a split, drawn one after the other in two alternating
 * colours so each is told from its neighbour (the callers pick two that differ
 * from the selection's accent); a ring marks every cut. A
 * circle's first piece starts at a cut too (`round`).
 */
export function strokePieces(g: CanvasRenderingContext2D, view: ViewTransform, pieces: readonly Entity[], colors: readonly [string, string], halo: string, round = false): void {
  // A halo of the canvas colour first, so the pieces stand out from the selection drawn beneath them.
  const outlines = pieces.map((p) => outlinesOf(entityGeometry(p)));
  for (const os of outlines) for (const o of os) strokePath(g, view, o.pts, { color: halo, width: 6, closed: o.closed });
  outlines.forEach((os, i) => {
    for (const o of os) strokePath(g, view, o.pts, { color: colors[i % 2], width: 3.5, closed: o.closed });
  });
  const cuts = (round ? pieces : pieces.slice(1)).map(startOf).filter((p): p is Vec2 => !!p);
  markVertices(g, view, cuts, colors[1], false);
}
