import type { AppContext } from '../app/context';
import type { EditOperation } from '../contracts/generated/EditOperation';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import { Signal } from '../core/signal';
import { entityGeometry, type Entity, type EntityGeometry } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { offsetEntity, offsetThroughDistance, type OffsetResult } from '../model/ops/offset';
import { nearestS, pathOf, pointAtS } from '../model/ops/path';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { fenceEdits, planFence, type FencePlan } from './fenceTool';
import { drawTag, strokeGeometry, strokePath } from './preview';
import { markVertices } from './reshapePreview';
import type { Tool, ToolPointer } from './Tool';

/**
 * Tools that act on the edge under the cursor rather than on a selection.
 * This file holds the base and offset, trim, extend; corner tools live in
 * ./cornerTools, break/divide/vertex in ./pathEditTools.
 */
export abstract class EdgePickTool implements Tool {
  abstract readonly id: string;
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  get snaps(): boolean {
    return false;
  }
  protected hover: { entity: Entity; world: Vec2 } | null = null;
  protected readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  protected editable = (e: Entity) => !this.ctx.doc.layers.isLocked(e.layerId);

  activate(): void {
    this.refresh();
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
  }

  protected abstract refresh(): void;

  pointerMove(p: ToolPointer): void {
    const e = this.ctx.view.pickEdge(p.screen, this.editable);
    this.hover = e ? { entity: e, world: p.raw } : null;
    this.ctx.selection.hover.set(e?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  /**
   * Cutting open a polygon with holes would lose the holes; such tools say
   * so instead. Returns true when `e` was refused.
   */
  protected refuseHoled(e: Entity, action: string): boolean {
    if (e.kind !== 'polygon' || !e.holes?.length) return false;
    this.ctx.log.warn(`Adalı alanda ${action} yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl kullanın.`);
    return true;
  }

  /**
   * Replaces `e` by `pieces` in one undo step, through `cad.entities.edit`
   * (docs/adr/0047); attributes and the label survive a single piece. The
   * first piece is `e` itself, changed: the part a trim leaves, the first
   * part of a break, a line that took a vertex. It keeps its slot and
   * persistent id; the other pieces are new objects (docs/adr/0014).
   * Whether it was written; the command's refusal is said.
   */
  protected replace(operation: EditOperation, e: Entity, pieces: EntityGeometry[]): boolean {
    const { doc } = this.ctx;
    const keep = pieces.length === 1 && e.kind !== 'polygon';
    const uid = uidOf(this.ctx, e);
    const [first, ...others] = pieces;
    const changes: EntityEdit[] = [
      first ? { kind: 'replace', uid, geometry: editGeometry(first), ...(keep && { keepData: true }) } : { kind: 'remove', uid },
      ...others.map((piece): EntityEdit => ({ kind: 'add', from: uid, geometry: editGeometry(piece), ...(keep && { keepData: true }) })),
    ];
    const written = writeEdit(this.ctx, operation, changes) !== null;
    this.ctx.selection.retain((id) => !!doc.get(id));
    this.hover = null;
    this.ctx.selection.hover.set(null);
    return written;
  }
}

// ── Ötele, buda, uzat ──────────────────────────────────────────────────

/**
 * A point on the other side of `e` from `side`, a little way past the object: what the
 * core's offset takes to make the copy on the other side (it picks the side by the point).
 * The step is half the distance, at most, so a circle is not left behind its centre.
 * Null when the point lies on the object.
 */
function oppositeSide(e: Entity, side: Vec2, distance: number): Vec2 | null {
  const path = pathOf(e);
  // A construction line has no path: any point of it will do, the line runs through it.
  const foot = path ? pointAtS(path, nearestS(path, side)) : e.kind === 'xline' || e.kind === 'ray' ? e.p : side;
  const gap = dist(foot, side);
  // A click on the object itself has no side, so no other one either.
  if (gap < 1e-9) return null;
  const step = Math.min(gap, distance) / 2;
  return { x: foot.x + ((foot.x - side.x) / gap) * step, y: foot.y + ((foot.y - side.y) / gap) * step };
}

/** A toggle's value in the prompt: shown only when it is on (docs/adr/0140). */
const whenOn = (on: boolean) => (on ? ': açık' : '');

/**
 * Ötele: a parallel copy at a distance, on the side clicked. Options (docs/adr/0140):
 * Noktadan geç (N), İki yana (I: copies on both sides at the distance) and Kaynağı
 * sil (S: the source is deleted in the same step). Written through `cad.entities.edit`,
 * one undo step.
 */
export class OffsetTool extends EdgePickTool {
  readonly id = 'offset';
  private static distance = 1;
  /** "Noktadan geç": the copy passes through the clicked point (distance from the mouse). */
  private static through = false;
  /** "İki yana": copies on both sides of the object. */
  private static both = false;
  /** "Kaynağı sil": the source goes in the same step. */
  private static erase = false;
  private target: Entity | null = null;
  private side: Vec2 | null = null;

  protected refresh(): void {
    const opts = `Noktadan geç (N)${whenOn(OffsetTool.through)} / İki yana (I)${whenOn(OffsetTool.both)} / Kaynağı sil (S)${whenOn(OffsetTool.erase)}`;
    // The distance is in the tag by the cursor over an object, so the prompt keeps to the options.
    if (!this.target) this.prompt.set(`Ötele: ötelenecek nesneye tıklayın [${opts}]`);
    else this.prompt.set(`Ötele: ${OffsetTool.through ? 'kopyanın geçeceği noktaya tıklayın' : OffsetTool.both ? 'kopyaların gideceği tarafa tıklayın (iki yana çıkar)' : 'kopyanın gideceği tarafa tıklayın'} [${opts}]`);
  }

  /** Offset distance for a side point: fixed, or the point's distance to the object (the core's, docs/adr/0047). */
  private distanceFor(e: Entity, p: Vec2): number {
    return OffsetTool.through ? offsetThroughDistance(e, p) : OffsetTool.distance;
  }

  /** The copies for a click at `side`: the one on its side and, with İki yana, the one on the other (none on the other when the click lies on the object). */
  private copies(e: Entity, d: number, side: Vec2): { results: OffsetResult[]; onObject: boolean } {
    const results = [offsetEntity(e, d, side)];
    const other = OffsetTool.both ? oppositeSide(e, side, d) : null;
    if (other) results.push(offsetEntity(e, d, other));
    return { results, onObject: OffsetTool.both && !other };
  }

  override get snaps(): boolean {
    return OffsetTool.through && !!this.target;
  }

  override pointerMove(p: ToolPointer): void {
    if (this.target) {
      this.side = p.world;
      return this.ctx.view.requestOverlay();
    }
    super.pointerMove(p);
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (!this.target) {
      const e = this.ctx.view.pickEdge(p.screen, this.editable);
      if (!e) return this.ctx.log.warn('Düzenlenebilir bir çizgi, çoklu çizgi, daire ya da yaya tıklayın.');
      this.target = e;
      this.side = p.raw;
      this.ctx.selection.hover.set(null);
      return this.refresh();
    }
    this.commit(this.target, p.world);
    this.target = null;
    this.refresh();
    this.ctx.view.requestOverlay();
  }

  private commit(target: Entity, at: Vec2): void {
    const { log, format } = this.ctx;
    const d = this.distanceFor(target, at);
    const { results, onObject } = this.copies(target, d, at);
    const geometries = results.flatMap((r) => ('geometry' in r ? [r.geometry] : []));
    if (!geometries.length) return log.warn(results.map((r) => ('error' in r ? r.error : '')).find(Boolean) ?? 'Öteleme sonucu geçerli bir şekil oluşmadı.');
    OffsetTool.distance = d;
    const uid = uidOf(this.ctx, target);
    // New objects from the target: its layer and colour (docs/adr/0047); the source goes in the same step when asked.
    const changes: EntityEdit[] = geometries.map((g): EntityEdit => ({ kind: 'add', from: uid, geometry: editGeometry(g) }));
    if (OffsetTool.erase) changes.push({ kind: 'remove', uid });
    if (!writeEdit(this.ctx, 'offset', changes)) return;
    this.ctx.selection.retain((id) => !!this.ctx.doc.get(id));
    for (const r of results) if ('error' in r && geometries.length) log.warn(`Öbür yandaki kopya oluşmadı: ${r.error}`);
    const copies = geometries.length === 1 ? `${format.length(d)} ötelenmiş kopya eklendi` : `${format.length(d)} uzaklıkta iki yana ${geometries.length} ötelenmiş kopya eklendi`;
    log.success(`${copies}${OffsetTool.erase ? '; kaynak silindi' : ''}.`);
    if (onObject) log.warn('Tıklanan nokta nesnenin üzerinde; hangi yan olduğu belli olmadığı için tek kopya yapıldı. İki yana için nesnenin bir yanına tıklayın.');
  }

  input(text: string): boolean {
    const key = text.trim().toLocaleUpperCase('tr-TR');
    // “İki yana” is I: the dotless key and the dotted one a Turkish keyboard types.
    const toggle = key === 'N' ? 'through' : key === 'I' || key === 'İ' ? 'both' : key === 'S' ? 'erase' : null;
    if (toggle) {
      OffsetTool[toggle] = !OffsetTool[toggle];
      this.refresh();
      this.ctx.view.requestOverlay();
      return true;
    }
    const n = parseNumber(text);
    if (n === null || n <= 0) return false;
    OffsetTool.distance = n;
    OffsetTool.through = false;
    this.refresh();
    this.ctx.view.requestOverlay();
    return true;
  }

  confirm(): void {
    if (this.target) {
      this.target = null;
      return this.refresh();
    }
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    if (!this.target) return false;
    this.target = null;
    this.refresh();
    this.ctx.view.requestOverlay();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.target && this.hover && !OffsetTool.through) {
      // Over an object: the distance a click uses, and how to change it.
      const pal = this.ctx.view.palette;
      drawTag(g, view.worldToScreen(this.hover.world), [`Mesafe ${this.ctx.format.length(OffsetTool.distance)}`, 'Değiştirmek için sayı yazın'], pal.accent, pal.labelHalo);
    }
    if (!this.target || !this.side) return;
    const pal = this.ctx.view.palette;
    const d = this.distanceFor(this.target, this.side);
    const { results } = this.copies(this.target, d, this.side);
    for (const r of results) if ('geometry' in r) strokeGeometry(g, view, r.geometry, { color: pal.accent, dash: [4, 3] });
    // The source: accent as it stays, danger when it goes.
    strokeGeometry(g, view, entityGeometry(this.target), OffsetTool.erase ? { color: pal.danger, dash: [5, 3], width: 2 } : { color: pal.accent });
    drawTag(g, view.worldToScreen(this.side), [`Mesafe ${this.ctx.format.length(d)}`, ...(OffsetTool.both ? ['İki yana'] : []), ...(OffsetTool.erase ? ['Kaynak silinecek'] : [])], pal.accent, pal.labelHalo);
  }
}

/**
 * Trim and extend share their boundaries: every visible edge (default,
 * AutoCAD's quick mode) or the objects picked with "Sınır seç" (S). The
 * chosen boundaries stay highlighted as the selection. Shift+click does
 * the other operation, as in AutoCAD.
 */
abstract class BoundaryEdgeTool extends EdgePickTool {
  /** Chosen boundaries; null: every visible edge (the geometry store gathers them, the target left out). */
  private bounds: Set<number> | null = null;
  protected pickingBounds = false;
  protected shiftHeld = false;
  /** Tıklayarak (each click acts) or Çit (a fence of clicked points acts on everything it crosses, docs/adr/0140). */
  private mode: 'click' | 'fence' = 'click';
  private fence: Vec2[] = [];
  /** Where the pointer is (snapped), the fence's rubber band, and what the fence with it would do. */
  private mouse: Vec2 | null = null;
  private plan: FencePlan | null = null;
  /** The fence's action: Buda trims what it crosses, Uzat extends it. */
  protected abstract readonly fenceAction: 'trim' | 'extend';

