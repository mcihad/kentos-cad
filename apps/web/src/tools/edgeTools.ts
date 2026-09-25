import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { entityGeometry, type Entity, type EntityGeometry, type NewEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { closestOnEdge } from '../model/geom/intersect';
import { entityEdges } from '../model/ops/edges';
import { offsetEntity } from '../model/ops/offset';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { drawTag, strokeGeometry } from './preview';
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

  /** Common attributes carried over to pieces of an edited entity. */
  protected inherit(e: Entity, geom: EntityGeometry, keepData: boolean): NewEntity {
    return { ...geom, layerId: e.layerId, color: e.color, attrs: keepData ? { ...e.attrs } : {}, label: keepData ? e.label : undefined } as NewEntity;
  }

  /**
   * Replaces `e` by `pieces` in one undo step (attributes survive a single
   * piece). The first piece is `e` itself, changed: the part a trim leaves,
   * the first part of a break, a line that took a vertex. It keeps its slot
   * and persistent id; the other pieces are new objects (docs/adr/0014).
   */
  protected replace(label: string, e: Entity, pieces: EntityGeometry[]): void {
    const { doc } = this.ctx;
    const keep = pieces.length === 1 && e.kind !== 'polygon';
    const [first, ...others] = pieces;
    doc.transact(label, () => {
      if (first) doc.replace(e.id, this.inherit(e, first, keep));
      else doc.remove([e.id]);
      for (const piece of others) doc.add(this.inherit(e, piece, keep));
    });
    this.ctx.selection.retain((id) => !!doc.get(id));
    this.hover = null;
    this.ctx.selection.hover.set(null);
  }
}

// ── Ötele, buda, uzat ──────────────────────────────────────────────────

export class OffsetTool extends EdgePickTool {
  readonly id = 'offset';
  private static distance = 1;
  /** "Noktadan geç": the copy passes through the clicked point (distance from the mouse). */
  private static through = false;
  private target: Entity | null = null;
  private side: Vec2 | null = null;

  protected refresh(): void {
    const d = this.ctx.format.length(OffsetTool.distance);
    const opt = `Noktadan geç (N): ${OffsetTool.through ? 'açık' : 'kapalı'}`;
    if (!this.target) this.prompt.set(`Ötele: ötelenecek nesneye tıklayın [${OffsetTool.through ? '' : `mesafe ${d}; mesafe için sayı yazın; `}${opt}]`);
    else this.prompt.set(`Ötele: ${OffsetTool.through ? 'kopyanın geçeceği noktaya tıklayın' : 'kopyanın gideceği tarafa tıklayın'} [${opt}]`);
  }

  /** Offset distance for a side point: fixed, or the point's distance to the object. */
  private distanceFor(e: Entity, p: Vec2): number {
    if (!OffsetTool.through) return OffsetTool.distance;
    let d = Infinity;
    for (const ed of entityEdges(e)) d = Math.min(d, closestOnEdge(ed, p).d);
    return d;
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
    const d = this.distanceFor(this.target, p.world);
    const r = offsetEntity(this.target, d, p.world);
    if ('error' in r) this.ctx.log.warn(r.error);
    else {
      OffsetTool.distance = d;
      this.ctx.doc.add(this.inherit(this.target, r.geometry, false));
      this.ctx.log.success(`${this.ctx.format.length(d)} ötelenmiş kopya eklendi.`);
    }
    this.target = null;
    this.refresh();
    this.ctx.view.requestOverlay();
  }

