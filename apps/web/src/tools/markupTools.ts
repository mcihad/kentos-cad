import type { AppContext } from '../app/context';
import { dist, signedArea, type Vec2 } from '../model/geometry';
import { cloudOf } from '../model/geom/shapes';
import { bulgePathOutline } from '../model/geom/bulge';
import type { ViewTransform } from '../viewport/Camera';
import { donutRings } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawArea, strokePath, tint } from './preview';

/** Paper sizes (mm) converted to world metres at the project's plot scale. */
const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

// ── Halka ──────────────────────────────────────────────────────────────

/** AutoCAD DONUT: a filled ring (inner diameter 0 gives a filled disc) at each click. */
export class DonutTool extends PointInputTool {
  readonly id = 'donut';
  protected readonly label = 'Halka';
  private static inner = 0.5;
  private static outer = 1;
  private ask: 'inner' | 'outer' | null = null;

  protected promptFor(): string {
    const f = this.ctx.format;
    const opts = `[İç çap (İ): ${f.length(DonutTool.inner)} / Dış çap (D): ${f.length(DonutTool.outer)}]`;
    if (this.ask) return `${this.ask === 'inner' ? 'iç' : 'dış'} çapı yazın ${opts}`;
    return `halkanın merkezine tıklayın ${opts}`;
  }

  protected override option(key: string): boolean {
    if (key === 'İ' || key === 'I') this.ask = 'inner';
    else if (key === 'D') this.ask = 'outer';
    else return false;
    this.refreshPrompt();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    if (this.ask && n !== null && !/[,;@<]/.test(text)) {
      if (this.ask === 'inner') {
        if (n < 0) this.ctx.log.warn('İç çap sıfır ya da pozitif olmalı.');
        else {
          // A bigger hole keeps the ring's width: the outer diameter follows.
          if (n >= DonutTool.outer) DonutTool.outer = n + (DonutTool.outer - DonutTool.inner);
          DonutTool.inner = n;
          this.ask = null;
        }
      } else if (n <= DonutTool.inner) this.ctx.log.warn(`Dış çap iç çaptan (${this.ctx.format.length(DonutTool.inner)}) büyük olmalı.`);
      else {
        DonutTool.outer = n;
        this.ask = null;
      }
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  private shape(c: Vec2): { ring: Vec2[]; holes?: Vec2[][] } {
    return donutRings(c, DonutTool.inner, DonutTool.outer);
  }

  protected onPoint(p: Vec2): void {
    if (this.ask) return;
    // A solid fill with the inner circle left out: what AutoCAD's wide polyline looks like.
    this.create({ kind: 'hatch', ...this.shape(p), pattern: { type: 'solid', angle: 0, spacing: 1 } });
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.hover || this.ask) return;
    const s = this.shape(this.hover);
    const pal = this.ctx.view.palette;
    drawArea(g, view, { outer: { pts: s.ring }, holes: (s.holes ?? []).map((pts) => ({ pts })) }, { color: pal.accent, fill: tint(pal.accent, 0.35), width: 1 });
  }
}

// ── Revizyon bulutu ────────────────────────────────────────────────────

/** AutoCAD REVCLOUD: a scalloped closed outline around changes, drawn as a polygon or a rectangle. */
export class RevCloudTool extends PointInputTool {
  readonly id = 'revcloud';
  protected readonly label = 'Revizyon bulutu';
  private static rect = true;
  private static arcMm = 8;
  private askArc = false;

  protected promptFor(n: number): string {
    if (this.askArc) return 'yay boyunu kâğıt milimetresi olarak yazın';
    const opts = `${RevCloudTool.rect ? 'Çokgen (Ç)' : 'Dikdörtgen (D)'} / Yay boyu (U): ${RevCloudTool.arcMm} mm`;
    if (RevCloudTool.rect) return n === 0 ? `dikdörtgenin bir köşesine tıklayın [${opts}]` : 'karşı köşeye tıklayın';
    return n === 0 ? `bulutun ilk köşesine tıklayın [${opts}]` : n < 3 ? 'sonraki köşeye tıklayın' : 'sonraki köşeye tıklayın ya da bitirmek için sağ tıklayın';
  }

  protected override option(key: string): boolean {
    if (this.pts.length) return false;
    if (key === 'D') RevCloudTool.rect = true;
    else if (key === 'Ç' || key === 'C') RevCloudTool.rect = false;
    else if (key === 'U') this.askArc = true;
    else return false;
    this.refreshPrompt();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    if (this.askArc && n !== null) {
      if (n > 0) RevCloudTool.arcMm = n;
      else this.ctx.log.warn('Yay boyu sıfırdan büyük olmalı.');
      this.askArc = false;
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  private ringFor(pts: readonly Vec2[]): Vec2[] {
    if (!RevCloudTool.rect) return [...pts];
    const [a, b] = pts;
    return [a, { x: b.x, y: a.y }, b, { x: a.x, y: b.y }];
  }

  protected onPoint(p: Vec2): void {
    if (this.askArc) return;
    if (this.last && dist(this.last, p) <= 1e-9) return;
    this.pts.push(p);
    if (RevCloudTool.rect && this.pts.length === 2) this.finish();
  }

  protected override finish(): void {
    const ring = this.ringFor(this.pts);
    const cloud = ring.length >= 3 && Math.abs(signedArea(ring)) > 1e-9 ? cloudOf(ring, paper(this.ctx, RevCloudTool.arcMm)) : null;
    if (cloud && this.create({ kind: 'polygon', ...cloud })) this.ctx.log.success(`Revizyon bulutu eklendi: ${cloud.pts.length} yay.`);
    else if (this.pts.length) this.ctx.log.warn('Bulut için alanı olan bir dikdörtgen ya da en az üç köşe gerekir.');
    super.finish();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const pts = this.hover ? [...this.pts, this.hover] : this.pts;
    if (pts.length < 2) return this.drawTracking(g, view);
    const ring = this.ringFor(pts);
    const cloud = ring.length >= 3 ? cloudOf(ring, paper(this.ctx, RevCloudTool.arcMm)) : null;
    if (cloud) strokePath(g, view, bulgePathOutline(cloud.pts, cloud.bulges, true), { color: pal.accent, closed: true, width: 1.5 });
    else strokePath(g, view, pts, { color: pal.accent, dash: [4, 3] });
    this.drawTracking(g, view);
  }
}