  override get snaps(): boolean {
    return this.mode === 'fence' && !this.pickingBounds;
  }

  /** Prompt tail: how boundaries are chosen now. */
  protected boundsHint(): string {
    const n = this.bounds?.size ?? 0;
    return this.bounds ? `sınır: seçilen ${n} nesne; Tüm kenarlar (T) / Sınır seç (S)` : 'sınır: görünen tüm kenarlar; Sınır seç (S)';
  }

  protected abstract actionPrompt(): string;
  /** What a fence does, in the fence prompt: “kestiği her parça budanır”. */
  protected abstract fenceText(): string;
  /** The message after a fence acted: “3 nesne budandı”. */
  protected abstract fenceDone(n: number): string;

  protected refresh(): void {
    const n = this.ctx.selection.size;
    if (this.pickingBounds) this.prompt.set(`${this.label}: sınır olacak nesnelere tıklayın, bitince sağ tıklayın (${n} seçili) [Tüm kenarlar (T)]`);
    else if (this.mode === 'fence') {
      const found = this.fenceCount();
      const step = !this.fence.length
        ? `çitin ilk noktasına tıklayın: ${this.fenceText()}`
        : found !== null
          ? `çitin sonraki noktasına tıklayın; sağ tık: uygula (${found})`
          : `çitin sonraki noktasına tıklayın (${this.fence.length} nokta)`;
      this.prompt.set(`${this.label}: ${step} [${this.fence.length ? 'Geri (G) / ' : ''}Tıklayarak (K) / ${this.boundsHint()}]`);
    } else this.prompt.set(`${this.label}: ${this.actionPrompt()} [${this.boundsHint()}; Shift+tık: ${this.otherLabel}]`);
    this.ctx.view.requestOverlay();
  }

