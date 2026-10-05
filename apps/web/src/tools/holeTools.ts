import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { entityArea, type Entity } from '../model/entities';
import { bulgeRingArea } from '../model/geom/bulge';
import type { Area } from '../model/geom/overlay';
import type { Vec2 } from '../model/geometry';
import { areasOfEntity } from '../model/ops/areas';
import { holeAdd, holeRemove, holeRing, type HoleRefusal, type Ring } from '../model/ops/reshapeBy';
import type { ViewTransform } from '../viewport/Camera';
import { createdIds, editGeometry, uidOf, writeEdit } from './editCommand';
import { PathTool } from './pathTool';
import { drawArea, drawTag, tint } from './preview';
import type { Tool, ToolPointer } from './Tool';

/**
 * Delik ekle, Deliği sil and Deliği doldur (docs/adr/0173 §5); the desktop's `holes.rs`, step for step.
 *
 * - Delik ekle draws its ring as Kapalı alan does; its first point finds the area (`holeTarget`: the selected one when
 *   it is around the point, else the smallest around it), the finished ring is cut from it.
 * - Deliği sil and Deliği doldur highlight the hole under the pointer; a click removes it, or writes a new area of its
 *   ring (the hole stays: the new area fills it) on the area's layer, in its colour and line weight, without its
 *   attributes and label.
 *
 * Every write is one `cad.entities.edit` step named after the tool; the command carries elevations by where the
 * vertices lie (docs/adr/0142). The geometry is the shared core's (`ops::holes`).
 */

/** Delik ekle's first point is in no area. */
export const NO_TARGET = 'Deliğin ekleneceği alanı seçin ya da halkaya bir alanın içinden başlayın.';
/** A click of Deliği sil or Deliği doldur outside every hole. */
export const NOT_IN_HOLE = 'Bir deliğin içine tıklayın.';

/** An area on an unlocked layer. */
function openArea(ctx: AppContext, e: Entity): boolean {
  return e.kind === 'polygon' && !ctx.doc.layers.isLocked(e.layerId);
}

/**
 * The area Delik ekle's ring starting at `p` goes into: the one selected, when it alone is and `p` is in it or in one
 * of its holes; else the smallest area around `p`, then the area with the smallest hole around it (a ring started in
 * a hole widens it). None on locked layers.
 */
export function holeTarget(ctx: AppContext, p: Vec2): Entity | null {
  const around = [...ctx.view.containing(p).map((c) => c.entity), ...ctx.view.holesAt(p).map((h) => h.entity)].filter((e) => openArea(ctx, e));
  const sel = [...ctx.selection.ids.value];
  const chosen = sel.length === 1 ? around.find((e) => e.id === sel[0]) : undefined;
  return chosen ?? around[0] ?? null;
}

/** The refusal in the tools' words. */
function refusalText(r: HoleRefusal): string {
  switch (r.why) {
    case 'notArea':
      return 'Delik yalnız kapalı alana eklenir.';
    case 'degenerate':
      return 'Delik en az üç köşeli, alanı olan bir halka olmalı.';
    case 'outside':
      return "Delik alanın içinde kalmalı; sınırı aşan bölümü çıkarmak için Alan çıkar'ı kullanın.";
    case 'splits':
      return "Delik alanı parçalara ayırırdı; alanı bölmek için Alan böl'ü kullanın.";
    case 'notInHole':
      return NOT_IN_HOLE;
  }
}

/** How many holes an area has, in all its parts. */
function holeCount(e: Entity): number {
  if (e.kind !== 'polygon') return 0;
  return (e.holes?.length ?? 0) + (e.parts ?? []).reduce((n, part) => n + (part.holes?.length ?? 0), 0);
}

/** The area's net size now, as the log says it. */
function areaText(ctx: AppContext, id: number): string {
  const e = ctx.doc.get(id);
  return ctx.format.area((e && entityArea(e)) ?? 0);
}

/** Delik ekle: Kapalı alan's ring, cut from the area its first point is in. */
export class HoleAddTool extends PathTool {
  /** The area the ring goes into, found at its first point. */
  private target: Entity | null = null;
  /** That area (or the one a click would take), lightly filled under the ring. */
  private targetAreas: Area[] = [];

  constructor(ctx: AppContext) {
    super(ctx, { id: 'holeAdd', label: 'Delik ekle', closed: true });
  }

  override deactivate(): void {
    super.deactivate();
    this.ctx.selection.hover.set(null);
  }

  protected override promptFor(n: number): string {
    return n === 0 ? 'deliğin ilk köşesini alanın içinde belirtin' : super.promptFor(n);
  }

  /** The area the ring goes into, or would at a click here, highlighted. */
  override pointerMove(p: ToolPointer): void {
    super.pointerMove(p);
    const area = this.pts.length ? this.target : this.hover && holeTarget(this.ctx, this.hover);
    this.ctx.selection.hover.set(area?.id ?? null);
    this.targetAreas = area ? areasOfEntity(area) : [];
  }

