import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import type { Entity, EntityGeometry, NewEntity } from '../model/entities';
import { bearingGrad, dist, type Vec2 } from '../model/geometry';
import { lineCreate } from '../product/lineCreate';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { drawTag, strokePath } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { writableLayer } from './targetLayer';
import { constrainPoint, drawTracking, pointFromText, type Tracking } from './tracking';

/**
 * Base for tools driven by a sequence of points (click or typed). Handles
 * ortho, typed coordinates, previews and writing to the active layer.
 * Enter finishes the current object; Esc (handled by ToolManager) leaves.
 */
export abstract class PointInputTool implements Tool {
  abstract readonly id: string;
  protected abstract readonly label: string;
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  protected pts: Vec2[] = [];
  protected hover: Vec2 | null = null;
  protected tracking: Tracking | null = null;
  protected readonly ctx: AppContext;
  /** Objects written for the object being drawn (a ray from its base, a line chain), newest last. */
  private made: number[] = [];
  /** The drawing's revision right after the newest of them was written or taken back. */
  private madeAt = -1;
  /**
   * True when the tool's step follows from its points alone, so Ctrl+Z can
   * take back one point; other tools without a Geri (G) start the object over.
   */
  protected readonly stepsFromPoints: boolean = false;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  get snaps(): boolean {
    return true;
  }

  snapFrom(): Vec2 | null {
    return this.last;
  }

  protected abstract promptFor(count: number): string;
  protected abstract onPoint(p: Vec2): void;

  activate(): void {
    this.refreshPrompt();
  }

  protected get last(): Vec2 | null {
    return this.pts.at(-1) ?? null;
  }

  get pointCount(): number {
    return this.pts.length;
  }

  pointerDown(p: ToolPointer): void {
    if (p.button === 0) this.accept(this.constrain(p));
  }

  pointerMove(p: ToolPointer): void {
    this.hover = this.constrain(p);
    this.ctx.view.requestOverlay();
  }

