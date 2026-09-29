import { dist, type Vec2 } from '../model/geometry';
import { lineLine } from '../model/geom/intersect';
import { bearingBearing, distanceDistance } from '../model/ops/construct';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { lineReach, plusMark, ringMark } from './constructPreview';
import { PointInputTool } from './drawTools';
import { drawTag, strokeGeometry, strokePath } from './preview';
import type { ToolPointer } from './Tool';
import { pointFromText } from './tracking';

type Method = 'distances' | 'bearings' | 'lines';
/** What an answer is: a point, or the number that belongs to the point before it. */
type Kind = 'point' | 'value';

const SAME = 1e-9;
const DASH = [5, 3];
const TAU = Math.PI * 2;

/** The answers of a method, in the order they are given. */
const SEQUENCE: Record<Method, readonly Kind[]> = {
  distances: ['point', 'value', 'point', 'value'],
  bearings: ['point', 'value', 'point', 'value'],
  lines: ['point', 'point', 'point', 'point'],
};

/** The survey bearing from `from` to `to`, radians from grid north, clockwise. */
const bearingOf = (from: Vec2, to: Vec2) => (((Math.atan2(to.x - from.x, to.y - from.y) % TAU) + TAU) % TAU);

/** Of the points, the one nearest to `at`. */
const nearest = (points: readonly Vec2[], at: Vec2): Vec2 | null => points.reduce<Vec2 | null>((best, p) => (!best || dist(p, at) < dist(best, at) ? p : best), null);

/**
 * Kesişim noktası (docs/adr/0140): a point found from what is known of it, three ways.
 * The ribbon starts a method by sending its letter right after the tool, and the same
 * letters work typed on the command line: `U`, `D`, `L`.
 *
 * - İki uzaklık (the default): a known point A and the distance from it, a known point
 *   B and the distance from it. A distance is typed or shown by clicking on its circle.
 *   The circles meet in up to two points: the click near the wanted one chooses it,
 *   Enter takes the one to the right of A towards B; a single meeting point (touching
 *   circles) is taken at once. Enter at a distance takes the one kept from last time.
 * - İki doğrultu (`D`): A and its direction, B and its direction. A direction is a
 *   survey bearing (from grid north, clockwise) typed in the project's angle unit, or
 *   shown by clicking along it. The meeting must lie ahead of both points.
 * - İki doğru (`L`): the four points of two lines, the first pair and the second; the
 *   lines meet wherever they extend to.
 *
 * What is picked so far is drawn live, the meeting too as soon as it exists. A meeting
 * that does not exist is said and the last answer asked again. Esc steps back one
 * answer; Enter with something begun and nothing to take starts over. The point goes on
 * the active layer through `cad.entities.create`, one undo step (“Kesişim noktası”); the message says its Y
 * and X. The meetings are the shared core's.
 */
export class IntersectPointTool extends PointInputTool {
  readonly id = 'intersectPoint';
  protected readonly label = 'Kesişim noktası';
  /** The distance last given, which Enter takes at a distance. */
  private static lastDistance = 10;
  private method: Method = 'distances';
  /** The distances (metres) or the bearings (radians from north, clockwise) given. */
  private vals: number[] = [];
  /** İki uzaklık: the two meeting points waiting for the choice. */
  private cands: Vec2[] = [];

  override get snaps(): boolean {
    return this.cands.length === 0;
  }

  private get taken(): number {
    return this.pts.length + this.vals.length;
  }

  private get fresh(): boolean {
    return this.taken === 0;
  }

  /** What is asked next; none once all is given (the choice, or nothing). */
  private get next(): Kind | null {
    return this.cands.length ? null : (SEQUENCE[this.method][this.taken] ?? null);
  }

  /** Which of the two points (0 or 1) a value or a point being asked belongs to. */
  private get index(): number {
    return Math.floor(this.taken / 2);
  }

  /** The value the cursor stands for at a value step: a distance from its point, or the bearing towards the cursor from it. */
  private valueAt(at: Vec2): number | null {
    const from = this.pts[this.vals.length];
    if (!from) return null;
    return this.method === 'bearings' ? bearingOf(from, at) : dist(from, at);
  }

  /** The meeting points of the answers so far, the cursor completing the one being asked. */
  private meetings(hover: Vec2 | null): Vec2[] {
    const vals = [...this.vals];
    const pts = [...this.pts];
    if (this.next === 'value' && hover) {
      const v = this.valueAt(hover);
      if (v === null) return [];
      vals.push(v);
    } else if (this.next === 'point' && hover && this.method === 'lines') pts.push(hover);
    return this.solveFor(pts, vals);
  }

  private solveFor(pts: readonly Vec2[], vals: readonly number[]): Vec2[] {
    if (this.method === 'lines') {
      if (pts.length < 4) return [];
      const hit = lineLine(pts[0], pts[1], pts[2], pts[3]);
      return hit ? [hit.p] : [];
    }
    if (pts.length < 2 || vals.length < 2) return [];
    if (this.method === 'distances') return distanceDistance(pts[0], vals[0], pts[1], vals[1]);
    const p = bearingBearing(pts[0], vals[0], pts[1], vals[1]);
    return p ? [p] : [];
  }

