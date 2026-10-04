import type { AppContext } from '../app/context';
import { SecondCrs } from '../app/secondCrs';
import { Signal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import { elevationAt } from '../product/elevationValues';
import type { ViewTransform } from '../viewport/Camera';
import { ringMark } from './constructPreview';
import { drawTag } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { pointFromText } from './tracking';

/**
 * Koordinat oku (`crs.query`, docs/adr/0140): every click (snapped) writes its Y and X
 * to the log in the project's formats, and its Z when the snapped object is a point that
 * has one, or the snapped place is a vertex (a line's end) that has an elevation
 * (docs/adr/0142). With a second coordinate system the point is said in it too, with how
 * sure the values are (docs/adr/0167 §2, §5): `ED50 TM30: Y=…, X=… (±2.1 m, …)`. Nothing is
 * written to the drawing; Esc ends. A tool of its own, not in the catalog (like paste): the
 * command starts it.
 */
export class CoordinateReadTool implements Tool {
  readonly id = 'coordinateRead';
  readonly prompt = new Signal('Koordinat oku: noktaya tıklayın ya da Y,X yazın; kenet çalışır [Bitir (Esc)]');
  readonly cursor = 'cross' as const;
  readonly snaps = true;
  private readonly ctx: AppContext;
  private hover: Vec2 | null = null;
  /** The point read last and what was said of it, kept on screen until the next. */
  private read: { at: Vec2; lines: string[] } | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button === 0) this.say(p.world, p.snap?.entityId);
  }

  acceptPoint(p: Vec2): boolean {
    this.say(p);
    return true;
  }

  input(text: string): boolean {
    const p = pointFromText(this.ctx, text, this.read?.at ?? null, this.hover);
    if (!p) return false;
    this.say(p);
    return true;
  }

  /** Right click or Enter ends, as in the other point tools. */
  confirm(): void {
    this.ctx.tools.exit();
  }

  /** The reading: `Y=…, X=…` and the Z of the point or the vertex snapped to, when it has one. */
  private say(at: Vec2, snappedId?: number): void {
    const { doc, format, log } = this.ctx;
    const snapped = snappedId !== undefined ? doc.get(snappedId) : undefined;
    const z = snapped?.kind === 'point' ? snapped.z : snapped ? (elevationAt(snapped, at) ?? undefined) : undefined;
    // East and north as the project's type names them (docs/adr/0165 §4).
    const [e, n] = [format.eastLabel, format.northLabel];
    const text = `${e}=${format.coord(at.x)}, ${n}=${format.coord(at.y)}${z !== undefined ? `, Z=${format.length(z, false)}` : ''}`;
    log.info(text);
    this.read = { at, lines: [`${e} ${format.coord(at.x)}`, `${n} ${format.coord(at.y)}`, ...(z !== undefined ? [`Z ${format.length(z, false)}`] : []), ...this.second(at)] };
    this.ctx.view.requestOverlay();
  }

  /** The point in the second coordinate system, said in the log; the tag's lines of it: its name, then its two values. */
  private second(at: Vec2): string[] {
    const { doc, format, log, prefs } = this.ctx;
    const second = SecondCrs.of(doc.settings);
    if (!second) return [];
    const t = second.point(at);
    if (!t) {
      log.warn(`${second.short}: nokta bu sistemin ulaştığı yerin dışında; değeri yazılmadı.`);
      return [];
    }
    log.info(`${second.short}: ${second.reading(t.point, format, prefs.geographic.value)} (${second.accuracy(t)})`);
    const [a, b] = second.values(t.point, format, prefs.geographic.value);
    return [second.short, ...[a, b].map(([name, v]) => (second.geographic ? v : `${name} ${v}`))];
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.read) return;
    const pal = this.ctx.view.palette;
    ringMark(g, view, this.read.at, pal.accent, 6);
    drawTag(g, view.worldToScreen(this.read.at), this.read.lines, pal.accent, pal.labelHalo);
  }
}
