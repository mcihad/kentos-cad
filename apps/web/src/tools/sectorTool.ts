import { dist, type Vec2 } from '../model/geometry';
import { sectorRing } from '../model/ops/construct';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawArea, drawTag, strokeGeometry, strokePath, tint } from './preview';
import { ringMark } from './constructPreview';

/** What the tool waits for next. */
type Stage = 'centre' | 'start' | 'startAngle' | 'end';

const SAME = 1e-9;
const TAU = Math.PI * 2;
/** An angle turned into 0 to a whole turn. */
const turn = (a: number) => ((a % TAU) + TAU) % TAU;

/**
 * Daire dilimi (docs/adr/0140): a closed pie slice on the point-input base like the
 * other drawing tools.
 *
 * - the centre, clicked or typed;
 * - the start: a point (the radius is its distance from the centre, the start angle
 *   its direction), or a typed radius and then the start angle, typed or pointed at;
 * - the end: a direction pointed at, or an angle typed. The arc is swept
 *   counter-clockwise from the start to the end.
 *
 * Typed angles are in the project's angle unit and count counter-clockwise from
 * east, as the arc tool's do. Esc steps back (the start, then the centre); Enter
 * or right click starts the slice over, or leaves when none is begun. The ring
 * (centre, the arc's two ends, the arc as the middle edge) is the shared core's;
 * it is written through `cad.polygon.create` with its bulges, one undo step.
 */
export class SectorTool extends PointInputTool {
  readonly id = 'sector';
  protected readonly label = 'Daire dilimi';
  /** The radius (metres) and the start angle (radians from east, counter-clockwise), once known. */
  private radius: number | null = null;
  private start: number | null = null;

  private get stage(): Stage {
    if (!this.pts.length) return 'centre';
    if (this.start !== null) return 'end';
    return this.radius !== null ? 'startAngle' : 'start';
  }

  protected promptFor(): string {
    const f = this.ctx.format;
    const back = this.pts.length ? ' [Geri (G)]' : '';
    switch (this.stage) {
      case 'centre':
        return 'dilimin merkezini belirtin';
      case 'start':
        return `başlangıç noktasını gösterin ya da yarıçapı yazın${back}`;
      case 'startAngle':
        return `başlangıç açısını yazın (${f.angleUnitName}, doğudan saat yönünün tersine) ya da yönünü gösterin [yarıçap ${f.length(this.radius!)}; Geri (G)]`;
      default:
        return `bitiş doğrultusunu gösterin ya da bitiş açısını yazın (${f.angleUnitName})${back}`;
    }
  }

  protected onPoint(p: Vec2): void {
    const c = this.pts[0];
    if (!c) {
      this.pts.push(p);
      return;
    }
    if (dist(c, p) < SAME) return void this.ctx.log.warn('Nokta merkezle çakışıyor; merkezden farklı bir yer gösterin.');
    const direction = Math.atan2(p.y - c.y, p.x - c.x);
    switch (this.stage) {
      case 'start':
        this.radius = dist(c, p);
        this.start = direction;
        break;
      case 'startAngle':
        this.start = direction;
        break;
      default:
        this.write(direction);
    }
  }

  /** The slice from the start to `end` (radians), written; a sweep of nothing is said and the end asked again. */
  private write(end: number): void {
    const c = this.pts[0];
    if (!c || this.radius === null || this.start === null) return;
    const ring = sectorRing(c, this.radius, this.start, end);
    if (!ring) return void this.ctx.log.warn('Bitiş doğrultusu başlangıçla aynı; dilim oluşmuyor. Başka bir doğrultu ya da açı verin.');
    const f = this.ctx.format;
    const sweep = turn(end - this.start);
    if (this.writeRing(ring.pts, ring.bulges)) this.ctx.log.success(`Daire dilimi eklendi: r = ${f.length(this.radius)}, açı ${f.angle(sweep)}`);
    this.reset();
  }

  protected override reset(): void {
    this.radius = null;
    this.start = null;
    super.reset();
  }

  /** Back one stage; false when nothing was begun. */
  private back(): boolean {
    if (this.start !== null || this.radius !== null) {
      this.start = null;
      this.radius = null;
    } else if (this.pts.length) this.pts = [];
    else return false;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  protected override option(key: string): boolean {
    return key === 'G' && this.back();
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = this.stage !== 'centre' && !/[,;@<]/.test(text) ? parseNumber(text) : null;
    if (n === null) return super.input(text);
    const { log, format } = this.ctx;
    switch (this.stage) {
      case 'start':
        if (!(n > 0)) log.warn('Yarıçap sıfırdan büyük olmalı.');
        else this.radius = n;
        break;
      case 'startAngle':
        this.start = format.angleFromTyped(n);
        break;
      default:
        this.write(format.angleFromTyped(n));
    }
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** With a slice begun it starts over; with none, the tool leaves. */
  override confirm(): void {
    if (!this.pts.length) return this.ctx.tools.exit();
    this.reset();
  }

  cancel(): boolean {
    return this.back();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    this.drawSlice(g, view);
    this.drawTracking(g, view);
  }

  private drawSlice(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const c = this.pts[0];
    const hover = this.hover;
    if (!c || !hover) return;
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const direction = Math.atan2(hover.y - c.y, hover.x - c.x);
    ringMark(g, view, c, pal.snap);
    if (this.stage === 'end') {
      const ring = sectorRing(c, this.radius!, this.start!, direction);
      if (!ring) {
        strokePath(g, view, [c, { x: c.x + this.radius! * Math.cos(this.start!), y: c.y + this.radius! * Math.sin(this.start!) }], { color: pal.danger, width: 1.5 });
        return drawTag(g, view.worldToScreen(hover), ['Süpürme yok'], pal.danger, pal.labelHalo);
      }
      drawArea(g, view, { outer: { pts: ring.pts, ...(ring.bulges && { bulges: ring.bulges }) }, holes: [] }, { color: pal.accent, fill: tint(pal.accent, 0.2), width: 1.5 });
      return drawTag(g, view.worldToScreen(hover), [`Açı ${f.angle(turn(direction - this.start!))}`, `Yarıçap ${f.length(this.radius!)}`], pal.accent, pal.labelHalo);
    }
    // The radius and the start being chosen: the circle it makes, dashed, and the line from the centre.
    const r = this.radius ?? dist(c, hover);
    const end = { x: c.x + r * Math.cos(direction), y: c.y + r * Math.sin(direction) };
    strokeGeometry(g, view, { kind: 'circle', c, r }, { color: pal.snap, dash: [5, 3], width: 1 });
    strokePath(g, view, [c, end], { color: pal.accent, width: 1.5 });
    drawTag(g, view.worldToScreen(hover), [`Yarıçap ${f.length(r)}`, `Açı ${f.angle(turn(direction))}`], pal.accent, pal.labelHalo);
  }
}
