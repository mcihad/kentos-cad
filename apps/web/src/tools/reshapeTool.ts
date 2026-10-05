import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import { entityArea, entityLength, type Entity } from '../model/entities';
import { bulgePathOutline } from '../model/geom/bulge';
import type { Area } from '../model/geom/overlay';
import type { Vec2 } from '../model/geometry';
import { areasOfEntity } from '../model/ops/areas';
import { MULTI_PART_REFUSED } from '../model/ops/parts';
import { reshapeBy, type ReshapeRefusal } from '../model/ops/reshapeBy';
import type { ViewTransform } from '../viewport/Camera';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { topologyOn } from './neighbours';
import { PathTool } from './pathTool';
import { drawArea, strokePath, tint } from './preview';
import type { ToolPointer } from './Tool';

/**
 * Biçim değiştir (docs/adr/0173 §2–§3); the desktop's `reshape_by.rs` and the path tool's `Shape::Reshape`, step for
 * step.
 *
 * - The object is an area, a line or a polyline on an unlocked layer: the one selected when the tool starts, else the
 *   one whose edge is clicked, else the smallest area around the click. It stays the object after each reshape; Geri
 *   (G) before the first point lets it go.
 * - The line is drawn point by point, straight edges only (no Yay, no İzle); the preview fills the area it would leave
 *   (or draws the path) and says its size.
 * - Enter writes it (`cad.entities.edit`'s `reshape`, the step “Biçim değiştir”): an area or a polyline in its place, a
 *   line becoming a polyline with its attributes and label; elevations by the command's rule. A refusal is said and the
 *   line stays to be put right.
 *
 * The reshape is the shared core's (`ops::reshape_by`).
 */

/** An object Biçim değiştir does not take. */
export const NOT_SHAPE = 'Biçim değiştir alan, çizgi ve çoklu çizgi içindir.';
/** More than one object selected when the tool starts. */
export const MANY = 'Biçimi değişecek tek alan ya da çizgi seçin.';
/** A click on no object. */
export const NO_OBJECT = 'Biçimi değişecek alana ya da çizgiye tıklayın.';
/** Topoloji on: the neighbours stay as they are (docs/adr/0173 §6). */
export const TOPOLOGY = "Topoloji açık: komşular Biçim değiştir'le değişmez.";

interface Target {
  id: number;
  /** An area (else a line or a polyline). */
  area: boolean;
}

/** The object Biçim değiştir takes; why not otherwise (null for one on a locked layer: it is not to be picked). */
function targetOf(ctx: AppContext, e: Entity): Target | string | null {
  if (ctx.doc.layers.isLocked(e.layerId)) return null;
  if (e.kind === 'polygon') return { id: e.id, area: true };
  if (e.kind === 'line' || e.kind === 'polyline') return { id: e.id, area: false };
  return NOT_SHAPE;
}

/** The object at the world point `p`: the one whose edge is there (any kind on an unlocked layer), else the smallest unlocked area around it. */
function pickAt(ctx: AppContext, p: Vec2): Entity | null {
  const open = (e: Entity) => !ctx.doc.layers.isLocked(e.layerId);
  return ctx.view.pickEdge(ctx.view.camera.worldToScreen(p), open) ?? ctx.view.containing(p).find((c) => open(c.entity) && c.entity.kind === 'polygon')?.entity ?? null;
}

/** The refusal in the tool's words. */
export function refusalText(r: ReshapeRefusal, area: boolean): string {
  switch (r.why) {
    case 'notShape':
      return NOT_SHAPE;
    case 'tooShort':
      return 'Hat en az iki noktalı olmalı.';
    case 'noCrossing':
      return area ? 'Hat alanın sınırını iki kez kesmeli ya da ona değmeli.' : 'Hat çizgiyi kesmeli ya da ona değmeli.';
    case 'manyParts':
      return 'Hat alanın birden çok parçasına değiyor; bir parçayı düzenleyin.';
    case 'touchesHole':
      return 'Hat bir deliğe değiyor; deliği Delik araçlarıyla düzenleyin.';
    case 'bothWays':
      return 'Hat alanı hem kesiyor hem büyütüyor; ikisini ayrı hatlarla yapın.';
    case 'apart':
      return 'Hattın kapattığı cepler alanı parçalara ayırırdı.';
    // Which part the sketch reshapes is not known (docs/adr/0174).
    case 'multiPart':
      return MULTI_PART_REFUSED;
  }
}

const noun = (area: boolean) => (area ? 'Alan' : 'Uzunluk');
/** The object's size: an area's net area, a path's length. */
const size = (e: Entity, area: boolean): number => (area ? entityArea(e) : entityLength(e)) ?? 0;

/** A size and its change as the log and the tag say them: `830.00 m²`, `−70.00 m²`. */
function sizes(ctx: AppContext, area: boolean, after: number, before: number): [string, string] {
  const show = (v: number) => (area ? ctx.format.area(v) : ctx.format.length(v));
  const d = after - before;
  return [show(after), `${d < 0 ? '−' : '+'}${show(Math.abs(d))}`];
}

/** What the preview shows of a reshape: the area it leaves or the path it draws, and its size with the change. */
interface Reshaped {
  areas: Area[];
  path: { pts: Vec2[]; bulges?: number[] } | null;
  line: string;
}

