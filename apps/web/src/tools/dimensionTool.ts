import type { AppContext } from '../app/context';
import type { Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { DIMENSION_STYLE_LABEL, dimensionOffsetAt, layoutDimension, linearAngleFor, signedOffset, type DimensionGeom, type DimensionStyle } from '../model/geom/dimension';
import { closestOnEdge, lineLine, type Edge } from '../model/geom/intersect';
import { entityEdges } from '../model/ops/edges';
import type { ViewTransform } from '../viewport/Camera';
import { edgeArms, radialDimension, vertexArms } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag, strokeGeometry, strokePath } from './preview';
import type { ToolPointer } from './Tool';

/** Paper sizes (mm) converted to world metres at the project's plot scale. */
const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

const MODE_KEYS: [DimensionStyle, string][] = [
  ['aligned', 'H'],
  ['linear', 'D'],
  ['angular', 'A'],
  ['radius', 'R'],
  ['diameter', 'Ç'],
];

type Seg = { a: Vec2; b: Vec2 };

/** The straight edge of `e` nearest to p. */
function straightEdgeAt(e: Entity, p: Vec2): Seg | null {
  let best: { s: Seg; d: number } | null = null;
  for (const ed of entityEdges(e)) {
    if (ed.kind !== 'seg' || dist(ed.a, ed.b) < 1e-9) continue;
    const d = closestOnEdge(ed, p).d;
    if (!best || d < best.d) best = { s: { a: ed.a, b: ed.b }, d };
  }
  return best?.s ?? null;
}

/** The circle of a circle, an arc or the arc segment of a path nearest to p. */
function circleAt(e: Entity, p: Vec2): { c: Vec2; r: number } | null {
  if (e.kind === 'circle' || e.kind === 'arc') return { c: e.c, r: e.r };
  let best: { e: Extract<Edge, { kind: 'arc' }>; d: number } | null = null;
  for (const ed of entityEdges(e)) {
    if (ed.kind !== 'arc') continue;
    const d = closestOnEdge(ed, p).d;
    if (!best || d < best.d) best = { e: ed, d };
  }
  return best ? { c: best.e.c, r: best.e.r } : null;
}

/**
 * Ölçü: aligned (two points and the line's place), linear ΔY/ΔX (the
 * direction follows where the line is placed unless locked), angular (two
 * edges, or vertex and two arm points; the sector follows where the arc is
 * placed), radius and diameter (a circle or arc, then the direction).
 */
export class DimensionTool extends PointInputTool {
  readonly id = 'dimension';
  protected readonly label = 'Ölçü';
  private static mode: DimensionStyle = 'aligned';
  private static lock: 0 | 90 | null = null;
  private static byVertex = false;
  /** Angular by edges: the two picked edges and where they were clicked. */
  private edges: (Seg & { at: Vec2 })[] = [];
  private circle: { c: Vec2; r: number } | null = null;

  private get mode(): DimensionStyle {
    return DimensionTool.mode;
  }

  /** Nothing picked yet: the style can still change. */
  private get fresh(): boolean {
    return !this.pts.length && !this.edges.length && !this.circle;
  }

  /** Stages where a click picks an edge or a circle rather than a point. */
  private get picksEdge(): boolean {
    if (this.mode === 'angular') return !DimensionTool.byVertex && this.edges.length < 2;
    return (this.mode === 'radius' || this.mode === 'diameter') && !this.circle;
  }

  override get snaps(): boolean {
    return !this.picksEdge;
  }

  private height(): number {
    return paper(this.ctx, 2.5);
  }

  /** Whether the next point places the dimension line (or arc, or leader). */
  private get placing(): boolean {
    switch (this.mode) {
      case 'angular':
        return DimensionTool.byVertex ? this.pts.length === 3 : this.edges.length === 2;
      case 'radius':
      case 'diameter':
        return !!this.circle;
      default:
        return this.pts.length === 2;
    }
  }