  protected abstract readonly label: string;
  protected abstract readonly otherLabel: string;

  input(text: string): boolean {
    const t = text.trim().toLocaleUpperCase('tr-TR');
    if (t === 'S') {
      this.pickingBounds = true;
      this.ctx.selection.set([...(this.bounds ?? [])]);
    } else if (t === 'T') {
      this.bounds = null;
      this.pickingBounds = false;
      this.ctx.selection.clear();
    } else if ((t === 'C' || t === 'Ç') && !this.pickingBounds) {
      this.mode = 'fence';
      this.fence = [];
      this.plan = null;
      this.hover = null;
      this.ctx.selection.hover.set(null);
    } else if (t === 'K' && this.mode === 'fence' && !this.pickingBounds) {
      this.mode = 'click';
      this.fence = [];
      this.plan = null;
    } else if (t === 'G' && this.mode === 'fence' && this.fence.length) {
      this.fence.pop();
      this.replan();
    } else return false;
    this.refresh();
    return true;
  }

  override pointerMove(p: ToolPointer): void {
    this.shiftHeld = p.shift;
    if (this.pickingBounds) return void this.ctx.selection.hover.set(this.ctx.view.pick(p.screen)?.id ?? null);
    if (this.mode === 'fence') {
      this.mouse = p.world;
      this.replan();
      return this.ctx.view.requestOverlay();
    }
    super.pointerMove(p);
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.pickingBounds) {
      const e = this.ctx.view.pick(p.screen);
      if (e) this.ctx.selection.toggle(e.id);
      return this.refresh();
    }
    if (this.mode === 'fence') {
      const last = this.fence.at(-1);
      if (last && dist(last, p.world) < 1e-9) return;
      this.fence.push(p.world);
      this.ctx.log.info(`  ${this.ctx.format.point(p.world)}`);
      this.replan();
      return this.refresh();
    }
    const e = this.ctx.view.pickEdge(p.screen, this.editable);
    if (!e) return this.ctx.log.warn(`${p.shift ? this.otherLabel : this.label} için düzenlenebilir bir kenara tıklayın.`);
    this.act(e, p.raw, p.shift);
  }