  // ── Prompt ────────────────────────────────────────────────────────────

  /** The other methods, while nothing is given, as options. */
  private options(): string {
    if (!this.fresh) return '';
    const all: [Method, string, string][] = [['distances', 'İki uzaklık', 'U'], ['bearings', 'İki doğrultu', 'D'], ['lines', 'İki doğru', 'L']];
    return ` [${all.filter(([m]) => m !== this.method).map(([, label, key]) => `${label} (${key})`).join(' / ')}]`;
  }

  protected promptFor(): string {
    const f = this.ctx.format;
    if (this.cands.length) return 'istediğiniz kesişime tıklayın [Sağdaki (Enter)]';
    const letter = this.index === 0 ? 'A' : 'B';
    if (this.method === 'lines') {
      const which = ['ilk', 'ikinci'][this.taken % 2];
      return `${Math.floor(this.taken / 2) + 1}. doğrunun ${which} noktasını belirtin${this.options()}`;
    }
    if (this.next === 'point') return `${letter} noktasını belirtin${this.options()}`;
    if (this.method === 'distances') return `${letter} noktasından uzaklığı yazın ya da çember üzerinde tıklayın (Enter: ${f.length(IntersectPointTool.lastDistance)})`;
    return `${letter} noktasından doğrultuyu yazın (${f.angleUnitName}, kuzeyden saat yönünde) ya da gösterin`;
  }

  // ── Answers ───────────────────────────────────────────────────────────

  /** A click or a computed point: `world` is where it snapped, `raw` where the pointer is (the choice between two meetings goes by the pointer). */
  private take(world: Vec2, raw: Vec2): void {
    const { log } = this.ctx;
    if (this.cands.length) return this.place(nearest(this.cands, raw));
    if (this.next === 'point') {
      log.info(`  ${this.ctx.format.point(world)}`);
      return this.point(world);
    }
    if (this.next === 'value') {
      const v = this.valueAt(world);
      const from = this.pts[this.vals.length];
      if (v === null || !from || dist(from, world) < SAME) return void log.warn('Nokta ilk noktayla çakışıyor; başka bir yer gösterin ya da değeri yazın.');
      this.value(v);
    }
  }

  private point(p: Vec2): void {
    // A second point on the first makes no line and no second circle centre.
    const pairStart = this.method === 'lines' ? this.pts.length % 2 === 1 : this.pts.length === 1;
    if (pairStart && this.last && dist(this.last, p) < SAME) return void this.ctx.log.warn('Nokta öncekiyle çakışıyor; başka bir yer gösterin.');
    this.pts.push(p);
    if (this.method === 'lines' && this.pts.length === 4) this.solveLines();
  }

  /** A distance or a bearing given for the point before it. */
  private value(v: number): void {
    const f = this.ctx.format;
    if (this.method === 'distances') IntersectPointTool.lastDistance = v;
    this.vals.push(v);
    this.ctx.log.info(this.method === 'bearings' ? `  Semt ${f.angle(v)}` : `  Uzaklık ${f.length(v)}`);
    if (this.vals.length === 2) this.solve();
  }

  /** Both answers of İki uzaklık and İki doğrultu are in. */
  private solve(): void {
    const [a, b] = this.pts;
    const [va, vb] = this.vals;
    const f = this.ctx.format;
    if (this.method === 'distances') {
      const meets = distanceDistance(a, va, b, vb);
      if (!meets.length) {
        this.ctx.log.warn(
          `İki uzaklık kesişmiyor: A ile B arası ${f.length(dist(a, b))}, uzaklıklar ${f.length(va)} ve ${f.length(vb)}. Kesişmeleri için toplamları en az ${f.length(dist(a, b))}, farkları en çok ${f.length(dist(a, b))} olmalı; ikinci uzaklığı değiştirin.`,
        );
        this.vals.pop();
      } else if (meets.length === 1) this.place(meets[0]);
      else this.cands = meets;
      return;
    }
    const p = bearingBearing(a, va, b, vb);
    if (p) return this.place(p);
    this.ctx.log.warn('Doğrultular kesişmiyor: paralel ya da kesişim noktalardan birinin arkasında kalıyor. İkinci doğrultuyu değiştirin.');
    this.vals.pop();
  }

  private solveLines(): void {
    const [p] = this.solveFor(this.pts, []);
    if (p) return this.place(p);
    this.ctx.log.warn('Doğrular paralel; kesişmiyorlar. Dördüncü noktayı değiştirin.');
    this.pts.pop();
  }

  /** Writes the point, and asks for the next one. */
  private place(at: Vec2 | null): void {
    if (!at) return;
    const { log, format } = this.ctx;
    const out = this.writeObjects([{ kind: 'point', p: at }], 'intersectPoint');
    if (out) log.success(`Kesişim noktası kondu: ${format.point(at)}`);
    this.pts = [];
    this.vals = [];
    this.cands = [];
  }

