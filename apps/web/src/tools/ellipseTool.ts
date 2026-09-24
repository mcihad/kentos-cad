import { dist, type Vec2 } from '../model/geometry';
import { normAngle } from '../model/geom/arc';
import { ellipseFromAxis, ellipseFromCenter, ellipsePoint, majorLength, paramAtPolar, tessellateEllipse, type EllipseGeom } from '../model/geom/ellipse';
import type { ViewTransform } from '../viewport/Camera';
import { ellipseParamToward, ellipseRotationHalf, midpoint } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';

const DEG = Math.PI / 180;

/**
 * AutoCAD ELLIPSE. Default: both ends of one axis, then the other half-axis
 * (distance from the centre, or D: a rotation angle, ratio = cos angle).
 * M starts from the centre; Y makes an elliptical arc — after the ellipse,
 * the start and end angles (from the major axis, counter-clockwise).
 */
export class EllipseTool extends PointInputTool {
  readonly id = 'ellipse';
  protected readonly label = 'Elips';
  private fromCenter = false;
  private arc = false;
  private rotationMode = false;
  /** Ellipse fixed while the arc's angles are being chosen. */
  private geom: EllipseGeom | null = null;
  private startT: number | null = null;

  protected promptFor(n: number): string {
    const what = this.arc ? 'Eliptik yay: ' : '';
    if (this.geom) return this.startT === null ? 'başlangıç açısını gösterin ya da yazın (büyük eksenden, derece)' : 'bitiş açısını gösterin ya da yazın';
    if (n === 0) return `${what}${this.fromCenter ? 'elipsin merkezini belirtin' : 'bir eksenin ilk ucunu belirtin'} [${this.fromCenter ? 'Eksenden (E)' : 'Merkez (M)'} / Yay (Y): ${this.arc ? 'açık' : 'kapalı'}]`;
    if (n === 1) return this.fromCenter ? 'bir eksenin ucunu belirtin' : 'eksenin diğer ucunu belirtin';
    return this.rotationMode ? 'döndürme açısını yazın (0 ile 89.4 derece arası)' : 'diğer yarı ekseni gösterin ya da uzunluğunu yazın [Döndürme (D)]';
  }

  protected override option(key: string): boolean {
    const n = this.pts.length;
    if (n === 0 && !this.geom && (key === 'M' || key === 'E')) this.fromCenter = key === 'M';
    else if (n === 0 && !this.geom && key === 'Y') this.arc = !this.arc;
    else if (n === 2 && !this.geom && key === 'D') this.rotationMode = true;
    else return false;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** Ellipse for the second axis given as a half-length. */
  private ellipseFor(otherHalf: number): EllipseGeom | null {
    const [p0, p1] = this.pts;
    return this.fromCenter ? ellipseFromCenter(p0, p1, otherHalf) : ellipseFromAxis(p0, p1, otherHalf);
  }

  private center(): Vec2 {
    const [p0, p1] = this.pts;
    return this.fromCenter ? p0 : midpoint(p0, p1);
  }

  /** Parameter at the polar angle of `p` seen from the centre, relative to the major axis. */
  private paramAt(p: Vec2): number {
    return ellipseParamToward(this.geom!, p);
  }

  protected onPoint(p: Vec2): void {
    if (this.geom) return this.angle(this.paramAt(p));
    const last = this.last;
    if (last && dist(last, p) < 1e-9) return;
    if (this.pts.length < 2) return void this.pts.push(p);
    if (this.rotationMode) return;
    this.shape(this.ellipseFor(dist(this.center(), p)));
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    const plain = n !== null && !/[,;@<]/.test(text);
    if (plain && this.geom) {
      this.angle(paramAtPolar(this.geom, n! * DEG));
      this.refreshPrompt();
      return true;
    }
    if (plain && this.pts.length === 2) {
      if (this.rotationMode) {
        if (n! < 0 || n! > 89.4) {
          this.ctx.log.warn('Döndürme açısı 0 ile 89.4 derece arasında olmalı.');
          return true;
        }
        // The other axis is the first one seen tilted by the angle: ratio = cos(angle).
        this.shape(this.ellipseFor(ellipseRotationHalf(this.pts[0], this.pts[1], this.fromCenter, n!)));
      } else if (n! > 0) this.shape(this.ellipseFor(n!));
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  /** A whole ellipse is created now; an arc waits for its angles. */
  private shape(g: EllipseGeom | null): void {
    if (!g) return this.ctx.log.warn('Eksenler sıfır uzunlukta olamaz.');
    if (this.arc) {
      this.geom = g;
      this.pts = [];
      return;
    }
    this.commit(g);
  }

  private angle(t: number): void {
    if (this.startT === null) {
      this.startT = t;
      return;
    }
    if (Math.abs(normAngle(t - this.startT)) < 1e-9) return this.ctx.log.warn('Bitiş açısı başlangıçla aynı olamaz.');
    this.commit({ ...this.geom!, t0: this.startT, t1: t });
  }

  private commit(g: EllipseGeom): void {
    const a = majorLength(g);
    if (this.create({ kind: 'ellipse', ...g })) this.ctx.log.success(`${g.t0 === g.t1 ? 'Elips' : 'Eliptik yay'} eklendi: ${this.ctx.format.length(a, false)} × ${this.ctx.format.length(a * g.ratio)} (yarı eksenler)`);
    this.reset();
  }

  protected override reset(): void {
    this.geom = null;
    this.startT = null;
    this.rotationMode = false;
    super.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const h = this.hover;
    if (!h) return;
    const f = this.ctx.format;
    if (this.geom) {
      strokePath(g, view, tessellateEllipse(this.geom), { color: pal.accent, closed: true, dash: [2, 4] });
      strokePath(g, view, [this.geom.c, h], { color: pal.accent, dash: [3, 3] });
      if (this.startT !== null) strokePath(g, view, tessellateEllipse({ ...this.geom, t0: this.startT, t1: this.paramAt(h) }), { color: pal.accent, width: 1.5 });
      else strokePath(g, view, [this.geom.c, ellipsePoint(this.geom, this.paramAt(h))], { color: pal.accent });
      return;
    }
    if (this.pts.length === 2 && !this.rotationMode) {
      const e = this.ellipseFor(dist(this.center(), h));
      strokePath(g, view, [this.center(), h], { color: pal.accent, dash: [3, 3] });
      if (e) {
        strokePath(g, view, tessellateEllipse(e), { color: pal.accent, closed: true });
        const a = majorLength(e);
        drawTag(g, view.worldToScreen(h), [`Yarı eksenler ${f.length(a, false)} × ${f.length(a * e.ratio)}`], pal.accent, pal.labelHalo);
      }
      return;
    }
    super.draw(g, view);
  }
}
