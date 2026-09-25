import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { ENTITY_KIND_LABEL, type Entity, type NewEntity } from '../model/entities';
import { dist, type Bounds, type Vec2 } from '../model/geometry';
import { translation } from '../model/geom/affine';
import { dimensionLabel } from '../model/geom/dimension';
import { explodeEntity } from '../model/ops/explode';
import { joinEntities } from '../model/ops/join';
import { stretchEntity } from '../model/ops/stretch';
import { transformedFrom } from '../model/ops/transform';
import type { ViewTransform } from '../viewport/Camera';
import { CoreStore } from '../wasm/core';
import { packEntities } from '../wasm/pack';
import { parseNumber } from './coordinateInput';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { drawTag, strokePath, strokePaths } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { constrainPoint, drawTracking, pointFromText, type Tracking } from './tracking';

/**
 * Selection tools that act at once: with objects already selected they run
 * immediately, otherwise the user picks and presses Enter.
 */
export abstract class SelectionActionTool extends SelectionFirstTool {
  protected abstract run(targets: Entity[]): void;

  protected begin(): void {
    const { doc, log } = this.ctx;
    const all = this.targets();
    const editable = all.filter((e) => !doc.layers.isLocked(e.layerId));
    if (editable.length < all.length) log.warn(`${all.length - editable.length} nesne kilitli katmanda olduğu için atlandı.`);
    if (editable.length) this.run(editable);
    // Leaving from inside activate() would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }
  protected stagePrompt(): string {
    return '';
  }
  protected point(): void {}
}

export class JoinTool extends SelectionActionTool {
  readonly id = 'join';
  protected readonly label = 'Birleştir';
  private static tolerance = 0.001;

  protected override pickHint(): string {
    return `[uç boşluğu toleransı ${this.ctx.format.length(JoinTool.tolerance)}; değiştirmek için sayı yazın]`;
  }

  override input(text: string): boolean {
    const n = parseNumber(text);
    if (!this.picking || n === null || n < 0) return super.input(text);
    JoinTool.tolerance = n;
    this.refresh();
    return true;
  }

  protected run(targets: Entity[]): void {
    const { doc, log, selection } = this.ctx;
    const { groups } = joinEntities(targets, Math.max(JoinTool.tolerance, 1e-9));
    if (!groups.length) {
      log.warn('Uçları birleşen çizgi, yay ya da açık çoklu çizgi bulunamadı. Toleransı artırmayı deneyin.');
      return;
    }
    // Each chain takes its first object's layer, colour, attributes and label.
    const joined = groups.map((g) => {
      const first = doc.get(g.sources[0])!;
      return { ...g.geometry, layerId: first.layerId, color: first.color, attrs: { ...first.attrs }, label: first.label } as NewEntity;
    });
    const created = doc.transact('Birleştir', () => {
      doc.remove(groups.flatMap((g) => g.sources));
      return doc.addMany(joined).map((e) => e.id);
    });
    selection.set(created);
    const kinds = groups.map((g) => ENTITY_KIND_LABEL[g.geometry.kind].toLocaleLowerCase('tr-TR'));
    log.success(`${groups.reduce((n, g) => n + g.sources.length, 0)} nesne birleştirildi: ${kinds.join(', ')}.`);
  }
}

export class ExplodeTool extends SelectionActionTool {
  readonly id = 'explode';
  protected readonly label = 'Patlat';

  protected run(targets: Entity[]): void {
    const { doc, log, selection, format } = this.ctx;
    const gone: number[] = [];
    const pieces: NewEntity[] = [];
    let firstError: string | null = null;
    for (const e of targets) {
      const r = explodeEntity(e, (l) => dimensionLabel(undefined, l, { length: (m) => format.length(m, false), angle: (a) => format.angle(a) }), this.ctx.doc.settings.drawingFont.value);
      if ('error' in r) {
        firstError ??= r.error;
        continue;
      }
      gone.push(e.id);
      for (const piece of r.pieces) pieces.push({ ...piece, layerId: e.layerId, color: e.color, attrs: {} } as NewEntity);
    }
    const exploded = gone.length;
    const created = doc.transact('Patlat', () => {
      doc.remove(gone);
      return doc.addMany(pieces).map((e) => e.id);
    });
    if (firstError && !exploded) return log.warn(firstError);
    selection.set(created);
    log.success(`${exploded} nesne patlatıldı: ${created.length} parça.${firstError ? ` Bazı nesneler atlandı: ${firstError}` : ''}`);
  }
}

// ── Esnet ──────────────────────────────────────────────────────────────

/**
 * Stretch: a crossing window picks the vertices to move (click two corners
 * or drag), then base and target points. With a selection, only selected
 * objects are affected.
 */
export class StretchTool implements Tool {
  readonly id = 'stretch';
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  private stage: 'c1' | 'c2' | 'base' | 'target' = 'c1';
  private c1: { world: Vec2; screen: Vec2 } | null = null;
  private window: Bounds | null = null;
  private targets: Entity[] = [];
  private base: Vec2 | null = null;
  private hover: Vec2 | null = null;
  private hoverScreen: Vec2 | null = null;
  private tracking: Tracking | null = null;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  get snaps(): boolean {
    return this.stage === 'base' || this.stage === 'target';
  }