  protected abstract act(e: Entity, at: Vec2, other: boolean): void;

  snapFrom(): Vec2 | null {
    return this.mode === 'fence' ? (this.fence.at(-1) ?? null) : null;
  }

  confirm(): void {
    if (this.mode === 'fence' && !this.pickingBounds) {
      if (this.fence.length >= 2) return this.applyFence();
      if (this.fence.length) {
        this.fence = [];
        this.plan = null;
        return this.refresh();
      }
      // Nothing drawn: the tool ends (the manager's `cancel` then finds click mode and lets it go).
      this.mode = 'click';
      return this.ctx.tools.exit();
    }
    if (!this.pickingBounds) return this.ctx.tools.exit();
    const ids = [...this.ctx.selection.ids.value];
    this.bounds = ids.length ? new Set(ids) : null;
    this.pickingBounds = false;
    this.refresh();
  }

  cancel(): boolean {
    if (!this.pickingBounds && this.mode === 'fence') {
      // One step back: the fence, then the method.
      if (this.fence.length) this.fence = [];
      else this.mode = 'click';
      this.plan = null;
      this.refresh();
      return true;
    }
    if (!this.pickingBounds) return false;
    this.pickingBounds = false;
    this.ctx.selection.set([...(this.bounds ?? [])]);
    this.refresh();
    return true;
  }

