import { tessellateCircle } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { dist, signedArea } from '../model/geometry';
import { cornersOfRing } from '../model/ops/fillet';
import { rectFromCorners, rectFromEdge, rectFromSize, regularPolygon, regularPolygonOnEdge, sideDistance } from '../model/geom/shapes';
import type { ViewTransform } from '../viewport/Camera';
import { directionAngle, regularPolygonRadius } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';
import type { ToolPointer } from './Tool';

const DEG = Math.PI / 180;
const fmtDeg = (rad: number) => `${+((rad / DEG) % 360).toFixed(4)}°`;

// ── Dikdörtgen (AutoCAD RECTANG) ───────────────────────────────────────

type CornerStyle = { kind: 'none' } | { kind: 'fillet' | 'chamfer'; size: number };

/**
 * Two opposite corners. Before the first corner: rounded (Y) or chamfered
 * (P) corners. Before the second: rotation (D, typed angle or a pointed
 * direction) and exact size (B: "length,width" then a click for the side).
 * Rotation and corner style stay for the next rectangles, as in AutoCAD.
 */
export class RectangleTool extends PointInputTool {
  readonly id = 'rectangle';
  protected readonly label = 'Dikdörtgen';
  private static rotation = 0;
  private static corners: CornerStyle = { kind: 'none' };
  private stage: 'first' | 'second' | 'cornerSize' | 'rotation' | 'size' | 'side' = 'first';
  private pendingCorner: 'fillet' | 'chamfer' = 'fillet';
  private size: { length: number; width: number } | null = null;

  protected promptFor(): string {
    const c = RectangleTool.corners;
    const f = this.ctx.format;
    switch (this.stage) {
      case 'cornerSize':
        return this.pendingCorner === 'fillet' ? 'köşe yarıçapını yazın (0: keskin köşe)' : 'pah mesafesini yazın (0: keskin köşe)';
      case 'rotation':
        return 'dönme açısını yazın (derece) ya da ilk köşeden bir yön gösterin';
      case 'size':
        return 'uzunluk ve genişliği yazın, ör. 20,10';
      case 'side':
        return 'dikdörtgenin hangi yana açılacağını gösterin';
      case 'second':
        return `karşı köşeyi belirtin [Döndür (D): ${fmtDeg(RectangleTool.rotation)} / Boyutlar (B)]`;
      default: {
        const yuv = c.kind === 'fillet' ? f.length(c.size) : 'kapalı';
        const pah = c.kind === 'chamfer' ? f.length(c.size) : 'kapalı';
        return `ilk köşeyi belirtin [Köşe yuvarla (Y): ${yuv} / Pah (P): ${pah}]`;
      }
    }
  }

