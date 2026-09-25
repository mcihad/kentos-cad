import type { AppContext } from '../app/context';
import { bearingGrad, dist, type Vec2 } from '../model/geometry';
import { bulgeArc, bulgeOfSweep, bulgePathLength, bulgePathOutline, bulgeRingArea, bulgeThrough, hasBulges, segmentTangent, tangentBulge } from '../model/geom/bulge';
import { polygonCreate } from '../product/polygonCreate';
import type { ViewTransform } from '../viewport/Camera';
import { centreBulge, offsetAlong, radialPoint, radiusBulge, unitToward } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath, tint } from './preview';
import type { ToolPointer } from './Tool';

/**
 * How the next arc segment is shaped (AutoCAD PLINE arc options). The
 * default follows the path's end tangent; the others last one segment.
 *   angle   included angle (typed), then the end point
 *   radius  radius (typed), then the end point; the short arc, bending the way the path turns
 *   centre  the centre, then a point on the ray to the end (counter-clockwise)
 *   second  a point on the arc, then the end point
 *   dir     the start direction, then the end point
 */
type ArcSpec =
  | { kind: 'tangent' }
  | { kind: 'angle'; sweep: number | null }
  | { kind: 'radius'; r: number | null }
  | { kind: 'centre'; c: Vec2 | null }
  | { kind: 'second'; via: Vec2 | null }
  | { kind: 'dir'; dir: Vec2 | null };

/**
 * Open polyline, closed polygon, or a parcel polygon with cadastral
 * attributes. Y switches to arc segments (continuing tangentially, or
 * shaped by the arc options), D back to lines; in line mode U continues
 * the last direction by a typed length.
 */
export class PathTool extends PointInputTool {
  readonly id: string;
  protected readonly label: string;
  private readonly closed: boolean;
  private readonly measureOnly: boolean;
  /** Set for parcel mode: target layer that also numbers new parcels. */
  private readonly parcelLayer?: string;
  /** One bulge per drawn segment (pts[i] → pts[i+1]). */
  private bulges: number[] = [];
  private arcMode = false;
  private spec: ArcSpec = { kind: 'tangent' };
  /** Point on the first arc of a path, which has no tangent to follow. */
  private arcVia: Vec2 | null = null;
  /** Line mode: waiting for a typed length along the last direction. */
  private askLength = false;
  /** Bulge of the closing segment: an arc when the shape was closed on its first vertex in arc mode. */
  private closing = 0;
  /** Where the pointer went down, while that click is being taken (closing on the first vertex). */
  private pressedAt: Vec2 | null = null;

  constructor(ctx: AppContext, opts: { id: string; label: string; closed: boolean; measureOnly?: boolean; parcelLayer?: string }) {
    super(ctx);
    this.id = opts.id;
    this.label = opts.label;
    this.closed = opts.closed;
    this.measureOnly = opts.measureOnly ?? false;
    this.parcelLayer = opts.parcelLayer;
  }

  protected promptFor(n: number): string {
    if (n === 0) return 'ilk noktayı belirtin';
    const min = this.closed ? 3 : 2;
    const done = n < min ? '' : ' / Bitir (Enter)';
    if (this.askLength) return 'son doğrultuda devam edilecek uzunluğu yazın';
    if (!this.arcMode) return `sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / Geri (G)${done}]`;
    const arcOpts = `Düz (D) / Açı (A) / Merkez (M) / Yarıçap (R) / İkinci nokta (İ) / Doğrultu (T) / Geri (G)${done}`;
    const s = this.spec;
    switch (s.kind) {
      case 'angle':
        return s.sweep === null ? 'yayın iç açısını derece olarak yazın (artı saat yönünün tersine)' : `yayın bitiş noktasını belirtin [${arcOpts}]`;
      case 'radius':
        return s.r === null ? 'yayın yarıçapını yazın' : `yayın bitiş noktasını belirtin [${arcOpts}]`;
      case 'centre':
        return s.c ? `yayın bitiş doğrultusunu gösterin [${arcOpts}]` : `yayın merkezini gösterin [${arcOpts}]`;
      case 'second':
        return s.via ? `yayın bitiş noktasını belirtin [${arcOpts}]` : `yayın üzerinden geçeceği bir nokta belirtin [${arcOpts}]`;
      case 'dir':
        return s.dir ? `yayın bitiş noktasını belirtin [${arcOpts}]` : `yayın başlangıç doğrultusunu gösterin [${arcOpts}]`;
      default:
        if (!this.tangent() && !this.arcVia) return `yayın üzerinden geçeceği bir nokta belirtin [${arcOpts}]`;
        return `yayın bitiş noktasını belirtin [${arcOpts}]`;
    }
  }

