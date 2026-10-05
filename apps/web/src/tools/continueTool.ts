import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import { entityLength, type Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { continuePath, pathEnds } from '../model/ops/reshapeBy';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { PathTool } from './pathTool';
import type { ToolPointer } from './Tool';

/**
 * Sürdür (docs/adr/0173 §4); the desktop's `continuation.rs` and the path tool's `Shape::Continue`, step for step.
 *
 * - The object is a line or an open polyline on an unlocked layer: the one selected when the tool starts (it goes on
 *   from the end nearer the pointer until the first new point), else the one whose edge is clicked (from the end nearer
 *   the click). Any other object: “Sürdür çizgi ve çoklu çizgi içindir.”
 * - Its end is the path's first point; the path is drawn as Çoklu çizgi's, an arc going on along the object's end
 *   tangent.
 * - Enter writes the object continued (`cad.entities.edit`'s `continue`, the step “Sürdür”): a polyline in its place, a
 *   line becoming one with its attributes and label. Its vertices keep their elevations; the new ones have none.
 *
 * The splice and the end directions are the shared core's (`ops::continuation`).
 */

/** An object Sürdür does not take. */
export const NOT_PATH = 'Sürdür çizgi ve çoklu çizgi içindir.';
/** A click on no object. */
export const NO_OBJECT = 'Sürdürülecek çizgiye ya da çoklu çizgiye, ucunun yakınında tıklayın.';
/** Enter with the end alone. */
export const NOTHING_DRAWN = 'Sürdürmek için en az bir nokta verin.';

interface Target {
  entity: Entity;
  first: Vec2;
  last: Vec2;
  outFirst: Vec2 | null;
  outLast: Vec2 | null;
  /** From its first end (else its last). */
  fromFirst: boolean;
  /** The end follows the pointer until the first new point (the object was selected beforehand). */
  open: boolean;
}

/** The object Sürdür takes; why not otherwise (null for one on a locked layer: it is not to be picked). */
function targetOf(ctx: AppContext, e: Entity): Target | string | null {
  if (ctx.doc.layers.isLocked(e.layerId)) return null;
  if (e.kind !== 'line' && e.kind !== 'polyline') return NOT_PATH;
  const ends = pathEnds(e);
  if (!ends) return NOT_PATH;
  return { entity: e, first: ends.first, last: ends.last, outFirst: ends.outFirst ?? null, outLast: ends.outLast ?? null, fromFirst: false, open: false };
}

const endOf = (t: Target): Vec2 => (t.fromFirst ? t.first : t.last);
/** Goes on from the end nearer `p` (the last on a tie). */
const nearer = (t: Target, p: Vec2): void => {
  t.fromFirst = dist(p, t.first) < dist(p, t.last);
};

/** The object whose edge is at the world point `p`, of any kind on an unlocked layer. */
function pickAt(ctx: AppContext, p: Vec2): Entity | null {
  return ctx.view.pickEdge(ctx.view.camera.worldToScreen(p), (e) => !ctx.doc.layers.isLocked(e.layerId));
}

/** The object's vertex elevations in the order of its vertices; null when it has none. */
function elevations(e: Entity): (number | null)[] | null {
  const zs = e.kind === 'line' ? [e.za ?? null, e.zb ?? null] : e.kind === 'polyline' ? (e.zs ?? null) : null;
  return zs && zs.some((z) => z !== null) ? zs : null;
}

const vertexCount = (e: Entity | undefined): number => (e?.kind === 'polyline' ? e.pts.length : 2);

/** Writes the object continued by the drawn path (its first point the end), one undo step “Sürdür”. */
function writeContinued(ctx: AppContext, t: Target, drawn: Vec2[], bulges: number[]): boolean {
  const e = ctx.doc.get(t.entity.id);
  if (!e) return false;
  const after = continuePath(e, t.fromFirst, drawn, bulges);
  if (!after) {
    ctx.log.warn(NOTHING_DRAWN);
    return false;
  }
  const [oldLength, oldCount] = [entityLength(e) ?? 0, vertexCount(e)];
  let geometry = editGeometry(after);
  // Its own vertices keep their elevations; the new ones have none.
  const zs = elevations(e);
  if (zs && geometry.kind === 'polyline' && geometry.pts.length >= zs.length) {
    const added = new Array<number | null>(geometry.pts.length - zs.length).fill(null);
    geometry = { ...geometry, zs: t.fromFirst ? [...added, ...zs] : [...zs, ...added] };
  }
  const uid = uidOf(ctx, e);
  // A line becomes a polyline: in its place, with its attributes and label.
  const change: EntityEdit = e.kind === 'line' ? { kind: 'replace', uid, geometry, keepData: true } : { kind: 'update', uid, geometry };
  if (!writeEdit(ctx, 'continue', [change])) return false;
  const now = ctx.doc.get(e.id);
  const length = (now && entityLength(now)) ?? 0;
  const f = ctx.format;
  ctx.log.success(`Sürdürüldü: ${Math.max(0, vertexCount(now) - oldCount)} köşe eklendi; uzunluk ${f.length(length)} oldu (+${f.length(length - oldLength)}).`);
  return true;
}

/** Sürdür: Çoklu çizgi's path from an end of a line or a polyline, added to it. */
export class ContinueTool extends PathTool {
  /** The object continued and its end, the path's first point; null while one is picked. */
  private target: Target | null = null;

  constructor(ctx: AppContext) {
    super(ctx, { id: 'continue', label: 'Sürdür', closed: false });
  }

  /** An object selected beforehand is the one continued, from the end nearer the pointer until the first new point. */
  override activate(): void {
    const sel = [...this.ctx.selection.ids.value];
    const e = !this.target && sel.length === 1 ? this.ctx.doc.get(sel[0]) : undefined;
    if (e) {
      const t = targetOf(this.ctx, e);
      if (typeof t === 'string') this.ctx.log.warn(t);
      else if (t) {
        t.open = true;
        if (this.hover) nearer(t, this.hover);
        this.pts = [endOf(t)];
        this.bulges = [];
        this.target = t;
      }
    }
    super.activate();
  }

  override deactivate(): void {
    super.deactivate();
    this.ctx.selection.hover.set(null);
  }

  protected override promptFor(n: number): string {
    return n === 0 ? 'sürdürülecek çizginin ucuna yakın tıklayın' : super.promptFor(n);
  }

  /** The object under the pointer while one is picked; then the object continued, its end following the pointer until fixed. */
  override pointerMove(p: ToolPointer): void {
    const t = this.target;
    if (!t) {
      this.ctx.selection.hover.set(pickAt(this.ctx, p.raw)?.id ?? null);
      this.hover = null;
      this.ctx.view.requestOverlay();
      return;
    }
    if (t.open) {
      nearer(t, p.raw);
      this.pts[0] = endOf(t);
    }
    this.ctx.selection.hover.set(t.entity.id);
    super.pointerMove(p);
  }

  /** The first point picks the object; a selected one's end is fixed by the first new point. */
  protected override onPoint(p: Vec2): void {
    if (!this.target) return this.pick(p);
    this.target.open = false;
    super.onPoint(p);
  }

  private pick(p: Vec2): void {
    const e = pickAt(this.ctx, p);
    const t = e && targetOf(this.ctx, e);
    if (!e || !t) return this.ctx.log.warn(NO_OBJECT);
    if (typeof t === 'string') return this.ctx.log.warn(t);
    nearer(t, p);
    this.pts.push(endOf(t));
    this.target = t;
    this.ctx.selection.hover.set(e.id);
  }

  protected override startTangent(): Vec2 | null {
    const t = this.target;
    return t && (t.fromFirst ? t.outFirst : t.outLast);
  }

  /** Geri with the end alone picks the object again. */
  protected override dropStart(): boolean {
    this.reset();
    this.ctx.selection.hover.set(null);
    return true;
  }

  /** The end alone draws nothing: the object is picked again. */
  protected override finish(): void {
    if (this.pts.length < 2) {
      this.ctx.log.warn(NOTHING_DRAWN);
      this.ctx.selection.hover.set(null);
      return this.reset();
    }
    super.finish();
  }

  protected override writeShape(pts: Vec2[], bulges: number[] | undefined): void {
    const t = this.target;
    this.ctx.selection.hover.set(null);
    if (t) writeContinued(this.ctx, t, pts, bulges ?? []);
  }

  protected override reset(): void {
    this.target = null;
    super.reset();
  }
}
