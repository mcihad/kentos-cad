import { dist, type Vec2 } from '../model/geometry';
import { pointsBetween, type Between } from '../model/ops/construct';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { dotMark, ringMark } from './constructPreview';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';

type Method = 'parts' | 'distances' | 'ratios';

const SAME = 1e-9;
/** Most points the tool places, and most the preview marks. */
const MAX_POINTS = 10_000;
const MAX_SHOWN = 2000;

/** Numbers with commas, semicolons or spaces between them; null for anything else, and for none at all. */
function numberList(text: string): number[] | null {
  const parts = text.trim().split(/[,;\s]+/).filter(Boolean);
  const list = parts.map((t) => (/^[-+]?\d+(?:\.\d+)?$/.test(t) ? +t : NaN));
  return list.length && list.every(Number.isFinite) ? list : null;
}

/**
 * Ara nokta (docs/adr/0140): points on the line between two points, three ways. The
 * ribbon starts a method by sending its letter right after the tool, and the same
 * letters work typed on the command line: `E`, `U`, `O`.
 *
 * - Eşit aralık (the default): the line in equal parts, the number of parts typed
 *   (2 to 10 000);
 * - Uzaklıkla (`U`): distances from the first point, metres, typed with commas
 *   between (`5, 12.5`);
 * - Oranla (`O`): fractions of the way from the first point to the second, 0 to 1,
 *   typed with commas between (`0.25, 0.5`).
 *
 * The two points are clicked (snapping) or typed. Before the second one the points to
 * come follow the cursor; once both are in they are shown at the value kept from
 * last time: a typed value writes at once, Enter writes with the kept one. The value
 * of each method stays for the session. The tool then asks for the next two points;
 * Esc steps back a point. The points go on the active layer through
 * `cad.entities.create`, one undo step; where they fall is the shared core's.
 */
export class PointsBetweenTool extends PointInputTool {
  readonly id = 'pointsBetween';
  protected readonly label = 'Ara nokta';
  private static parts = 4;
  private static distances: number[] = [];
  private static ratios: number[] = [];
  private method: Method = 'parts';

  /** The points between `a` and `b` at the kept value, or why there are none: a short reason and the whole sentence. */
  private compute(a: Vec2, b: Vec2): { pts: Vec2[] } | { short: string; why: string } {
    const f = this.ctx.format;
    const length = dist(a, b);
    if (length < SAME) return { short: 'Nokta çakışıyor', why: 'İki nokta çakışıyor; ikinci noktayı ilkinden ayrı bir yere koyun.' };
    let how: Between;
    if (this.method === 'parts') how = { parts: PointsBetweenTool.parts };
    else if (this.method === 'distances') {
      const list = PointsBetweenTool.distances;
      if (!list.length) return { short: 'Uzaklık yazın', why: 'Uzaklıkları yazın, ör. 5, 12.5.' };
      const out = list.find((d) => !(d >= 0 && d <= length));
      if (out !== undefined)
        return {
          short: 'Uzaklık dışarıda',
          why: `${f.length(out)} uzaklığı iki nokta arasının (${f.length(length)}) dışında kalıyor; 0 ile ${f.length(length)} arasında yazın.`,
        };
      how = { distances: list };
    } else {
      const list = PointsBetweenTool.ratios;
      if (!list.length) return { short: 'Oran yazın', why: 'Oranları yazın, ör. 0.25, 0.5.' };
      const out = list.find((t) => !(t >= 0 && t <= 1));
      if (out !== undefined) return { short: 'Oran dışarıda', why: `${out} oranı 0 ile 1 arasında değil; iki noktanın arasında kalan oranlar yazın.` };
      how = { ratios: list };
    }
    const pts = pointsBetween(a, b, how);
    return pts?.length ? { pts } : { short: 'Parça yok', why: 'Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.' };
  }

  /** What the kept value is, for the prompt: `4 parça`, `5, 12.5 m`, `0.25, 0.5`. */
  private kept(): string {
    const f = this.ctx.format;
    switch (this.method) {
      case 'parts':
        return `${PointsBetweenTool.parts} parça`;
      case 'distances':
        return PointsBetweenTool.distances.length ? `${PointsBetweenTool.distances.map((d) => f.length(d, false)).join(', ')} ${f.lengthUnitLabel}` : 'uzaklık yok';
      default:
        return PointsBetweenTool.ratios.length ? PointsBetweenTool.ratios.join(', ') : 'oran yok';
    }
  }

  /** The methods other than the running one, as options. */
  private options(): string {
    const all: [Method, string, string][] = [['parts', 'Eşit aralık', 'E'], ['distances', 'Uzaklıkla', 'U'], ['ratios', 'Oranla', 'O']];
    return all
      .filter(([m]) => m !== this.method)
      .map(([, label, key]) => `${label} (${key})`)
      .join(' / ');
  }