  input(text: string): boolean {
    if (text.trim().toLocaleUpperCase('tr-TR') === 'N') {
      OffsetTool.through = !OffsetTool.through;
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
    if (!this.target || !this.side) return;
    const pal = this.ctx.view.palette;
    const d = this.distanceFor(this.target, this.side);
    const r = offsetEntity(this.target, d, this.side);
    if ('geometry' in r) strokeGeometry(g, view, r.geometry, { color: pal.accent, dash: [4, 3] });
    strokeGeometry(g, view, entityGeometry(this.target), { color: pal.accent });
    drawTag(g, view.worldToScreen(this.side), [`Mesafe ${this.ctx.format.length(d)}`], pal.accent, pal.labelHalo);
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

  /** Prompt tail: how boundaries are chosen now. */
  protected boundsHint(): string {
    const n = this.bounds?.size ?? 0;
    return this.bounds ? `sınır: seçilen ${n} nesne; Tüm kenarlar (T) / Sınır seç (S)` : 'sınır: görünen tüm kenarlar; Sınır seç (S)';
  }

  protected abstract actionPrompt(): string;

  protected refresh(): void {
    const n = this.ctx.selection.size;
    this.prompt.set(
      this.pickingBounds
        ? `${this.label}: sınır olacak nesnelere tıklayın, bitince sağ tıklayın (${n} seçili) [Tüm kenarlar (T)]`
        : `${this.label}: ${this.actionPrompt()} [${this.boundsHint()}; Shift+tık: ${this.otherLabel}]`,
    );
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
    } else return false;
    this.refresh();
    return true;
  }

  override pointerMove(p: ToolPointer): void {
    this.shiftHeld = p.shift;
    if (!this.pickingBounds) return super.pointerMove(p);
    this.ctx.selection.hover.set(this.ctx.view.pick(p.screen)?.id ?? null);
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.pickingBounds) {
      const e = this.ctx.view.pick(p.screen);
      if (e) this.ctx.selection.toggle(e.id);
      return this.refresh();
    }
    const e = this.ctx.view.pickEdge(p.screen, this.editable);
    if (!e) return this.ctx.log.warn(`${p.shift ? this.otherLabel : this.label} için düzenlenebilir bir kenara tıklayın.`);
    this.act(e, p.raw, p.shift);
  }

  protected abstract act(e: Entity, at: Vec2, other: boolean): void;

  confirm(): void {
    if (!this.pickingBounds) return this.ctx.tools.exit();
    const ids = [...this.ctx.selection.ids.value];
    this.bounds = ids.length ? new Set(ids) : null;
    this.pickingBounds = false;
    this.refresh();
  }

  cancel(): boolean {
    if (!this.pickingBounds) return false;
    this.pickingBounds = false;
    this.ctx.selection.set([...(this.bounds ?? [])]);
    this.refresh();
    return true;
  }

  protected trimAt(e: Entity, at: Vec2): void {
    if (this.refuseHoled(e, 'budama')) return;
    const r = this.ctx.view.trim(e, at, this.bounds);
    if ('error' in r) return this.ctx.log.warn(r.error);
    this.replace('Buda', e, r.pieces);
    this.ctx.log.success(`Budandı: ${r.pieces.length} parça kaldı.`);
  }

  protected extendAt(e: Entity, at: Vec2): void {
    const r = this.ctx.view.extend(e, at, this.bounds);
    if ('error' in r) return this.ctx.log.warn(r.error);
    this.ctx.doc.update(e.id, r.geometry as Partial<Entity>);
    this.ctx.log.success('Uzatıldı.');
  }

  /** Preview of trim (red goes, accent stays) or extend (dashed result). */
  protected preview(g: CanvasRenderingContext2D, view: ViewTransform, trim: boolean): void {
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

export class TrimTool extends BoundaryEdgeTool {
  readonly id = 'trim';
  protected readonly label = 'Buda';
  protected readonly otherLabel = 'uzat';

  protected actionPrompt(): string {
    return 'silinecek parçaya tıklayın';
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

  protected actionPrompt(): string {
    return 'uzatılacak ucun yakınına tıklayın';
  }

  protected act(e: Entity, at: Vec2, other: boolean): void {
    if (other) this.trimAt(e, at);
    else this.extendAt(e, at);
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    this.preview(g, view, this.shiftHeld);
  }
}
