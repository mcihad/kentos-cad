import type { AppContext } from '../app/context';
import type { Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';

/** Preview marks of the construction tools (docs/adr/0140): rings, plus signs, dots, and how far a line is drawn. */

/** A ring around a point (a point given). */
export function ringMark(g: CanvasRenderingContext2D, view: ViewTransform, p: Vec2, color: string, radius = 5): void {
  const s = view.worldToScreen(p);
  g.save();
  g.strokeStyle = color;
  g.lineWidth = 1.75;
  g.setLineDash([]);
  g.beginPath();
  g.arc(s.x, s.y, radius, 0, Math.PI * 2);
  g.stroke();
  g.restore();
}

/** A plus sign (the meeting point that will be taken). */
export function plusMark(g: CanvasRenderingContext2D, view: ViewTransform, p: Vec2, color: string, size = 7): void {
  const s = view.worldToScreen(p);
  g.save();
  g.strokeStyle = color;
  g.lineWidth = 2;
  g.setLineDash([]);
  g.beginPath();
  g.moveTo(s.x - size, s.y);
  g.lineTo(s.x + size, s.y);
  g.moveTo(s.x, s.y - size);
  g.lineTo(s.x, s.y + size);
  g.stroke();
  g.restore();
}

/** A small filled dot with a halo (a point about to be placed). */
export function dotMark(g: CanvasRenderingContext2D, view: ViewTransform, p: Vec2, color: string, halo: string, radius = 3.5): void {
  const s = view.worldToScreen(p);
  g.save();
  g.setLineDash([]);
  g.beginPath();
  g.arc(s.x, s.y, radius, 0, Math.PI * 2);
  g.fillStyle = color;
  g.strokeStyle = halo;
  g.lineWidth = 2;
  g.stroke();
  g.fill();
  g.restore();
}

/** How far an unbounded line is drawn each way: twice the view's diagonal, so it always leaves the screen. */
export function lineReach(ctx: AppContext): number {
  const b = (ctx.view as { camera?: { visibleBounds(): { minX: number; minY: number; maxX: number; maxY: number } } }).camera?.visibleBounds();
  return b ? 2 * Math.hypot(b.maxX - b.minX, b.maxY - b.minY) : 1000;
}
