import type { EditOperation } from '../contracts/generated/EditOperation';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import { entityGeometry, type Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { allCorners, reverseEntity, simplifyEntity, type CornerValue, type Reshaped } from '../model/ops/reshape';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { SelectionActionTool } from './editTools';
import { drawTag, strokeGeometry } from './preview';
import { markVertices, strokeDirections } from './reshapePreview';

/**
 * Tools that reshape whole objects with one value (docs/adr/0140): every
 * corner rounded or cut (Tüm köşeleri yuvarla, Tüm köşelere pah), the
 * vertices within a tolerance dropped (Sadeleştir). The geometry comes from
 * the shared core (`allCorners`, `simplifyEntity`); the tools write it
 * through `cad.entities.edit`, all objects of one confirm as one undo step.
 *
 * Objects are picked before or after (a selection skips the step). The value
 * is typed and remembered; the result is previewed live and applied by
 * Enter, right click or a click in the drawing.
 */

/** What the value does to every target: the objects that change, and the counts the message tells. */
interface Plan {
  items: { entity: Entity; result: Reshaped }[];
  done: number;
  skipped: number;
  deviation: number;
  /** Objects the value found nothing to do in. */
  unchanged: number;
}

abstract class ValueReshapeTool<V> extends SelectionFirstTool {
  protected abstract readonly operation: EditOperation;
  /** Kinds of object the tool works on, in words ("çoklu çizgi ya da alan"), for its messages. */
  protected abstract readonly kindsText: string;
  protected abstract accepts(e: Entity): boolean;
  protected abstract current(): V | null;
  protected abstract remember(v: V): void;
  /** The value typed, or null when not understood; says why itself when it is a value but a wrong one. */
  protected abstract parse(text: string): V | null | 'refused';
  protected abstract compute(e: Entity, v: V): Reshaped | null;
  /** The value in the prompt, "yarıçap 2 m". */
  protected abstract valueText(v: V): string;
  protected abstract askText(): string;
  /** The tag beside the cursor, from the plan. */
  protected abstract tag(plan: Plan): string[];
  /** The message after the write. */
  protected abstract doneText(plan: Plan): string;
  /** The message when the value did nothing. */
  protected abstract nothingText(plan: Plan): string;

  private targetList: Entity[] = [];
  private plan: Plan | null = null;

  protected begin(): void {
    const { doc, log } = this.ctx;
    const all = this.targets();
    const fitting = all.filter((e) => this.accepts(e));
    const editable = fitting.filter((e) => !doc.layers.isLocked(e.layerId));
    if (fitting.length < all.length) log.warn(`${all.length - fitting.length} nesne ${this.kindsText} olmadığı için atlandı.`);
    if (editable.length < fitting.length) log.warn(`${fitting.length - editable.length} nesne kilitli katmanda olduğu için atlandı.`);
    if (!editable.length) {
      if (fitting.length === editable.length) log.warn(`Seçilenlerde ${this.kindsText} yok; ${this.kindsText} seçin.`);
      this.picking = true;
      this.targetList = [];
      this.plan = null;
      return;
    }
    this.targetList = editable;
    this.recompute();
  }

  private recompute(): void {
    const v = this.current();
    if (v === null) {
      this.plan = null;
      return;
    }
    const plan: Plan = { items: [], done: 0, skipped: 0, deviation: 0, unchanged: 0 };
    for (const entity of this.targetList) {
      const result = this.compute(entity, v);
      if (!result || result.done === 0) {
        plan.unchanged++;
        // A corner the value did not fit is still counted, on an object nothing was done in.
        plan.skipped += result?.skipped ?? 0;
        continue;
      }
      plan.items.push({ entity, result });
      plan.done += result.done;
      plan.skipped += result.skipped;
      plan.deviation = Math.max(plan.deviation, result.deviation);
    }
    this.plan = plan;
  }

  protected stagePrompt(): string {
    const v = this.current();
    const n = this.targetList.length;
    if (v === null) return `${n} nesne için ${this.askText()}`;
    return `${n} nesne için ${this.askText()} [Uygula (Enter); ${this.valueText(v)}]`;
  }

  protected point(): void {
    this.apply();
  }

  override input(text: string): boolean {
    if (this.picking) return false;
    const v = this.parse(text);
    if (v === null) return false;
    if (v === 'refused') return true;
    this.remember(v);
    this.recompute();
    this.refresh();
    return true;
  }

  override confirm(): void {
    if (this.picking) return super.confirm();
    this.apply();
  }

  cancel(): boolean {
    // One step back: from the value to the picking of objects.
    if (this.picking) return false;
    this.picking = true;
    this.plan = null;
    this.hover = null;
    this.refresh();
    return true;
  }

  private apply(): void {
    const { log, selection } = this.ctx;
    if (this.current() === null) return log.warn(`Önce ${this.askText().split(';')[0]}.`);
    this.recompute();
    const plan = this.plan!;
    if (!plan.items.length) return log.warn(this.nothingText(plan));
    const changes: EntityEdit[] = plan.items.map(({ entity, result }) => ({ kind: 'update', uid: uidOf(this.ctx, entity), geometry: editGeometry(entityGeometry(result.entity)) }));
    if (!writeEdit(this.ctx, this.operation, changes)) return;
    selection.retain((id) => !!this.ctx.doc.get(id));
    log.success(this.doneText(plan));
    // Done: the manager asks `cancel` before it leaves, and a tool already back at picking lets it.
    this.picking = true;
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking || !this.plan) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    const { plan } = this;
    for (const { result } of plan.items.slice(0, MAX_GHOSTS)) strokeGeometry(g, view, entityGeometry(result.entity), { color: pal.accent, width: 2.25 });
    this.drawMarks(g, view, plan, pal.danger);
    if (this.hover) drawTag(g, view.worldToScreen(this.hover), [...this.tag(plan), 'Tıklayın: uygula'], pal.accent, pal.labelHalo);
  }

  /** Extra marks on the preview (Sadeleştir strikes the vertices that go). */
  protected drawMarks(_g: CanvasRenderingContext2D, _view: ViewTransform, _plan: Plan, _color: string): void {}
}