  protected override reset(): void {
    this.targetAreas = [];
    super.reset();
  }

  /** The area the ring goes into, lightly filled under the ring. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    for (const a of this.targetAreas) drawArea(g, view, a, { color: pal.accent, fill: tint(pal.accent, 0.12), width: 1 });
    super.draw(g, view);
  }

  protected override onPoint(p: Vec2): void {
    // The ring starts in the area it goes into.
    if (!this.pts.length) {
      this.target = holeTarget(this.ctx, p);
      if (!this.target) return this.ctx.log.warn(NO_TARGET);
    }
    super.onPoint(p);
  }

  protected override writeShape(pts: Vec2[], bulges: number[] | undefined): void {
    const { ctx } = this;
    const before = this.target && ctx.doc.get(this.target.id);
    this.ctx.selection.hover.set(null);
    if (!before) return;
    const ring: Ring = { pts, ...(bulges && { bulges }) };
    const out = holeAdd(before, ring);
    if ('refusal' in out) return ctx.log.warn(refusalText(out.refusal));
    const merged = holeCount(out.done) <= holeCount(before);
    if (!writeEdit(ctx, 'holeAdd', [{ kind: 'update', uid: uidOf(ctx, before), geometry: editGeometry(out.done) }])) return;
    const size = areaText(ctx, before.id);
    ctx.log.success(merged ? `Delik eklendi, var olan delikle birleşti; alan ${size} oldu.` : `Delik eklendi; alan ${size} oldu.`);
  }
}

/** The hole under a point: its area and its ring. */
interface Found {
  entity: Entity;
  at: Vec2;
  ring: Ring;
}

/** The smallest hole around `p` of an area on an unlocked layer. */
function found(ctx: AppContext, p: Vec2): Found | null {
  for (const h of ctx.view.holesAt(p)) {
    if (!openArea(ctx, h.entity)) continue;
    const ring = holeRing(h.entity, p);
    if ('done' in ring) return { entity: h.entity, at: p, ring: ring.done };
  }
  return null;
}

/** Deliği sil or Deliği doldur: a click in a hole removes it, or fills it with a new area. */
export class HoleClickTool implements Tool {
  readonly id: 'holeRemove' | 'holeFill';
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  private readonly fill: boolean;
  private hover: Found | null = null;

  constructor(ctx: AppContext, fill: boolean) {
    this.ctx = ctx;
    this.fill = fill;
    this.id = fill ? 'holeFill' : 'holeRemove';
  }

  private get label(): string {
    return this.fill ? 'Deliği doldur' : 'Deliği sil';
  }

  activate(): void {
    this.prompt.set(`${this.label}: ${this.fill ? 'doldurulacak' : 'silinecek'} deliğin içine tıklayın`);
  }

  pointerMove(p: ToolPointer): void {
    this.hover = found(this.ctx, p.raw);
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const hole = found(this.ctx, p.raw);
    if (!hole) return this.ctx.log.warn(NOT_IN_HOLE);
    if (this.fill) this.fillAt(hole);
    else this.removeAt(hole);
    // What the click changed is under the pointer now.
    this.hover = found(this.ctx, p.raw);
    this.ctx.view.requestOverlay();
  }

  private removeAt(hole: Found): void {
    const { ctx } = this;
    const out = holeRemove(hole.entity, hole.at);
    if ('refusal' in out) return ctx.log.warn(refusalText(out.refusal));
    if (!writeEdit(ctx, 'holeRemove', [{ kind: 'update', uid: uidOf(ctx, hole.entity), geometry: editGeometry(out.done) }])) return;
    ctx.log.success(`Delik silindi; alan ${areaText(ctx, hole.entity.id)} oldu.`);
  }

  private fillAt(hole: Found): void {
    const { ctx } = this;
    const geometry = editGeometry({ kind: 'polygon', pts: hole.ring.pts, ...(hole.ring.bulges && { bulges: hole.ring.bulges }) });
    const out = writeEdit(ctx, 'holeFill', [{ kind: 'add', from: uidOf(ctx, hole.entity), geometry }]);
    if (!out) return;
    const made = createdIds(ctx, out);
    ctx.selection.set(made);
    ctx.log.success(`Delik dolduruldu: yeni alan ${made.length ? areaText(ctx, made[0]) : ''}.`);
  }

  input(): boolean {
    return false;
  }

  /** Enter, Space or a quick right click leave the tool. */
  confirm(): void {
    this.ctx.tools.exit();
  }

  /** The hole under the pointer filled lightly, its area by it. */
  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const h = this.hover;
    if (!h) return;
    const pal = this.ctx.view.palette;
    drawArea(g, view, { outer: h.ring, holes: [] }, { color: pal.accent, fill: tint(pal.accent, 0.16), width: 2 });
    const size = Math.abs(bulgeRingArea(h.ring.pts, h.ring.bulges));
    drawTag(g, view.worldToScreen(h.at), [`Delik ${this.ctx.format.area(size)}`], pal.accent, pal.labelHalo);
  }
}
