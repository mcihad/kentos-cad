import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { dist, type Vec2 } from '../model/geometry';
import { alongLine, clockwiseAngle, distanceIntersection, lineIntersection, sideOffsets, sidePoint } from '../model/geom/survey';
import type { ViewTransform } from '../viewport/Camera';
import { alongRatio, calcPolar, midpoint, nearestOf } from './constructions';
import { parseNumber } from './coordinateInput';
import { drawTag, strokePath } from './preview';
import type { Tool, ToolPointer } from './Tool';

export type CalcKind = 'side' | 'distances' | 'lines' | 'along' | 'polar' | 'mid';

/**
 * Netcad's "Koordinat hesap makinası": point constructions a command can
 * use whenever it waits for a point. Each is a transparent tool: it picks
 * its references on the drawing (with snaps), takes the typed values and
 * hands the computed point back to the command as if clicked.
 */
export const CALC_KINDS: { kind: CalcKind; label: string; alias: string; icon: string; description: string }[] = [
  {
    kind: 'side',
    label: 'Yan nokta (dik ayak, dik boy)',
    alias: 'YAN',
    icon: 'calcSide',
    description: 'Ölçü krokisindeki gibi: bir hat boyunca dik ayak, ona dik dik boy (sağa artı). A ve B’ye tıklayın, “12.5,3” yazın.',
  },
  {
    kind: 'distances',
    label: 'Kenar kesişimi',
    alias: 'KKES',
    icon: 'calcDistances',
    description: 'İki noktaya uzaklığı bilinen nokta (şeritle ölçülmüş köşe). A ve B’ye tıklayın, “d1,d2” yazın, iki çözümden birine tıklayın.',
  },
  {
    kind: 'lines',
    label: 'Doğru kesişimi (4 nokta)',
    alias: 'DKES',
    icon: 'calcLines',
    description: 'İki doğrunun, uzantıları dahil, kesiştiği nokta. Birinci doğrunun iki noktasına, sonra ikincinin iki noktasına tıklayın.',
  },
  {
    kind: 'along',
    label: 'Hat üzerinde nokta',
    alias: 'HAT',
    icon: 'calcAlong',
    description: 'A–B hattı üzerinde, A’dan uzaklıkla ya da oranla (1/3) nokta. A ve B’ye tıklayın, uzaklığı yazın ya da hatta tıklayın.',
  },
  {
    kind: 'polar',
    label: 'Açı ve mesafe',
    alias: 'AM',
    icon: 'calcPolar',
    description: 'Takeometre gibi: durulan noktadan (S), bakılan noktaya (R) göre saat yönünde açı ve mesafe. S ve R’ye tıklayın, “açı,mesafe” yazın.',
  },
  {
    kind: 'mid',
    label: 'İki nokta ortası',
    alias: 'ORTA',
    icon: 'calcMid',
    description: 'İki noktanın tam ortası. İki noktaya tıklayın.',
  },
];

const REFS: Record<CalcKind, string[]> = {
  side: ['hattın başlangıç noktası (A)', 'hattın doğrultu noktası (B)'],
  distances: ['birinci nokta (A)', 'ikinci nokta (B)'],
  lines: ['birinci doğrunun ilk noktası (A)', 'birinci doğrunun ikinci noktası (B)', 'ikinci doğrunun ilk noktası (C)', 'ikinci doğrunun ikinci noktası (D)'],
  along: ['hattın başlangıcı (A)', 'hattın sonu (B)'],
  polar: ['durulan nokta (S)', 'bakılan nokta (R)'],
  mid: ['birinci nokta', 'ikinci nokta'],
};
const LETTERS: Record<CalcKind, string[]> = { side: ['A', 'B'], distances: ['A', 'B'], lines: ['A', 'B', 'C', 'D'], along: ['A', 'B'], polar: ['S', 'R'], mid: ['1', '2'] };

/** Whether the running command can take a calculated point now. */
export function canCalcPoint(ctx: AppContext): boolean {
  const t = ctx.tools.active;
  return !ctx.tools.nested && !!t.acceptPoint && t.snaps && (t.id !== 'select' || !!t.activeGrip?.());
}