  protected override option(key: string): boolean {
    if (this.stage === 'first' && (key === 'Y' || key === 'P')) {
      this.pendingCorner = key === 'Y' ? 'fillet' : 'chamfer';
      this.stage = 'cornerSize';
    } else if (this.stage === 'second' && key === 'D') this.stage = 'rotation';
    else if (this.stage === 'second' && key === 'B') this.stage = 'size';
    else return false;
    this.refreshPrompt();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    if (this.stage === 'cornerSize') {
      if (n === null || n < 0) return false;
      RectangleTool.corners = n > 0 ? { kind: this.pendingCorner, size: n } : { kind: 'none' };
      this.stage = 'first';
      this.refreshPrompt();
      return true;
    }
    if (this.stage === 'rotation') {
      if (n === null) return false;
      RectangleTool.rotation = n * DEG;
      this.stage = 'second';
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    if (this.stage === 'size') {
      const m = text.trim().match(/^(\d+(?:\.\d+)?)\s*[,; ]\s*(\d+(?:\.\d+)?)$/);
      if (!m || +m[1] <= 0 || +m[2] <= 0) return false;
      this.size = { length: +m[1], width: +m[2] };
      this.stage = 'side';
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  protected onPoint(p: Vec2): void {
    const a = this.last;
    if (this.stage === 'first' || !a) {
      this.pts = [p];
      this.stage = 'second';
      return;
    }
    if (this.stage === 'rotation') {
      if (dist(a, p) > 1e-9) RectangleTool.rotation = directionAngle(a, p);
      this.stage = 'second';
      return;
    }
    const ring = this.ring(a, p);
    if (ring) this.commit(ring);
    else this.ctx.log.warn('Dikdörtgenin kenarları sıfır olamaz; başka bir köşe gösterin.');
  }

  private ring(a: Vec2, p: Vec2): Vec2[] | null {
    if (this.stage === 'side' && this.size) return rectFromSize(a, this.size.length, this.size.width, RectangleTool.rotation, p);
    return rectFromCorners(a, p, RectangleTool.rotation);
  }

  /** Corner style applied to every vertex, by the core (`cornersOfRing`, docs/adr/0032); the plain ring when it cannot be. */
  private styled(ring: Vec2[]): { pts: Vec2[]; bulges?: number[] } {
    const c = RectangleTool.corners;
    if (c.kind === 'none') return { pts: ring };
    const r = cornersOfRing(ring, c.kind === 'fillet' ? { radius: c.size } : { d1: c.size, d2: c.size });
    if ('error' in r) {
      this.ctx.log.warn(`Köşeler işlenmedi: ${r.error}`);
      return { pts: ring };
    }
    return r;
  }

  private commit(ring: Vec2[]): void {
    const g = this.styled(ring);
    const w = dist(ring[0], ring[1]);
    const h = dist(ring[1], ring[2]);
    if (this.create({ kind: 'polygon', pts: g.pts, ...(g.bulges && { bulges: g.bulges }) })) {
      this.ctx.log.success(`Dikdörtgen eklendi: ${this.ctx.format.length(w, false)} × ${this.ctx.format.length(h)}`);
    }
    this.size = null;
    this.stage = 'first';
    this.pts = [];
  }

  protected override reset(): void {
    this.stage = 'first';
    this.size = null;
    super.reset();
  }

  protected override constrain(p: ToolPointer): Vec2 {
    // Corners are free (object snaps and tracking still apply): ortho/polar would flatten the box.
    return p.world;
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const a = this.last;
    if (!a || !this.hover) return;
    const pal = this.ctx.view.palette;
    if (this.stage === 'rotation') {
      strokePath(g, view, [a, this.hover], { color: pal.accent, dash: [3, 3] });
      drawTag(g, view.worldToScreen(this.hover), [`Açı ${fmtDeg(directionAngle(a, this.hover))}`], pal.accent, pal.labelHalo);
      return;
    }
    if (this.stage !== 'second' && this.stage !== 'side') return;
    const ring = this.ring(a, this.hover);
    if (!ring) return;
    strokePath(g, view, ring, { color: pal.accent, closed: true });
    const f = this.ctx.format;
    drawTag(g, view.worldToScreen(this.hover), [`${f.length(dist(ring[0], ring[1]), false)} × ${f.length(dist(ring[1], ring[2]))}`, `Alan ${f.area(Math.abs(signedArea(ring)))}`], pal.accent, pal.labelHalo);
  }
}

// ── Döndürülmüş dikdörtgen ─────────────────────────────────────────────

/**
 * Rotated rectangle: draw one edge (length and direction with snaps, ortho,
 * polar, tracking or a typed distance), then pull it sideways for the
 * width — click, or type the width (the mouse side decides where it goes).
 */
export class RotatedRectangleTool extends PointInputTool {
  readonly id = 'rectangle3';
  protected readonly label = 'Döndürülmüş dikdörtgen';
  protected override readonly stepsFromPoints = true;

  protected promptFor(n: number): string {
    if (n === 0) return 'kenarın ilk noktasını belirtin';
    if (n === 1) return 'kenarın ikinci noktasını belirtin ya da uzunluk yazın';
    return 'genişliği fareyle yana çekerek gösterin ya da yazın';
  }

  protected onPoint(p: Vec2): void {
    if (this.pts.length < 2) {
      if (!this.last || dist(this.last, p) > 1e-9) this.pts.push(p);
      return;
    }
    this.commit(sideDistance(this.pts[0], this.pts[1], p));
  }

  override input(text: string): boolean {
    const n = parseNumber(text);
    if (this.pts.length === 2 && n !== null && n > 0 && !/[,;@<]/.test(text)) {
      // Typed width goes to the side the mouse is on.
      const side = this.hover && sideDistance(this.pts[0], this.pts[1], this.hover) < 0 ? -1 : 1;
      this.commit(n * side);
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  private commit(width: number): void {
    const ring = rectFromEdge(this.pts[0], this.pts[1], width);
    if (!ring) return this.ctx.log.warn('Genişlik sıfır olamaz; kenardan uzaklaşarak tıklayın.');
    if (this.create({ kind: 'polygon', pts: ring })) {
      const f = this.ctx.format;
      this.ctx.log.success(`Dikdörtgen eklendi: ${f.length(dist(ring[0], ring[1]), false)} × ${f.length(Math.abs(width))}`);
    }
    this.pts = [];
    this.ctx.view.requestOverlay();
  }

  protected override constrain(p: ToolPointer): Vec2 {
    // The width is measured square to the edge whatever the mouse does.
    return this.pts.length === 2 ? p.world : super.constrain(p);
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.pts.length < 2 || !this.hover) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    const [a, b] = this.pts;
    const w = sideDistance(a, b, this.hover);
    const ring = rectFromEdge(a, b, w);
    strokePath(g, view, [a, b], { color: pal.accent, width: 2 });
    if (!ring) return;
    strokePath(g, view, ring, { color: pal.accent, closed: true });
    const f = this.ctx.format;
    drawTag(g, view.worldToScreen(this.hover), [`${f.length(dist(a, b), false)} × ${f.length(Math.abs(w))}`, `Alan ${f.area(Math.abs(signedArea(ring)))}`], pal.accent, pal.labelHalo);
  }
}

// ── Düzgün çokgen (AutoCAD POLYGON) ────────────────────────────────────

/**
 * Regular polygon: the number of sides (typed, or S), then either a centre
 * and a vertex (inscribed) or an edge middle (circumscribed, toggled with
 * Ç), or two ends of one edge (K). A typed radius keeps the bottom edge
 * horizontal.
 */
export class RegularPolygonTool extends PointInputTool {
  readonly id = 'regularPolygon';
  protected readonly label = 'Düzgün çokgen';
  private static sides = 6;
  private static inscribed = true;
  private byEdge = false;
  private askSides = false;

  protected promptFor(n: number): string {
    const s = RegularPolygonTool.sides;
    const circle = RegularPolygonTool.inscribed ? 'köşeler üzerinde' : 'kenarlara teğet';
    if (this.askSides) return 'kenar sayısını yazın (3 ile 1024 arası)';
    if (this.byEdge) return n === 0 ? `kenarın ilk ucunu belirtin [Kenar sayısı (S): ${s} / Merkezden (M)]` : 'kenarın ikinci ucunu belirtin';
    if (n === 0) return `merkezi belirtin ya da kenar sayısını yazın [Kenar sayısı (S): ${s} / Çember (Ç): ${circle} / Kenardan (K)]`;
    return RegularPolygonTool.inscribed ? 'bir köşeyi gösterin ya da çember yarıçapını yazın' : 'bir kenarın ortasını gösterin ya da iç teğet çember yarıçapını yazın';
  }

  protected override option(key: string): boolean {
    if (this.pts.length) return false;
    if (key === 'S') this.askSides = true;
    else if (key === 'Ç' || key === 'C') RegularPolygonTool.inscribed = !RegularPolygonTool.inscribed;
    else if (key === 'K') this.byEdge = true;
    else if (key === 'M') this.byEdge = false;
    else return false;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    const plain = n !== null && !/[,;@<]/.test(text);
    // Before any point, a bare number is the side count; after the centre, a radius.
    if (plain && (this.askSides || this.pts.length === 0)) {
      if (!Number.isInteger(n) || n! < 3 || n! > 1024) {
        this.ctx.log.warn('Kenar sayısı 3 ile 1024 arasında bir tam sayı olmalı.');
        return true;
      }
      RegularPolygonTool.sides = n!;
      this.askSides = false;
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    if (plain && !this.byEdge && this.pts.length === 1 && n! > 0) {
      // Bottom edge horizontal: the edge middle sits straight below the centre.
      this.commit(regularPolygonRadius(this.pts[0], RegularPolygonTool.sides, n!, RegularPolygonTool.inscribed));
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  private shape(p: Vec2): Vec2[] | null {
    const sides = RegularPolygonTool.sides;
    if (this.byEdge) return regularPolygonOnEdge(this.pts[0], p, sides);
    return regularPolygon(this.pts[0], sides, p, RegularPolygonTool.inscribed ? 'inscribed' : 'circumscribed');
  }

  protected onPoint(p: Vec2): void {
    if (this.askSides) return;
    if (!this.pts.length) return void this.pts.push(p);
    this.commit(this.shape(p));
  }

  private commit(ring: Vec2[] | null): void {
    if (!ring) return this.ctx.log.warn('Çokgen için merkezden uzakta bir nokta gösterin.');
    if (this.create({ kind: 'polygon', pts: ring })) {
      const f = this.ctx.format;
      this.ctx.log.success(`${ring.length} kenarlı düzgün çokgen eklendi: kenar ${f.length(dist(ring[0], ring[1]))}, alan ${f.area(Math.abs(signedArea(ring)))}`);
    }
    this.pts = [];
    this.ctx.view.requestOverlay();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.pts.length || !this.hover) return super.draw(g, view);
    const ring = this.shape(this.hover);
    const pal = this.ctx.view.palette;
    if (!ring) return;
    const c = this.pts[0];
    if (!this.byEdge) {
      strokePath(g, view, tessellateCircle(c, dist(c, this.hover), 96), { color: pal.accent, closed: true, dash: [2, 4] });
      strokePath(g, view, [c, this.hover], { color: pal.accent, dash: [3, 3] });
    }
    strokePath(g, view, ring, { color: pal.accent, closed: true });
    const f = this.ctx.format;
    drawTag(g, view.worldToScreen(this.hover), [`${ring.length} kenar`, `Kenar ${f.length(dist(ring[0], ring[1]))}`, `Alan ${f.area(Math.abs(signedArea(ring)))}`], pal.accent, pal.labelHalo);
  }
}
