import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { dist, type Vec2 } from '../model/geometry';
import { alongLine, clockwiseAngle, distanceIntersection, lineIntersection, sideOffsets, sidePoint } from '../model/geom/survey';
import type { ViewTransform } from '../viewport/Camera';
import { alongRatio, bisectorNearest, bisectorPoint, calcPolar, kmValue, midpoint, nearestOf, slopeHorizontal } from './constructions';
import { parseLength } from './coordinateInput';
import { namedPointAt } from './namedPoint';
import { calcNumber, km, kmAndOffset, pickRoute, routeAt, routeRead, type Route } from './pointCalcRoute';
import { drawTag, strokePath } from './preview';
import type { Tool, ToolPointer } from './Tool';

export type CalcKind = 'side' | 'distances' | 'lines' | 'along' | 'polar' | 'mid' | 'object' | 'km' | 'name' | 'slope' | 'bisector';

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
  // Nokta hesaplayıcı ekleri (docs/adr/0188).
  {
    kind: 'object',
    label: 'Obje üzerinde nokta',
    alias: 'OBJE',
    icon: 'calcObject',
    description:
      'Bir nesnenin yolunda, başlangıçtan uzaklık ve dik sapmayla (sağa artı) nokta. Çizgiye, yaya, daireye, elipse, eğriye ya da alana tıklayın (yakın uç başlangıçtır), “12.5” ya da “12.5,2” yazın.',
  },
  {
    kind: 'km',
    label: 'Km ve sapma',
    alias: 'KM',
    icon: 'calcKm',
    description: 'Güzergâhın km’siyle ve dik sapmasıyla (sağa artı) nokta; km ilk köşede Başlangıç’tır (B). Güzergâha tıklayın, “0+125.5” ya da “0+125.5,-3” yazın.',
  },
  {
    kind: 'name',
    label: 'Nokta adından',
    alias: 'NAD',
    icon: 'calcName',
    description: 'Adı bilinen noktanın yeri. Adını yazın: “P12” ya da “#P12”.',
  },
  {
    kind: 'slope',
    label: 'Mesafe ve eğim',
    alias: 'EGIM',
    icon: 'calcSlope',
    description: 'Eğik ölçülmüş mesafenin yatayıyla A’dan B’ye doğru nokta. A ve B’ye tıklayın, “eğik mesafe,eğim” yazın (eğim yüzde, ör. 25,8).',
  },
  {
    kind: 'bisector',
    label: 'Açıortay',
    alias: 'AO',
    icon: 'calcBisector',
    description: 'Bir açının ortayında, köşeden uzaklıkla nokta. Köşeye (K), sonra iki kolun birer noktasına (A, B) tıklayın; uzaklığı yazın ya da açıortayda tıklayın.',
  },
];

const TR_FOLD: Record<string, string> = { Ç: 'C', Ş: 'S', Ğ: 'G', Ö: 'O', Ü: 'U', İ: 'I' };
/** Turkish upper case without Turkish marks (`eğim` → `EGIM`). */
const foldTr = (text: string): string => [...text.toLocaleUpperCase('tr-TR')].map((c) => TR_FOLD[c] ?? c).join('');

/** The construction a typed alias names (YAN, kkes: Turkish upper case; then without Turkish marks, so “eğim” is EGIM). */
export function calcByAlias(text: string): (typeof CALC_KINDS)[number] | undefined {
  const t = text.trim().toLocaleUpperCase('tr-TR');
  return CALC_KINDS.find((k) => k.alias === t) ?? CALC_KINDS.find((k) => foldTr(k.alias) === foldTr(t));
}