  // ── Çit ──────────────────────────────────────────────────────────────

  /** The fence with the pointer as its next point, as the preview shows it. */
  private replan(): void {
    const pts = this.mouse ? [...this.fence, this.mouse] : this.fence;
    this.plan = this.fence.length ? planFence(this.ctx, this.fenceAction, pts, this.bounds, MAX_PREVIEW) : null;
  }

  /** How many objects the fence does something to, for the prompt; null before it is a fence. */
  private fenceCount(): string | null {
    if (this.fence.length < 2) return null;
    const plan = planFence(this.ctx, this.fenceAction, this.fence, this.bounds, MAX_PREVIEW);
    const n = plan.trims.length + plan.extends.length;
    return n ? `${n} nesne ${this.fenceAction === 'trim' ? 'budanacak' : 'uzatılacak'}` : 'kesilen nesne yok';
  }

  private applyFence(): void {
    const { log, selection } = this.ctx;
    const plan = planFence(this.ctx, this.fenceAction, this.fence, this.bounds);
    const n = plan.trims.length + plan.extends.length;
    if (plan.locked) log.warn(`${plan.locked} nesne kilitli katmanda olduğu için atlandı.`);
    if (!n) {
      log.warn(
        plan.failed
          ? `Çitin kestiği ${plan.failed} nesne ${this.fenceAction === 'trim' ? 'budanamadı' : 'uzatılamadı'}: ${this.fenceAction === 'trim' ? 'kesici kenar yok ya da alan adalı' : 'ucunun önünde kenar yok'}.`
          : 'Çit hiçbir düzenlenebilir nesneyi kesmiyor; çiti nesnelerin üzerinden geçirin.',
      );
      this.fence = [];
      this.plan = null;
      return this.refresh();
    }
    if (writeEdit(this.ctx, this.fenceAction, fenceEdits(this.ctx, plan))) {
      selection.retain((id) => !!this.ctx.doc.get(id));
      log.success(`${this.fenceDone(n)}${plan.failed ? `; ${plan.failed} nesne yapılamadı` : ''}.`);
    }
    this.fence = [];
    this.plan = null;
    this.hover = null;
    this.refresh();
  }

  private drawFence(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const trim = this.fenceAction === 'trim';
    const pts = this.mouse ? [...this.fence, this.mouse] : this.fence;
    for (const c of this.plan?.trims ?? []) {
      // The whole object dashed red, the pieces that stay solid on top: what remains red is what goes.
      strokeGeometry(g, view, entityGeometry(c.entity), { color: pal.danger, dash: [5, 3], width: 2 });
      for (const piece of c.pieces) strokeGeometry(g, view, entityGeometry(piece), { color: pal.accent, width: 1.5 });
    }
    for (const c of this.plan?.extends ?? []) strokeGeometry(g, view, c.geometry, { color: pal.accent, dash: [4, 3], width: 1.5 });
    if (this.plan?.crossings.length) markVertices(g, view, this.plan.crossings, trim ? pal.danger : pal.accent, trim);
    strokePath(g, view, pts, { color: pal.snap, dash: [7, 4], width: 1.5 });
    if (this.fence.length) markVertices(g, view, this.fence, pal.snap, false);
    if (this.mouse && this.fence.length) {
      const n = (this.plan?.trims.length ?? 0) + (this.plan?.extends.length ?? 0);
      drawTag(g, view.worldToScreen(this.mouse), [n ? `${n} nesne ${trim ? 'budanacak' : 'uzatılacak'}` : 'Nesne yok', 'Sağ tık: uygula'], n ? pal.accent : pal.danger, pal.labelHalo);
    }
  }