  /** Travel direction at the last vertex (end tangent of the last segment). */
  private tangent(): Vec2 | null {
    const n = this.pts.length;
    return n >= 2 ? segmentTangent(this.pts[n - 2], this.pts[n - 1], this.bulges[n - 2] ?? 0, true) : null;
  }

  /** Where a segment towards p really ends (the centre option puts it on the circle). */
  private endFor(p: Vec2): Vec2 {
    const s = this.spec;
    const last = this.last;
    if (!this.arcMode || s.kind !== 'centre' || !s.c || !last) return p;
    return radialPoint(s.c, dist(s.c, last), p) ?? p;
  }

  /** Bulge the next segment to `p` would get, or null if impossible (or still waiting for a value). */
  private nextBulge(p: Vec2): number | null {
    const last = this.last;
    if (!last || !this.arcMode) return 0;
    const s = this.spec;
    switch (s.kind) {
      case 'angle':
        return s.sweep === null ? null : bulgeOfSweep(s.sweep);
      case 'radius':
        // Bends the way the path turns towards p; counter-clockwise when there is no tangent yet.
        return s.r === null ? null : radiusBulge(last, p, s.r, this.tangent());
      case 'centre':
        return s.c ? centreBulge(s.c, last, this.endFor(p)) : null;
      case 'second':
        return s.via ? bulgeThrough(last, s.via, p) : null;
      case 'dir':
        return s.dir ? tangentBulge(last, s.dir, p) : null;
      default: {
        const t = this.tangent();
        if (t) return tangentBulge(last, t, p);
        return this.arcVia ? bulgeThrough(last, this.arcVia, p) : null;
      }
    }
  }

  protected onPoint(p: Vec2): void {
    const last = this.last;
    if (!last) return void this.pts.push(p);
    const s = this.spec;
    // Options that take a point before the end point.
    if (this.arcMode) {
      if (s.kind === 'centre' && !s.c) return void (s.c = p);
      if (s.kind === 'second' && !s.via) return void (s.via = p);
      if (s.kind === 'dir' && !s.dir) {
        if (dist(last, p) > 1e-9) s.dir = unitToward(last, p);
        return;
      }
      if (s.kind === 'tangent' && !this.tangent() && !this.arcVia) return void (this.arcVia = p);
    }
    const end = this.endFor(p);
    if (dist(last, end) <= 1e-9) return;
    if (this.closesAt(end)) return this.closeOnFirst();
    const bulge = this.nextBulge(end);
    if (bulge === null) {
      if (s.kind === 'radius' && s.r !== null) return this.ctx.log.warn(`Kiriş yarıçapın iki katından (${this.ctx.format.length(2 * s.r)}) uzun; daha yakın bir nokta seçin.`);
      return this.ctx.log.warn('Bu nokta yayın tam arkasında kalıyor; başka bir nokta seçin.');
    }
    this.pts.push(end);
    this.bulges.push(bulge);
    this.arcVia = null;
    // Arc options shape one segment; the path then continues tangentially.
    this.spec = { kind: 'tangent' };
  }

  override pointerDown(p: ToolPointer): void {
    this.pressedAt = p.screen;
    super.pointerDown(p);
    this.pressedAt = null;
  }

  /**
   * A closed shape ends when its first vertex is given again (docs/adr/0018):
   * typed exactly, or clicked within the snap aperture once there are three
   * vertices. The first vertex is never written twice.
   */
  private closesAt(end: Vec2): boolean {
    const first = this.pts[0];
    if (!this.closed || !first) return false;
    if (dist(first, end) <= 1e-9) return true;
    if (!this.pressedAt || this.pts.length < 3) return false;
    const s = this.ctx.view.camera.worldToScreen(first);
    return Math.hypot(s.x - this.pressedAt.x, s.y - this.pressedAt.y) <= this.ctx.prefs.snapAperture.value;
  }