  protected promptFor(n: number): string {
    const modes = this.fresh
      ? MODE_KEYS.filter(([m]) => m !== this.mode)
          .map(([m, k]) => `${DIMENSION_STYLE_LABEL[m]} (${k})`)
          .join(' / ')
      : '';
    let step: string;
    let opts = '';
    switch (this.mode) {
      case 'linear':
        step = n === 0 ? 'doğrusal ölçünün (ΔY / ΔX) ilk noktasını belirtin' : n === 1 ? 'ikinci ölçü noktasını belirtin' : 'ölçü çizgisinin yerini gösterin ya da mesafe yazın';
        if (n === 2) {
          const l = DimensionTool.lock;
          opts = `Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): ${l === 0 ? 'yatay' : l === 90 ? 'düşey' : 'imleçten'}`;
        }
        break;
      case 'angular':
        if (DimensionTool.byVertex) step = ['açının köşesini gösterin', 'birinci kolun üzerinde bir nokta gösterin', 'ikinci kolun üzerinde bir nokta gösterin', 'yayın yerini gösterin ya da yarıçap yazın'][Math.min(n, 3)];
        else step = ['açı ölçüsü için birinci kenara tıklayın', 'ikinci kenara tıklayın', 'yayın yerini gösterin ya da yarıçap yazın'][this.edges.length];
        if (this.fresh) opts = DimensionTool.byVertex ? 'Kenarlardan (K)' : 'Köşeden (K)';
        break;
      case 'radius':
      case 'diameter':
        step = this.circle
          ? 'ölçünün doğrultusunu gösterin; daireden dışarı çekince yazı dışarı alınır'
          : `${this.mode === 'radius' ? 'yarıçapı' : 'çapı'} ölçülecek daireye ya da yaya tıklayın`;
        break;
      default:
        step = n === 0 ? 'hizalı ölçünün ilk noktasını belirtin' : n === 1 ? 'ikinci ölçü noktasını belirtin' : 'ölçü çizgisinin yerini gösterin ya da mesafe yazın';
    }
    const all = [opts, modes].filter(Boolean).join(' / ');
    return all ? `${step} [${all}]` : step;
  }

  protected override option(key: string): boolean {
    if (this.fresh) {
      const m = MODE_KEYS.find(([, k]) => k === key || (k === 'Ç' && key === 'C'));
      if (m) {
        DimensionTool.mode = m[0];
        return this.changed();
      }
      if (key === 'K' && this.mode === 'angular') {
        DimensionTool.byVertex = !DimensionTool.byVertex;
        return this.changed();
      }
    }
    if (this.mode === 'linear' && this.pts.length === 2) {
      if (key === 'Y') DimensionTool.lock = 0;
      else if (key === 'X') DimensionTool.lock = 90;
      else if (key === 'O') DimensionTool.lock = null;
      else return false;
      return this.changed();
    }
    return false;
  }