  protected trimAt(e: Entity, at: Vec2): void {
    if (this.refuseHoled(e, 'budama')) return;
    const r = this.ctx.view.trim(e, at, this.bounds);
    if ('error' in r) return this.ctx.log.warn(r.error);
    if (this.replace('trim', e, r.pieces)) this.ctx.log.success(`Budandı: ${r.pieces.length} parça kaldı.`);
  }

  protected extendAt(e: Entity, at: Vec2): void {
    const r = this.ctx.view.extend(e, at, this.bounds);
    if ('error' in r) return this.ctx.log.warn(r.error);
    if (writeEdit(this.ctx, 'extend', [{ kind: 'update', uid: uidOf(this.ctx, e), geometry: editGeometry(r.geometry) }])) this.ctx.log.success('Uzatıldı.');
  }

  /** Preview of trim (red goes, accent stays) or extend (dashed result). */
  protected preview(g: CanvasRenderingContext2D, view: ViewTransform, trim: boolean): void {
    if (this.mode === 'fence' && !this.pickingBounds) return this.drawFence(g, view);
    if (!this.hover || this.pickingBounds) return;
    const e = this.hover.entity;
    const pal = this.ctx.view.palette;
    if (trim) {
      if (e.kind === 'polygon' && e.holes?.length) return;
      const r = this.ctx.view.trim(e, this.hover.world, this.bounds);
      if ('error' in r) return;
      // The whole object dashed red, kept pieces solid on top: what remains red is what goes.
      strokeGeometry(g, view, entityGeometry(e), { color: pal.danger, dash: [5, 3], width: 2 });
      for (const piece of r.pieces) strokeGeometry(g, view, piece, { color: pal.accent, width: 1.5 });
      return;
    }
    const r = this.ctx.view.extend(e, this.hover.world, this.bounds);
    if ('geometry' in r) strokeGeometry(g, view, r.geometry, { color: pal.accent, dash: [4, 3], width: 1.5 });
  }
}

/** Most objects the live preview of a fence works out; the write itself does them all. */
const MAX_PREVIEW = 400;

export class TrimTool extends BoundaryEdgeTool {
  readonly id = 'trim';
  protected readonly label = 'Buda';
  protected readonly otherLabel = 'uzat';
  protected readonly fenceAction = 'trim';

  protected actionPrompt(): string {
    return 'silinecek parçaya tıklayın';
  }

  protected fenceText(): string {
    return 'kestiği her parça budanır';
  }

  protected fenceDone(n: number): string {
    return `Çit: ${n} nesne budandı`;
  }

  protected act(e: Entity, at: Vec2, other: boolean): void {
    if (other) this.extendAt(e, at);
    else this.trimAt(e, at);
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    this.preview(g, view, !this.shiftHeld);
  }
}

export class ExtendTool extends BoundaryEdgeTool {
  readonly id = 'extend';
  protected readonly label = 'Uzat';
  protected readonly otherLabel = 'buda';
  protected readonly fenceAction = 'extend';

  protected actionPrompt(): string {
    return 'uzatılacak ucun yakınına tıklayın';
  }

  protected fenceText(): string {
    return 'kestiği her nesnenin ucu uzatılır';
  }

  protected fenceDone(n: number): string {
    return `Çit: ${n} nesne uzatıldı`;
  }

  protected act(e: Entity, at: Vec2, other: boolean): void {
    if (other) this.trimAt(e, at);
    else this.extendAt(e, at);
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    this.preview(g, view, this.shiftHeld);
  }
}
