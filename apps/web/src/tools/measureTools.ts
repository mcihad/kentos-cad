import type { AppContext } from '../app/context';
import { bearingGrad, dist, type Vec2 } from '../model/geometry';
import { bulgePathLength, bulgeRingArea } from '../model/geom/bulge';
import { netArea, type Area, type Ring } from '../model/geom/region';
import { polygonCreate } from '../product/polygonCreate';
import type { ViewTransform } from '../viewport/Camera';
import { indexMark, ringMark } from './constructPreview';
import { PathTool } from './pathTool';
import { drawArea, drawTag, strokePath, tint } from './preview';
import type { ToolPointer } from './Tool';
import { VisibleFaces } from './visibleFaces';

/**
 * Mesafe ölç and Alan hesapla with their ADR 0141 options, on top of the path tool they share the
 * clicking of points with (pathTool.ts, `measureOnly`). Both write nothing to the drawing, but for
 * “Alan olarak çiz”, which draws the area last measured through `cad.polygon.create`.
 */

/** A toggle's state in the prompt: shown only when it is on (docs/adr/0140, Ötele's pattern). */
const whenOn = (on: boolean) => (on ? ': açık' : '');

/**
 * Mesafe ölç: the legs of a path, each with its length and bearing, and the total at the end.
 * With “Sabit ilk nokta” (S; kept for the session) every new point is measured from the first one
 * instead: the tag by the cursor gives the distance and bearing from it, the preview draws rays from
 * it, each click writes `n: mesafe, semt` to the log, and no total is written.
 */
export class DistanceTool extends PathTool {
  private static fixed = false;

  constructor(ctx: AppContext) {
    super(ctx, { id: 'measure', label: 'Mesafe ölç', closed: false, measureOnly: true });
  }

  private get fixed(): boolean {
    return DistanceTool.fixed;
  }

  protected override promptFor(n: number): string {
    const chip = `Sabit ilk nokta (S)${whenOn(this.fixed)}`;
    // The chip is offered before the first point and, once on, throughout: the chain's own options
    // (Yay, Uzunluk, Geri) keep their prompt as they had it.
    if (n === 0) return `ilk noktayı belirtin [${chip}]`;
    if (!this.fixed) return super.promptFor(n);
    return `sonraki noktayı belirtin [${chip} / Geri (G)${n >= 2 ? ' / Bitir (Enter)' : ''}]`;
  }

  /** Rays are measured from the first point, so that is what relative input, ortho and perpendicular snaps refer to. */
  protected override get last(): Vec2 | null {
    return this.fixed ? (this.pts[0] ?? null) : super.last;
  }

  protected override option(key: string): boolean {
    // Where the chip is offered: before the first point, and while it is on. A chain under way is not turned into rays.
    if (key === 'S' && (!this.pts.length || this.fixed)) {
      DistanceTool.fixed = !DistanceTool.fixed;
      // A run of one kind is not carried over into the other: it starts again, with straight legs.
      super.option('D');
      this.reset();
      return true;
    }
    if (this.fixed && key !== 'G') return false;
    return super.option(key);
  }

  protected override onPoint(p: Vec2): void {
    const first = this.pts[0];
    if (!this.fixed || !first) return super.onPoint(p);
    if (dist(first, p) <= 1e-9) return;
    this.pts.push(p);
    this.ctx.log.info(`${this.pts.length - 1}: ${this.reading(first, p)}`);
  }

  /** A reading as the tag and the log give it: the distance and the bearing, in the project's formats. */
  private reading(from: Vec2, to: Vec2): string {
    const f = this.ctx.format;
    return `${f.length(dist(from, to))}, semt ${f.bearing(bearingGrad(from, to))}`;
  }

  protected override finish(): void {
    // Rays have no total: each was written as it was measured.
    if (this.fixed && this.pts.length >= 2) return this.reset();
    super.finish();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const first = this.pts[0];
    if (!this.fixed || !first) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    ringMark(g, view, first, pal.snap);
    this.pts.slice(1).forEach((p, i) => {
      strokePath(g, view, [first, p], { color: pal.accent, width: 1.5 });
      indexMark(g, view, p, String(i + 1), pal.accent, pal.labelHalo);
    });
    const at = this.hover;
    if (at && dist(first, at) > 1e-9) {
      strokePath(g, view, [first, at], { color: pal.accent, dash: [4, 3] });
      drawTag(g, view.worldToScreen(at), [f.length(dist(first, at)), `Semt ${f.bearing(bearingGrad(first, at))}`], pal.accent, pal.labelHalo);
    }
    this.drawTracking(g, view);
  }
}

/**
 * A region's perimeter as İçine tıkla says it: its outer ring's and its islands', as a polygon's
 * (holes included, as in GIS): the region drawn with Alan olarak çiz shows the same in Öznitelikler.
 */
function regionPerimeter(area: Area): number {
  return [area.outer, ...area.holes].reduce((sum, r) => sum + bulgePathLength(r.pts, r.bulges, true), 0);
}

/** An area as measured: its outer ring, its holes, and its net area (m²). */
interface Measured {
  ring: Ring;
  holes: Ring[];
  area: number;
}