/** Biçim değiştir: a line of straight edges that reshapes an area or a path. */
export class ReshapeTool extends PathTool {
  /** The object reshaped, kept from one line to the next; null while one is picked. */
  private target: Target | null = null;
  /** What the line to the cursor would make of it. */
  private reshaped: Reshaped | null = null;

  constructor(ctx: AppContext) {
    super(ctx, { id: 'reshape', label: 'Biçim değiştir', closed: false, straight: true });
  }

  /** The object selected beforehand, one alone; Topoloji leaves the neighbours as they are. */
  override activate(): void {
    if (!this.target) {
      if (topologyOn(this.ctx)) this.ctx.log.info(TOPOLOGY);
      const sel = [...this.ctx.selection.ids.value];
      if (sel.length > 1) this.ctx.log.warn(MANY);
      const e = sel.length === 1 ? this.ctx.doc.get(sel[0]) : undefined;
      const t = e && targetOf(this.ctx, e);
      if (typeof t === 'string') this.ctx.log.warn(t);
      else if (t) this.target = t;
    }
    super.activate();
  }

  override deactivate(): void {
    super.deactivate();
    this.ctx.selection.hover.set(null);
  }

  protected override promptFor(n: number): string {
    if (n === 0) return this.target ? 'hattın ilk noktasını belirtin [Geri (G)]' : 'biçimi değişecek alanı ya da çizgiyi seçin';
    return super.promptFor(n);
  }

  /** The object under the pointer while one is picked; then the object reshaped, and what the line would make of it. */
  override pointerMove(p: ToolPointer): void {
    const t = this.target;
    if (!t) {
      this.ctx.selection.hover.set(pickAt(this.ctx, p.raw)?.id ?? null);
      this.hover = null;
      this.ctx.view.requestOverlay();
      return;
    }
    this.ctx.selection.hover.set(t.id);
    super.pointerMove(p);
    this.reshaped = this.preview(t, this.previewPath().pts);
  }

  private preview(t: Target, sketch: Vec2[]): Reshaped | null {
    const e = this.ctx.doc.get(t.id);
    const out = e && reshapeBy(e, sketch);
    if (!e || !out || 'refusal' in out) return null;
    const after = out.done;
    const [now, change] = sizes(this.ctx, t.area, size(after, t.area), size(e, t.area));
    const line = `${noun(t.area)} ${now} (${change})`;
    if (after.kind === 'polygon') return { areas: areasOfEntity(after), path: null, line };
    if (after.kind === 'polyline') return { areas: [], path: { pts: after.pts, ...(after.bulges && { bulges: after.bulges }) }, line };
    return null;
  }

  /** The first point picks the object while there is none. */
  protected override onPoint(p: Vec2): void {
    if (this.target) return super.onPoint(p);
    const e = pickAt(this.ctx, p);
    const t = e && targetOf(this.ctx, e);
    if (!e || !t) return this.ctx.log.warn(NO_OBJECT);
    if (typeof t === 'string') return this.ctx.log.warn(t);
    this.target = t;
    this.ctx.selection.hover.set(e.id);
  }

  /** Geri before the first point lets the object go. */
  protected override option(key: string): boolean {
    if (key === 'G' && !this.pts.length && this.target) {
      this.target = null;
      this.reshaped = null;
      this.ctx.selection.hover.set(null);
      this.refreshPrompt();
      this.ctx.view.requestOverlay();
      return true;
    }
    return super.option(key);
  }

  /** The object stays for the next line; a refused line stays to be put right. */
  protected override finish(): void {
    const t = this.target;
    if (!t || this.pts.length < 2) return super.finish();
    if (this.write(t, [...this.pts])) this.reset();
  }

  private write(t: Target, sketch: Vec2[]): boolean {
    const { ctx } = this;
    const e = ctx.doc.get(t.id);
    if (!e) return false;
    const before = size(e, t.area);
    const out = reshapeBy(e, sketch);
    if ('refusal' in out) {
      ctx.log.warn(refusalText(out.refusal, t.area));
      return false;
    }
    const uid = uidOf(ctx, e);
    const geometry = editGeometry(out.done);
    // A line becomes a polyline: in its place, with its attributes and label.
    const change: EntityEdit = e.kind === 'line' ? { kind: 'replace', uid, geometry, keepData: true } : { kind: 'update', uid, geometry };
    if (!writeEdit(ctx, 'reshape', [change])) return false;
    const now = ctx.doc.get(t.id);
    const [after, d] = sizes(ctx, t.area, now ? size(now, t.area) : 0, before);
    ctx.log.success(`${noun(t.area)} ${after} oldu (${d}).`);
    return true;
  }

  protected override reset(): void {
    this.reshaped = null;
    super.reset();
  }

  protected override extraTag(): string[] {
    return this.reshaped ? [this.reshaped.line] : [];
  }

  /** What the line would make of the object, under the line. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const r = this.reshaped;
    if (r) {
      const pal = this.ctx.view.palette;
      for (const a of r.areas) drawArea(g, view, a, { color: pal.accent, fill: tint(pal.accent, 0.16), width: 2 });
      if (r.path) strokePath(g, view, bulgePathOutline(r.path.pts, r.path.bulges, false), { color: pal.accent, width: 2.5 });
    }
    super.draw(g, view);
  }
}
