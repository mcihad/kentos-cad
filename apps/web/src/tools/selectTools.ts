import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { dist, type Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { dotMark } from './constructPreview';
import { PointInputTool } from './drawTools';
import { drawSelectionCircle, drawSelectionPolygon, drawTag, strokePath } from './preview';
import type { ConfirmMods, Tool, ToolPointer } from './Tool';
import { pointFromText } from './tracking';
import { selectableIds } from './selectable';
import { ringProblem, type PolygonMode } from '../viewport/picking';

/**
 * The selection tools of docs/adr/0141 and 0187, beside Seç: Çitle seç, Daireyle seç, Çokgenle seç and İçeren alanı
 * seç. Each asks the geometry store (`ctx.view.inFence`, `inCircle`, `inPolygon`, `containing`: the shared core's,
 * visible objects only), lets the selection filter (docs/adr/0187 §5) pass what it holds, and puts the answer in the
 * selection, replacing it, or adding to it when Shift is held as the tool finishes. Çitle seç, Daireyle seç and
 * Çokgenle seç then go back to Seç; İçeren alanı seç stays for more clicks. Nothing is written to the drawing.
 */

/** A toggle's state in the prompt: shown only when it is on (docs/adr/0140, Ötele's pattern). */
const whenOn = (on: boolean) => (on ? ': açık' : '');

/** The result in the selection: replaces it, or joins it when Shift is held. */
function take(ctx: AppContext, ids: readonly number[], add: boolean): void {
  if (add) ctx.selection.add(ids);
  else ctx.selection.set(ids);
}

/**
 * Çitle seç: a fence of clicked points; what it crosses is selected: an edge of an object, the body
 * of a text, a point within the pick aperture of it. Geri (G) drops the last point; Enter or a right
 * click ends. Shift held at the last click, or Shift+Enter, adds to the selection (a right click with
 * Shift opens the snap menu, so it cannot end).
 */
export class FenceSelectTool extends PointInputTool {
  readonly id = 'selectFence';
  protected readonly label = 'Çitle seç';
  /** Whether Shift was held at the last click: ending then adds to the selection (so does Shift+Enter). */
  private shift = false;

  protected promptFor(n: number): string {
    return n === 0 ? 'çitin ilk noktasına tıklayın' : 'sonraki noktaya tıklayın [Geri (G) / Bitir (Enter)]';
  }

  protected onPoint(p: Vec2): void {
    const last = this.last;
    if (last && dist(last, p) <= 1e-9) return;
    this.pts.push(p);
  }

  protected override option(key: string): boolean {
    if (key !== 'G' || !this.pts.length) return false;
    this.pts.pop();
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override pointerDown(p: ToolPointer): void {
    this.shift = p.shift;
    super.pointerDown(p);
  }

  override confirm(mods?: ConfirmMods): void {
    if (!this.pts.length) return this.ctx.tools.exit();
    if (this.pts.length < 2) return void this.ctx.log.warn('Çit için en az iki nokta gerekir; ikinci noktayı gösterin.');
    const ids = selectableIds(this.ctx, this.ctx.view.inFence(this.pts));
    if (!ids.length) this.ctx.log.warn('Çit hiçbir nesneyi kesmedi.');
    else {
      take(this.ctx, ids, mods?.shift ?? this.shift);
      this.ctx.log.info(`Çit ${ids.length} nesneyi kesti; seçildi.`);
    }
    this.pts = [];
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    strokePath(g, view, this.hover ? [...this.pts, this.hover] : this.pts, { color: pal.accent, dash: [6, 4], width: 1.5 });
    for (const p of this.pts) dotMark(g, view, p, pal.accent, pal.labelHalo, 3);
    this.drawTracking(g, view);
  }
}

/**
 * Daireyle seç: a centre, then a radius shown or typed; what lies wholly inside the circle is
 * selected, and with Kesişen (K, kept for the session) what it touches too and the closed objects
 * around it. The radius is the true distance: ortho and polar tracking do not bend it.
 */
export class CircleSelectTool extends PointInputTool {
  readonly id = 'selectCircle';
  protected readonly label = 'Daireyle seç';
  private static crossing = false;
  private shift = false;

  protected promptFor(n: number): string {
    return `${n === 0 ? 'dairenin merkezine tıklayın' : 'yarıçapı gösterin ya da yazın'} [Kesişen (K)${whenOn(CircleSelectTool.crossing)}]`;
  }

  protected onPoint(p: Vec2): void {
    const c = this.pts[0];
    if (!c) return void this.pts.push(p);
    this.select(c, dist(c, p));
  }

  protected override option(key: string): boolean {
    if (key !== 'K') return false;
    CircleSelectTool.crossing = !CircleSelectTool.crossing;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** A typed number is the radius; a typed point is where the circle passes. */
  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const r = parseNumber(text);
    if (this.pts[0] && r !== null && !/[,;@<]/.test(text)) {
      this.select(this.pts[0], this.ctx.format.toMetres(r));
      return true;
    }
    return super.input(text);
  }

  /** The radius point is taken as shown (and snapped): ortho and polar belong to the centre's neighbours, not to a distance. */
  protected override constrain(p: ToolPointer): Vec2 {
    if (!this.pts.length) return super.constrain(p);
    this.tracking = null;
    return p.world;
  }

  override pointerDown(p: ToolPointer): void {
    this.shift = p.shift;
    super.pointerDown(p);
  }

  private select(c: Vec2, r: number): void {
    const { log } = this.ctx;
    if (!(r > 0)) return void log.warn('Yarıçap sıfırdan büyük olmalı.');
    const crossing = CircleSelectTool.crossing;
    const ids = selectableIds(this.ctx, this.ctx.view.inCircle(c, r, crossing));
    if (!ids.length) log.warn('Dairede nesne yok.');
    else {
      take(this.ctx, ids, this.shift);
      log.info(crossing ? `Daireye dokunan ${ids.length} nesne; seçildi.` : `Dairenin içinde ${ids.length} nesne; seçildi.`);
    }
    this.pts = [];
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const c = this.pts[0];
    const at = this.hover;
    if (!c || !at) return;
    const pal = this.ctx.view.palette;
    const r = dist(c, at);
    drawSelectionCircle(g, view, c, r, CircleSelectTool.crossing, pal.snap);
    strokePath(g, view, [c, at], { color: pal.accent, dash: [2, 3] });
    drawTag(g, view.worldToScreen(at), [`R ${this.ctx.format.length(r)}`], pal.accent, pal.labelHalo);
    this.drawTracking(g, view);
  }
}

/** What Çokgenle seç says it found, and found none of, by its mode. */
const POLYGON_FOUND: Record<PolygonMode, (n: number) => string> = {
  inside: (n) => `Çokgenin içinde ${n} nesne; seçildi.`,
  crossing: (n) => `Çokgene dokunan ${n} nesne; seçildi.`,
  outside: (n) => `Çokgenin dışında ${n} nesne; seçildi.`,
};
const POLYGON_NONE: Record<PolygonMode, string> = {
  inside: 'Çokgenin içinde nesne yok.',
  crossing: 'Çokgene dokunan nesne yok.',
  outside: 'Çokgenin dışında nesne yok.',
};
/** The modes' options in the prompt and the keys that choose them. */
const POLYGON_OPTIONS: readonly [PolygonMode, string, string][] = [
  ['inside', 'İçindekiler', 'İ'],
  ['crossing', 'Kesişenler', 'K'],
  ['outside', 'Dışındakiler', 'D'],
];

/**
 * Çokgenle seç (docs/adr/0187 §2): the polygon's corners clicked (snaps, ortho and polar tracking from the last), Geri
 * (G) drops the last, Enter or a right click ends. İçindekiler (İ) takes what lies wholly inside, Kesişenler (K) what it
 * touches too, Dışındakiler (D) every visible object it does not touch; the mode is kept for the session. A polygon of
 * fewer than three corners, or crossing itself, is said and the tool waits. Shift held at the last click, or
 * Shift+Enter, adds to the selection.
 */
export class PolygonSelectTool extends PointInputTool {
  readonly id = 'selectPolygon';
  protected readonly label = 'Çokgenle seç';
  private static mode: PolygonMode = 'inside';
  private shift = false;

  protected promptFor(n: number): string {
    const modes = POLYGON_OPTIONS.map(([m, name, key]) => `${name} (${key})${whenOn(PolygonSelectTool.mode === m)}`).join(' / ');
    return n === 0 ? `çokgenin ilk köşesine tıklayın [${modes}]` : `sonraki köşeye tıklayın [Geri (G) / Bitir (Enter) / ${modes}]`;
  }

  protected onPoint(p: Vec2): void {
    const last = this.last;
    if (last && dist(last, p) <= 1e-9) return;
    this.pts.push(p);
  }

  protected override option(key: string): boolean {
    if (key === 'G' && this.pts.length) {
      this.pts.pop();
    } else {
      const mode = POLYGON_OPTIONS.find(([, , k]) => k === key)?.[0];
      if (!mode) return false;
      PolygonSelectTool.mode = mode;
    }
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override pointerDown(p: ToolPointer): void {
    this.shift = p.shift;
    super.pointerDown(p);
  }

  override confirm(mods?: ConfirmMods): void {
    if (!this.pts.length) return this.ctx.tools.exit();
    const problem = ringProblem(this.pts);
    if (problem) return void this.ctx.log.warn(problem);
    const mode = PolygonSelectTool.mode;
    const ids = selectableIds(this.ctx, this.ctx.view.inPolygon(this.pts, mode));
    if (!ids.length) this.ctx.log.warn(POLYGON_NONE[mode]);
    else {
      take(this.ctx, ids, mods?.shift ?? this.shift);
      this.ctx.log.info(POLYGON_FOUND[mode](ids.length));
    }
    this.pts = [];
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const ring = this.hover && this.pts.length ? [...this.pts, this.hover] : this.pts;
    drawSelectionPolygon(g, view, ring, PolygonSelectTool.mode, pal.snap);
    for (const p of this.pts) dotMark(g, view, p, pal.accent, pal.labelHalo, 3);
    this.drawTracking(g, view);
  }
}

/**
 * İçeren alanı seç: a click selects the smallest closed object around it (an area with its holes off,
 * a circle, a whole ellipse, a closed curve). Clicking the same place again (within the pick aperture)
 * moves to the next larger one (parcel, block, district), and after the last back to the smallest.
 * The tag says which of how many, and its area. It stays for more clicks; Esc or a right click ends.
 */
export class ContainingSelectTool implements Tool {
  readonly id = 'selectContaining';
  readonly prompt = new Signal('İçeren alanı seç: alanın içine tıklayın');
  readonly cursor = 'cross' as const;
  readonly snaps = false;
  /** The last click that found something: where, which of the closed objects around it, of how many, its area. */
  private last: { at: Vec2; index: number; count: number; area: number } | null = null;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  pointerDown(p: ToolPointer): void {
    if (p.button === 0) this.click(p.raw, p.shift);
  }

  /** A typed point is a click there. */
  input(text: string): boolean {
    const at = pointFromText(this.ctx, text, null, null);
    if (!at) return false;
    this.click(at, false);
    return true;
  }

  acceptPoint(p: Vec2): boolean {
    this.click(p, false);
    return true;
  }

  private click(at: Vec2, add: boolean): void {
    const { ctx } = this;
    const all = ctx.view.containing(at);
    // The kinds the selection filter holds (docs/adr/0187 §5).
    const kept = new Set(selectableIds(ctx, all.map((f) => f.entity.id)));
    const found = all.filter((f) => kept.has(f.entity.id));
    if (!found.length) return void ctx.log.warn('Tıklanan noktayı içeren kapalı alan yok.');
    // The same place again is the next larger one; anywhere else starts at the smallest.
    const again = this.last && dist(this.last.at, at) <= ctx.view.worldTolerance(ctx.prefs.pickAperture.value);
    const index = again ? (this.last!.index + 1) % found.length : 0;
    const { entity, area } = found[index];
    take(ctx, [entity.id], add);
    this.last = { at, index, count: found.length, area };
    ctx.log.info(`Alan seçildi (${index + 1}/${found.length}, ${ctx.format.area(area)}).`);
    ctx.view.requestOverlay();
  }

  /** Esc leaves (the manager) and so does Enter or a right click; the selection stays. */
  confirm(): void {
    this.ctx.tools.exit();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const l = this.last;
    if (!l) return;
    const pal = this.ctx.view.palette;
    drawTag(g, view.worldToScreen(l.at), [`${l.index + 1}/${l.count} · ${this.ctx.format.area(l.area)}`], pal.accent, pal.labelHalo);
  }
}
