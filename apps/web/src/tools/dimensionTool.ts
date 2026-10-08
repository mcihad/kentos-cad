import type { AppContext } from '../app/context';
import type { Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { DIMENSION_STYLE_LABEL, dimensionFault, dimensionOffsetAt, layoutDimension, linearAngleFor, ordinateAxisFor, signedOffset, type DimensionGeom, type DimensionStyle } from '../model/geom/dimension';
import { closestOnEdge, lineLine, type Edge } from '../model/geom/intersect';
import { entityEdges } from '../model/ops/edges';
import { elevationAt } from '../product/elevationValues';
import type { ViewTransform } from '../viewport/Camera';
import type { SnapHit } from '../viewport/picking';
import { arcLengthEnds, edgeArms, radialDimension, vertexArms } from './constructions';
import { parseLength, parseNumber } from './coordinateInput';
import { rememberDimension } from './dimChainTools';
import { PointInputTool } from './drawTools';
import { drawTag, strokeGeometry, strokeLayout, strokePath } from './preview';
import type { OptionChoice, ToolPointer } from './Tool';
import { dimensionLookNow, dimensionStyleChoices, dimensionStyleName, standardDimensionHeight, stylesShown, takeDimensionStyle } from './styleOption';
import { lookOfDimension, type DimensionLook } from '../model/annotationStyles';

/** Paper sizes (mm) converted to world metres at the project's plot scale. */
export const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

/**
 * Zemin (Z) of Ölçülendirme and Hızlı ölçü: the value written over the drawing's background, kept for the app's life
 * (docs/adr/0147 §7; the desktop's `Memory::dimension_mask`).
 */
export const dimensionZemin = { on: false };

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
  ['jogged', 'I'],
  ['azimuth', 'T'],
  ['slope', 'E'],
];

/** Semt and Eğim: an arrow beside two points, or an edge. */
const arrowed = (mode: DimensionStyle) => mode === 'azimuth' || mode === 'slope';

type ArcEdge = Extract<Edge, { kind: 'arc' }>;

