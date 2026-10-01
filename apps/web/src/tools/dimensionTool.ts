import type { AppContext } from '../app/context';
import type { Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { DIMENSION_STYLE_LABEL, dimensionOffsetAt, layoutDimension, linearAngleFor, ordinateAxisFor, signedOffset, type DimensionGeom, type DimensionStyle } from '../model/geom/dimension';
import { closestOnEdge, lineLine, type Edge } from '../model/geom/intersect';
import { entityEdges } from '../model/ops/edges';
import type { ViewTransform } from '../viewport/Camera';
import { arcLengthEnds, edgeArms, radialDimension, vertexArms } from './constructions';
import { parseNumber } from './coordinateInput';
import { rememberDimension } from './dimChainTools';
import { PointInputTool } from './drawTools';
import { drawTag, strokeGeometry, strokePath } from './preview';
import type { ToolPointer } from './Tool';

/** Paper sizes (mm) converted to world metres at the project's plot scale. */
const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

/** What the log says when a dimension is added (a compound noun takes -sü: “Açı ölçüsü”, not “Açı ölçü”). */
const ADDED: Record<DimensionStyle, string> = {
  aligned: 'Hizalı ölçü eklendi',
  linear: 'Doğrusal ölçü eklendi',
  angular: 'Açı ölçüsü eklendi',
  radius: 'Yarıçap ölçüsü eklendi',
  diameter: 'Çap ölçüsü eklendi',
  ordinate: 'Koordinat ölçüsü eklendi',
  arcLength: 'Yay uzunluğu ölçüsü eklendi',
  jogged: 'Kırıklı yarıçap ölçüsü eklendi',
  azimuth: 'Semt ölçüsü eklendi',
  slope: 'Eğim ölçüsü eklendi',
};

const MODE_KEYS: [DimensionStyle, string][] = [
  ['aligned', 'H'],
  ['linear', 'D'],
  ['angular', 'A'],
  ['radius', 'R'],
  ['diameter', 'Ç'],
  ['ordinate', 'O'],
  ['arcLength', 'U'],
];

type ArcEdge = Extract<Edge, { kind: 'arc' }>;

/** Why a dimension of a style does not form where the cursor is. */
function noDimension(mode: DimensionStyle): string {
  if (mode === 'ordinate') return 'Çizginin ucu noktaya çok yakın; imleci noktadan eksene dik yönde uzaklaştırın.';
  if (mode === 'arcLength') return 'Bu yerde ölçü oluşmuyor; ölçü yayı merkeze ulaşıyor ya da iki nokta aynı yerde.';
  return 'Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor ya da yay yarıçapı sıfır.';
}

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

/** The arc of an arc, or the arc segment of a path nearest to p: Yay uzunluğu's pick (a circle has no ends to measure between). */
function arcEdgeAt(e: Entity, p: Vec2): ArcEdge | null {
  let best: { e: ArcEdge; d: number } | null = null;
  for (const ed of entityEdges(e)) {
    if (ed.kind !== 'arc' || !arcLengthEnds(ed, null)) continue;
    const d = closestOnEdge(ed, p).d;
    if (!best || d < best.d) best = { e: ed, d };
  }
  return best?.e ?? null;
}

/**
 * Ölçü: aligned (two points and the line's place), linear ΔY/ΔX (the
 * direction follows where the line is placed unless locked), angular (two
 * edges, or vertex and two arm points; the sector follows where the arc is
 * placed), radius and diameter (a circle or arc, then the direction);
 * Koordinat (a point, then its line's end; the axis follows the cursor unless
 * locked) and Yay uzunluğu (an arc or a path's arc segment, with Kısmi two
 * points on it, then where the dimension arc goes; docs/adr/0147 §7).
 */
export class DimensionTool extends PointInputTool {
  readonly id = 'dimension';
  protected readonly label = 'Ölçü';
  private static mode: DimensionStyle = 'aligned';
  private static lock: 0 | 90 | null = null;
  private static byVertex = false;
  /** Koordinat's axis lock (0 its Y, 90 its X; null: from the cursor) and Yay uzunluğu's Kısmi (docs/adr/0147 §7). */
  private static ordinateLock: 0 | 90 | null = null;
  private static arcPartial = false;
  /** Angular by edges: the two picked edges and where they were clicked. */
  private edges: (Seg & { at: Vec2 })[] = [];
  private circle: { c: Vec2; r: number } | null = null;
  /** Yay uzunluğu: the picked arc. */
  private arc: ArcEdge | null = null;

  private get mode(): DimensionStyle {
    return DimensionTool.mode;
  }

  /** Nothing picked yet: the style can still change. */
  private get fresh(): boolean {
    return !this.pts.length && !this.edges.length && !this.circle && !this.arc;
  }

  /** Stages where a click picks an edge or a circle rather than a point. */
  private get picksEdge(): boolean {
    if (this.mode === 'angular') return !DimensionTool.byVertex && this.edges.length < 2;
    if (this.mode === 'arcLength') return !this.arc;
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
      case 'ordinate':
        return this.pts.length === 1;
      case 'arcLength':
        return !!this.arc && (!DimensionTool.arcPartial || this.pts.length === 2);
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
      case 'ordinate': {
        step = n === 0 ? 'koordinat ölçüsünün noktasını belirtin' : 'çizginin ucunu gösterin ya da uzunluğunu yazın';
        if (n === 1) {
          const l = DimensionTool.ordinateLock;
          opts = `Y koordinatı (Y) / X koordinatı (X) / Eksen (O): ${l === 0 ? 'Y' : l === 90 ? 'X' : 'imleçten'}`;
        }
        break;
      }
      case 'arcLength':
        step = !this.arc
          ? 'yay uzunluğu ölçülecek yaya tıklayın'
          : this.placing
            ? 'ölçü yayının yerini gösterin ya da uzaklık yazın'
            : n === 0
              ? 'yayın üstünde ölçünün başlangıcını gösterin'
              : 'yayın üstünde ölçünün sonunu gösterin';
        if (this.fresh) opts = DimensionTool.arcPartial ? 'Bütün yay (K)' : 'Kısmi (K)';
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
      if (key === 'K' && this.mode === 'arcLength') {
        DimensionTool.arcPartial = !DimensionTool.arcPartial;
        return this.changed();
      }
    }
    if (this.mode === 'ordinate' && this.pts.length === 1) {
      if (key === 'Y') DimensionTool.ordinateLock = 0;
      else if (key === 'X') DimensionTool.ordinateLock = 90;
      else if (key === 'O') DimensionTool.ordinateLock = null;
      else return false;
      return this.changed();
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
    if (this.mode === 'arcLength') {
      const arc = e && arcEdgeAt(e, p.raw);
      if (!arc) return this.ctx.log.warn("Bir yaya ya da çoklu çizginin ya da alanın yaylı kenarına tıklayın; tam daire için Yarıçap ya da Çap'ı kullanın.");
      this.arc = arc;
    } else if (this.mode === 'angular') {
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
    // Radius and diameter are placed by pointing only (their prompt asks for no number); an ordinate's typed
    // number is its line's length toward the cursor, as every point tool takes one (docs/adr/0147 §7).
    if (this.placing && n !== null && !/[,;@<]/.test(text) && this.mode !== 'radius' && this.mode !== 'diameter' && this.mode !== 'ordinate') {
      this.commit(this.geomAt(this.hover ?? this.pts[0] ?? this.edges[0]?.at ?? { x: 0, y: 0 }, n));
      return true;
    }
    // An edge or a circle is picked with the mouse: a typed point would pick nothing, yet end the tool's
    // fresh start (its style options would go). It is refused, as on the desktop (docs/adr/0061).
    if (this.picksEdge) return false;
    return super.input(text);
  }

  override confirm(): void {
    if (this.edges.length || this.circle || this.arc) return this.reset();
    super.confirm();
  }

  /**
   * Ctrl+Z (docs/adr/0018), newest first: a picked circle or the last picked
   * edge goes back before what the base takes back (a point, the dimension
   * just written). It used to skip the picks: the previous dimension, or the
   * drawing, was undone while the picks stayed.
   */
  override undoStep(): boolean {
    // Yay uzunluğu's points on the arc go before the arc (docs/adr/0147 §7).
    if (this.arc && this.pts.length) this.pts.pop();
    else if (this.circle) this.circle = null;
    else if (this.edges.length) this.edges.pop();
    else if (this.arc) this.arc = null;
    else return super.undoStep();
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  protected override reset(): void {
    this.edges = [];
    this.circle = null;
    this.arc = null;
    super.reset();
  }

  /**
   * The dimension placed at `loc`. A typed value replaces the signed distance
   * of the dimension line (positive = left of the measured direction, so the
   * keyboard alone gives the side), or the arc's radius.
   */
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
        return { ...pick, offset: typed === undefined ? dist(pick.c, loc) : Math.abs(typed), height, style: 'angular' };
      }
      case 'ordinate': {
        const [a] = this.pts;
        return { a, b: loc, offset: 0, height, style: 'ordinate', angle: DimensionTool.ordinateLock ?? ordinateAxisFor(a, loc) };
      }
      case 'arcLength': {
        const ends = this.arc && arcLengthEnds(this.arc, this.pts.length === 2 ? [this.pts[0], this.pts[1]] : null);
        if (!ends) return null;
        const g: DimensionGeom = { ...ends, offset: 0, height, style: 'arcLength' };
        return { ...g, offset: typed ?? dimensionOffsetAt(g, loc) };
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
      this.ctx.log.warn(noDimension(this.mode));
      return;
    }
    const { style, ...rest } = g;
    // Written by `cad.entities.create` (step “Ekle”); a refusal (a locked layer) is said by it, nothing is added.
    const out = this.writeObjects([{ kind: 'dimension', ...rest, ...(style && style !== 'aligned' && { style }) }]);
    if (out) {
      // Zincir ölçü and Baz ölçü continue from the newest aligned or linear one (docs/adr/0140).
      rememberDimension(this.ctx.doc.uidOf(out.ids[0]), style);
      this.ctx.log.success(`${ADDED[style ?? 'aligned']}: ${this.ctx.view.dimensionText(l)}`);
    }
    this.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    for (const s of this.edges) strokePath(g, view, [s.a, s.b], { color: pal.snap, width: 2 });
    if (this.circle) strokeGeometry(g, view, { kind: 'circle', ...this.circle }, { color: pal.snap, width: 2 });
    // Yay uzunluğu's arc, and with Kısmi the part between its first point and the cursor, its length by the cursor.
    if (this.arc) {
      strokeArcEnds(g, view, arcLengthEnds(this.arc, null), pal.snap, 2);
      if (this.pts.length === 1 && this.hover) {
        const part = arcLengthEnds(this.arc, [this.pts[0], this.hover]);
        strokeArcEnds(g, view, part, pal.accent, 3);
        const l = part && layoutDimension({ ...part, offset: 0, height: this.height(), style: 'arcLength' });
        if (l) drawTag(g, view.worldToScreen(this.hover), [this.ctx.format.length(l.value)], pal.accent, pal.labelHalo);
      }
      if (!this.placing) return;
    }
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

/** An arc length's arc (or its part) drawn from its ends about its centre. */
function strokeArcEnds(g: CanvasRenderingContext2D, view: ViewTransform, ends: { a: Vec2; b: Vec2; c: Vec2 } | null, color: string, width: number): void {
  if (!ends) return;
  const angle = (p: Vec2) => Math.atan2(p.y - ends.c.y, p.x - ends.c.x);
  strokeGeometry(g, view, { kind: 'arc', c: ends.c, r: dist(ends.c, ends.a), a0: angle(ends.a), a1: angle(ends.b) }, { color, width });
}