  activate(): void {
    this.refresh();
  }

  snapFrom(): Vec2 | null {
    return this.stage === 'target' ? this.base : null;
  }

  private refresh(): void {
    const texts = {
      c1: 'Esnet: taşınacak köşeleri içine alan pencerenin ilk köşesini belirtin',
      c2: 'Esnet: pencerenin karşı köşesini belirtin',
      base: `Esnet: ${this.targets.length} nesne için temel noktayı belirtin`,
      target: 'Esnet: hedef noktayı belirtin ya da @dY,dX yazın',
    };
    this.prompt.set(texts[this.stage]);
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.stage === 'c1') {
      this.c1 = { world: p.raw, screen: p.screen };
      this.stage = 'c2';
      return this.refresh();
    }
    if (this.stage === 'c2') return this.closeWindow(p.raw);
    this.point(this.constrain(p));
  }

  pointerUp(p: ToolPointer): void {
    // Dragging the window works like two clicks.
    if (this.stage === 'c2' && this.c1 && Math.hypot(p.screen.x - this.c1.screen.x, p.screen.y - this.c1.screen.y) > 4) this.closeWindow(p.raw);
  }

  pointerMove(p: ToolPointer): void {
    this.hoverScreen = p.screen;
    this.hover = this.stage === 'base' || this.stage === 'target' ? this.constrain(p) : p.raw;
    this.ctx.view.requestOverlay();
  }

  private constrain(p: ToolPointer): Vec2 {
    const r = constrainPoint(this.ctx, this.stage === 'target' ? this.base : null, p);
    this.tracking = r.tracking;
    return r.point;
  }

  private closeWindow(w: Vec2): void {
    const a = this.c1!.world;
    const r = { minX: Math.min(a.x, w.x), minY: Math.min(a.y, w.y), maxX: Math.max(a.x, w.x), maxY: Math.max(a.y, w.y) };
    const { doc, selection } = this.ctx;
    const restrict = selection.size > 0;
    const ids = this.ctx.view.pickRect(r, true).filter((id) => !restrict || selection.has(id));
    const all = ids.map((id) => doc.get(id)).filter((e): e is Entity => !!e && stretchEntity(e, r, 0, 0) !== null);
    this.targets = all.filter((e) => !doc.layers.isLocked(e.layerId));
    if (this.targets.length < all.length) this.ctx.log.warn(`${all.length - this.targets.length} nesne kilitli katmanda olduğu için atlandı.`);
    if (!this.targets.length) {
      this.ctx.log.warn('Pencerede köşesi olan düzenlenebilir nesne yok; yeniden deneyin.');
      this.stage = 'c1';
      this.c1 = null;
      return this.refresh();
    }
    this.window = r;
    this.stage = 'base';
    this.refresh();
  }

  private point(p: Vec2): void {
    if (this.stage === 'base') {
      this.base = p;
      this.stage = 'target';
      return this.refresh();
    }
    if (this.stage !== 'target' || !this.base || !this.window) return;
    const dx = p.x - this.base.x;
    const dy = p.y - this.base.y;
    const { doc, log, format } = this.ctx;
    const patches: (Partial<Entity> & { id: number })[] = [];
    for (const e of this.targets) {
      const g = stretchEntity(e, this.window!, dx, dy);
      if (g) patches.push({ ...(g as Partial<Entity>), id: e.id });
    }
    const n = doc.updateMany(patches, 'Esnet');
    log.success(`${n} nesne esnetildi: ΔY ${format.length(dx, false)}  ΔX ${format.length(dy, false)}`);
    this.ctx.tools.exit();
  }

  acceptPoint(p: Vec2): boolean {
    if (this.stage !== 'base' && this.stage !== 'target') return false;
    this.point(p);
    return true;
  }

  input(text: string): boolean {
    if (this.stage !== 'base' && this.stage !== 'target') return false;
    const pt = pointFromText(this.ctx, text, this.stage === 'target' ? this.base : null, this.hover);
    if (!pt) return false;
    this.point(pt);
    return true;
  }

  confirm(): void {
    this.ctx.tools.exit();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    if (this.stage === 'c2' && this.c1 && this.hoverScreen) {
      const a = this.c1.screen;
      const b = this.hoverScreen;
      g.save();
      g.fillStyle = pal.snap;
      g.globalAlpha = 0.1;
      g.fillRect(Math.min(a.x, b.x), Math.min(a.y, b.y), Math.abs(b.x - a.x), Math.abs(b.y - a.y));
      g.globalAlpha = 1;
      g.strokeStyle = pal.snap;
      g.setLineDash([5, 4]);
      g.strokeRect(Math.min(a.x, b.x) + 0.5, Math.min(a.y, b.y) + 0.5, Math.abs(b.x - a.x), Math.abs(b.y - a.y));
      g.restore();
      return;
    }
    if (!this.window) return;
    const w = this.window;
    strokePath(g, view, [{ x: w.minX, y: w.minY }, { x: w.maxX, y: w.minY }, { x: w.maxX, y: w.maxY }, { x: w.minX, y: w.maxY }], { color: pal.snap, closed: true, dash: [5, 4] });
    const dx = this.base && this.hover ? this.hover.x - this.base.x : 0;
    const dy = this.base && this.hover ? this.hover.y - this.base.y : 0;
    const ids = this.targets.slice(0, MAX_GHOSTS).map((e) => e.id);
    strokePaths(g, view, this.ctx.view.stretchGhosts(ids, w, dx, dy), { color: pal.accent, dash: [4, 3] });
    if (this.base && this.hover) {
      strokePath(g, view, [this.base, this.hover], { color: pal.accent });
      drawTag(g, view.worldToScreen(this.hover), [this.ctx.format.length(dist(this.base, this.hover))], pal.accent, pal.labelHalo);
      if (this.tracking) drawTracking(g, view, this.tracking, this.hover, pal.accent, pal.labelHalo);
    }
  }
}

