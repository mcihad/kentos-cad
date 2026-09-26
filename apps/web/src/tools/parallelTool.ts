import type { EntityGeometry } from '../model/entities';
import { bearingGrad, dist, type Vec2 } from '../model/geometry';
import { cleanAxis, corridorArea, parallelSides } from '../model/geom/parallel';
import { netArea } from '../model/geom/region';
import { polygonOfArea } from '../model/ops/areas';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawArea, drawTag, strokePath, tint } from './preview';

type Side = 'left' | 'right';

/**
 * Paralel çizgi (Netcad): the axis is drawn point by point while lines
 * appear at the left and right distances (road edges, kerbs, walls). The
 * axis itself can be kept or left out, and "Alan olarak" writes the
 * corridor between the sides as one area (a closed axis: a ring with a
 * hole). Distances are typed or shown with two clicks.
 */
export class ParallelLineTool extends PointInputTool {
  readonly id = 'parallel';
  protected readonly label = 'Paralel çizgi';
  private static left = 5;
  private static right = 5;
  private static axis = true;
  private static asArea = false;
  /** Side whose distance is being asked for, and the first point when shown with two clicks. */
  private ask: Side | null = null;
  private measureFrom: Vec2 | null = null;

  protected override get last(): Vec2 | null {
    return this.ask ? this.measureFrom : (this.pts.at(-1) ?? null);
  }

  private settings(): string {
    const f = this.ctx.format;
    const S = ParallelLineTool;
    return `Sol (S): ${f.length(S.left)} / Sağ (A): ${f.length(S.right)} / Eksen (E): ${S.axis ? 'çizilir' : 'çizilmez'} / Alan olarak (U): ${S.asArea ? 'evet' : 'hayır'}`;
  }

  protected promptFor(n: number): string {
    if (this.ask) {
      const side = this.ask === 'left' ? 'sol' : 'sağ';
      return this.measureFrom ? `${side} mesafenin ikinci noktasını gösterin` : `${side} mesafeyi yazın ya da iki noktayla gösterin`;
    }
    if (n === 0) return `eksenin ilk noktasını gösterin [${this.settings()}]`;
    const more = n >= 2 ? ' ya da bitirmek için sağ tıklayın' : '';
    return `eksenin sonraki noktasını gösterin${more} [Geri (G)${n >= 3 ? ' / Kapat (K)' : ''} / ${this.settings()}]`;
  }

  protected override option(key: string): boolean {
    const S = ParallelLineTool;
    if (key === 'S' || key === 'A') {
      this.ask = key === 'S' ? 'left' : 'right';
      this.measureFrom = null;
    } else if (key === 'E') S.axis = !S.axis;
    else if (key === 'U') S.asArea = !S.asArea;
    else if (key === 'G' && this.pts.length && !this.ask) this.pts.pop();
    else if (key === 'K' && this.pts.length >= 3 && !this.ask) {
      this.commit(true);
      return true;
    } else return false;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    if (this.ask && n !== null && !/[,;@<]/.test(text)) {
      if (n < 0) {
        this.ctx.log.warn('Mesafe sıfır ya da pozitif olmalı; karşı taraf için diğer seçeneği kullanın.');
        return true;
      }
      this.setDistance(n);
      return true;
    }
    return super.input(text);
  }

  private setDistance(d: number): void {
    if (this.ask === 'left') ParallelLineTool.left = d;
    else ParallelLineTool.right = d;
    this.ctx.log.info(`  ${this.ask === 'left' ? 'Sol' : 'Sağ'} mesafe: ${this.ctx.format.length(d)}`);
    this.ask = null;
    this.measureFrom = null;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  protected onPoint(p: Vec2): void {
    if (this.ask) {
      if (!this.measureFrom) this.measureFrom = p;
      else this.setDistance(dist(this.measureFrom, p));
      return;
    }
    this.pts.push(p);
  }

  override confirm(): void {
    if (this.ask) {
      // Leaving the distance prompt keeps the old value.
      this.ask = null;
      this.measureFrom = null;
      this.refreshPrompt();
      return;
    }
    super.confirm();
  }

  protected override finish(): void {
    if (cleanAxis(this.pts, false).length >= 2) this.commit(false);
    else this.reset();
  }

  /**
   * The sides (or the corridor) and the axis, written through
   * `cad.entities.create` (docs/adr/0057) as one undo step, “Paralel çizgi”:
   * all of them, or none with the command's reason.
   */
  private commit(closed: boolean): void {
    const S = ParallelLineTool;
    const axis = cleanAxis(this.pts, closed);
    if (axis.length < (closed ? 3 : 2)) return this.reset();
    const { format, log } = this.ctx;
    const kind = closed ? 'polygon' : 'polyline';
    const objects: EntityGeometry[] = [];
    let summary = '';
    if (S.asArea) {
      const area = corridorArea(axis, S.left, S.right, closed);
      if (area) {
        objects.push(polygonOfArea(area));
        summary = `alan ${format.area(netArea(area))}`;
      }
    } else {
      const sides = parallelSides(axis, S.left, S.right, closed);
      for (const side of [sides.left, sides.right]) if (side) objects.push({ kind, pts: side });
    }
    if (S.axis) objects.push({ kind, pts: axis });
    if (!objects.length) {
      if (!S.axis && !(S.left > 0) && !(S.right > 0)) log.warn('Sol ve sağ mesafe sıfır ve eksen çizilmiyor: çizilecek bir şey yok.');
    } else if (this.writeObjects(objects, 'parallel')) {
      const width = format.length(S.left + S.right);
      log.success(`Paralel çizgi: ${objects.length} nesne, genişlik ${width}${summary ? `, ${summary}` : ''}.`);
    }
    this.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    if (this.ask) {
      if (this.measureFrom && this.hover) {
        strokePath(g, view, [this.measureFrom, this.hover], { color: pal.accent, dash: [4, 3] });
        drawTag(g, view.worldToScreen(this.hover), [f.length(dist(this.measureFrom, this.hover))], pal.accent, pal.labelHalo);
      }
      this.drawTracking(g, view);
      return;
    }
    const chain = this.hover ? [...this.pts, this.hover] : this.pts;
    if (chain.length < 2) return this.drawTracking(g, view);
    const S = ParallelLineTool;
    if (S.asArea) {
      const area = corridorArea(chain, S.left, S.right, false);
      if (area) drawArea(g, view, area, { color: pal.accent, fill: tint(pal.accent, 0.16), width: 1.5 });
    } else {
      const sides = parallelSides(chain, S.left, S.right, false);
      for (const side of [sides.left, sides.right]) if (side) strokePath(g, view, side, { color: pal.accent, width: 1.5 });
    }
    // The axis: solid when it will be drawn, dashed when it only guides.
    strokePath(g, view, chain, { color: pal.accent, dash: S.axis ? undefined : [6, 4] });
    const last = this.pts.at(-1);
    if (last && this.hover) drawTag(g, view.worldToScreen(this.hover), [f.length(dist(last, this.hover)), `Semt ${f.bearing(bearingGrad(last, this.hover))}`], pal.accent, pal.labelHalo);
    this.drawTracking(g, view);
  }
}
