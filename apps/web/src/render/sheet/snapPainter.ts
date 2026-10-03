import type { GapBadge } from '../../contracts/generated/sheet/GapBadge';
import type { Guide } from '../../contracts/generated/sheet/Guide';
import type { RectUm } from '../../contracts/generated/sheet/RectUm';
import type { SnapLine } from '../../contracts/generated/sheet/SnapLine';
import type { SpacingMark } from '../../contracts/generated/sheet/SpacingMark';
import { crisp, hairRect, type PaperColors } from './paperPainter';
import type { PaperViewport } from './paperView';

/**
 * A drag's aids on the paper (docs/sheet/design.md §5): the smart guides the
 * engine snapped to, the distances to the nearest neighbours with their
 * badges (“12.5 mm”, the engine's text), the equal-spacing marks, a
 * resize's matching sizes, the frame of an item being drawn, a guide being
 * dragged and a turn's angle. The engine has worked out every line and
 * number; this only paints them, in the accent.
 */

/** Paper micrometres on the screen (CSS px). */
const sx = (v: PaperViewport, um: number) => v.x + (um / 1000) * v.scale;
const sy = (v: PaperViewport, um: number) => v.y + (um / 1000) * v.scale;

export function paintSnapLines(g: CanvasRenderingContext2D, dpr: number, view: PaperViewport, lines: readonly SnapLine[], c: PaperColors): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.save();
  g.strokeStyle = c.accent;
  g.lineWidth = 1;
  for (const l of lines) {
    g.setLineDash(l.kind === 'grid' || l.kind === 'margin' ? [3, 3] : []);
    g.beginPath();
    if (l.axis === 'x') {
      const x = crisp(sx(view, l.at), dpr);
      g.moveTo(x, sy(view, l.from));
      g.lineTo(x, sy(view, l.to));
    } else {
      const y = crisp(sy(view, l.at), dpr);
      g.moveTo(sx(view, l.from), y);
      g.lineTo(sx(view, l.to), y);
    }
    g.stroke();
  }
  g.restore();
}

/** A badge's size for a text (CSS px). */
export function badgeSize(g: CanvasRenderingContext2D, text: string, c: PaperColors): { width: number; height: number } {
  g.save();
  g.font = `600 10.5px ${c.font}`;
  const width = Math.ceil(g.measureText(text).width) + 10;
  g.restore();
  return { width, height: 16 };
}

/** A small rounded badge with a text, centred on a point (CSS px). */
export function badge(g: CanvasRenderingContext2D, at: { x: number; y: number }, text: string, c: PaperColors): void {
  const { width: w, height: h } = badgeSize(g, text, c);
  g.save();
  g.font = `600 10.5px ${c.font}`;
  g.fillStyle = c.badge;
  g.beginPath();
  g.roundRect(Math.round(at.x - w / 2), Math.round(at.y - h / 2), w, h, 4);
  g.fill();
  g.fillStyle = c.badgeText;
  g.textAlign = 'center';
  g.textBaseline = 'middle';
  g.fillText(text, Math.round(at.x), Math.round(at.y) + 0.5);
  g.restore();
}

export function paintGaps(g: CanvasRenderingContext2D, dpr: number, view: PaperViewport, gaps: readonly GapBadge[], c: PaperColors): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.save();
  g.strokeStyle = c.accent;
  g.lineWidth = 1;
  g.setLineDash([]);
  for (const gap of gaps) {
    const a = { x: sx(view, gap.from[0]), y: sy(view, gap.from[1]) };
    const b = { x: sx(view, gap.to[0]), y: sy(view, gap.to[1]) };
    g.beginPath();
    g.moveTo(a.x, a.y);
    g.lineTo(b.x, b.y);
    // End ticks across the measured line.
    const across = gap.axis === 'x' ? { x: 0, y: 4 } : { x: 4, y: 0 };
    for (const p of [a, b]) {
      g.moveTo(p.x - across.x, p.y - across.y);
      g.lineTo(p.x + across.x, p.y + across.y);
    }
    g.stroke();
    // On the line's middle; beside it when the gap is too short for the badge (it would cover the items).
    const size = badgeSize(g, gap.text, c);
    const length = Math.hypot(b.x - a.x, b.y - a.y);
    const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    const room = gap.axis === 'x' ? size.width + 8 : size.height + 8;
    const at = length >= room ? mid : gap.axis === 'x' ? { x: mid.x, y: mid.y + size.height / 2 + 7 } : { x: mid.x + size.width / 2 + 7, y: mid.y };
    if (length > 4) badge(g, at, gap.text, c);
  }
  g.restore();
}