  protected promptFor(n: number): string {
    const kept = this.kept();
    const opts = this.options();
    if (n === 0) return `ilk noktayı belirtin [${kept}; ${opts}]`;
    if (n === 1) return `ikinci noktayı belirtin [${kept}; ${opts}]`;
    const ask =
      this.method === 'parts'
        ? `parça sayısını yazın (Enter: ${kept})`
        : this.method === 'distances'
          ? `uzaklıkları virgülle yazın, ör. 5, 12.5 (Enter: ${kept})`
          : `oranları virgülle yazın, ör. 0.25, 0.5 (Enter: ${kept})`;
    return `${ask} [${opts}]`;
  }

  /** A point given: the first two are kept; with both in a click has nothing to say. */
  protected onPoint(p: Vec2): void {
    if (this.pts.length >= 2) return;
    if (this.pts.length && dist(this.pts[0], p) < SAME) return void this.ctx.log.warn('İkinci nokta ilkiyle çakışıyor; başka bir yer gösterin.');
    this.pts.push(p);
  }

  override acceptPoint(p: Vec2): boolean {
    if (this.pts.length >= 2) return false;
    return super.acceptPoint(p);
  }

  /** Writes the points at the kept value and starts over; a refusal is said and the two points stay. */
  private write(): void {
    const [a, b] = this.pts;
    if (!a || !b) return;
    const r = this.compute(a, b);
    if ('why' in r) return void this.ctx.log.warn(r.why);
    if (r.pts.length > MAX_POINTS) return void this.ctx.log.warn('10 000’den fazla nokta oluşacak; daha az değer yazın.');
    if (this.writeObjects(r.pts.map((p) => ({ kind: 'point' as const, p })), 'pointsBetween')) this.ctx.log.success(`${r.pts.length} nokta kondu.`);
    this.reset();
  }

  /** The value typed for the running method; false when it is not one. */
  private value(text: string): boolean {
    const { log } = this.ctx;
    if (this.method === 'parts') {
      const n = parseNumber(text);
      if (n === null || /[,;@<]/.test(text)) return false;
      if (!Number.isInteger(n) || n < 2 || n > MAX_POINTS) {
        log.warn('Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.');
        return true;
      }
      PointsBetweenTool.parts = n;
    } else {
      const list = numberList(text);
      if (!list) return false;
      // Distances are typed in the project's unit (docs/adr/0165 §2); ratios have none.
      if (this.method === 'distances') PointsBetweenTool.distances = list.map((d) => this.ctx.format.toMetres(d));
      else PointsBetweenTool.ratios = list;
    }
    this.write();
    return true;
  }

  private switchTo(method: Method): true {
    this.method = method;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  protected override option(key: string): boolean {
    if (key === 'E') return this.switchTo('parts');
    if (key === 'U') return this.switchTo('distances');
    if (key === 'O') return this.switchTo('ratios');
    if (key !== 'G' || !this.pts.length) return false;
    return this.cancel();
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    if (this.pts.length >= 2) return this.value(text);
    return super.input(text);
  }

  /** Both points in: writes at the kept value. One point: it is let go. None: the tool leaves. */
  override confirm(): void {
    if (this.pts.length >= 2) return this.write();
    if (this.pts.length === 1) {
      this.pts = [];
      this.refreshPrompt();
      return this.ctx.view.requestOverlay();
    }
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    if (!this.pts.length) return false;
    this.pts.pop();
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** The line between the points with the points to come on it; before the second point the cursor stands for it. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const a = this.pts[0];
    const b = this.pts[1] ?? this.hover;
    if (!a || !b) return;
    const pal = this.ctx.view.palette;
    strokePath(g, view, [a, b], { color: pal.accent, dash: [5, 3], width: 1.25 });
    ringMark(g, view, a, pal.snap);
    ringMark(g, view, b, pal.snap);
    const r = this.compute(a, b);
    const f = this.ctx.format;
    let lines: string[];
    let tone = pal.accent;
    if ('pts' in r) {
      const n = r.pts.length;
      lines = [`${n} nokta`, ...(this.method === 'parts' ? [`aralık ${f.length(dist(a, b) / (n + 1))}`] : [])];
      if (n <= MAX_SHOWN) for (const p of r.pts) dotMark(g, view, p, pal.accent, pal.labelHalo);
    } else {
      lines = [r.short];
      tone = pal.danger;
    }
    if (this.hover) drawTag(g, view.worldToScreen(this.hover), lines, tone, pal.labelHalo);
    this.drawTracking(g, view);
  }
}