export function startPointCalc(ctx: AppContext, kind: CalcKind): void {
  if (!canCalcPoint(ctx)) return ctx.log.warn('Nokta hesabı, nokta bekleyen bir komut sırasında kullanılır.');
  const def = CALC_KINDS.find((k) => k.kind === kind)!;
  ctx.tools.nest(new PointCalcTool(ctx, kind), `Nokta hesabı: ${def.label}`);
}

class PointCalcTool implements Tool {
  readonly id = 'pointCalc';
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  readonly snaps = true;
  private readonly ctx: AppContext;
  private readonly kind: CalcKind;
  private pts: Vec2[] = [];
  private hover: Vec2 | null = null;
  /** Kenar kesişimi: both solutions, waiting for the user to pick one. */
  private candidates: Vec2[] = [];

  constructor(ctx: AppContext, kind: CalcKind) {
    this.ctx = ctx;
    this.kind = kind;
  }

  private get label(): string {
    return CALC_KINDS.find((k) => k.kind === this.kind)!.label;
  }

  activate(): void {
    this.refresh();
  }

  private refresh(): void {
    const refs = REFS[this.kind];
    const unitName = this.ctx.doc.settings.angleUnit.value === 'deg' ? 'derece' : 'grad';
    let step: string;
    if (this.candidates.length) step = 'iki çözümden istediğinize tıklayın [Sağdaki (Enter)]';
    else if (this.pts.length < refs.length) step = `${refs[this.pts.length]} gösterin`;
    else if (this.kind === 'side') step = 'dik ayak ve dik boyu yazın: absis,ordinat (ordinat sağa artı)';
    else if (this.kind === 'distances') step = 'A ve B noktalarına uzaklıkları yazın: d1,d2';
    else if (this.kind === 'along') step = 'A’dan uzaklığı yazın ya da a/b oranı (ör. 1/3); ya da hat üzerinde tıklayın';
    else step = `açı (${unitName}, saat yönünde) ve mesafeyi yazın: açı,mesafe`;
    this.prompt.set(`${this.label}: ${step}`);
    this.ctx.view.requestOverlay();
  }

