import { entityOutline, polygonRing, type EntityGeometry, type RingGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { layoutDimension } from '../model/geom/dimension';
import type { ViewTransform } from '../viewport/Camera';

/** Shared drawing helpers for tool previews (numbers go through ctx.format). */

export function strokePath(
  g: CanvasRenderingContext2D,
  view: ViewTransform,
  pts: readonly Vec2[],
  opts: { color: string; closed?: boolean; dash?: number[]; width?: number; fill?: string },
): void {
  if (pts.length < 2) return;
  g.save();
  g.beginPath();
  pts.forEach((p, i) => {
    const s = view.worldToScreen(p);
    i ? g.lineTo(s.x, s.y) : g.moveTo(s.x, s.y);
  });
  if (opts.closed) g.closePath();
  if (opts.fill) {
    g.fillStyle = opts.fill;
    g.fill();
  }
  g.setLineDash(opts.dash ?? []);
  g.lineWidth = opts.width ?? 1;
  g.strokeStyle = opts.color;
  g.stroke();
  g.restore();
}

/** Small tag next to the cursor: "23.412 m  ∠ 32.4°". */
export function drawTag(g: CanvasRenderingContext2D, at: Vec2, lines: string[], color: string, bg: string): void {
  if (!lines.length) return;
  g.save();
  g.font = '500 11px Barlow, system-ui, sans-serif';
  const w = Math.max(...lines.map((l) => g.measureText(l).width)) + 12;
  const h = lines.length * 14 + 6;
  const x = Math.round(at.x + 16);
  const y = Math.round(at.y + 16);
  g.fillStyle = bg;
  g.globalAlpha = 0.92;
  g.fillRect(x, y, w, h);
  g.globalAlpha = 1;
  g.strokeStyle = color;
  g.lineWidth = 1;
  g.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
  g.fillStyle = color;
  g.textBaseline = 'top';
  lines.forEach((l, i) => g.fillText(l, x + 6, y + 4 + i * 14));
  g.restore();
}

/** Outline of any entity geometry (circles/arcs tessellated), e.g. modify-tool ghosts. */
export function strokeGeometry(
  g: CanvasRenderingContext2D,
  view: ViewTransform,
  geom: EntityGeometry,
  opts: { color: string; dash?: number[]; width?: number },
): void {
  if (geom.kind === 'point' || geom.kind === 'text') {
    const s = view.worldToScreen(geom.p);
    g.save();
    g.strokeStyle = opts.color;
    g.setLineDash([]);
    g.strokeRect(Math.round(s.x) - 3.5, Math.round(s.y) - 3.5, 7, 7);
    g.restore();
    return;
  }
  if (geom.kind === 'dimension') {
    for (const [a, b] of layoutDimension(geom)?.lines ?? []) strokePath(g, view, [a, b], opts);
    return;
  }
  strokePath(g, view, entityOutline(geom, 64), { ...opts, closed: geom.kind === 'polygon' || geom.kind === 'circle' });
  if (geom.kind === 'polygon') for (const h of geom.holes ?? []) strokePath(g, view, polygonRing(h), { ...opts, closed: true });
}

/**
 * Outlines the geometry store computed (ghosts of moved, stretched or
 * pasted objects): `flags, n, x0, y0, …` per path, flags 0 open, 1 closed,
 * 2 a marker drawn as a small square, as `strokeGeometry` draws points.
 */
export function strokePaths(g: CanvasRenderingContext2D, view: ViewTransform, paths: Float64Array, opts: { color: string; dash?: number[]; width?: number }): void {
  for (let i = 0; i + 1 < paths.length; ) {
    const flags = paths[i];
    const n = paths[i + 1];
    const pts: Vec2[] = [];
    for (let k = 0; k < n; k++) pts.push({ x: paths[i + 2 + 2 * k], y: paths[i + 3 + 2 * k] });
    i += 2 + 2 * n;
    if (flags === 2) {
      if (!pts.length) continue;
      const s = view.worldToScreen(pts[0]);
      g.save();
      g.strokeStyle = opts.color;
      g.setLineDash([]);
      g.strokeRect(Math.round(s.x) - 3.5, Math.round(s.y) - 3.5, 7, 7);
      g.restore();
    } else strokePath(g, view, pts, { ...opts, closed: flags === 1 });
  }
}

/** An area (outer ring and holes) filled with the even–odd rule and outlined: area tool previews. */
export function drawArea(
  g: CanvasRenderingContext2D,
  view: ViewTransform,
  area: { outer: RingGeometry; holes: readonly RingGeometry[] },
  opts: { color: string; fill?: string; dash?: number[]; width?: number },
): void {
  g.save();
  g.beginPath();
  for (const r of [area.outer, ...area.holes]) {
    polygonRing(r).forEach((p, i) => {
      const s = view.worldToScreen(p);
      i ? g.lineTo(s.x, s.y) : g.moveTo(s.x, s.y);
    });
    g.closePath();
  }
  if (opts.fill) {
    g.fillStyle = opts.fill;
    g.fill('evenodd');
  }
  g.setLineDash(opts.dash ?? []);
  g.lineWidth = opts.width ?? 1.5;
  g.strokeStyle = opts.color;
  g.stroke();
  g.restore();
}

/**
 * Selection box between two screen points: left→right is a window (solid,
 * blue: fully inside), right→left a crossing (dashed, snap colour: touching).
 */
export function drawSelectionBox(g: CanvasRenderingContext2D, a: Vec2, b: Vec2, crossingColor: string): void {
  const crossing = b.x < a.x;
  const color = crossing ? crossingColor : '#6DB3F2';
  g.save();
  g.fillStyle = color;
  g.globalAlpha = 0.1;
  g.fillRect(Math.min(a.x, b.x), Math.min(a.y, b.y), Math.abs(b.x - a.x), Math.abs(b.y - a.y));
  g.globalAlpha = 1;
  g.strokeStyle = color;
  g.setLineDash(crossing ? [5, 4] : []);
  g.strokeRect(Math.min(a.x, b.x) + 0.5, Math.min(a.y, b.y) + 0.5, Math.abs(b.x - a.x), Math.abs(b.y - a.y));
  g.restore();
}

/** A palette colour (#rrggbb) at the given opacity, for translucent preview fills. */
export function tint(color: string, alpha: number): string {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color.trim());
  return m ? `rgba(${parseInt(m[1], 16)}, ${parseInt(m[2], 16)}, ${parseInt(m[3], 16)}, ${alpha})` : color;
}