  /** Back one answer; false when nothing was given. */
  private back(): boolean {
    if (this.cands.length) {
      this.cands = [];
      this.vals.pop();
    } else if (this.taken === 0) return false;
    else if (SEQUENCE[this.method][this.taken - 1] === 'point') this.pts.pop();
    else this.vals.pop();
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  // ── Pointer and keys ──────────────────────────────────────────────────

  override pointerMove(p: ToolPointer): void {
    this.hover = this.cands.length ? p.raw : this.next === 'point' ? this.constrain(p) : p.world;
    this.ctx.view.requestOverlay();
  }

  override pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    this.take(this.next === 'point' ? this.constrain(p) : p.world, p.raw);
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  override acceptPoint(p: Vec2): boolean {
    if (this.cands.length) return false;
    this.take(p, p);
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  protected onPoint(): void {}

  private switchTo(method: Method): true {
    this.method = method;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  protected override option(key: string): boolean {
    if (key === 'G') return this.back();
    if (!this.fresh) return false;
    if (key === 'U') return this.switchTo('distances');
    if (key === 'D') return this.switchTo('bearings');
    if (key === 'L') return this.switchTo('lines');
    return false;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    let handled = false;
    if (this.next === 'value') {
      const n = /[,;@<]/.test(text) ? null : parseNumber(text);
      if (n === null) return false;
      if (this.method === 'distances') {
        if (n <= 0) this.ctx.log.warn('Uzaklık sıfırdan büyük olmalı.');
        else this.value(n);
      } else this.value((((this.ctx.format.angleFromTyped(n) % TAU) + TAU) % TAU));
      handled = true;
    } else if (this.next === 'point') {
      const p = pointFromText(this.ctx, text, this.last, this.hover);
      if (!p) return false;
      this.take(p, p);
      handled = true;
    }
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return handled;
  }

  /** The choice: the meeting right of A towards B. A distance: the one kept. Otherwise what is begun starts over, and with nothing begun the tool leaves. */
  override confirm(): void {
    if (this.cands.length === 2) {
      const [a, b] = this.pts;
      const [p, q] = this.cands;
      const right = (c: Vec2) => (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) < 0;
      this.place(right(p) ? p : q);
    } else if (this.method === 'distances' && this.next === 'value') this.value(IntersectPointTool.lastDistance);
    else if (this.fresh) return this.ctx.tools.exit();
    else {
      this.pts = [];
      this.vals = [];
    }
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  cancel(): boolean {
    return this.back();
  }

  // ── Preview ───────────────────────────────────────────────────────────

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const hover = this.hover;
    const reach = lineReach(this.ctx);
    let tag: string[] = [];
    // Where each answer stands: solid once given, dashed under the cursor.
    this.pts.forEach((p, i) => {
      ringMark(g, view, p, pal.snap);
      if (this.method === 'lines') return;
      const given = this.vals.length > i;
      const live = this.vals.length === i && this.next === 'value';
      const v = given ? this.vals[i] : live && hover ? this.valueAt(hover) : null;
      if (v === null) return;
      const dash = given ? undefined : DASH;
      if (this.method === 'distances') {
        strokeGeometry(g, view, { kind: 'circle', c: p, r: v }, { color: pal.snap, ...(dash && { dash }), width: 1.25 });
        if (live && hover) {
          strokePath(g, view, [p, hover], { color: pal.snap });
          tag = [`Uzaklık ${f.length(v)}`];
        }
      } else {
        strokePath(g, view, [p, { x: p.x + Math.sin(v) * reach, y: p.y + Math.cos(v) * reach }], { color: pal.snap, ...(dash && { dash }), width: 1.25 });
        if (live && hover) tag = [`Semt ${f.angle(v)}`];
      }
    });
    if (this.method === 'lines') this.drawLines(g, view, hover, reach);
    // What the answers make: the meetings, the nearer one to the cursor marked.
    const meets = this.cands.length ? this.cands : this.meetings(hover);
    const near = hover ? nearest(meets, hover) : null;
    for (const m of meets) (near === m ? plusMark : ringMark)(g, view, m, pal.accent, near === m ? 7 : 6);
    if (near && hover) tag = [...tag, `Y ${f.coord(near.x)}`, `X ${f.coord(near.y)}`];
    if (tag.length && hover) drawTag(g, view.worldToScreen(hover), tag, pal.accent, pal.labelHalo);
    this.drawTracking(g, view);
  }

  /** İki doğru: each pair drawn as the whole line, solid once its two points are given. */
  private drawLines(g: CanvasRenderingContext2D, view: ViewTransform, hover: Vec2 | null, reach: number): void {
    const pal = this.ctx.view.palette;
    const pts = this.next === 'point' && hover ? [...this.pts, hover] : this.pts;
    for (let i = 0; i + 1 < pts.length; i += 2) {
      const [p, q] = [pts[i], pts[i + 1]];
      const l = Math.max(dist(p, q), SAME);
      const [ux, uy] = [((q.x - p.x) / l) * reach, ((q.y - p.y) / l) * reach];
      const given = i + 2 <= this.pts.length;
      strokePath(g, view, [{ x: p.x - ux, y: p.y - uy }, { x: p.x + ux, y: p.y + uy }], { color: pal.snap, ...(!given && { dash: DASH }), width: 1.25 });
    }
  }
}