const cornerObject = (e: Entity) => e.kind === 'polyline' || e.kind === 'polygon';

/** "12 köşe" style counts, the words of the corner tools' messages. */
const skippedText = (n: number) => `${n} köşe sığmadığı ya da yaya komşu olduğu için atlandı`;

export class FilletAllTool extends ValueReshapeTool<{ radius: number }> {
  readonly id = 'filletAll';
  protected readonly label = 'Tüm köşeleri yuvarla';
  protected readonly operation = 'fillet';
  protected readonly kindsText = 'çoklu çizgi ya da alan';
  private static last: { radius: number } | null = null;

  protected accepts(e: Entity): boolean {
    return cornerObject(e);
  }
  protected current() {
    return FilletAllTool.last;
  }
  protected remember(v: { radius: number }): void {
    FilletAllTool.last = v;
  }
  protected parse(text: string) {
    const n = parseNumber(text);
    if (n === null || /[,;@<]/.test(text)) return null;
    if (!(n > 0)) {
      this.ctx.log.warn('Yarıçap sıfırdan büyük olmalı.');
      return 'refused' as const;
    }
    return { radius: n };
  }
  protected compute(e: Entity, v: CornerValue) {
    return allCorners(e, v);
  }
  protected valueText(v: { radius: number }): string {
    return `yarıçap ${this.ctx.format.length(v.radius)}`;
  }
  protected askText(): string {
    return 'yarıçapı yazın';
  }
  protected tag(plan: Plan): string[] {
    return [`${plan.done} köşe yuvarlanacak`, ...(plan.skipped ? [`${plan.skipped} köşe atlanacak`] : [])];
  }
  protected doneText(plan: Plan): string {
    return `${plan.done} köşe yuvarlandı${plan.skipped ? `; ${skippedText(plan.skipped)}` : ''}.`;
  }
  protected nothingText(plan: Plan): string {
    return plan.skipped ? `Hiçbir köşe yuvarlanamadı: ${skippedText(plan.skipped)}. Daha küçük bir yarıçap yazın.` : 'Seçili nesnelerde yuvarlanacak köşe yok.';
  }
}

export class ChamferAllTool extends ValueReshapeTool<{ d1: number; d2: number }> {
  readonly id = 'chamferAll';
  protected readonly label = 'Tüm köşelere pah';
  protected readonly operation = 'chamfer';
  protected readonly kindsText = 'çoklu çizgi ya da alan';
  private static last: { d1: number; d2: number } | null = null;

  protected accepts(e: Entity): boolean {
    return cornerObject(e);
  }
  protected current() {
    return ChamferAllTool.last;
  }
  protected remember(v: { d1: number; d2: number }): void {
    ChamferAllTool.last = v;
  }
  protected parse(text: string) {
    const m = text.trim().match(/^(\d+(?:\.\d+)?)(?:\s*[,;]\s*(\d+(?:\.\d+)?))?$/);
    if (!m) return null;
    const v = { d1: +m[1], d2: m[2] !== undefined ? +m[2] : +m[1] };
    if (!(v.d1 > 0 && v.d2 > 0)) {
      this.ctx.log.warn('Pah mesafeleri sıfırdan büyük olmalı.');
      return 'refused' as const;
    }
    return v;
  }
  protected compute(e: Entity, v: CornerValue) {
    return allCorners(e, v);
  }
  private text(v: { d1: number; d2: number }): string {
    const f = this.ctx.format;
    return v.d1 === v.d2 ? f.length(v.d1) : `${f.length(v.d1, false)} ile ${f.length(v.d2)}`;
  }
  protected valueText(v: { d1: number; d2: number }): string {
    return `mesafe ${this.text(v)}`;
  }
  protected askText(): string {
    return 'mesafeyi yazın (d ya da d1,d2)';
  }
  protected tag(plan: Plan): string[] {
    return [`${plan.done} köşeye pah kırılacak`, ...(plan.skipped ? [`${plan.skipped} köşe atlanacak`] : [])];
  }
  protected doneText(plan: Plan): string {
    return `${plan.done} köşeye pah kırıldı${plan.skipped ? `; ${skippedText(plan.skipped)}` : ''}.`;
  }
  protected nothingText(plan: Plan): string {
    return plan.skipped ? `Hiçbir köşeye pah kırılamadı: ${skippedText(plan.skipped)}. Daha küçük bir mesafe yazın.` : 'Seçili nesnelerde pah kırılacak köşe yok.';
  }
}