// ── Yapıştır ───────────────────────────────────────────────────────────

/** Places clipboard entities: their base point follows the cursor until clicked. */
export class PasteTool implements Tool {
  readonly id = 'paste';
  readonly prompt = new Signal('Yapıştır: yerleştirme noktasını belirtin ya da koordinat yazın');
  readonly cursor = 'cross' as const;
  readonly snaps = true;
  private hover: Vec2 | null = null;
  private readonly ctx: AppContext;
  private readonly items: NewEntity[];
  private readonly base: Vec2;
  /** The copies' own geometry store (they are not in the drawing): their ghosts and the pasted geometry come from it. */
  private store: CoreStore | null = null;

  constructor(ctx: AppContext, items: NewEntity[], base: Vec2) {
    this.ctx = ctx;
    this.items = items;
    this.base = base;
  }

  pointerDown(p: ToolPointer): void {
    if (p.button === 0) this.place(p.world);
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    this.ctx.view.requestOverlay();
  }

  input(text: string): boolean {
    const pt = pointFromText(this.ctx, text, this.base, this.hover);
    if (!pt) return false;
    this.place(pt);
    return true;
  }

  confirm(): void {
    this.ctx.tools.exit();
  }

  acceptPoint(p: Vec2): boolean {
    this.place(p);
    return true;
  }

  private place(at: Vec2): void {
    const ids = pasteEntities(this.ctx, this.items, at.x - this.base.x, at.y - this.base.y, this.copies());
    if (ids.length) this.ctx.selection.set(ids);
    this.ctx.tools.exit();
  }

  private copies(): CoreStore {
    return (this.store ??= storeOf(this.items));
  }

  deactivate(): void {
    this.store?.dispose();
    this.store = null;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.hover) return;
    const m = translation(this.hover.x - this.base.x, this.hover.y - this.base.y);
    const pal = this.ctx.view.palette;
    const shown = Math.min(this.items.length, MAX_GHOSTS);
    strokePaths(g, view, this.copies().transformOutlines(numbered(shown), Float64Array.from(m), shown), { color: pal.accent, dash: [4, 3] });
  }
}

/** Ids 1…n: how a store of objects not in the drawing numbers them. */
function numbered(n: number): Float64Array {
  return Float64Array.from({ length: n }, (_, i) => i + 1);
}

/** Objects that are not in the drawing (the clipboard's) in a geometry store of their own, numbered 1…n. */
function storeOf(items: readonly NewEntity[]): CoreStore {
  const store = new CoreStore();
  const p = packEntities(items.map((e, i) => ({ ...e, id: i + 1 })));
  store.putPacked(p.nums, p.strings);
  return store;
}

/**
 * Adds copies of `items` moved by (dx, dy) in one undo step. Entities keep
 * their layer when it exists and is unlocked, otherwise they go to the
 * active layer. `store` holds the items as 1…n (the paste tool's); without
 * it one is made for the call. Returns the new ids.
 */
export function pasteEntities(ctx: AppContext, items: readonly NewEntity[], dx: number, dy: number, store?: CoreStore): number[] {
  const { doc, log } = ctx;
  const active = doc.layers.active.value;
  if (doc.layers.isLocked(active) && items.some((e) => !doc.layers.get(e.layerId) || doc.layers.isLocked(e.layerId))) {
    log.warn(`“${doc.layers.get(active)?.name}” katmanı kilitli; yapıştırılamadı.`);
    return [];
  }
  // One call to the core for all of them: it moves its own copies and only the new geometry comes back,
  // as numbers; the pasted objects share nothing with the clipboard.
  const own = store ?? storeOf(items);
  let moved: NewEntity[];
  try {
    const ids = numbered(items.length);
    moved = transformedFrom(items, ids, 1, own.transformPacked(ids, Float64Array.from(translation(dx, dy))));
  } finally {
    if (!store) own.dispose();
  }
  const ids = doc
    .addMany(
      moved.map((e) => ({ ...e, layerId: doc.layers.get(e.layerId) && !doc.layers.isLocked(e.layerId) ? e.layerId : active }) as NewEntity),
      'Yapıştır',
    )
    .map((e) => e.id);
  log.success(`${ids.length} nesne yapıştırıldı.`);
  return ids;
}