  input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    const pt = pointFromText(this.ctx, text, this.last, this.hover);
    if (!pt) return false;
    this.accept(pt);
    return true;
  }

  /** Letter options such as "K" (kapat) or "G" (geri). */
  protected option(_key: string): boolean {
    return false;
  }

  confirm(): void {
    if (!this.pts.length) this.ctx.tools.exit();
    else this.finish();
  }

  /** Commit what is pending and start over. */
  protected finish(): void {
    this.reset();
  }

  protected reset(): void {
    this.pts = [];
    this.made = [];
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  /**
   * Ctrl+Z while the command runs (docs/adr/0018): takes back its newest step
   * and returns true, or false when nothing is pending and the drawing is
   * undone instead. Newest first: the tool's own Geri (G), then an object
   * written for the object being drawn, then a point of the draft.
   */
  undoStep(): boolean {
    if (this.pts.length && this.option('G')) return true;
    if (this.undoLastMade()) {
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    if (!this.pts.length) return false;
    if (!this.stepsFromPoints) {
      this.reset();
      return true;
    }
    this.pts.pop();
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /**
   * Takes back the newest object this command wrote for the object being
   * drawn, as an undo, when the drawing has not changed since: a later
   * Ctrl+Z on the drawing then cannot bring it back. False otherwise.
   */
  protected undoLastMade(): boolean {
    if (!this.made.length || this.ctx.doc.revision !== this.madeAt) return false;
    this.made.pop();
    this.ctx.doc.undo();
    this.madeAt = this.ctx.doc.revision;
    return true;
  }

  acceptPoint(p: Vec2): boolean {
    this.accept(p);
    return true;
  }

  protected accept(p: Vec2): void {
    this.ctx.log.info(`  ${this.ctx.format.point(p)}`);
    // A first point starts a new object: what was written before is the drawing's to undo.
    if (!this.pts.length) this.made = [];
    this.onPoint(p);
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  protected constrain(p: ToolPointer): Vec2 {
    const { point, tracking } = constrainPoint(this.ctx, this.last, p);
    this.tracking = tracking;
    return point;
  }

  protected refreshPrompt(): void {
    this.prompt.set(`${this.label}: ${this.promptFor(this.pts.length)}`);
  }

  /** Target layer for new entities, or null (with a message) when not writable. */
  protected targetLayer(preferred?: string): string | null {
    return writableLayer(this.ctx, preferred);
  }

  protected create(geom: EntityGeometry, extra: { attrs?: Record<string, string>; label?: string; layerId?: string } = {}): Entity | null {
    const layerId = this.targetLayer(extra.layerId);
    if (!layerId) return null;
    const color = this.ctx.settings.color.value ?? undefined;
    const e = this.ctx.doc.add({ ...geom, layerId, color, attrs: extra.attrs ?? {}, label: extra.label } as NewEntity);
    this.noteMade(e.id);
    return e;
  }

  /** Records an object written for the object being drawn, so Ctrl+Z and Geri (G) can take it back as an undo. */
  protected noteMade(id: number): void {
    this.made.push(id);
    this.madeAt = this.ctx.doc.revision;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const chain = this.hover ? [...this.pts, this.hover] : this.pts;
    strokePath(g, view, chain, { color: pal.accent });
    const last = this.last;
    if (last && this.hover) {
      const s = view.worldToScreen(this.hover);
      drawTag(g, s, [this.ctx.format.length(dist(last, this.hover)), `Semt ${this.ctx.format.bearing(bearingGrad(last, this.hover))}`], pal.accent, pal.labelHalo);
    }
    this.drawTracking(g, view);
  }

  protected drawTracking(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.tracking && this.hover) drawTracking(g, view, this.tracking, this.hover, this.ctx.view.palette.accent, this.ctx.view.palette.labelHalo);
  }
}

export class LineTool extends PointInputTool {
  readonly id = 'line';
  protected readonly label = 'Çizgi';
  /** Lines drawn in this chain, so G can take the last one back (AutoCAD LINE → Undo). */
  private created: number[] = [];

  protected promptFor(n: number): string {
    if (n === 0) return 'ilk noktayı belirtin';
    const opts = [this.created.length ? 'Geri (G)' : '', n >= 3 ? 'Kapat (K)' : '', 'Bitir (Enter)'].filter(Boolean).join(' / ');
    return `sonraki noktayı belirtin [${opts}]`;
  }

  protected onPoint(p: Vec2): void {
    const last = this.last;
    if (last && dist(last, p) <= 1e-9) return;
    if (last) {
      const id = this.createLine(last, p);
      if (id === null) return;
      this.created.push(id);
    }
    this.pts.push(p);
  }

  /**
   * Each segment of the chain is written by the product command
   * `cad.line.create` (docs/adr/0027): its own object and its own undo step,
   * as before. What the tool knows implicitly is explicit in the input
   * (CMD-07): the active layer and the current colour. The messages stay the
   * tool's: the command's refusal or warning (the locked and hidden layer
   * texts, word for word). The new line's slot, or null when refused.
   */
  private createLine(a: Vec2, b: Vec2): number | null {
    const color = this.ctx.settings.color.value;
    const result = lineCreate.execute({ doc: this.ctx.doc }, { layerId: this.ctx.doc.layers.active.value, a, b, ...(color !== null && { color }) });
    if (result.status !== 'completed') {
      if ('error' in result) this.ctx.log.warn(result.error.message);
      return null;
    }
    for (const w of result.warnings) this.ctx.log.warn(w.message);
    this.noteMade(result.output.id);
    return result.output.id;
  }

  protected override option(key: string): boolean {
    if (key === 'G' && this.created.length) {
      const id = this.created.pop()!;
      // Taken back as an undo when nothing changed since, so a later Ctrl+Z cannot bring the line back.
      if (!this.undoLastMade()) this.ctx.doc.remove([id]);
      this.pts.pop();
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    if (key !== 'K' || this.pts.length < 3) return false;
    this.onPoint(this.pts[0]);
    this.finish();
    return true;
  }

  protected override finish(): void {
    if (this.created.length) this.ctx.log.success(`${this.created.length} çizgi eklendi.`);
    this.created = [];
    super.finish();
  }
}

/** Places points; with `askZ` it waits for an elevation after each click (kot noktası). */
export class PointTool extends PointInputTool {
  readonly id: string;
  protected readonly label: string;
  private readonly askZ: boolean;
  private readonly layerId?: string;
  private pendingZ: Vec2 | null = null;

  constructor(ctx: AppContext, opts: { id: string; label: string; askZ: boolean; layerId?: string }) {
    super(ctx);
    this.id = opts.id;
    this.label = opts.label;
    this.askZ = opts.askZ;
    this.layerId = opts.layerId;
  }

  protected promptFor(): string {
    return this.pendingZ ? 'kot değerini yazın (m)' : 'nokta konumunu belirtin';
  }

  protected onPoint(p: Vec2): void {
    if (this.askZ) {
      this.pendingZ = p;
      return;
    }
    this.create({ kind: 'point', p }, { layerId: this.layerId });
  }

  override input(text: string): boolean {
    if (this.pendingZ) {
      const z = parseNumber(text);
      if (z === null) return false;
      this.create({ kind: 'point', p: this.pendingZ, z }, { layerId: this.layerId, label: z.toFixed(2), attrs: { Tür: 'Kot noktası', 'Z (m)': z.toFixed(3) } });
      this.pendingZ = null;
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    return super.input(text);
  }

  override pointerDown(p: ToolPointer): void {
    if (!this.pendingZ) super.pointerDown(p);
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.pendingZ) return;
    const s = view.worldToScreen(this.pendingZ);
    const pal = this.ctx.view.palette;
    drawTag(g, s, ['Kot?'], pal.accent, pal.labelHalo);
  }
}

export class EraseTool implements Tool {
  readonly id = 'erase';
  readonly prompt = new Signal('Sil: silinecek nesneye tıklayın');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    if (!this.ctx.selection.size) return;
    this.eraseIds([...this.ctx.selection.ids.value]);
    queueMicrotask(() => this.ctx.tools.exit());
  }

  pointerMove(p: ToolPointer): void {
    this.ctx.selection.hover.set(this.ctx.view.pick(p.screen)?.id ?? null);
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const hit = this.ctx.view.pick(p.screen);
    if (hit) this.eraseIds([hit.id]);
  }

  private eraseIds(ids: number[]): void {
    const { doc, log, selection } = this.ctx;
    const ok = ids.filter((id) => {
      const e = doc.get(id);
      return e && !doc.layers.isLocked(e.layerId);
    });
    if (ok.length < ids.length) log.warn(`${ids.length - ok.length} nesne kilitli katmanda olduğu için silinmedi.`);
    if (!ok.length) return;
    doc.remove(ok);
    selection.retain((id) => !!doc.get(id));
    selection.hover.set(null);
    log.success(`${ok.length} nesne silindi.`);
  }
}

/** Stand-in for tools whose behaviour is on the roadmap; keeps the UI honest. */
export class PendingTool implements Tool {
  readonly id: string;
  readonly prompt: Signal<string>;
  readonly cursor = 'cross' as const;
  readonly snaps = true;

  constructor(id: string, label: string) {
    this.id = id;
    this.prompt = new Signal(`${label}: bu araç henüz geliştirme aşamasında. Çıkmak için Esc`);
  }
}
