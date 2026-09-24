import { entityLength, tessellateCircle, type Entity } from '../model/entities';
import { angleDeg, dist, type Vec2 } from '../model/geometry';
import { arcThrough, circleThrough, normAngle, tessellateArc, type ArcGeom } from '../model/geom/arc';
import {
  arcStartCenterAngle,
  arcStartCenterChord,
  arcStartCenterEnd,
  arcStartEndAngle,
  arcStartEndCenter,
  arcStartEndDirection,
  arcStartEndRadius,
} from '../model/geom/shapes';
import { closestOnEdge, type Edge } from '../model/geom/intersect';
import { catmullRom } from '../model/geom/spline';
import { tangentTangentRadius, tangentTangentTangent } from '../model/geom/tangentCircle';
import { entityEdges } from '../model/ops/edges';
import type { ViewTransform } from '../viewport/Camera';
import { circleOnDiameter, degDirection, endTangent } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';
import type { ToolPointer } from './Tool';

const deg = (rad: number) => (rad * 180) / Math.PI;

// ── Yay (AutoCAD ARC) ──────────────────────────────────────────────────

type ArcMode = 'three' | 'startCenter' | 'startEnd' | 'centerStart' | 'continue';
type ArcSub = 'end' | 'angle' | 'chord' | 'center' | 'direction' | 'radius';

/**
 * Every AutoCAD arc construction, chosen with the options as you go:
 *   üç nokta (default) · başlangıç–merkez–bitiş / açı / kiriş (M after the
 *   start) · başlangıç–bitiş–merkez / açı / yön / yarıçap (B after the
 *   start) · merkez–başlangıç–bitiş / açı / kiriş (M first) · devam: tangent
 *   to the last line, arc or polyline (D first).
 * Arcs are stored counter-clockwise; a clockwise pick gives the same curve.
 */
export class ArcTool extends PointInputTool {
  readonly id = 'arc';
  protected readonly label = 'Yay';
  private mode: ArcMode = 'three';
  private sub: ArcSub = 'end';
  private contDir: Vec2 | null = null;

  protected promptFor(n: number): string {
    switch (this.mode) {
      case 'continue':
        return 'bitiş noktasını belirtin (son nesneye teğet devam)';
      case 'startCenter':
      case 'centerStart': {
        const firstAsk = this.mode === 'startCenter' ? 'yayın merkezini belirtin' : n === 0 ? 'yayın merkezini belirtin' : 'başlangıç noktasını belirtin';
        return n < 2 ? firstAsk : this.thirdAroundCenter();
      }
      case 'startEnd':
        return n < 2 ? 'bitiş noktasını belirtin' : this.thirdStartEnd();
      default:
        if (n === 0) return 'başlangıç noktasını belirtin [Merkez (M) / Devam (D)]';
        if (n === 1) return 'yay üzerinde ikinci bir nokta belirtin [Merkez (M) / Bitiş (B)]';
        return 'bitiş noktasını belirtin';
    }
  }

  private thirdAroundCenter(): string {
    if (this.sub === 'angle') return 'yay açısını yazın ya da gösterin (derece, saat yönünün tersine) [Bitiş noktası (N) / Kiriş (U)]';
    if (this.sub === 'chord') return 'kiriş boyunu yazın ya da gösterin (eksi: büyük yay) [Bitiş noktası (N) / Açı (A)]';
    return 'bitiş noktasını belirtin [Açı (A) / Kiriş (U)]';
  }

  private thirdStartEnd(): string {
    switch (this.sub) {
      case 'angle':
        return 'yay açısını yazın ya da gösterin [Merkez (M) / Yön (Y) / Yarıçap (R)]';
      case 'direction':
        return 'başlangıçtaki teğet yönünü gösterin ya da açı yazın [Merkez (M) / Açı (A) / Yarıçap (R)]';
      case 'radius':
        return 'yarıçapı gösterin ya da yazın (eksi: büyük yay) [Merkez (M) / Açı (A) / Yön (Y)]';
      default:
        return 'yayın merkezini belirtin [Açı (A) / Yön (Y) / Yarıçap (R)]';
    }
  }