/** Why a dimension of a style does not form where the cursor is. */
function noDimension(mode: DimensionStyle): string {
  if (mode === 'ordinate') return 'Çizginin ucu noktaya çok yakın; imleci noktadan eksene dik yönde uzaklaştırın.';
  if (mode === 'arcLength') return 'Bu yerde ölçü oluşmuyor; ölçü yayı merkeze ulaşıyor ya da iki nokta aynı yerde.';
  if (mode === 'jogged') return 'Gösterilen merkez, yarıçap boyunca yaydaki noktadan geride ve yarıçapa yakın olmalı; başka bir yer gösterin.';
  if (arrowed(mode)) return 'Kenarın iki ucu aynı nokta; ölçülecek kenar yok. Başka bir nokta gösterin.';
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
  /** Doğrusal's measuring direction, degrees counter-clockwise from east: 0 ΔY, 90 ΔX, another typed with Açı (A); null from the cursor. */
  private static lock: number | null = null;
  private static byVertex = false;
  /** Koordinat's axis lock (0 its Y, 90 its X; null: from the cursor) and Yay uzunluğu's Kısmi (docs/adr/0147 §7). */
  private static ordinateLock: 0 | 90 | null = null;
  private static arcPartial = false;
  /** Semt's and Eğim's Kenardan: an edge clicked gives the two points (docs/adr/0147 §7). */
  private static byEdge = false;
  /** Angular by edges: the two picked edges and where they were clicked. */
  private edges: (Seg & { at: Vec2 })[] = [];
  private circle: { c: Vec2; r: number } | null = null;
  /** Yay uzunluğu: the picked arc. */
  private arc: ArcEdge | null = null;
  /** Eğim: each point's elevation, beside `pts`; null while it is asked for. */
  private zs: (number | null)[] = [];
  /** The snap under the pointer as it went down: where Eğim's elevation comes from. */
  private snap: SnapHit | null = null;
  /** Doğrusal's Açı (A): the next number is the measuring direction. */
  private askingAngle = false;
  /** Stil (S): a dimension style's name asked for (docs/adr/0183 §4). */
  private askingStyle = false;
  /** Açı from an arc: its vertex and ends; from a circle: its centre and the point clicked on it (docs/adr/0147 §7). */
  private angleArc: { c: Vec2; a: Vec2; b: Vec2 } | null = null;
  private angleCircle: { c: Vec2; p1: Vec2 } | null = null;

  private get mode(): DimensionStyle {
    return DimensionTool.mode;
  }

  /** Nothing picked yet: the style can still change. */
  private get fresh(): boolean {
    return !this.pts.length && !this.edges.length && !this.circle && !this.arc && !this.angleArc && !this.angleCircle;
  }

  /** Stages where a click picks an edge or a circle rather than a point. */
  private get picksEdge(): boolean {
    if (this.mode === 'angular') return !DimensionTool.byVertex && this.edges.length < 2 && !this.angleArc && !this.angleCircle;
    if (this.mode === 'arcLength') return !this.arc;
    if (arrowed(this.mode)) return DimensionTool.byEdge && !this.pts.length;
    return (this.mode === 'radius' || this.mode === 'diameter' || this.mode === 'jogged') && !this.circle;
  }

  /** Eğim's point whose elevation is asked for (0 or 1), or −1. */
  private get asking(): number {
    return this.mode === 'slope' ? this.zs.indexOf(null) : -1;
  }

  /** No snapping while an edge is picked or Eğim waits for an elevation. */
  override get snaps(): boolean {
    return !this.picksEdge && this.asking < 0;
  }

  /** The value's height and the look, the dimension style's in a CAD project (docs/adr/0183 §4); else 2.5 mm and none. */
  private styled(): { look: DimensionLook; height: number } {
    return dimensionLookNow(this.ctx, standardDimensionHeight(this.ctx));
  }

  private height(): number {
    return this.styled().height;
  }

  /** Whether the next point places the dimension line (or arc, or leader). */
  private get placing(): boolean {
    switch (this.mode) {
      case 'angular':
        if (DimensionTool.byVertex) return this.pts.length === 3;
        return this.edges.length === 2 || !!this.angleArc || (!!this.angleCircle && this.pts.length === 1);
      case 'radius':
      case 'diameter':
        return !!this.circle;
      case 'ordinate':
        return this.pts.length === 1;
      case 'arcLength':
        return !!this.arc && (!DimensionTool.arcPartial || this.pts.length === 2);
      case 'jogged':
        return !!this.circle && this.pts.length === 2;
      case 'azimuth':
        return this.pts.length === 2;
      case 'slope':
        return this.pts.length === 2 && this.asking < 0;
      default:
        return this.pts.length === 2;
    }
  }

  protected promptFor(n: number): string {
    if (this.askingStyle) return `ölçü stilini menüden seçin ya da adını yazın [Stil (S): ${dimensionStyleName(this.ctx)}]`;
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
        if (n === 2 && this.askingAngle) {
          step = `ölçme doğrultusunu yazın (${this.ctx.format.angleUnitName}, doğudan saatin tersine)`;
          break;
        }
        if (n === 2) {
          const l = DimensionTool.lock;
          const way = l === null ? 'imleçten' : l === 0 ? 'yatay' : l === 90 ? 'düşey' : this.ctx.format.angle((l * Math.PI) / 180);
          opts = `Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): ${way} / Açı (A)`;
        }
        break;
      case 'angular':
        if (DimensionTool.byVertex) step = ['açının köşesini gösterin', 'birinci kolun üzerinde bir nokta gösterin', 'ikinci kolun üzerinde bir nokta gösterin', 'yayın yerini gösterin ya da yarıçap yazın'][Math.min(n, 3)];
        else if (this.placing) step = 'yayın yerini gösterin ya da yarıçap yazın';
        else if (this.angleCircle) step = 'açının ikinci noktasını gösterin';
        else step = this.edges.length ? 'ikinci kenara tıklayın' : 'açı ölçüsü için bir kenara, yaya ya da daireye tıklayın';
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
      case 'jogged':
        step = !this.circle
          ? 'kırıklı yarıçapı ölçülecek daireye ya da yaya tıklayın'
          : ['çizginin başlayacağı merkezi gösterin', 'yaydaki noktayı gösterin', 'kırığın yerini gösterin ya da uzaklığını yazın'][Math.min(n, 2)];
        break;
      case 'azimuth':
      case 'slope': {
        const slope = this.mode === 'slope';
        if (this.picksEdge) step = slope ? 'eğimi ölçülecek kenara tıklayın' : 'semti ölçülecek kenara tıklayın';
        else if (this.asking === 0) step = `birinci noktanın kotunu yazın (${this.ctx.format.lengthUnitLabel})`;
        else if (this.asking === 1) step = `ikinci noktanın kotunu yazın (${this.ctx.format.lengthUnitLabel})`;
        else if (n === 0) step = slope ? 'eğim ölçüsünün birinci noktasını gösterin' : 'semt ölçüsünün başlangıcını gösterin';
        else if (n === 1) step = slope ? 'ikinci noktayı gösterin' : 'kenarın sonunu gösterin';
        else step = 'okun yerini gösterin ya da uzaklık yazın';
        if (this.fresh) opts = DimensionTool.byEdge ? 'Noktalardan (K)' : 'Kenardan (K)';
        break;
      }
      default:
        step = n === 0 ? 'hizalı ölçünün ilk noktasını belirtin' : n === 1 ? 'ikinci ölçü noktasını belirtin' : 'ölçü çizgisinin yerini gösterin ya da mesafe yazın';
    }
    // Stil and Zemin while nothing is picked and while the dimension is placed (docs/adr/0147 §7, 0183 §4).
    const offered = (this.fresh || this.placing) && !this.askingAngle;
    const stil = offered && stylesShown(this.ctx) ? `Stil (S): ${dimensionStyleName(this.ctx)}` : '';
    const zemin = offered ? `Zemin (Z): ${dimensionZemin.on ? 'açık' : 'kapalı'}` : '';
    const all = [opts, stil, zemin, modes].filter(Boolean).join(' / ');
    return all ? `${step} [${all}]` : step;
  }

  protected override option(key: string): boolean {
    if (key === 'Z' && !this.askingAngle) {
      dimensionZemin.on = !dimensionZemin.on;
      return this.changed();
    }
    // Stil while Zemin is offered: a CAD project's dimension styles (docs/adr/0183 §4).
    if (key === 'S' && !this.askingAngle && stylesShown(this.ctx) && (this.fresh || this.placing)) {
      this.askingStyle = true;
      return this.changed();
    }
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
      if (key === 'K' && arrowed(this.mode)) {
        DimensionTool.byEdge = !DimensionTool.byEdge;
        return this.changed();
      }
      // Kırıklı yarıçap's I, typed dotted on an English layout.
      if (key === 'İ') {
        DimensionTool.mode = 'jogged';
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
    if (this.mode === 'linear' && this.pts.length === 2 && !this.askingAngle) {
      if (key === 'Y') DimensionTool.lock = 0;
      else if (key === 'X') DimensionTool.lock = 90;
      else if (key === 'O') DimensionTool.lock = null;
      else if (key === 'A') this.askingAngle = true;
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
    if (!this.picksEdge) {
      this.snap = p.snap;
      return super.pointerDown(p);
    }
    const e = this.ctx.view.pickEdge(p.screen);
    // Semt and Eğim by an edge: its two ends, Eğim their elevations (docs/adr/0147 §7).
    if (arrowed(this.mode)) {
      const s = e && straightEdgeAt(e, p.raw);
      if (!e || !s) return this.ctx.log.warn('Bir çizgiye ya da çoklu çizginin ya da alanın düz kenarına tıklayın; iki nokta göstermek için “Noktalardan” seçin.');
      if (this.mode === 'slope') this.zs = [elevationAt(e, s.a), elevationAt(e, s.b)];
      this.pts = [s.a, s.b];
      this.ctx.selection.hover.set(null);
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return;
    }
    if (this.mode === 'arcLength') {
      const arc = e && arcEdgeAt(e, p.raw);
      if (!arc) return this.ctx.log.warn("Bir yaya ya da çoklu çizginin ya da alanın yaylı kenarına tıklayın; tam daire için Yarıçap ya da Çap'ı kullanın.");
      this.arc = arc;
    } else if (this.mode === 'angular' && !this.edges.length && e && this.angleFrom(e, p.raw)) {
      // Yaydan: the arc's own angle; Daireden: from the point clicked on the circle to a second point.
    } else if (this.mode === 'angular') {
      const s = e && straightEdgeAt(e, p.raw);
      // The first pick also takes an arc or a circle (angleFrom); the second is a straight edge only.
      if (!s)
        return this.ctx.log.warn(
          this.edges.length
            ? 'Açının kenarı olarak düz bir çizgiye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin.'
            : 'Açı için düz bir kenara, yaya ya da daireye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin.',
        );
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
    const snap = this.snap;
    this.snap = null;
    // Eğim waits for the elevation it asked for.
    if (this.asking >= 0) return this.ctx.log.warn('Önce noktanın kotunu metre olarak yazın; geri almak için Ctrl+Z.');
    if (!this.placing) {
      if (!this.last || dist(this.last, p) > 1e-9) {
        // Kırıklı yarıçap's point on the arc must leave room for a jog from the centre shown.
        if (this.mode === 'jogged' && this.pts.length === 1) {
          const g = this.joggedAt(this.pts[0], p, 0);
          if (!g || dimensionFault(g)) return this.ctx.log.warn(noDimension('jogged'));
        }
        this.pts.push(p);
        if (this.mode === 'slope') this.zs.push(this.snappedElevation(snap, p));
      }
      return;
    }
    this.commit(this.geomAt(p));
  }

  /**
   * Açı's first pick on an arc or a circle (docs/adr/0147 §7): Yaydan, the arc's own angle about its centre (its ends
   * counter-clockwise); Daireden, from the point clicked, put on the circle, to a second point. False for another edge.
   */
  private angleFrom(e: Entity, at: Vec2): boolean {
    let best: { ed: ArcEdge; d: number } | null = null;
    for (const ed of entityEdges(e)) {
      if (ed.kind !== 'arc') continue;
      const d = closestOnEdge(ed, at).d;
      if (!best || d < best.d) best = { ed, d };
    }
    const straight = straightEdgeAt(e, at);
    if (!best || (straight && closestOnEdge({ kind: 'seg', a: straight.a, b: straight.b }, at).d < best.d)) return false;
    const ends = arcLengthEnds(best.ed, null);
    if (ends) this.angleArc = ends;
    else {
      const { c, r } = best.ed;
      const l = dist(c, at);
      if (l < 1e-9) return false;
      this.angleCircle = { c, p1: { x: c.x + ((at.x - c.x) / l) * r, y: c.y + ((at.y - c.y) / l) * r } };
    }
    return true;
  }

  /** The elevation of the point, or of the vertex of a line, a polyline or an area, a snap stands on (docs/adr/0142). */
  private snappedElevation(snap: SnapHit | null, p: Vec2): number | null {
    if (!snap || dist(snap.point, p) > 1e-9) return null;
    const e = this.ctx.doc.get(snap.entityId);
    return e ? elevationAt(e, snap.point) : null;
  }

  /** Kırıklı yarıçap from the centre shown to the circle's point toward `on`, its jog `offset` along the radius. */
  private joggedAt(shown: Vec2, on: Vec2, offset: number): DimensionGeom | null {
    const circle = this.circle;
    if (!circle) return null;
    const l = dist(circle.c, on);
    if (l < 1e-9) return null;
    const b = { x: circle.c.x + ((on.x - circle.c.x) / l) * circle.r, y: circle.c.y + ((on.y - circle.c.y) / l) * circle.r };
    return { a: circle.c, b, c: shown, offset, height: this.height(), style: 'jogged' };
  }

  /** Stil's menu: Standart, the project's dimension styles and their window (docs/adr/0183 §4). */
  optionChoices(key: string): readonly OptionChoice[] | null {
    if (key !== 'S' || !stylesShown(this.ctx) || !(this.askingStyle || ((this.fresh || this.placing) && !this.askingAngle))) return null;
    return dimensionStyleChoices(this.ctx);
  }

  /** A style's name is words: Space types a space (docs/adr/0183 §4). */
  takesWords(): boolean {
    return this.askingStyle;
  }

  chooseOption(key: string, typed: string): boolean {
    if (key !== 'S' || !stylesShown(this.ctx) || !(this.askingStyle || ((this.fresh || this.placing) && !this.askingAngle))) return false;
    if (takeDimensionStyle(this.ctx, typed)) this.askingStyle = false;
    return this.changed();
  }

  override input(text: string): boolean {
    // A dimension style's name: one the project has none of is said, and the tool waits for another.
    if (this.askingStyle) {
      if (takeDimensionStyle(this.ctx, text)) this.askingStyle = false;
      return this.changed();
    }
    // Eğim's elevation asked for: a number, in the project's unit (docs/adr/0165 §2).
    if (this.asking >= 0) {
      const z = parseLength(this.ctx.format, text);
      if (z === null || /[,;@<]/.test(text)) return false;
      this.zs[this.asking] = z;
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    // Doğrusal's measuring direction asked for, in the project's angle unit.
    if (this.askingAngle) {
      const a = parseNumber(text);
      if (a === null || /[,;@<]/.test(text)) return false;
      DimensionTool.lock = (this.ctx.format.angleFromTyped(a) * 180) / Math.PI;
      this.askingAngle = false;
      return this.changed();
    }
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(text);
    // Radius and diameter are placed by pointing only (their prompt asks for no number); an ordinate's typed
    // number is its line's length toward the cursor, as every point tool takes one (docs/adr/0147 §7).
    if (this.placing && n !== null && !/[,;@<]/.test(text) && this.mode !== 'radius' && this.mode !== 'diameter' && this.mode !== 'ordinate') {
      this.commit(this.geomAt(this.hover ?? this.pts[0] ?? this.edges[0]?.at ?? { x: 0, y: 0 }, this.ctx.format.toMetres(n)));
      return true;
    }
    // An edge or a circle is picked with the mouse: a typed point would pick nothing, yet end the tool's
    // fresh start (its style options would go). It is refused, as on the desktop (docs/adr/0061).
    if (this.picksEdge) return false;
    return super.input(text);
  }

  override confirm(): void {
    if (this.askingStyle) {
      this.askingStyle = false;
      this.changed();
      return;
    }
    if (this.edges.length || this.circle || this.arc || this.angleArc || this.angleCircle) return this.reset();
    super.confirm();
  }

  /**
   * Ctrl+Z (docs/adr/0018), newest first: a picked circle or the last picked
   * edge goes back before what the base takes back (a point, the dimension
   * just written). It used to skip the picks: the previous dimension, or the
   * drawing, was undone while the picks stayed.
   */
  override undoStep(): boolean {
    // Yay uzunluğu's points on the arc go before the arc, Kırıklı yarıçap's before its circle; Eğim's with their
    // elevations, an edge's two ends together (docs/adr/0147 §7).
    if ((this.arc || this.circle || this.angleCircle) && this.pts.length) this.pts.pop();
    else if (this.angleArc || this.angleCircle) {
      this.angleArc = null;
      this.angleCircle = null;
    }
    else if (arrowed(this.mode) && this.pts.length) {
      if (DimensionTool.byEdge) this.pts = [];
      else this.pts.pop();
      this.zs = this.zs.slice(0, this.pts.length);
    }
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
    this.zs = [];
    this.snap = null;
    this.askingAngle = false;
    this.askingStyle = false;
    this.angleArc = null;
    this.angleCircle = null;
    super.reset();
  }

  /**
   * The dimension placed at `loc`. A typed value replaces the signed distance
   * of the dimension line (positive = left of the measured direction, so the
   * keyboard alone gives the side), or the arc's radius.
   */
  private geomAt(loc: Vec2, typed?: number): DimensionGeom | null {
    const g = this.shapeAt(loc, typed);
    return g && { ...g, ...this.styled().look };
  }

  private shapeAt(loc: Vec2, typed?: number): DimensionGeom | null {
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
      case 'jogged': {
        const g = this.pts.length === 2 ? this.joggedAt(this.pts[0], this.pts[1], 0) : null;
        return g && { ...g, offset: typed ?? dimensionOffsetAt(g, loc) };
      }
      case 'azimuth':
      case 'slope': {
        const [a, b] = this.pts;
        const g: DimensionGeom = { a, b, offset: typed ?? signedOffset(a, b, loc), height, style: this.mode };
        if (this.mode === 'slope') return this.zs[0] == null || this.zs[1] == null ? null : { ...g, za: this.zs[0], zb: this.zs[1] };
        return g;
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
    if (this.angleArc) return this.angleArc;
    if (this.angleCircle) return this.pts.length ? vertexArms(this.angleCircle.c, this.angleCircle.p1, this.pts[0], loc) : null;
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
    const out = this.writeObjects([{ kind: 'dimension', ...rest, ...(style && style !== 'aligned' && { style }), ...(dimensionZemin.on && { mask: true }) }]);
    const look = lookOfDimension(g);
    if (out) {
      // Zincir ölçü and Baz ölçü continue from the newest aligned or linear one (docs/adr/0140).
      rememberDimension(this.ctx.doc.uidOf(out.ids[0]), style);
      this.ctx.log.success(`${ADDED[style ?? 'aligned']}: ${this.ctx.view.dimensionText(l, look)}`);
    }
    this.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    for (const s of this.edges) strokePath(g, view, [s.a, s.b], { color: pal.snap, width: 2 });
    if (this.circle) strokeGeometry(g, view, { kind: 'circle', ...this.circle }, { color: pal.snap, width: 2 });
    // Daireden: the arm to the point clicked on the circle, and to the cursor until the second point is given.
    if (this.angleCircle && !this.placing) {
      strokePath(g, view, [this.angleCircle.c, this.angleCircle.p1], { color: pal.snap, width: 2 });
      if (this.hover) strokePath(g, view, [this.angleCircle.c, this.hover], { color: pal.accent });
    }
    const look = this.styled().look;
    // Kırıklı yarıçap's point on the arc: the dimension as it would be, its jog at the centre shown.
    if (this.mode === 'jogged' && this.circle && this.pts.length === 1 && this.hover) {
      const jog = this.joggedAt(this.pts[0], this.hover, 0);
      const l = jog && layoutDimension({ ...jog, ...look });
      if (l) {
        strokeLayout(g, view, l, pal.accent);
        drawTag(g, view.worldToScreen(this.hover), [this.ctx.view.dimensionText(l, look)], pal.accent, pal.labelHalo);
      }
      return;
    }
    // Yay uzunluğu's arc, and with Kısmi the part between its first point and the cursor, its length by the cursor.
    if (this.arc) {
      strokeArcEnds(g, view, arcLengthEnds(this.arc, null), pal.snap, 2);
      if (this.pts.length === 1 && this.hover) {
        const part = arcLengthEnds(this.arc, [this.pts[0], this.hover]);
        strokeArcEnds(g, view, part, pal.accent, 3);
        const l = part && layoutDimension({ ...part, offset: 0, height: this.height(), style: 'arcLength', ...look });
        if (l) drawTag(g, view.worldToScreen(this.hover), [this.ctx.format.length(l.value)], pal.accent, pal.labelHalo);
      }
      if (!this.placing) return;
    }
    if (this.placing && this.hover) {
      const d = this.geomAt(this.hover);
      const l = d && layoutDimension(d);
      if (!l) return;
      strokeLayout(g, view, l, pal.accent);
      drawTag(g, view.worldToScreen(this.hover), [this.ctx.view.dimensionText(l, look)], pal.accent, pal.labelHalo);
      return;
    }
    // Eğim waiting for an elevation: nothing on the cursor (no next point is asked for).
    if (this.asking >= 0) return;
    if (!this.picksEdge) super.draw(g, view);
  }
}

/** An arc length's arc (or its part) drawn from its ends about its centre. */
function strokeArcEnds(g: CanvasRenderingContext2D, view: ViewTransform, ends: { a: Vec2; b: Vec2; c: Vec2 } | null, color: string, width: number): void {
  if (!ends) return;
  const angle = (p: Vec2) => Math.atan2(p.y - ends.c.y, p.x - ends.c.x);
  strokeGeometry(g, view, { kind: 'arc', c: ends.c, r: dist(ends.c, ends.a), a0: angle(ends.a), a1: angle(ends.b) }, { color, width });
}