export class SimplifyTool extends ValueReshapeTool<{ tolerance: number }> {
  readonly id = 'simplify';
  protected readonly label = 'Sadeleştir';
  protected readonly operation = 'simplify';
  protected readonly kindsText = 'çoklu çizgi ya da alan';
  private static last: { tolerance: number } | null = null;

  protected accepts(e: Entity): boolean {
    return cornerObject(e);
  }
  protected current() {
    return SimplifyTool.last;
  }
  protected remember(v: { tolerance: number }): void {
    SimplifyTool.last = v;
  }
  protected parse(text: string) {
    const n = parseNumber(text);
    if (n === null || /[,;@<]/.test(text)) return null;
    if (!(n > 0)) {
      this.ctx.log.warn('Tolerans sıfırdan büyük olmalı.');
      return 'refused' as const;
    }
    return { tolerance: n };
  }
  protected compute(e: Entity, v: { tolerance: number }) {
    return simplifyEntity(e, v.tolerance);
  }
  protected valueText(v: { tolerance: number }): string {
    return `tolerans ${this.ctx.format.length(v.tolerance)}`;
  }
  protected askText(): string {
    return 'toleransı yazın (m)';
  }
  protected tag(plan: Plan): string[] {
    return [`${plan.done} köşe atılacak`, `en büyük sapma ${this.ctx.format.length(plan.deviation)}`];
  }
  protected doneText(plan: Plan): string {
    return `Sadeleştir: ${plan.done} köşe atıldı; en büyük sapma ${this.ctx.format.length(plan.deviation)}.`;
  }
  protected nothingText(): string {
    return 'Bu toleransla atılacak köşe yok; daha büyük bir tolerans yazın.';
  }
  protected override drawMarks(g: CanvasRenderingContext2D, view: ViewTransform, plan: Plan, color: string): void {
    for (const { entity, result } of plan.items.slice(0, MAX_GHOSTS)) markVertices(g, view, removedVertices(entity, result.entity), color, true);
  }
}

/** The vertices `before` has that `after` no longer has: the ones a simplification dropped. */
export function removedVertices(before: Entity, after: Entity): Vec2[] {
  const rings = (e: Entity): Vec2[][] => (e.kind === 'polyline' ? [e.pts] : e.kind === 'polygon' ? [e.pts, ...(e.holes ?? []).map((h) => h.pts)] : []);
  const kept = new Set(rings(after).flat().map((p) => `${p.x},${p.y}`));
  return rings(before)
    .flat()
    .filter((p) => !kept.has(`${p.x},${p.y}`));
}

// ── Yönü çevir ─────────────────────────────────────────────────────────

/**
 * Yönü çevir: the selected objects drawn the other way (docs/adr/0140). With
 * objects selected it acts at once; otherwise the user picks and presses
 * Enter. While picking, arrows show the direction each object will have.
 */
export class ReverseTool extends SelectionActionTool {
  readonly id = 'reverse';
  protected readonly label = 'Yönü çevir';

  protected run(targets: Entity[]): void {
    const { log } = this.ctx;
    const changes: EntityEdit[] = [];
    for (const e of targets) {
      const r = reverseEntity(e);
      if (r) changes.push({ kind: 'update', uid: uidOf(this.ctx, e), geometry: editGeometry(entityGeometry(r)) });
    }
    if (!changes.length) return log.warn('Seçili nesnelerin yönü çevrilemez. Çizgi, çoklu çizgi, eğri ve alanın yönü çevrilir; yay, elips ve daire hep saat yönünün tersine çizilir.');
    if (changes.length < targets.length) log.warn(`${targets.length - changes.length} nesnenin yönü olmadığı için atlandı (yay, elips, daire, nokta, yazı…).`);
    if (!writeEdit(this.ctx, 'reverse', changes)) return;
    log.success(`${changes.length} nesnenin yönü çevrildi.`);
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    super.draw(g, view);
    const { doc, selection, view: v } = this.ctx;
    const pal = v.palette;
    const ids = new Set(selection.ids.value);
    const hover = selection.hover.value;
    if (hover !== null) ids.add(hover);
    let n = 0;
    for (const id of ids) {
      const e = doc.get(id);
      if (!e || doc.layers.isLocked(e.layerId) || n++ >= MAX_GHOSTS) continue;
      const r = reverseEntity(e);
      if (r) strokeDirections(g, view, r, pal.accent, pal.labelHalo);
    }
  }
}