  private closeOnFirst(): void {
    if (this.pts.length < 3) return this.ctx.log.warn(`${this.label} için en az 3 köşe gerekir; ilk köşe ikinci kez eklenmedi.`);
    // In arc mode the segment back to the first vertex is the arc being drawn.
    const bulge = this.nextBulge(this.pts[0]);
    if (bulge === null) return this.ctx.log.warn('İlk köşe yayın tam arkasında kalıyor; alanı Enter ile düz kenarla kapatın.');
    this.closing = bulge;
    this.finish();
  }

  protected override option(key: string): boolean {
    const arcKeys: Record<string, () => ArcSpec> = {
      A: () => ({ kind: 'angle', sweep: null }),
      M: () => ({ kind: 'centre', c: null }),
      R: () => ({ kind: 'radius', r: null }),
      İ: () => ({ kind: 'second', via: null }),
      I: () => ({ kind: 'second', via: null }),
      T: () => ({ kind: 'dir', dir: null }),
    };
    if (key === 'Y' || key === 'D') {
      this.arcMode = key === 'Y';
      this.arcVia = null;
      this.spec = { kind: 'tangent' };
      this.askLength = false;
    } else if (this.arcMode && arcKeys[key] && this.pts.length) this.spec = arcKeys[key]();
    else if (!this.arcMode && key === 'U' && this.pts.length) {
      if (!this.tangent()) {
        this.ctx.log.warn('Uzunlukla devam için önce bir parça çizin; ilk parçanın doğrultusu yok.');
        return true;
      }
      this.askLength = true;
    } else if (key === 'G' && this.arcVia) this.arcVia = null;
    else if (key === 'G' && this.pts.length) {
      this.pts.pop();
      this.bulges.pop();
      this.spec = { kind: 'tangent' };
    } else return false;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    const plain = n !== null && !/[,;@<]/.test(text);
    const s = this.spec;
    if (plain && this.askLength) {
      const t = this.tangent();
      if (t && n! > 0) {
        this.askLength = false;
        this.accept(offsetAlong(this.last!, t, n!));
      } else this.ctx.log.warn('Uzunluk sıfırdan büyük olmalı.');
      return true;
    }
    if (plain && this.arcMode && s.kind === 'angle' && s.sweep === null) {
      if (Math.abs(n!) < 1e-9 || Math.abs(n!) >= 360) this.ctx.log.warn('İç açı 0 ile ±360 derece arasında olmalı.');
      else s.sweep = (n! * Math.PI) / 180;
      this.refreshPrompt();
      return true;
    }
    if (plain && this.arcMode && s.kind === 'radius' && s.r === null) {
      if (n! > 0) s.r = n!;
      else this.ctx.log.warn('Yarıçap sıfırdan büyük olmalı.');
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  protected override reset(): void {
    this.bulges = [];
    this.arcVia = null;
    this.arcMode = false;
    this.spec = { kind: 'tangent' };
    this.askLength = false;
    this.closing = 0;
    super.reset();
  }

  /** Bulges for the finished shape; a polygon closes straight unless it was closed on its first vertex with an arc. */
  private fullBulges(): number[] | undefined {
    const all = [...this.bulges, this.closing];
    return hasBulges(all) ? all : undefined;
  }

  protected override finish(): void {
    const min = this.closed ? 3 : 2;
    if (this.pts.length < min) {
      this.ctx.log.warn(`${this.label} için en az ${min} nokta gerekir.`);
      return super.finish();
    }
    const pts = [...this.pts];
    const bulges = this.fullBulges();
    const f = this.ctx.format;
    const area = () => Math.abs(bulgeRingArea(pts, bulges));
    if (this.measureOnly) {
      if (this.closed) this.ctx.log.success(`Alan ${f.area(area())}   Çevre ${f.length(bulgePathLength(pts, bulges, true))}`);
      else this.ctx.log.success(`Toplam uzunluk ${f.length(bulgePathLength(pts, bulges, false))} (${pts.length - 1} kenar)`);
      return super.finish();
    }
    const geom = { kind: this.closed ? ('polygon' as const) : ('polyline' as const), pts, ...(bulges && { bulges }) };
    if (this.parcelLayer) this.createParcel(geom, this.parcelLayer);
    else if (this.closed) this.createPolygon(pts, bulges, area);
    else if (this.create(geom)) this.ctx.log.success(`Çoklu çizgi eklendi: ${f.length(bulgePathLength(pts, bulges, false))}`);
    super.finish();
  }

  /**
   * A closed area is written by the product command `cad.polygon.create`
   * (docs/adr/0022), as on the desktop. What the tool knows implicitly is
   * explicit in the command's input (CMD-07): the active layer and the
   * current colour. The messages stay the tool's: the command's refusal or
   * warning (the locked and hidden layer texts, word for word), then the
   * area. Parcels, polylines and measuring still write directly.
   */
  private createPolygon(pts: Vec2[], bulges: number[] | undefined, area: () => number): void {
    const color = this.ctx.settings.color.value;
    const result = polygonCreate.execute({ doc: this.ctx.doc }, { layerId: this.ctx.doc.layers.active.value, pts, ...(bulges && { bulges }), ...(color !== null && { color }) });
    if (result.status !== 'completed') {
      if ('error' in result) this.ctx.log.warn(result.error.message);
      return;
    }
    for (const w of result.warnings) this.ctx.log.warn(w.message);
    this.ctx.log.success(`Kapalı alan eklendi: ${this.ctx.format.area(area())}`);
  }

  private createParcel(geom: { kind: 'polygon' | 'polyline'; pts: Vec2[]; bulges?: number[] }, layerId: string): void {
    const parcels = this.ctx.doc.byLayer(layerId);
    const next = parcels.reduce((m, e) => Math.max(m, parseInt(e.attrs.Parsel ?? '0', 10) || 0), 0) + 1;
    const area = Math.abs(bulgeRingArea(geom.pts, geom.bulges));
    const e = this.create(geom, {
      layerId,
      label: String(next),
      attrs: { Ada: '', Parsel: String(next), Mahalle: '', Nitelik: 'Arsa', 'Tapu alanı (m²)': area.toFixed(2), Pafta: '' },
    });
    if (e) {
      this.ctx.selection.set([e.id]);
      this.ctx.log.success(`Parsel ${next} oluşturuldu: ${this.ctx.format.area(area)}. Ada ve mahalle bilgisini Öznitelikler panelinden girin.`);
    }
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const pts = [...this.pts];
    const bulges = [...this.bulges];
    const end = this.hover ? this.endFor(this.hover) : null;
    const hb = end ? this.nextBulge(end) : null;
    if (end && hb !== null) {
      pts.push(end);
      bulges.push(hb);
    }
    bulges.push(0);
    if (this.closed && pts.length >= 3) {
      strokePath(g, view, bulgePathOutline(pts, bulges, true), { color: pal.accent, closed: true, dash: [4, 4], fill: tint(pal.accent, 0.08) });
    }
    strokePath(g, view, bulgePathOutline(pts, bulges, false), { color: pal.accent });
    const s = this.spec;
    const last = this.last;
    // Helper lines for points given before the end point.
    const guide = s.kind === 'second' ? s.via : s.kind === 'centre' ? s.c : this.arcVia;
    if (guide && last) strokePath(g, view, [last, guide], { color: pal.accent, dash: [2, 3] });
    else if (this.arcMode && last && this.hover && hb === null) strokePath(g, view, [last, this.hover], { color: pal.accent, dash: [2, 3] });
    if (s.kind === 'centre' && s.c && this.hover) strokePath(g, view, [s.c, this.hover], { color: pal.accent, dash: [2, 3] });
    if (!this.hover || !last || !end) return;
    const arc = hb ? bulgeArc(last, end, hb) : null;
    const lines = arc
      ? [`Yay r ${f.length(arc.r)}`, `Yay boyu ${f.length(arc.r * Math.abs(arc.sweep))}`]
      : [f.length(dist(last, end)), `Semt ${f.bearing(bearingGrad(last, end))}`];
    if (this.measureOnly && !this.closed) lines.push(`Toplam ${f.length(bulgePathLength(pts, bulges, false))}`);
    if (this.closed && pts.length >= 3) lines.push(`Alan ${f.area(Math.abs(bulgeRingArea(pts, bulges)))}`);
    drawTag(g, view.worldToScreen(this.hover), lines, pal.accent, pal.labelHalo);
    this.drawTracking(g, view);
  }
}