  private changed(): true {
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override pointerMove(p: ToolPointer): void {
    if (this.picksEdge) {
      this.ctx.selection.hover.set(this.ctx.view.pickEdge(p.screen)?.id ?? null);
      this.hover = p.raw;
      this.ctx.view.requestOverlay();
      return;
    }
    super.pointerMove(p);
  }

  override pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (!this.picksEdge) return super.pointerDown(p);
    const e = this.ctx.view.pickEdge(p.screen);
    if (this.mode === 'angular') {
      const s = e && straightEdgeAt(e, p.raw);
      if (!s) return this.ctx.log.warn('Açının kenarı olarak düz bir çizgiye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin.');
      if (this.edges.length === 1 && !lineLine(this.edges[0].a, this.edges[0].b, s.a, s.b)) return this.ctx.log.warn('Kenarlar paralel; aralarında açı yok.');
      this.edges.push({ ...s, at: p.raw });
    } else {
      const c = e && circleAt(e, p.raw);
      if (!c) return this.ctx.log.warn('Bir daireye, yaya ya da çoklu çizginin yay parçasına tıklayın.');
      this.circle = c;
    }
    this.ctx.selection.hover.set(null);
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  protected onPoint(p: Vec2): void {
    if (!this.placing) {
      if (!this.last || dist(this.last, p) > 1e-9) this.pts.push(p);
      return;
    }
    this.commit(this.geomAt(p));
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    if (this.placing && n !== null && !/[,;@<]/.test(text)) {
      this.commit(this.geomAt(this.hover ?? this.pts[0] ?? this.edges[0]?.at ?? { x: 0, y: 0 }, n));
      return true;
    }
    return super.input(text);
  }

  override confirm(): void {
    if (this.edges.length || this.circle) return this.reset();
    super.confirm();
  }

  protected override reset(): void {
    this.edges = [];
    this.circle = null;
    super.reset();
  }

  /** The dimension placed at `loc`; a typed value replaces the offset (distance, radius). */
  private geomAt(loc: Vec2, typed?: number): DimensionGeom | null {
    const height = this.height();
    switch (this.mode) {
      case 'aligned': {
        const [a, b] = this.pts;
        return { a, b, offset: typed ?? signedOffset(a, b, loc), height };
      }
      case 'linear': {
        const [a, b] = this.pts;
        const angle = DimensionTool.lock ?? linearAngleFor(a, b, loc);
        const g: DimensionGeom = { a, b, offset: 0, height, style: 'linear', angle };
        return { ...g, offset: typed ?? dimensionOffsetAt(g, loc) };
      }
      case 'angular': {
        const pick = this.armsAt(loc);
        if (!pick) return null;
        return { ...pick, offset: typed ?? dist(pick.c, loc), height, style: 'angular' };
      }
      default: {
        const circle = this.circle;
        if (!circle) return null;
        const { b, offset } = radialDimension(circle.c, circle.r, loc);
        return { a: circle.c, b, offset, height, style: this.mode };
      }
    }
  }

  /**
   * Vertex and arm points for an angular dimension whose arc goes through
   * `loc`: from p1 to p2 counter-clockwise or the other way round; between
   * edges, each arm as far as its edge was clicked (at least a little).
   */
  private armsAt(loc: Vec2): { c: Vec2; a: Vec2; b: Vec2 } | null {
    if (DimensionTool.byVertex) {
      const [c, p1, p2] = this.pts;
      return vertexArms(c, p1, p2, loc);
    }
    const [e1, e2] = this.edges;
    return edgeArms(e1, e2, loc);
  }

  private commit(g: DimensionGeom | null): void {
    const l = g && layoutDimension(g);
    if (!g || !l) {
      this.ctx.log.warn('Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor ya da yay yarıçapı sıfır.');
      return;
    }
    const { style, ...rest } = g;
    const e = this.create({ kind: 'dimension', ...rest, ...(style && style !== 'aligned' && { style }) });
    if (e) this.ctx.log.success(`${DIMENSION_STYLE_LABEL[style ?? 'aligned']} ölçü eklendi: ${this.ctx.view.dimensionText(l)}`);
    this.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    for (const s of this.edges) strokePath(g, view, [s.a, s.b], { color: pal.snap, width: 2 });
    if (this.circle) strokeGeometry(g, view, { kind: 'circle', ...this.circle }, { color: pal.snap, width: 2 });
    if (this.placing && this.hover) {
      const d = this.geomAt(this.hover);
      const l = d && layoutDimension(d);
      if (!l) return;
      for (const [p, q] of l.lines) strokePath(g, view, [p, q], { color: pal.accent });
      drawTag(g, view.worldToScreen(this.hover), [this.ctx.view.dimensionText(l)], pal.accent, pal.labelHalo);
      return;
    }
    if (!this.picksEdge) super.draw(g, view);
  }
}