/** Equal gaps: each one marked with the same pair of short strokes at its middle. */
export function paintSpacing(g: CanvasRenderingContext2D, dpr: number, view: PaperViewport, marks: readonly SpacingMark[], c: PaperColors): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.save();
  g.strokeStyle = c.accent;
  g.lineWidth = 1.5;
  g.setLineDash([]);
  for (const m of marks)
    for (const [p0, p1] of m.segments) {
      const a = { x: sx(view, p0[0]), y: sy(view, p0[1]) };
      const b = { x: sx(view, p1[0]), y: sy(view, p1[1]) };
      g.globalAlpha = 0.55;
      g.beginPath();
      g.moveTo(a.x, a.y);
      g.lineTo(b.x, b.y);
      g.stroke();
      g.globalAlpha = 1;
      const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
      g.beginPath();
      for (const d of [-2, 2]) {
        if (m.axis === 'x') {
          g.moveTo(mid.x + d, mid.y - 4);
          g.lineTo(mid.x + d, mid.y + 4);
        } else {
          g.moveTo(mid.x - 4, mid.y + d);
          g.lineTo(mid.x + 4, mid.y + d);
        }
      }
      g.stroke();
    }
  g.restore();
}

/** The frame of an item being drawn (µm), with its size. */
export function paintNewFrame(g: CanvasRenderingContext2D, dpr: number, view: PaperViewport, r: RectUm, c: PaperColors, size: (mm: number) => string): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  const x = sx(view, r.left);
  const y = sy(view, r.top);
  const w = (r.width / 1000) * view.scale;
  const h = (r.height / 1000) * view.scale;
  g.save();
  g.fillStyle = c.accent;
  g.globalAlpha = 0.08;
  g.fillRect(x, y, w, h);
  g.globalAlpha = 1;
  g.strokeStyle = c.accent;
  g.lineWidth = 1;
  g.setLineDash([5, 3]);
  hairRect(g, x, y, w, h, dpr);
  g.restore();
  badge(g, { x: x + w / 2, y: y + h + 14 }, `${size(r.width / 1000)} × ${size(r.height / 1000)} mm`, c);
}

/** A guide being dragged: across the whole desk; faded when it would be dropped (back over its ruler). */
export function paintGuideDrag(g: CanvasRenderingContext2D, dpr: number, view: PaperViewport, guide: { axis: Guide['axis']; at: number; remove: boolean }, area: { width: number; height: number }, c: PaperColors, size: (mm: number) => string): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.save();
  g.strokeStyle = c.window;
  g.globalAlpha = guide.remove ? 0.35 : 1;
  g.lineWidth = 1;
  g.setLineDash(guide.remove ? [4, 4] : []);
  g.beginPath();
  if (guide.axis === 'x') {
    const x = crisp(sx(view, guide.at), dpr);
    g.moveTo(x, 0);
    g.lineTo(x, area.height);
  } else {
    const y = crisp(sy(view, guide.at), dpr);
    g.moveTo(0, y);
    g.lineTo(area.width, y);
  }
  g.stroke();
  g.restore();
  if (!guide.remove) badge(g, guide.axis === 'x' ? { x: sx(view, guide.at), y: 14 } : { x: 34, y: sy(view, guide.at) }, `${size(guide.at / 1000)} mm`, c);
}