  protected override option(key: string): boolean {
    const n = this.pts.length;
    if (n === 0 && this.mode === 'three' && key === 'M') this.mode = 'centerStart';
    else if (n === 0 && this.mode === 'three' && key === 'D') {
      const last = this.lastEnd();
      if (!last) {
        this.ctx.log.warn('Devam edilecek bir çizgi, yay ya da çoklu çizgi yok.');
        return true;
      }
      this.mode = 'continue';
      this.pts = [last.p];
      this.contDir = last.dir;
    } else if (n === 1 && this.mode === 'three' && (key === 'M' || key === 'B')) this.mode = key === 'M' ? 'startCenter' : 'startEnd';
    else if (n === 2 && (this.mode === 'startCenter' || this.mode === 'centerStart') && ['A', 'U', 'N'].includes(key)) {
      this.sub = key === 'A' ? 'angle' : key === 'U' ? 'chord' : 'end';
    } else if (n === 2 && this.mode === 'startEnd' && ['M', 'A', 'Y', 'R'].includes(key)) {
      this.sub = key === 'M' ? 'center' : key === 'A' ? 'angle' : key === 'Y' ? 'direction' : 'radius';
    } else return false;
    if (this.mode === 'startEnd' && n < 2 && this.sub === 'end') this.sub = 'center';
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** End point and travel direction of the newest line, arc or polyline (a zero-length line is passed over). */
  private lastEnd(): { p: Vec2; dir: Vec2 } | null {
    const all = [...this.ctx.doc.all()];
    for (let i = all.length - 1; i >= 0; i--) {
      const e = all[i];
      if (e.kind !== 'line' && e.kind !== 'arc' && e.kind !== 'polyline') continue;
      const end = endTangent(e);
      if (end) return end;
    }
    return null;
  }

  /** The arc the third input (a point) would make in the current mode. */
  private arcFor(p: Vec2): ArcGeom | null {
    const [p0, p1] = this.pts;
    switch (this.mode) {
      case 'continue':
        return this.contDir ? arcStartEndDirection(p0, p, this.contDir) : null;
      case 'three':
        return arcThrough(p0, p1, p);
      case 'startCenter':
      case 'centerStart': {
        const [s, c] = this.mode === 'startCenter' ? [p0, p1] : [p1, p0];
        if (this.sub === 'chord') return arcStartCenterChord(s, c, dist(s, p));
        return arcStartCenterEnd(s, c, p);
      }
      case 'startEnd': {
        if (this.sub === 'center') return arcStartEndCenter(p0, p1, p);
        if (this.sub === 'direction') return arcStartEndDirection(p0, p1, { x: p.x - p0.x, y: p.y - p0.y });
        if (this.sub === 'radius') return arcStartEndRadius(p0, p1, dist(p1, p));
        // Included angle shown by a direction from the start, measured from east.
        return arcStartEndAngle(p0, p1, angleDeg(p0, p));
      }
    }
  }

  private ready(): boolean {
    return this.mode === 'continue' ? this.pts.length === 1 : this.pts.length === 2;
  }

  protected onPoint(p: Vec2): void {
    const last = this.last;
    if (last && dist(last, p) < 1e-9) return;
    if (this.ready()) return this.commit(this.arcFor(p));
    this.pts.push(p);
  }

  override input(text: string): boolean {
    const n = parseNumber(text);
    if (this.ready() && this.mode !== 'continue' && n !== null && !/[,;@<]/.test(text)) {
      const [p0, p1] = this.pts;
      let g: ArcGeom | null = null;
      if (this.mode === 'startCenter' || this.mode === 'centerStart') {
        const [s, c] = this.mode === 'startCenter' ? [p0, p1] : [p1, p0];
        g = this.sub === 'chord' ? arcStartCenterChord(s, c, n) : arcStartCenterAngle(s, c, n);
      } else if (this.mode === 'startEnd') {
        if (this.sub === 'radius') g = arcStartEndRadius(p0, p1, n);
        else if (this.sub === 'direction') g = arcStartEndDirection(p0, p1, degDirection(n));
        else g = arcStartEndAngle(p0, p1, n);
      } else return super.input(text);
      this.commit(g);
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  private commit(g: ArcGeom | null): void {
    if (!g) {
      this.ctx.log.warn('Bu değerlerle yay oluşmuyor (noktalar aynı doğruda ya da yarıçap kiriş için küçük).');
      return;
    }
    if (this.create({ kind: 'arc', ...g })) this.ctx.log.success(`Yay eklendi: r = ${this.ctx.format.length(g.r)}, açı ${deg(normAngle(g.a1 - g.a0) || 2 * Math.PI).toFixed(4)}°`);
    this.pts = [];
    this.mode = 'three';
    this.sub = 'end';
    this.contDir = null;
    this.ctx.view.requestOverlay();
  }

  protected override reset(): void {
    this.mode = 'three';
    this.sub = 'end';
    this.contDir = null;
    super.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const h = this.hover;
    if (!h) return;
    if (this.ready()) {
      const arc = this.arcFor(h);
      const [p0, p1] = this.pts;
      if (this.mode === 'startCenter' || this.mode === 'centerStart') {
        const c = this.mode === 'startCenter' ? p1 : p0;
        strokePath(g, view, [c, h], { color: pal.accent, dash: [3, 3] });
      } else if (this.mode === 'startEnd' && this.sub !== 'center') strokePath(g, view, [this.sub === 'radius' ? p1 : p0, h], { color: pal.accent, dash: [3, 3] });
      if (arc) {
        strokePath(g, view, tessellateArc(arc), { color: pal.accent, width: 1.5 });
        drawTag(g, view.worldToScreen(h), [`r ${this.ctx.format.length(arc.r)}`, `Açı ${deg(normAngle(arc.a1 - arc.a0) || 2 * Math.PI).toFixed(2)}°`], pal.accent, pal.labelHalo);
      }
      return;
    }
    const [p0] = this.pts;
    if (p0 && (this.mode === 'startCenter' || this.mode === 'centerStart')) {
      // Choosing the centre (or the start around a centre): the circle the arc will lie on.
      const [c, onCircle] = this.mode === 'startCenter' ? [h, p0] : [p0, h];
      strokePath(g, view, tessellateCircle(c, dist(c, onCircle), 96), { color: pal.accent, closed: true, dash: [2, 4] });
    }
    super.draw(g, view);
  }
}

// ── Daire ──────────────────────────────────────────────────────────────

type CircleMode = 'center' | 'two' | 'three' | 'ttr' | 'ttt';
interface TangentPick {
  edge: Edge;
  pick: Vec2;
}

/**
 * Circle by centre and radius (default), 2N (diameter ends), 3N (three
 * points on it), TTY (tangent to two objects with a radius) or TTT
 * (tangent to three objects).
 */
export class CircleTool extends PointInputTool {
  readonly id = 'circle';
  protected readonly label = 'Daire';
  private static lastRadius = 0;
  private mode: CircleMode = 'center';
  private tangents: TangentPick[] = [];
  /** Centre mode: the second input is a diameter (AutoCAD "Çap"). */
  private diameter = false;

  protected promptFor(n: number): string {
    switch (this.mode) {
      case 'two':
        return n === 0 ? 'çapın ilk ucunu belirtin' : 'çapın ikinci ucunu belirtin';
      case 'three':
        return ['ilk noktayı belirtin', 'ikinci noktayı belirtin', 'üçüncü noktayı belirtin'][n] ?? '';
      case 'ttr': {
        const r = CircleTool.lastRadius > 0 ? ` (Enter: ${this.ctx.format.length(CircleTool.lastRadius)})` : '';
        return ['ilk teğet çizgi, yay ya da daireyi seçin', 'ikinci teğet nesneyi seçin', `yarıçapı yazın${r}`][this.tangents.length];
      }
      case 'ttt':
        return ['birinci teğet çizgi, yay ya da daireyi seçin', 'ikinci teğet nesneyi seçin', 'üçüncü teğet nesneyi seçin'][this.tangents.length] ?? '';
      default:
        if (n === 0) return 'merkez noktasını belirtin [2 nokta (2N) / 3 nokta (3N) / Teğet-teğet-yarıçap (TTY) / Teğet-teğet-teğet (TTT)]';
        return this.diameter ? 'çapı gösterin ya da yazın [Yarıçap (R)]' : 'yarıçapı gösterin ya da yazın [Çap (Ç)]';
    }
  }

  override get snaps(): boolean {
    return this.mode !== 'ttr' && this.mode !== 'ttt';
  }

  protected override option(key: string): boolean {
    if (this.mode === 'center' && this.pts.length === 1 && (key === 'Ç' || key === 'C' || key === 'R')) {
      this.diameter = key !== 'R';
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    const modes: Record<string, CircleMode> = { '2N': 'two', '3N': 'three', TTY: 'ttr', TTT: 'ttt', M: 'center' };
    if (!modes[key] || this.pts.length) return false;
    this.mode = modes[key];
    this.tangents = [];
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override pointerDown(p: ToolPointer): void {
    if (this.mode !== 'ttr' && this.mode !== 'ttt') return super.pointerDown(p);
    if (p.button !== 0 || this.tangents.length >= (this.mode === 'ttt' ? 3 : 2)) return;
    const e = this.ctx.view.pickEdge(p.screen, (x) => ['line', 'polyline', 'polygon', 'arc', 'circle', 'xline', 'ray'].includes(x.kind));
    if (!e) return this.ctx.log.warn('Teğet olunacak bir çizgi, çoklu çizgi, yay ya da daireye tıklayın.');
    const edge = nearestEdge(e, p.raw);
    if (!edge) return;
    this.tangents.push({ edge, pick: p.raw });
    if (this.mode === 'ttt' && this.tangents.length === 3) {
      const [t1, t2, t3] = this.tangents;
      const c = tangentTangentTangent([t1.edge, t2.edge, t3.edge], [t1.pick, t2.pick, t3.pick]);
      this.tangents = [];
      if (c) this.commit(c.c, c.r);
      else this.ctx.log.warn('Üç nesneye birden teğet bir daire bulunamadı; nesnelere teğet noktalarının yakınından tıklayın.');
    }
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  protected onPoint(p: Vec2): void {
    const last = this.last;
    if (last && dist(last, p) < 1e-9) return;
    this.pts.push(p);
    const [a, b, c] = this.pts;
    if (this.mode === 'center' && b) return this.commit(a, this.diameter ? dist(a, b) / 2 : dist(a, b));
    if (this.mode === 'two' && b) {
      const circle = circleOnDiameter(a, b);
      return this.commit(circle.c, circle.r);
    }
    if (this.mode === 'three' && c) {
      const circle = circleThrough(a, b, c);
      if (!circle) {
        this.ctx.log.warn('Üç nokta aynı doğru üzerinde; daire çizilemez.');
        this.pts = [];
        return;
      }
      this.commit(circle.c, circle.r);
    }
  }

  override input(text: string): boolean {
    const r = parseNumber(text);
    const radiusTyped = r !== null && r > 0 && !/[,;@<]/.test(text);
    if (this.mode === 'ttr' && this.tangents.length === 2) {
      if (!radiusTyped) return false;
      this.commitTangent(r!);
      return true;
    }
    if (this.mode === 'center' && this.last && radiusTyped) {
      this.commit(this.last, this.diameter ? r! / 2 : r!);
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  override confirm(): void {
    if (this.mode === 'ttr' && this.tangents.length === 2 && CircleTool.lastRadius > 0) return this.commitTangent(CircleTool.lastRadius);
    if ((this.mode === 'ttr' || this.mode === 'ttt') && this.tangents.length) {
      this.tangents = [];
      return this.refreshPrompt();
    }
    super.confirm();
  }

  private ttrCircle(r: number) {
    const [t1, t2] = this.tangents;
    return tangentTangentRadius(t1.edge, t1.pick, t2.edge, t2.pick, r);
  }

  private commitTangent(r: number): void {
    const c = this.ttrCircle(r);
    if (!c) {
      this.ctx.log.warn('Bu yarıçapla iki nesneye birden teğet bir daire yok.');
      return;
    }
    this.tangents = [];
    this.commit(c.c, c.r);
    this.refreshPrompt();
  }

  private commit(c: Vec2, r: number): void {
    if (r > 1e-9 && this.create({ kind: 'circle', c, r })) {
      CircleTool.lastRadius = r;
      this.ctx.log.success(`Daire eklendi: r = ${this.ctx.format.length(r)}`);
    }
    this.pts = [];
    this.ctx.view.requestOverlay();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    if (this.mode === 'ttr' || this.mode === 'ttt') {
      for (const t of this.tangents) {
        const s = view.worldToScreen(t.pick);
        g.save();
        g.strokeStyle = pal.accent;
        g.strokeRect(Math.round(s.x) - 4.5, Math.round(s.y) - 4.5, 9, 9);
        g.restore();
      }
      const c = this.tangents.length === 2 && CircleTool.lastRadius > 0 ? this.ttrCircle(CircleTool.lastRadius) : null;
      if (c) strokePath(g, view, tessellateCircle(c.c, c.r, 96), { color: pal.accent, closed: true, dash: [4, 3] });
      return;
    }
    const h = this.hover;
    if (!h || !this.pts.length) return super.draw(g, view);
    const [a, b] = this.pts;
    let circle: { c: Vec2; r: number } | null = null;
    if (this.mode === 'center') circle = { c: a, r: this.diameter ? dist(a, h) / 2 : dist(a, h) };
    else if (this.mode === 'two') circle = circleOnDiameter(a, h);
    else if (b) circle = circleThrough(a, b, h);
    if (!circle) return super.draw(g, view);
    strokePath(g, view, tessellateCircle(circle.c, circle.r, 96), { color: pal.accent, closed: true });
    strokePath(g, view, [this.mode === 'center' ? a : circle.c, h], { color: pal.accent, dash: [3, 3] });
    drawTag(g, view.worldToScreen(h), [`r ${this.ctx.format.length(circle.r)}`], pal.accent, pal.labelHalo);
  }
}

/** The edge of `e` nearest to p (a polyline's clicked segment, a circle itself). */
function nearestEdge(e: Entity, p: Vec2): Edge | null {
  let best: { edge: Edge; d: number } | null = null;
  for (const edge of entityEdges(e)) {
    const d = closestOnEdge(edge, p).d;
    if (!best || d < best.d) best = { edge, d };
  }
  return best?.edge ?? null;
}

// ── Eğri ───────────────────────────────────────────────────────────────

/** Smooth curve through clicked points; Enter finishes, K closes it. */
export class SplineTool extends PointInputTool {
  readonly id = 'spline';
  protected readonly label = 'Eğri';

  protected promptFor(n: number): string {
    if (n === 0) return 'ilk noktayı belirtin';
    return n < 2 ? 'sonraki noktayı belirtin' : 'sonraki noktayı belirtin [Kapat (K) / Geri (G) / Bitir (Enter)]';
  }

  protected onPoint(p: Vec2): void {
    const last = this.last;
    if (!last || dist(last, p) > 1e-9) this.pts.push(p);
  }

  protected override option(key: string): boolean {
    if (key === 'G' && this.pts.length) {
      this.pts.pop();
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    if (key === 'K' && this.pts.length >= 3) {
      this.commit(true);
      return true;
    }
    return false;
  }

  protected override finish(): void {
    if (this.pts.length >= 2) this.commit(false);
    else super.finish();
  }

  private commit(closed: boolean): void {
    const e = this.create({ kind: 'spline', pts: [...this.pts], closed });
    if (e) this.ctx.log.success(`${closed ? 'Kapalı eğri' : 'Eğri'} eklendi: ${this.pts.length} nokta, ${this.ctx.format.length(entityLength(e)!)}`);
    this.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const chain = this.hover ? [...this.pts, this.hover] : this.pts;
    if (chain.length >= 2) strokePath(g, view, catmullRom(chain, false), { color: pal.accent });
    strokePath(g, view, chain, { color: pal.accent, dash: [2, 4] });
    this.drawTracking(g, view);
  }
}

