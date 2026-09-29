import { dist, type Vec2 } from '../model/geometry';
import { angleAt } from '../model/ops/construct';
import type { ViewTransform } from '../viewport/Camera';
import { ringMark } from './constructPreview';
import { PointInputTool } from './drawTools';
import { drawTag, strokeGeometry, strokePath } from './preview';

const SAME = 1e-9;

/**
 * Açı ölç (docs/adr/0140): the angle at a vertex between two arms, three clicks or typed
 * points: the vertex, the first arm, the second. The angle and its explement are drawn
 * live (the arc of the angle solid, the explement dashed beyond it) and written in the
 * project's angle unit; the result goes to the log and the tool asks again. It writes
 * nothing to the drawing. Esc (or Geri) steps back a point; Enter starts over. The
 * angle is the shared core's (`angleAt`).
 */
export class MeasureAngleTool extends PointInputTool {
  readonly id = 'measureAngle';
  protected readonly label = 'Açı ölç';
  /** The angle measured last, kept on the drawing until the next measuring begins. */
  private done: { v: Vec2; p1: Vec2; p2: Vec2 } | null = null;

  protected promptFor(n: number): string {
    const back = n ? ' [Geri (G)]' : '';
    if (n === 0) return 'açının tepe noktasını belirtin';
    if (n === 1) return `birinci kolun bir noktasını belirtin${back}`;
    return `ikinci kolun bir noktasını belirtin${back}`;
  }

  protected onPoint(p: Vec2): void {
    const vertex = this.pts[0];
    if (vertex && dist(vertex, p) < SAME) return void this.ctx.log.warn(`Nokta tepe noktasıyla çakışıyor; ${this.pts.length === 1 ? 'birinci' : 'ikinci'} kolun başka bir yerini gösterin.`);
    this.done = null;
    this.pts.push(p);
    if (this.pts.length < 3) return;
    const [v, p1, p2] = this.pts;
    const a = angleAt(v, p1, p2);
    if (a) {
      const f = this.ctx.format;
      this.ctx.log.success(`Açı ${f.angle(a.inner)}; dış açı ${f.angle(a.outer)}.`);
      this.done = { v, p1, p2 };
    } else this.ctx.log.warn('Kollardan biri tepe noktasıyla çakışıyor; açı ölçülemedi.');
    this.reset();
  }

  protected override option(key: string): boolean {
    if (key !== 'G' || !this.pts.length) return false;
    return this.cancel();
  }

  override confirm(): void {
    if (!this.pts.length) return this.ctx.tools.exit();
    this.reset();
  }

  cancel(): boolean {
    if (!this.pts.length) {
      // The measurement left on the drawing goes first, then the tool.
      if (!this.done) return false;
      this.done = null;
      this.ctx.view.requestOverlay();
      return true;
    }
    this.pts.pop();
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.pts.length && this.done) return this.drawDone(g, view, this.done);
    const v = this.pts[0];
    const hover = this.hover;
    if (!v || !hover) return;
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const p1 = this.pts[1] ?? hover;
    const p2 = this.pts[1] ? hover : null;
    ringMark(g, view, v, pal.snap);
    strokePath(g, view, [v, p1], { color: pal.accent, width: 1.5 });
    if (p2) strokePath(g, view, [v, p2], { color: pal.accent, width: 1.5 });
    if (p2) this.drawAngle(g, view, v, p1, p2, hover);
    else drawTag(g, view.worldToScreen(hover), [`Kol ${f.length(dist(v, p1))}`], pal.accent, pal.labelHalo);
    this.drawTracking(g, view);
  }

  /** The angle measured last: the arms, the arcs and the values beside the vertex. */
  private drawDone(g: CanvasRenderingContext2D, view: ViewTransform, m: { v: Vec2; p1: Vec2; p2: Vec2 }): void {
    const pal = this.ctx.view.palette;
    strokePath(g, view, [m.v, m.p1], { color: pal.snap, width: 1.5 });
    strokePath(g, view, [m.v, m.p2], { color: pal.snap, width: 1.5 });
    ringMark(g, view, m.v, pal.snap);
    this.drawAngle(g, view, m.v, m.p1, m.p2, m.v);
  }

  /** The angle's arc, the explement's dashed beyond it, and the tag with both values. */
  private drawAngle(g: CanvasRenderingContext2D, view: ViewTransform, v: Vec2, p1: Vec2, p2: Vec2, hover: Vec2): void {
    const a = angleAt(v, p1, p2);
    const pal = this.ctx.view.palette;
    if (!a) return;
    const f = this.ctx.format;
    const arm = Math.min(dist(v, p1), dist(v, p2));
    const t1 = Math.atan2(p1.y - v.y, p1.x - v.x);
    const t2 = Math.atan2(p2.y - v.y, p2.x - v.x);
    // The inner angle runs counter-clockwise from one arm to the other, whichever is first.
    const [from, to] = a.sweep <= Math.PI ? [t1, t2] : [t2, t1];
    strokeGeometry(g, view, { kind: 'arc', c: v, r: arm * 0.35, a0: from, a1: to }, { color: pal.accent, width: 2 });
    strokeGeometry(g, view, { kind: 'arc', c: v, r: arm * 0.5, a0: to, a1: from }, { color: pal.snap, dash: [4, 3], width: 1.25 });
    drawTag(g, view.worldToScreen(hover), [`Açı ${f.angle(a.inner)}`, `Dış açı ${f.angle(a.outer)}`], pal.accent, pal.labelHalo);
  }
}
