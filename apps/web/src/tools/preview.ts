import { entityOutline, polygonRing, type EntityGeometry, type RingGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { layoutDimension, type DimensionLayout } from '../model/geom/dimension';
import { faceFont, leanOf, type Face } from '../render/drawingFaces';
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

/**
 * A text to come as it will be drawn, faint (Etiketleri yazıya çevir, docs/adr/0175 §3; Koordinat yaz, 0185 §5):
 * `height` metres high in its face (the drawing's family as text objects are drawn, italic 400, without one) and width
 * factor, turned `rotation` degrees, its point on its alignment (the middle of a line is half its height over the
 * baseline, docs/adr/0145; none: the left of the baseline), over its mask in `mask` when it will have one. Under a
 * pixel high it is not drawn.
 */
export function drawTextGhost(
  g: CanvasRenderingContext2D,
  view: ViewTransform,
  t: { p: Vec2; text: string; height: number; rotation: number; align?: string; face?: Face; widthFactor?: number },
  opts: { color: string; font: string; mask: string | null },
): void {
  const px = t.height * view.scale;
  if (!(px >= 1)) return;
  const s = view.worldToScreen(t.p);
  const align = t.align ?? '';
  const factor = t.widthFactor ?? 1;
  g.save();
  g.translate(s.x, s.y);
  g.rotate((-t.rotation * Math.PI) / 180);
  const lean = t.face ? leanOf(t.face) : 0;
  g.font = faceFont(t.face ?? {}, px, opts.font, 'italic 400');
  const w = g.measureText(t.text).width * factor;
  const along = /Center$/.test(align) ? 0.5 : /Right$/.test(align) ? 1 : 0;
  const up = /^middle/.test(align) ? 0.5 : /^top/.test(align) ? 1 : /^bottom/.test(align) ? -0.2 : 0;
  const x0 = -along * w;
  // From p to the baseline: down the screen by `up` of the height.
  const base = up * px;
  if (opts.mask) {
    const m = px * 0.1;
    g.fillStyle = opts.mask;
    g.fillRect(x0 - m, base - px * 1.15 - m, w + 2 * m, px * 1.38 + 2 * m);
  }
  g.globalAlpha = 0.65;
  g.fillStyle = opts.color;
  g.textAlign = 'left';
  g.textBaseline = 'alphabetic';
  // The width factor stretches the letters along, the slant leans them (docs/adr/0145, 0183 §2).
  g.translate(x0, base);
  g.transform(factor, 0, -lean, 1, 0, 0);
  g.fillText(t.text, 0, 0);
  g.restore();
}

/** Small tag next to the cursor: "23.412 m  ∠ 32.4°". */
export function drawTag(g: CanvasRenderingContext2D, at: Vec2, lines: string[], color: string, bg: string): void {
  if (!lines.length) return;
  g.save();
  g.font = '500 11px Barlow, system-ui, sans-serif';
  const w = Math.max(...lines.map((l) => g.measureText(l).width)) + 12;
  const h = lines.length * 14 + 6;
  // Beside the cursor, on its other side where the drawing's edge would cut the tag off.
  const canvas = g.canvas as HTMLCanvasElement | undefined;
  const room = { w: canvas?.clientWidth, h: canvas?.clientHeight };
  let x = Math.round(at.x + 16);
  let y = Math.round(at.y + 16);
  if (room.w && x + w > room.w - 2) x = Math.max(2, Math.round(at.x - 16 - w));
  if (room.h && y + h > room.h - 2) y = Math.max(2, Math.round(at.y - 16 - h));
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
  if (geom.kind === 'polygon') {
    for (const h of geom.holes ?? []) strokePath(g, view, polygonRing(h), { ...opts, closed: true });
    // The other parts of a multi-part area (docs/adr/0143), each with its holes.
    for (const part of geom.parts ?? []) {
      strokePath(g, view, polygonRing(part), { ...opts, closed: true });
      for (const h of part.holes ?? []) strokePath(g, view, polygonRing(h), { ...opts, closed: true });
    }
  }
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

/** The window selection's blue (what lies fully inside); a crossing takes the snap colour. */
const WINDOW_COLOR = '#6DB3F2';

/**
 * Selection box between two screen points: left→right is a window (solid,
 * blue: fully inside), right→left a crossing (dashed, snap colour: touching).
 */
export function drawSelectionBox(g: CanvasRenderingContext2D, a: Vec2, b: Vec2, crossingColor: string): void {
  const crossing = b.x < a.x;
  const color = crossing ? crossingColor : WINDOW_COLOR;
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

/**
 * The circle of Daireyle seç: dashed, its inside lightly filled, in the window's blue when it takes what
 * lies wholly inside and in `crossingColor` when it takes what it touches too, as the selection box is.
 */
export function drawSelectionCircle(g: CanvasRenderingContext2D, view: ViewTransform, c: Vec2, r: number, crossing: boolean, crossingColor: string): void {
  const s = view.worldToScreen(c);
  const color = crossing ? crossingColor : WINDOW_COLOR;
  g.save();
  g.beginPath();
  g.arc(s.x, s.y, r * view.scale, 0, Math.PI * 2);
  g.fillStyle = color;
  g.globalAlpha = 0.1;
  g.fill();
  g.globalAlpha = 1;
  g.strokeStyle = color;
  g.lineWidth = 1.5;
  g.setLineDash([5, 4]);
  g.stroke();
  g.restore();
}

/**
 * The polygon of Çokgenle seç (docs/adr/0187 §2), closed back to its first corner: dashed, as the selection box and
 * circle are, its inside lightly filled in the window's blue for İçindekiler and in `crossingColor` for Kesişenler;
 * Dışındakiler leaves it unfilled.
 */
export function drawSelectionPolygon(g: CanvasRenderingContext2D, view: ViewTransform, pts: readonly Vec2[], mode: 'inside' | 'crossing' | 'outside', crossingColor: string): void {
  if (pts.length < 2) return;
  const color = mode === 'crossing' ? crossingColor : WINDOW_COLOR;
  g.save();
  g.beginPath();
  pts.forEach((p, i) => {
    const s = view.worldToScreen(p);
    if (i === 0) g.moveTo(s.x, s.y);
    else g.lineTo(s.x, s.y);
  });
  g.closePath();
  if (mode !== 'outside') {
    g.fillStyle = color;
    g.globalAlpha = 0.1;
    g.fill();
    g.globalAlpha = 1;
  }
  g.strokeStyle = color;
  g.lineWidth = 1.5;
  g.setLineDash([5, 4]);
  g.stroke();
  g.restore();
}

/** A palette colour (#rrggbb) at the given opacity, for translucent preview fills. */
export function tint(color: string, alpha: number): string {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color.trim());
  return m ? `rgba(${parseInt(m[1], 16)}, ${parseInt(m[2], 16)}, ${parseInt(m[3], 16)}, ${alpha})` : color;
}

/** A dimension's lines as it would be, a look's filled arrowheads and dots outlined (docs/adr/0183 §3). */
export function strokeLayout(g: CanvasRenderingContext2D, view: ViewTransform, l: DimensionLayout, color: string): void {
  for (const [p, q] of l.lines) strokePath(g, view, [p, q], { color });
  for (const ring of l.fills ?? []) strokePath(g, view, [...ring, ring[0]], { color });
}