/**
 * Alan hesapla: the area of clicked corners, with its perimeter. Options (docs/adr/0141):
 * “İçine tıkla” (I; kept for the session) finds the region around a click as İçine tıklayarak alan
 * does (a closed object or the face of the visible lines, the islands inside it as holes) and
 * reports it the same way; “Alan olarak çiz” (A), offered once something was measured, writes the
 * area last measured, holes and all, to the active layer through `cad.polygon.create` as one undo
 * step of that name. The chip stays until the next measurement starts or the tool ends.
 */
export class AreaMeasureTool extends PathTool {
  private static inside = false;
  private readonly faces: VisibleFaces;
  /** The region under the cursor in İçine tıkla. */
  private candidate: { area: Area | null } | null = null;
  /** The area measured last, kept on the drawing and behind the chip until the next measurement. */
  private measured: Measured | null = null;

  constructor(ctx: AppContext) {
    super(ctx, { id: 'area', label: 'Alan hesapla', closed: true, measureOnly: true });
    this.faces = new VisibleFaces(ctx);
  }

  private get inside(): boolean {
    return AreaMeasureTool.inside;
  }

  override activate(): void {
    this.faces.attach();
    super.activate();
  }

  deactivate(): void {
    this.faces.detach();
  }

  /** Regions are found by clicking inside them, wherever the cursor is: nothing to snap to. */
  override get snaps(): boolean {
    return !this.inside;
  }

  protected override promptFor(n: number): string {
    if (n > 0) return super.promptFor(n);
    const chips = [`İçine tıkla (I)${whenOn(this.inside)}`, ...(this.measured ? ['Alan olarak çiz (A)'] : [])];
    return `${this.inside ? 'alanı ölçülecek bölgenin içine tıklayın' : 'ilk noktayı belirtin'} [${chips.join(' / ')}]`;
  }

  protected override option(key: string): boolean {
    // Both letters are options of the arc's stage too (İkinci nokta, Açı): only before the first point are they these.
    if (!this.pts.length && (key === 'I' || key === 'İ')) {
      AreaMeasureTool.inside = !AreaMeasureTool.inside;
      this.candidate = null;
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    if (!this.pts.length && key === 'A' && this.measured) {
      this.drawMeasured(this.measured);
      return true;
    }
    return super.option(key);
  }

  override pointerMove(p: ToolPointer): void {
    super.pointerMove(p);
    if (this.inside) this.candidate = { area: this.faces.at(p.raw, true) };
  }

  protected override onPoint(p: Vec2): void {
    if (this.inside) return this.measureInside(p);
    // A first corner starts a new measurement: the last one is no longer offered.
    if (!this.pts.length) this.measured = null;
    super.onPoint(p);
  }

  /** A click in İçine tıkla: the region around it, its area and perimeter as any measurement gives them. */
  private measureInside(p: Vec2): void {
    const area = this.faces.at(p, true);
    if (!area) return void this.ctx.log.warn('Tıklanan noktayı çevreleyen kapalı bölge yok.');
    const f = this.ctx.format;
    const net = netArea(area);
    this.ctx.log.success(`Alan ${f.area(net)}   Çevre ${f.length(regionPerimeter(area))}`);
    this.measured = { ring: area.outer, holes: area.holes, area: net };
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  protected override finish(): void {
    const ring = this.pts.length >= 3 ? { pts: [...this.pts], bulges: this.fullBulges() } : null;
    super.finish();
    if (!ring) return;
    this.measured = { ring, holes: [], area: Math.abs(bulgeRingArea(ring.pts, ring.bulges)) };
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  /**
   * Alan olarak çiz: the area last measured on the active layer, in the current colour and line
   * weight, through `cad.polygon.create`; one undo step, “Alan olarak çiz”. What the command refuses
   * (a locked layer) it says in its own words, and nothing is written.
   */
  private drawMeasured(m: Measured): void {
    const { doc, format, log } = this.ctx;
    const { ring, holes } = m;
    const input = {
      layerId: doc.layers.active.value,
      pts: ring.pts,
      ...(ring.bulges && { bulges: ring.bulges }),
      ...(holes.length > 0 && { holes }),
      ...this.colour(),
      ...this.weight(),
    };
    const out = this.written(doc.transact('Alan olarak çiz', () => polygonCreate.execute({ doc }, input)));
    if (out) log.success(`Alan olarak çizildi: ${format.area(m.area)}.`);
    this.ctx.view.requestOverlay();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const m = this.measured;
    // What “Alan olarak çiz” would write, until the next measurement.
    if (m) drawArea(g, view, { outer: m.ring, holes: m.holes }, { color: pal.snap, fill: tint(pal.snap, 0.14), dash: [5, 4], width: 1.5 });
    if (!this.inside) return super.draw(g, view);
    const area = this.candidate?.area;
    const at = this.hover;
    if (!area || !at) return;
    drawArea(g, view, area, { color: pal.accent, fill: tint(pal.accent, 0.16), width: 2 });
    const lines = [`Alan ${f.area(netArea(area))}`, `Çevre ${f.length(regionPerimeter(area))}`];
    if (area.holes.length) lines.push(`${area.holes.length} ada`);
    drawTag(g, view.worldToScreen(at), lines, pal.accent, pal.labelHalo);
  }
}