const REFS: Record<CalcKind, string[]> = {
  side: ['hattın başlangıç noktası (A)', 'hattın doğrultu noktası (B)'],
  distances: ['birinci nokta (A)', 'ikinci nokta (B)'],
  lines: ['birinci doğrunun ilk noktası (A)', 'birinci doğrunun ikinci noktası (B)', 'ikinci doğrunun ilk noktası (C)', 'ikinci doğrunun ikinci noktası (D)'],
  along: ['hattın başlangıcı (A)', 'hattın sonu (B)'],
  polar: ['durulan nokta (S)', 'bakılan nokta (R)'],
  mid: ['birinci nokta', 'ikinci nokta'],
  slope: ['başlangıç noktası (A)', 'doğrultu noktası (B)'],
  bisector: ['açının köşesi (K)', 'birinci kolun noktası (A)', 'ikinci kolun noktası (B)'],
  // An object (Obje, Km) or nothing (Nokta adından): no point to show.
  object: [],
  km: [],
  name: [],
};
/** Obje üzerinde nokta and Km walk an object picked first. */
const walks = (kind: CalcKind): boolean => kind === 'object' || kind === 'km';
/** Why Açıortay has no bisector. */
const BISECTOR_CORNER = 'Köşe bir kolun noktasıyla çakışıyor; açıortay yok. Kolların köşeden ayrı noktalarını gösterin.';
const capitalize = (s: string): string => s.charAt(0).toLocaleUpperCase('tr-TR') + s.slice(1);
const LETTERS: Record<CalcKind, string[]> = {
  side: ['A', 'B'],
  distances: ['A', 'B'],
  lines: ['A', 'B', 'C', 'D'],
  along: ['A', 'B'],
  polar: ['S', 'R'],
  mid: ['1', '2'],
  slope: ['A', 'B'],
  bisector: ['K', 'A', 'B'],
  object: [],
  km: [],
  name: [],
};

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
  /** Km ve sapma's first km, metres, for the session (the desktop's `Memory::calc_km_start`; docs/adr/0188 §2). */
  static kmStart = 0;
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
  /** Obje üzerinde nokta and Km: the object walked, once picked. */
  private route: Route | null = null;
  /** Km's Başlangıç (B) asked: the next text typed is the route's first km. */
  private askingStart = false;
  private done = false;

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
    const f = this.ctx.format;
    const unitName = this.ctx.doc.settings.angleUnit.value === 'deg' ? 'derece' : 'grad';
    let step: string;
    if (this.candidates.length) step = 'iki çözümden istediğinize tıklayın [Sağdaki (Enter)]';
    else if (this.askingStart) step = `güzergâhın başındaki km’yi yazın (şimdi ${km(PointCalcTool.kmStart, f)})`;
    else if (walks(this.kind) && !this.route)
      step = this.kind === 'km' ? 'güzergâha tıklayın (km’si ilk köşesinde başlar)' : 'nesneye tıklayın (çizgi, yay, daire, elips, eğri ya da alan; yakın uç başlangıçtır)';
    else if (this.pts.length < refs.length) step = `${refs[this.pts.length]} gösterin`;
    else if (this.kind === 'side') step = 'dik ayak ve dik boyu yazın: absis,ordinat (ordinat sağa artı)';
    else if (this.kind === 'distances') step = 'A ve B noktalarına uzaklıkları yazın: d1,d2';
    else if (this.kind === 'along') step = 'A’dan uzaklığı yazın ya da a/b oranı (ör. 1/3); ya da hat üzerinde tıklayın';
    else if (this.kind === 'object') step = 'başlangıçtan uzaklığı yazın: uzaklık ya da uzaklık,sapma (sapma sağa artı); ya da nesnede tıklayın';
    else if (this.kind === 'km') step = 'km’yi yazın: km ya da km,sapma (sapma sağa artı); ya da güzergâhta tıklayın';
    else if (this.kind === 'name') step = 'noktanın adını yazın (P12 ya da #P12)';
    else if (this.kind === 'slope') step = 'eğik mesafeyi ve yüzde eğimi yazın: mesafe,eğim';
    else if (this.kind === 'bisector') step = 'köşeden uzaklığı yazın; ya da açıortayda tıklayın';
    else step = `açı (${unitName}, saat yönünde) ve mesafeyi yazın: açı,mesafe`;
    if (this.kind === 'km' && !this.askingStart && !this.candidates.length) step += ` [Başlangıç (B): ${km(PointCalcTool.kmStart, f)}]`;
    this.prompt.set(`${this.label}: ${step}`);
    this.ctx.view.requestOverlay();
  }

  snapFrom(): Vec2 | null {
    return this.pts.at(-1) ?? null;
  }

  /** References picked so far (the object walked counts). */
  get pointCount(): number {
    return this.pts.length + (this.route ? 1 : 0);
  }

  /**
   * Ctrl+Z while the calculator runs: newest first, Km's Başlangıç asked, the choice between two solutions (back to
   * typing the values), else the last reference picked (the object walked last). True even with nothing to take
   * back: the drawing is never undone under the suspended command; Esc leaves the calculator.
   */
  undoStep(): boolean {
    if (this.askingStart) this.askingStart = false;
    else if (this.candidates.length) this.candidates = [];
    else if (!this.pts.pop()) this.route = null;
    this.refresh();
    return true;
  }

  /** Esc while Km's Başlangıç is asked goes back to the value; else the calculator leaves. */
  cancel(): boolean {
    if (!this.askingStart) return false;
    this.askingStart = false;
    this.refresh();
    return true;
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0 || this.askingStart) return;
    if (walks(this.kind) && !this.route) {
      // The object under the cursor, not the snap's point (it may be a neighbour's).
      this.route = pickRoute(this.ctx, p.screen, p.raw, this.kind === 'object');
      return this.refresh();
    }
    this.click(p.world);
  }

  /**
   * A point given as if clicked (`#ad`, docs/adr/0188 §3): a reference, the place clicked once they are shown, or
   * Nokta adından's point; not where an object is asked for.
   */
  acceptPoint(p: Vec2): boolean {
    if (this.done || this.askingStart || (walks(this.kind) && !this.route)) return false;
    if (this.kind === 'name') this.finish(p);
    else this.click(p);
    return true;
  }

  /** A click once the step takes one: Kenar kesişimi's solution, a reference, or a place on a line, the object or the bisector. */
  private click(at: Vec2): void {
    if (this.candidates.length) return this.finish(nearestOf(this.candidates, at));
    const need = REFS[this.kind].length;
    if (this.pts.length < need) {
      if (this.pts.length && dist(this.pts.at(-1)!, at) < 1e-9) return;
      this.pts.push(at);
      if (this.pts.length === need) this.whenAllPicked();
      return this.refresh();
    }
    if (this.kind === 'along') {
      // Hat üzerinde: a click on the line places the point at its projection.
      const o = sideOffsets(this.pts[0], this.pts[1], at);
      if (o) this.finish(alongLine(this.pts[0], this.pts[1], o.absis));
    } else if (walks(this.kind) && this.route) {
      // The object's point nearest the click, no offset.
      const r = routeRead(this.route, at);
      if (r) this.finish(routeAt(this.route, r.s, 0, this.ctx.format).point);
    } else if (this.kind === 'bisector') {
      const q = bisectorNearest(this.pts[0], this.pts[1], this.pts[2], at);
      if (q) this.finish(q);
      else this.ctx.log.warn(BISECTOR_CORNER);
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

  /**
   * Whatever is typed while the calculator runs is its own: what it cannot
   * take is refused in its own words, saying what the step waits for, and
   * the step stays. References are shown on the drawing, never typed.
   */
  input(text: string): boolean {
    const t = text.trim();
    if (this.askingStart) {
      const v = kmValue(t);
      if (v === null) this.ctx.log.warn(`“${t}” anlaşılamadı. ${this.expected()}`);
      else {
        PointCalcTool.kmStart = v;
        this.askingStart = false;
        this.refresh();
      }
      return true;
    }
    // Km's Başlangıç, at any step.
    if (this.kind === 'km' && t.toLocaleUpperCase('tr-TR') === 'B') {
      this.askingStart = true;
      this.refresh();
      return true;
    }
    const taken =
      walks(this.kind) || this.kind === 'name' ? (this.kind === 'name' || !!this.route) && this.takeWalk(t) : this.pts.length >= REFS[this.kind].length && this.take(t);
    if (!taken) this.ctx.log.warn(`“${t}” anlaşılamadı. ${this.expected()}`);
    return true;
  }

  /** What the step waits for, as a refusal says it. */
  private expected(): string {
    const refs = REFS[this.kind];
    if (this.askingStart) return 'Güzergâhın başındaki km’yi yazın: k+mmm.mmm ya da metre (ör. 0+000).';
    if (walks(this.kind) && !this.route) return 'Nesneyi çizimde gösterin: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da alan.';
    if (this.pts.length < refs.length) return `${capitalize(refs[this.pts.length])} çizimde gösterin.`;
    if (this.candidates.length) return 'İki çözümden istediğinize tıklayın ya da Enter’la sağdakini alın.';
    switch (this.kind) {
      case 'side':
        return 'Dik ayak ve dik boyu yazın: absis,ordinat (ör. 12.5,3).';
      case 'distances':
        return 'A ve B noktalarına uzaklıkları yazın: d1,d2 (ör. 10,8).';
      case 'along':
        return 'A’dan uzaklığı yazın ya da a/b oranı (ör. 1/3).';
      case 'object':
        return 'Başlangıçtan uzaklığı yazın, sapmayla da: uzaklık,sapma (ör. 12.5,2).';
      case 'km':
        return `Km’yi yazın, sapmayla da: km,sapma (ör. ${km(PointCalcTool.kmStart + 12.5, this.ctx.format)},2).`;
      case 'name':
        return 'Noktanın adını yazın (ör. P12 ya da #P12).';
      case 'slope':
        return 'Eğik mesafeyi ve yüzde eğimi yazın: mesafe,eğim (ör. 25,8).';
      case 'bisector':
        return 'Köşeden açıortay boyunca uzaklığı yazın ya da açıortayda tıklayın.';
      default: {
        // Açı ve mesafe (İki nokta ortası and Doğru kesişimi take no value: they finish on their last reference).
        const deg = this.ctx.doc.settings.angleUnit.value === 'deg';
        return `Açı (${deg ? 'derece' : 'grad'}, saat yönünde) ve mesafeyi yazın: açı,mesafe (ör. ${deg ? '90' : '100'},25).`;
      }
    }
  }

  /** Takes a value typed once every reference is shown; false when the kind cannot read it. */
  private take(t: string): boolean {
    const pairOf = (text: string) => text.match(/^(-?\d+(?:\.\d+)?)\s*[,; ]\s*(-?\d+(?:\.\d+)?)$/);
    const pair = pairOf(t);
    const [a, b] = this.pts;
    // Lengths are typed in the project's unit (docs/adr/0165 §2).
    const m = (typed: string) => this.ctx.format.toMetres(+typed);
    switch (this.kind) {
      case 'side':
        if (!pair) return false;
        this.finish(sidePoint(a, b, m(pair[1]), m(pair[2])));
        return true;
      case 'distances': {
        if (!pair) return false;
        const sol = distanceIntersection(a, b, m(pair[1]), m(pair[2]));
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
        const n = parseLength(this.ctx.format, t);
        if (n === null) return false;
        this.finish(alongLine(a, b, n));
        return true;
      }
      case 'polar': {
        if (!pair) return false;
        this.finish(calcPolar(a, b, +pair[1], this.ctx.doc.settings.angleUnit.value, m(pair[2])));
        return true;
      }
      case 'slope': {
        // A % after the slope is read too.
        const sp = pairOf(t.replace(/%$/, '').trim());
        if (!sp) return false;
        const f = this.ctx.format;
        const h = slopeHorizontal(m(sp[1]), +sp[2]);
        this.ctx.log.info(`Yatay uzaklık ${f.length(h.horizontal)}, yükseklik farkı ${f.length(h.rise)}.`);
        this.finish(alongLine(a, b, h.horizontal));
        return true;
      }
      case 'bisector': {
        const d = calcNumber(t);
        if (d === null) return false;
        const q = bisectorPoint(a, b, this.pts[2], this.ctx.format.toMetres(d));
        if (q) this.finish(q);
        else this.ctx.log.warn(BISECTOR_CORNER);
        return true;
      }
      default:
        return false;
    }
  }

  /** A value typed for Obje üzerinde nokta, Km ve sapma or Nokta adından: false when the kind cannot read it. */
  private takeWalk(t: string): boolean {
    const f = this.ctx.format;
    if (this.kind === 'name') {
      const name = t.replace(/^#/, '').trim();
      if (!name) return false;
      const at = namedPointAt(this.ctx, name);
      if (typeof at === 'string') this.ctx.log.warn(at);
      else this.finish(at);
      return true;
    }
    const route = this.route;
    if (!route) return false;
    const start = PointCalcTool.kmStart;
    let s: number;
    let offset: number;
    if (this.kind === 'object') {
      const pair = t.match(/^(-?\d+(?:\.\d+)?)\s*[,; ]\s*(-?\d+(?:\.\d+)?)$/);
      const single = calcNumber(t);
      if (pair) [s, offset] = [f.toMetres(+pair[1]), f.toMetres(+pair[2])];
      else if (single !== null) [s, offset] = [f.toMetres(single), 0];
      else return false;
    } else {
      const k = kmAndOffset(t);
      if (!k) return false;
      // The km is metres whatever the project's unit (§2); the offset is in it.
      [s, offset] = [k.km - start, f.toMetres(k.offset)];
    }
    const at = routeAt(route, s, offset, f);
    if (at.point) this.finish(at.point);
    else if (this.kind === 'km') this.ctx.log.warn(`Km ${km(start, f)} ile ${km(start + at.length, f)} arasında olmalı.`);
    else this.ctx.log.warn(`Uzaklık 0 ile yolun uzunluğu ${f.length(at.length)} arasında olmalı.`);
    return true;
  }

  confirm(): void {
    if (this.candidates.length) return this.finish(this.candidates[0]);
    this.ctx.tools.unnest(null);
  }

  private finish(p: Vec2 | null): void {
    if (!p) return this.ctx.log.warn('Nokta hesaplanamadı: referans noktaları çakışıyor.');
    this.ctx.log.info(`Hesaplanan nokta: ${this.ctx.format.point(p)}`);
    this.done = true;
    this.ctx.tools.unnest(p);
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const mark = (p: Vec2, text: string, radius = 4) => {
      const s = view.worldToScreen(p);
      g.save();
      g.strokeStyle = pal.snap;
      g.lineWidth = 1.5;
      g.beginPath();
      g.arc(s.x, s.y, radius, 0, Math.PI * 2);
      g.stroke();
      g.fillStyle = pal.snap;
      g.font = '600 11px Barlow, system-ui, sans-serif';
      if (text) g.fillText(text, s.x + 7, s.y - 6);
      g.restore();
    };
    this.pts.forEach((p, i) => mark(p, LETTERS[this.kind][i]));
    // The walked object's start: A, or Km's first km.
    if (this.route) mark(this.route.start, this.kind === 'km' ? km(PointCalcTool.kmStart, f) : 'A');
    const [a, b, c] = this.pts;
    const reach = (Math.hypot(view.width, view.height) / view.scale) * 2;
    const far = (p: Vec2, q: Vec2) => {
      // The reference line runs well past its points (lines are unbounded here).
      const l = dist(p, q) || 1;
      return [{ x: p.x - ((q.x - p.x) / l) * reach, y: p.y - ((q.y - p.y) / l) * reach }, { x: p.x + ((q.x - p.x) / l) * reach, y: p.y + ((q.y - p.y) / l) * reach }];
    };
    if (a && b && this.kind !== 'mid' && this.kind !== 'bisector')
      strokePath(g, view, this.kind === 'lines' || this.kind === 'side' || this.kind === 'along' || this.kind === 'slope' ? far(a, b) : [a, b], { color: pal.snap, dash: [4, 4] });
    // Açıortay: its arms as shown, the bisector once both are.
    if (this.kind === 'bisector' && a) {
      for (const arm of [b, c]) if (arm) strokePath(g, view, [a, arm], { color: pal.snap, dash: [4, 4] });
      const ray = b && c ? bisectorPoint(a, b, c, reach) : null;
      if (ray) strokePath(g, view, [a, ray], { color: pal.snap, dash: [2, 3] });
    }
    if (c && this.hover && this.kind === 'lines') strokePath(g, view, far(c, this.hover), { color: pal.snap, dash: [4, 4] });
    for (const q of this.candidates) mark(q, '?');
    const h = this.hover;
    if (!h || this.askingStart) return;
    if (this.route) {
      // The object's point under the cursor and the offset to it.
      const r = routeRead(this.route, h);
      if (!r) return;
      const on = routeAt(this.route, r.s, 0, f).point;
      if (on) {
        mark(on, '', 3);
        strokePath(g, view, [on, h], { color: pal.snap, dash: [2, 3] });
      }
      const first = this.kind === 'km' ? `Km ${km(PointCalcTool.kmStart + r.s, f)}` : `Başlangıçtan ${f.length(r.s)}`;
      return drawTag(g, view.worldToScreen(h), [first, `Sapma ${f.length(r.offset)}`], pal.snap, pal.labelHalo);
    }
    if (this.kind === 'bisector') {
      if (a && b && c) {
        const on = bisectorNearest(a, b, c, h);
        if (on) {
          mark(on, '', 3);
          drawTag(g, view.worldToScreen(h), [`K’dan ${f.length(dist(a, on))}`], pal.snap, pal.labelHalo);
        }
      } else if (a) strokePath(g, view, [a, h], { color: pal.snap, dash: [2, 3] });
      return;
    }
    if (a && !b) strokePath(g, view, [a, h], { color: pal.snap, dash: [2, 3] });
    if (!a || !b || this.candidates.length) return;
    // Live readings of the cursor, in the terms the values are typed.
    const lines: string[] = [];
    if (this.kind === 'side') {
      const o = sideOffsets(a, b, h);
      if (o) lines.push(`Dik ayak ${f.length(o.absis)}`, `Dik boy ${f.length(o.ordinat)}`);
    } else if (this.kind === 'along' || this.kind === 'slope') {
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
