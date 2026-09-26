import { angleDeg, type Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { unitToward, xlineDirection } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';

/** Preview of an infinite line: long enough to cross any view. */
function strokeInfinite(g: CanvasRenderingContext2D, view: ViewTransform, p: Vec2, dir: Vec2, ray: boolean, color: string): void {
  const r = (Math.hypot(view.width, view.height) / view.scale) * 4;
  strokePath(g, view, [ray ? p : { x: p.x - dir.x * r, y: p.y - dir.y * r }, { x: p.x + dir.x * r, y: p.y + dir.y * r }], { color, dash: [6, 4] });
}

type XMode = 'point' | 'horizontal' | 'vertical' | 'angle' | 'bisect';

/**
 * AutoCAD XLINE: infinite construction lines through a point, as many as
 * you click. Y horizontal, D vertical, A a typed angle, B the bisector of
 * an angle (vertex, then a point on each arm).
 */
export class XlineTool extends PointInputTool {
  readonly id = 'xline';
  protected readonly label = 'Yardımcı çizgi';
  private mode: XMode = 'point';
  private static angle = 0;
  private askAngle = false;

  protected promptFor(n: number): string {
    if (this.askAngle) return 'açıyı yazın (derece, doğudan saat yönünün tersine)';
    switch (this.mode) {
      case 'horizontal':
      case 'vertical':
        return `geçeceği noktayı belirtin (${this.mode === 'horizontal' ? 'yatay' : 'düşey'}) [Bitir (Enter)]`;
      case 'angle':
        return `geçeceği noktayı belirtin (${+XlineTool.angle.toFixed(4)}°) [Bitir (Enter)]`;
      case 'bisect':
        return n === 0 ? 'açının köşesini belirtin' : n === 1 ? 'açının başlangıç kolunda bir nokta belirtin' : 'açının bitiş kolunda bir nokta belirtin [Bitir (Enter)]';
      default:
        return n === 0 ? 'bir nokta belirtin [Yatay (Y) / Düşey (D) / Açı (A) / Açıortay (B)]' : 'geçeceği noktayı belirtin [Bitir (Enter)]';
    }
  }

  protected override option(key: string): boolean {
    if (this.pts.length) return false;
    const modes: Record<string, XMode> = { Y: 'horizontal', D: 'vertical', A: 'angle', B: 'bisect' };
    if (!modes[key]) return false;
    this.mode = modes[key];
    this.askAngle = key === 'A';
    this.refreshPrompt();
    return true;
  }

  override input(text: string): boolean {
    if (this.askAngle) {
      const n = parseNumber(text);
      if (n === null) return false;
      XlineTool.angle = n;
      this.askAngle = false;
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  /** Direction of the line through `p` in the current mode (null: not enough input yet); a bisector is square to opposite arms. */
  private dirFor(p: Vec2): Vec2 | null {
    return xlineDirection(this.mode, this.pts, p, XlineTool.angle);
  }

  /** Base point of the line to create for a click at p. */
  private baseFor(p: Vec2): Vec2 {
    return this.mode === 'point' || this.mode === 'bisect' ? this.pts[0] : p;
  }

  protected onPoint(p: Vec2): void {
    const needsBase = this.mode === 'point' || this.mode === 'bisect';
    if (needsBase && this.pts.length === 0) return void this.pts.push(p);
    if (this.mode === 'bisect' && this.pts.length === 1) return void this.pts.push(p);
    const dir = this.dirFor(p);
    if (!dir) return;
    // Through `cad.entities.create` (docs/adr/0057): each line its own object and undo step, “Ekle”.
    if (this.writeObjects([{ kind: 'xline', p: this.baseFor(p), dir }])) this.ctx.log.success('Yardımcı çizgi eklendi.');
  }

  protected override reset(): void {
    this.mode = 'point';
    this.askAngle = false;
    super.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const h = this.hover;
    if (!h) return;
    const pal = this.ctx.view.palette;
    if (this.mode === 'bisect' && this.pts.length === 1) strokePath(g, view, [this.pts[0], h], { color: pal.accent, dash: [3, 3] });
    const dir = this.dirFor(h);
    const needsBase = this.mode === 'point' || this.mode === 'bisect';
    if (dir && (!needsBase || this.pts.length)) {
      strokeInfinite(g, view, this.baseFor(h), dir, false, pal.accent);
      // A line has no sense of direction: show its angle in 0–180°.
      const a = ((angleDeg({ x: 0, y: 0 }, dir) % 180) + 180) % 180;
      drawTag(g, view.worldToScreen(h), [`Açı ${a.toFixed(2)}°`], pal.accent, pal.labelHalo);
    }
    this.drawTracking(g, view);
  }
}

/** AutoCAD RAY: from one start point, a ray towards every clicked point. */
export class RayTool extends PointInputTool {
  readonly id = 'ray';
  protected readonly label = 'Işın';

  protected promptFor(n: number): string {
    return n === 0 ? 'başlangıç noktasını belirtin' : 'geçeceği noktayı belirtin [Bitir (Enter)]';
  }

  protected onPoint(p: Vec2): void {
    if (!this.pts.length) return void this.pts.push(p);
    const dir = unitToward(this.pts[0], p);
    if (dir && this.writeObjects([{ kind: 'ray', p: this.pts[0], dir }])) this.ctx.log.success('Işın eklendi.');
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const h = this.hover;
    if (!h || !this.pts.length) return super.draw(g, view);
    const dir = unitToward(this.pts[0], h);
    if (dir) strokeInfinite(g, view, this.pts[0], dir, true, this.ctx.view.palette.accent);
    this.drawTracking(g, view);
  }
}