  snapFrom(): Vec2 | null {
    return this.pts.at(-1) ?? null;
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.candidates.length) return this.finish(nearestOf(this.candidates, p.world));
    const need = REFS[this.kind].length;
    if (this.pts.length < need) {
      if (this.pts.length && dist(this.pts.at(-1)!, p.world) < 1e-9) return;
      this.pts.push(p.world);
      if (this.pts.length === need) this.whenAllPicked();
      return this.refresh();
    }
    // Hat üzerinde: a click on the line places the point at its projection.
    if (this.kind === 'along') {
      const o = sideOffsets(this.pts[0], this.pts[1], p.world);
      if (o) this.finish(alongLine(this.pts[0], this.pts[1], o.absis));
    }
  }

  private whenAllPicked(): void {
    const [a, b, c, d] = this.pts;
    if (this.kind === 'mid') this.finish(midpoint(a, b));
    else if (this.kind === 'lines') {
      const x = lineIntersection(a, b, c, d);
      if (x) this.finish(x);
      else {
        this.ctx.log.warn('Doğrular paralel; kesişim yok. Başka iki nokta gösterin.');
        this.pts = this.pts.slice(0, 2);
      }
    }
  }

  input(text: string): boolean {
    const t = text.trim();
    if (this.pts.length < REFS[this.kind].length) return false;
    const pair = t.match(/^(-?\d+(?:\.\d+)?)\s*[,; ]\s*(-?\d+(?:\.\d+)?)$/);
    const [a, b] = this.pts;
    switch (this.kind) {
      case 'side':
        if (!pair) return false;
        this.finish(sidePoint(a, b, +pair[1], +pair[2]));
        return true;
      case 'distances': {
        if (!pair) return false;
        const sol = distanceIntersection(a, b, +pair[1], +pair[2]);
        if (!sol.length) {
          this.ctx.log.warn('Bu uzaklıklarla kesişim yok: iki uzaklığın toplamı A–B aralığından küçük ya da farkı büyük.');
          return true;
        }
        if (sol.length === 1) this.finish(sol[0]);
        else {
          this.candidates = sol;
          this.refresh();
        }
        return true;
      }
      case 'along': {
        const ratio = t.match(/^(\d+(?:\.\d+)?)\s*\/\s*(\d+(?:\.\d+)?)$/);
        if (ratio && +ratio[2] > 0) {
          this.finish(alongRatio(a, b, +ratio[1], +ratio[2]));
          return true;
        }
        const n = parseNumber(t);
        if (n === null) return false;
        this.finish(alongLine(a, b, n));
        return true;
      }
      case 'polar': {
        if (!pair) return false;
        this.finish(calcPolar(a, b, +pair[1], this.ctx.doc.settings.angleUnit.value, +pair[2]));
        return true;
      }
      default:
        return false;
    }
  }

  confirm(): void {
    if (this.candidates.length) return this.finish(this.candidates[0]);
    this.ctx.tools.unnest(null);
  }

  private finish(p: Vec2 | null): void {
    if (!p) return this.ctx.log.warn('Nokta hesaplanamadı: referans noktaları çakışıyor.');
    this.ctx.log.info(`Hesaplanan nokta: ${this.ctx.format.point(p)}`);
    this.ctx.tools.unnest(p);
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const mark = (p: Vec2, text: string) => {
      const s = view.worldToScreen(p);
      g.save();
      g.strokeStyle = pal.snap;
      g.lineWidth = 1.5;
      g.beginPath();
      g.arc(s.x, s.y, 4, 0, Math.PI * 2);
      g.stroke();
      g.fillStyle = pal.snap;
      g.font = '600 11px Barlow, system-ui, sans-serif';
      g.fillText(text, s.x + 7, s.y - 6);
      g.restore();
    };
    this.pts.forEach((p, i) => mark(p, LETTERS[this.kind][i]));
    const [a, b, c] = this.pts;
    const far = (p: Vec2, q: Vec2) => {
      // The reference line runs well past its points (lines are unbounded here).
      const r = (Math.hypot(view.width, view.height) / view.scale) * 2;
      const l = dist(p, q) || 1;
      return [{ x: p.x - ((q.x - p.x) / l) * r, y: p.y - ((q.y - p.y) / l) * r }, { x: p.x + ((q.x - p.x) / l) * r, y: p.y + ((q.y - p.y) / l) * r }];
    };
    if (a && b && this.kind !== 'mid') strokePath(g, view, this.kind === 'lines' || this.kind === 'side' || this.kind === 'along' ? far(a, b) : [a, b], { color: pal.snap, dash: [4, 4] });
    if (c && this.hover && this.kind === 'lines') strokePath(g, view, far(c, this.hover), { color: pal.snap, dash: [4, 4] });
    for (const q of this.candidates) mark(q, '?');
    const h = this.hover;
    if (!h) return;
    if (a && !b) strokePath(g, view, [a, h], { color: pal.snap, dash: [2, 3] });
    if (!a || !b || this.candidates.length) return;
    // Live readings of the cursor, in the terms the values are typed.
    const lines: string[] = [];
    if (this.kind === 'side') {
      const o = sideOffsets(a, b, h);
      if (o) lines.push(`Dik ayak ${f.length(o.absis)}`, `Dik boy ${f.length(o.ordinat)}`);
    } else if (this.kind === 'along') {
      const o = sideOffsets(a, b, h);
      if (o) lines.push(`A’dan ${f.length(o.absis)}`);
    } else if (this.kind === 'polar') {
      const grad = (clockwiseAngle(a, b, h) * 200) / Math.PI;
      lines.push(`Açı ${f.bearing(grad)}`, `Mesafe ${f.length(dist(a, h))}`);
      strokePath(g, view, [a, h], { color: pal.snap, dash: [2, 3] });
    } else if (this.kind === 'distances') lines.push(`A’ya ${f.length(dist(a, h))}`, `B’ye ${f.length(dist(b, h))}`);
    if (lines.length) drawTag(g, view.worldToScreen(h), lines, pal.snap, pal.labelHalo);
  }
}
