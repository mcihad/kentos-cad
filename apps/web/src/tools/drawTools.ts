import type { AppContext } from '../app/context';
import type { CommandResult } from '../contracts/generated/CommandResult';
import type { CreateOperation } from '../contracts/generated/CreateOperation';
import type { EntitiesCreated } from '../contracts/generated/EntitiesCreated';
import { Signal } from '../core/signal';
import type { EntityGeometry } from '../model/entities';
import { bearingGrad, dist, type Vec2 } from '../model/geometry';
import { entitiesDelete } from '../product/entitiesDelete';
import { lineCreate } from '../product/lineCreate';
import { pointCreate } from '../product/pointCreate';
import { polygonCreate } from '../product/polygonCreate';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import * as createCommand from './createCommand';
import { drawTag, strokePath } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { writeOnStandardLayer } from './standardLayer';
import { writableLayer } from './targetLayer';
import { constrainPoint, drawTracking, pointFromText, type Tracking } from './tracking';
import { fixed } from '../core/displayNumber';
import { bulgeRingArea } from '../model/geom/bulge';
import { clippedGeometry, clipNewArea, sayClipped, writtenArea } from './overlap';

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

  /** Records an object written for the object being drawn, so Ctrl+Z and Geri (G) can take it back as an undo. */
  protected noteMade(id: number): void {
    this.made.push(id);
    this.madeAt = this.ctx.doc.revision;
  }

  /**
   * What was written so far can no longer be taken back as an undo: a step
   * that is not the tool's (a deletion) now sits above it in the history.
   */
  protected forgetMade(): void {
    this.made = [];
  }

  /**
   * A product command's answer, as the drawing tools report it (docs/adr/0027,
   * 0032): a refusal's message (the locked and hidden layer texts are the
   * tools' own words, kept by the commands), else the warnings of the write;
   * the object written is noted for Ctrl+Z. The output, or null when refused.
   */
  protected written<T extends { id: number }>(result: CommandResult<T>): T | null {
    if (result.status !== 'completed') {
      if ('error' in result) this.ctx.log.warn(result.error.message);
      return null;
    }
    for (const w of result.warnings) this.ctx.log.warn(w.message);
    this.noteMade(result.output.id);
    return result.output;
  }

  /**
   * A closed shape the tool built (a rectangle, a regular polygon) through
   * the product command `cad.polygon.create` (docs/adr/0032): the active
   * layer, the current colour and line weight explicit. Whether it was written.
   */
  protected writeRing(pts: Vec2[], bulges?: number[]): boolean {
    const input = { layerId: this.ctx.doc.layers.active.value, pts, ...(bulges && { bulges }), ...this.colour(), ...this.weight() };
    return this.written(polygonCreate.execute({ doc: this.ctx.doc }, input)) !== null;
  }

  /**
   * A new area the tool built by its outline (a rectangle, a regular polygon, a sector) written as `writeRing` writes
   * it, the overlap control first (docs/adr/0162 §2): what overlaps the neighbours is cut away and what is left
   * written as one object through `cad.entities.create`. Its area as written, or null when nothing was written.
   */
  protected writeArea(pts: Vec2[], bulges?: number[]): number | null {
    const clipped = clipNewArea(this.ctx, { outer: { pts, ...(bulges && { bulges }) }, holes: [] }, this.ctx.doc.layers.active.value);
    if (!clipped) return this.writeRing(pts, bulges) ? Math.abs(bulgeRingArea(pts, bulges)) : null;
    sayClipped(this.ctx, clipped);
    if (!clipped.areas.length) return null;
    return this.writeObjects([clippedGeometry(clipped.areas)]) ? writtenArea(clipped.areas) : null;
  }

  /**
   * Objects the tool built (an ellipse, a spline, a construction line …)
   * through the product command `cad.entities.create` (docs/adr/0057): the
   * active layer, the current colour and line weight explicit; one undo step, “Ekle” or
   * the tool's `operation`, noted for Ctrl+Z. The output, or null when
   * refused (the refusal said).
   */
  protected writeObjects(geometries: EntityGeometry[], operation?: CreateOperation, attrs?: Record<string, string>): EntitiesCreated | null {
    const out = createCommand.writeObjects(this.ctx, geometries, operation, attrs);
    if (out) this.noteMade(out.ids[0]);
    return out;
  }

  /** The current colour, explicit in a command's input (CMD-07); absent: the layer's. */
  protected colour(): { color?: string } {
    const color = this.ctx.settings.color.value;
    return color !== null ? { color } : {};
  }

  /** The current line weight (the toolbar's Kalınlık, docs/adr/0139), explicit in a command's input; absent: the layer's. */
  protected weight(): { lineWeight?: number } {
    const lineWeight = this.ctx.settings.lineWeight.value;
    return lineWeight !== null ? { lineWeight } : {};
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
    return this.written(lineCreate.execute({ doc: this.ctx.doc }, { layerId: this.ctx.doc.layers.active.value, a, b, ...this.colour(), ...this.weight() }))?.id ?? null;
  }

  protected override option(key: string): boolean {
    if (key === 'G' && this.created.length) {
      const id = this.created.at(-1)!;
      // Taken back as an undo when nothing changed since, so a later Ctrl+Z cannot bring the line back;
      // else deleted. A line it may not delete (on a layer locked since) stays, and so does the chain.
      if (!this.undoLastMade() && !this.eraseLine(id)) return true;
      this.created.pop();
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

  /**
   * A line of the chain deleted through the product command
   * `cad.entities.delete` (docs/adr/0029), as the erase tool deletes: one undo
   * step, “Sil”. Its lock rule holds: a line on a locked layer is not deleted
   * and the refusal is said. A line already gone counts as deleted. Whether
   * it is gone.
   */
  private eraseLine(id: number): boolean {
    const { doc, log } = this.ctx;
    const uid = doc.uidOf(id);
    if (uid === undefined) return true;
    const result = entitiesDelete.execute({ doc }, { uids: [uid] });
    if (result.status !== 'completed') {
      if ('error' in result) log.warn(result.error.message);
      return false;
    }
    // The deletion is the newest step now: what the chain wrote before it can only be deleted too.
    this.forgetMade();
    return true;
  }

  protected override finish(): void {
    if (this.created.length) this.ctx.log.success(`${this.created.length} çizgi eklendi.`);
    this.created = [];
    super.finish();
  }
}

/**
 * A tool that always writes to its own layer (Parsel → parsel, Kot noktası →
 * kot) says so when that layer is locked, in its own words, and writes
 * nothing: activating another layer would not help, as the commands' text
 * suggests. True when locked. A missing or hidden layer is left to the
 * command, whose words fit them.
 */
export function fixedLayerLocked(ctx: AppContext, layerId: string, tool: string): boolean {
  const layers = ctx.doc.layers;
  const node = layers.get(layerId);
  if (!node || !layers.isLocked(layerId)) return false;
  ctx.log.warn(`“${node.name}” katmanı kilitli; ${tool} bu katmana yazar. Kilidi Katmanlar panelinden açın.`);
  return true;
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
    this.writePoint({ p });
  }

  override input(text: string): boolean {
    if (this.pendingZ) {
      const z = parseNumber(text);
      if (z === null) return false;
      this.writePoint({ p: this.pendingZ, z, label: fixed(z, 2), attrs: { Tür: 'Kot noktası', 'Z (m)': fixed(z, 3) } });
      this.pendingZ = null;
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    return super.input(text);
  }

  /**
   * Writes one point through the product command `cad.point.create`
   * (docs/adr/0032): the tool's own layer (the spot elevations', opened first
   * when the drawing lacks it, docs/adr/0067) or the active one, and the
   * current colour, explicit in its input (CMD-07).
   */
  private writePoint(fields: { p: Vec2; z?: number; label?: string; attrs?: Record<string, string> }): void {
    if (this.layerId && fixedLayerLocked(this.ctx, this.layerId, this.label)) return;
    const layerId = this.layerId ?? this.ctx.doc.layers.active.value;
    const write = () => pointCreate.execute({ doc: this.ctx.doc }, { layerId, ...fields, ...this.colour() });
    // A drawing without the elevation layer gets it, in the point's own undo step (tools/standardLayer.ts).
    this.written(this.layerId ? writeOnStandardLayer(this.ctx, layerId, 'kot noktası', write) : write());
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

  /**
   * Deletes through the product command `cad.entities.delete` (docs/adr/0029),
   * as the desktop does: the selection (or the picked object) is made explicit
   * as persistent ids (CMD-07). Objects on a locked layer stay: the command
   * warns and deletes the rest, or refuses when every one is locked; one undo
   * step, “Sil”.
   */
  private eraseIds(ids: number[]): void {
    const { doc, log, selection } = this.ctx;
    const uids = ids.map((id) => doc.uidOf(id)).filter((uid): uid is string => uid !== undefined);
    const result = entitiesDelete.execute({ doc }, { uids });
    if (result.status !== 'completed') {
      if ('error' in result) log.warn(result.error.message);
      return;
    }
    for (const w of result.warnings) log.warn(w.message);
    selection.retain((id) => !!doc.get(id));
    selection.hover.set(null);
    log.success(`${result.output.removed.length} nesne silindi.`);
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
