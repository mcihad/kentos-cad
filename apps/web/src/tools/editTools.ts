import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import type { NewObject } from '../contracts/generated/NewObject';
import { Signal } from '../core/signal';
import { ENTITY_KIND_LABEL, type Entity, type EntityGeometry, type NewEntity } from '../model/entities';
import { dist, type Bounds, type Vec2 } from '../model/geometry';
import { translation } from '../model/geom/affine';
import { dimensionLabel } from '../model/geom/dimension';
import { explodeEntity } from '../model/ops/explode';
import { joinEntities } from '../model/ops/join';
import { joinChain } from '../model/ops/trace';
import { stretchEntity } from '../model/ops/stretch';
import { topologyChanges } from '../model/ops/topologyEdit';
import { transformedFrom } from '../model/ops/transform';
import { entitiesCreate } from '../product/entitiesCreate';
import { geometryOf } from '../product/entitiesEdit';
import { entitiesSet } from '../product/entitiesSet';
import type { ViewTransform } from '../viewport/Camera';
import { CoreStore } from '../wasm/core';
import { packEntities } from '../wasm/pack';
import { parseNumber } from './coordinateInput';
import { createdIds, editGeometry, uidOf, writeEdit } from './editCommand';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { neighboursAt, neighboursOf, putRight, sayNeighbours, topologyOn } from './neighbours';
import { drawTag, strokeGeometry, strokePath, strokePaths } from './preview';
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

const whenOn = (on: boolean) => (on ? ': açık' : '');

/** The kinds that have ends to join (`ops::join`'s chains). */
const CHAINED = new Set<Entity['kind']>(['line', 'arc', 'polyline']);

/**
 * Birleştir. Zincir (Z, kept for the session; docs/adr/0161 §2): a click on a line, an arc or an open polyline joins it
 * with the objects joined end to end with it among the visible ones (the core's `joinChain`), up to a free end, a
 * junction, a locked object or the start again, and the tool leaves.
 */
export class JoinTool extends SelectionActionTool {
  readonly id = 'join';
  protected readonly label = 'Birleştir';
  private static tolerance = 0.001;
  private static chain = false;

  protected override pickHint(): string {
    return `[uç boşluğu toleransı ${this.ctx.format.length(JoinTool.tolerance)}; değiştirmek için sayı yazın; Zincir (Z)${whenOn(JoinTool.chain)}]`;
  }

  protected override pickStep(n: number): string {
    return JoinTool.chain ? 'zincirin bir nesnesine tıklayın' : super.pickStep(n);
  }

  override input(text: string): boolean {
    if (this.picking && text.trim().toLocaleUpperCase('tr-TR') === 'Z') {
      JoinTool.chain = !JoinTool.chain;
      this.refresh();
      return true;
    }
    const n = parseNumber(text);
    if (!this.picking || n === null || n < 0) return super.input(text);
    JoinTool.tolerance = n;
    this.refresh();
    return true;
  }

  /** Zincir: the clicked object's chain is joined and the tool leaves; when none can be (said), another may be clicked. */
  protected override picked(id: number): boolean {
    if (!JoinTool.chain) return false;
    if (this.joinChain(id)) queueMicrotask(() => this.ctx.tools.exit());
    return true;
  }

  /** Zincir: the visible lines, arcs and polylines walked from `id` by the core, then joined as a selection is. Whether a chain was joined. */
  private joinChain(id: number): boolean {
    const { doc, log, view } = this.ctx;
    const seed = doc.get(id);
    const refuse = (text: string) => (log.warn(text), false);
    if (!seed || !CHAINED.has(seed.kind)) return refuse('Zincir bir çizgiden, yaydan ya da açık çoklu çizgiden başlar.');
    if (doc.layers.isLocked(seed.layerId)) return refuse('Kilitli katmandaki nesne birleştirilemez.');
    const objects = view.entitiesIn(view.camera.visibleBounds()).filter((e) => CHAINED.has(e.kind));
    let at = objects.findIndex((e) => e.id === id);
    if (at < 0) at = objects.push(seed) - 1;
    const found = joinChain(
      objects.map((e) => ({ shape: e, locked: doc.layers.isLocked(e.layerId) })),
      at,
      Math.max(JoinTool.tolerance, 1e-9),
    );
    if (found.members.length < 2) return refuse('Bu nesneye ucu ucuna bağlanan nesne yok; zincir kurulamadı.');
    // The clicked object keeps its place, its persistent id and its data (docs/adr/0161 §2).
    this.run(
      found.members.map((i) => objects[i]),
      id,
    );
    if (found.locked) log.warn('Zincir kilitli katmandaki bir nesnede durdu.');
    return true;
  }

  /** Joins `targets`; `keep`, when one of a chain, is the object that chain becomes (Zincir's clicked one). */
  protected run(targets: Entity[], keep?: number): void {
    const { log, selection } = this.ctx;
    const { groups } = joinEntities(targets, Math.max(JoinTool.tolerance, 1e-9));
    if (!groups.length) {
      log.warn('Uçları birleşen çizgi, yay ya da açık çoklu çizgi bulunamadı. Toleransı artırmayı deneyin.');
      return;
    }
    // Each chain is its first object, joined (AutoCAD JOIN): it keeps its slot, persistent id
    // (docs/adr/0014), layer, colour, attributes and label; the others are gone. One edit
    // through cad.entities.edit (docs/adr/0047).
    const keeper = (sources: number[]) => (keep !== undefined && sources.includes(keep) ? keep : sources[0]);
    const changes = groups.flatMap((g): EntityEdit[] => [
      { kind: 'replace', uid: uidOf(this.ctx, keeper(g.sources)), geometry: editGeometry(g.geometry), keepData: true },
      ...g.sources.filter((id) => id !== keeper(g.sources)).map((id): EntityEdit => ({ kind: 'remove', uid: uidOf(this.ctx, id) })),
    ]);
    if (!writeEdit(this.ctx, 'join', changes)) return;
    selection.set(groups.map((g) => keeper(g.sources)));
    const kinds = groups.map((g) => ENTITY_KIND_LABEL[g.geometry.kind].toLocaleLowerCase('tr-TR'));
    log.success(`${groups.reduce((n, g) => n + g.sources.length, 0)} nesne birleştirildi: ${kinds.join(', ')}.`);
  }
}

/**
 * A block's object exploded from the insert `from` (docs/adr/0144): an `add` with the object's own layer when the
 * drawing has it as a layer (else the insert's), its colour and line weight (the insert's when it has none: the core
 * gives them), its attributes and label.
 */
function blockPiece(ctx: AppContext, from: string, piece: EntityGeometry & Record<string, unknown>): EntityEdit {
  const layer = typeof piece.layerId === 'string' && ctx.doc.layers.get(piece.layerId)?.type === 'layer' ? piece.layerId : undefined;
  const attrs = piece.attrs && typeof piece.attrs === 'object' ? (piece.attrs as Record<string, string>) : undefined;
  return {
    kind: 'add',
    from,
    geometry: editGeometry(piece),
    ...(layer !== undefined && { layerId: layer }),
    ...(typeof piece.color === 'string' && { color: piece.color }),
    ...(typeof piece.lineWeight === 'number' && { lineWeight: piece.lineWeight }),
    ...(attrs && { attrs: { ...attrs } }),
    ...(typeof piece.label === 'string' && { label: piece.label }),
  };
}

export class ExplodeTool extends SelectionActionTool {
  readonly id = 'explode';
  protected readonly label = 'Patlat';

  protected run(targets: Entity[]): void {
    const { log, selection, format } = this.ctx;
    // Each object exploded goes; its pieces are new objects from it (its layer and colour), all
    // in one edit through cad.entities.edit (docs/adr/0047).
    const changes: EntityEdit[] = [];
    let exploded = 0;
    let firstError: string | null = null;
    for (const e of targets) {
      // A block's insert opens into its definition's objects, one level (docs/adr/0144): each keeps its own layer,
      // colour, line weight and data; what it lacks is the insert's.
      const r =
        e.kind === 'insert'
          ? this.ctx.view.explodeInsert(e)
          : explodeEntity(e, (l) => dimensionLabel(undefined, l, { length: (m) => format.length(m, false), angle: (a) => format.angle(a), percent: (v) => format.percent(v) }), this.ctx.doc.settings.drawingFont.value);
      if ('error' in r) {
        firstError ??= r.error;
        continue;
      }
      const uid = uidOf(this.ctx, e);
      exploded++;
      changes.push(
        { kind: 'remove', uid },
        ...r.pieces.map((piece): EntityEdit => (e.kind === 'insert' ? blockPiece(this.ctx, uid, piece as EntityGeometry & Record<string, unknown>) : { kind: 'add', from: uid, geometry: editGeometry(piece) })),
      );
    }
    if (firstError && !exploded) return log.warn(firstError);
    const out = writeEdit(this.ctx, 'explode', changes);
    if (!out) return;
    const created = createdIds(this.ctx, out);
    selection.set(created);
    log.success(`${exploded} nesne patlatıldı: ${created.length} parça.${firstError ? ` Bazı nesneler atlandı: ${firstError}` : ''}`);
  }
}

// ── Esnet ──────────────────────────────────────────────────────────────

/**
 * Stretch: a crossing window picks the vertices to move (click two corners
 * or drag), then base and target points. With a selection, only selected
 * objects are affected. The core stretches each object; the tool writes
 * them through the product command `cad.entities.edit` (docs/adr/0047).
 * With Topolojik düzenleme on, the objects sharing a moved vertex go with
 * them, selected or not (docs/adr/0160 §3).
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
  /** The vertices the window moves and the objects sharing them (Topolojik düzenleme), found once per window for the preview. */
  private follow: { at: Vec2[]; found: Entity[] } | null = null;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  get snaps(): boolean {
    return this.stage === 'base' || this.stage === 'target';
  }

  /**
   * The neighbours that follow the window's vertices (docs/adr/0160 §3): the vertices the stretch moves, found as the
   * core's moves of a stretch one metre east, and the objects sharing them that are not stretched themselves. Null
   * while the mode is off; found when first asked, since the mode can be turned on while the tool waits.
   */
  private followers(): { at: Vec2[]; found: Entity[] } | null {
    if (!topologyOn(this.ctx) || !this.window) return null;
    if (!this.follow) {
      const points = this.ctx.settings.topologyPoints.value;
      const at: Vec2[] = [];
      for (const e of this.targets) {
        const g = e.kind === 'point' && !points ? null : stretchEntity(e, this.window, 1, 0);
        if (g) for (const c of topologyChanges(e, { ...e, ...g } as Entity)) if (c.kind === 'move') at.push(c.at);
      }
      this.follow = { at, found: neighboursAt(this.ctx, at, new Set(this.targets.map((e) => e.id))) };
    }
    return this.follow;
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
    this.follow = null;
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
    const { log, format } = this.ctx;
    // Each object's new geometry from the core, written through cad.entities.edit as one
    // undo step, “Esnet” (docs/adr/0047): slot, persistent id and every other field kept.
    const changes: EntityEdit[] = [];
    const edits: [Entity, Entity][] = [];
    for (const e of this.targets) {
      const g = stretchEntity(e, this.window!, dx, dy);
      if (!g) continue;
      changes.push({ kind: 'update', uid: uidOf(this.ctx, e), geometry: editGeometry(g) });
      edits.push([e, { ...e, ...g } as Entity]);
    }
    // The neighbours sharing a moved vertex go in the same step, found again on the drawing as it is now.
    const follow = neighboursOf(this.ctx, edits);
    const out = changes.length ? writeEdit(this.ctx, 'stretch', [...changes, ...(follow?.changes ?? [])]) : { changed: [] };
    if (out) {
      // The stretched objects are counted; the neighbours are said after them.
      const stretched = new Set(changes.map((c) => (c.kind === 'update' ? c.uid : '')));
      log.success(`${out.changed.filter((uid) => stretched.has(uid)).length} nesne esnetildi: ΔY ${format.length(dx, false)}  ΔX ${format.length(dy, false)}`);
      if (changes.length) sayNeighbours(this.ctx, follow);
    }
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
    // The neighbours that follow (docs/adr/0160 §5), as the stretched objects.
    const follow = (dx || dy) && this.followers();
    if (follow && follow.found.length) {
      const moves = follow.at.map((at) => ({ kind: 'move' as const, at, to: { x: at.x + dx, y: at.y + dy } }));
      for (const shape of putRight(this.ctx, follow.found, moves).shapes) strokeGeometry(g, view, shape, { color: pal.accent, dash: [4, 3] });
    }
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

/** A paste the product commands refused: its message, said once everything written was taken back. */
class PasteRefused extends Error {}

/**
 * Adds copies of `items` moved by (dx, dy) in one undo step, “Yapıştır”.
 * Entities keep their layer when it exists (a layer, not a group) and is
 * unlocked, otherwise they go to the active layer. They are written through
 * the product commands (TODOS.md CMD-07): `cad.entities.create` for each run
 * of objects going to the same layer, in the items' order (so their slots
 * are too), and `cad.entities.set` for their symbols, which a new object of
 * the create command does not carry. A refusal takes back all of it and is
 * said; a hidden layer is said once. `store` holds the items as 1…n (the
 * paste tool's); without it one is made for the call. Returns the new ids,
 * in the items' order.
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
  const layerOf = (e: NewEntity) => (doc.layers.get(e.layerId)?.type === 'layer' && !doc.layers.isLocked(e.layerId) ? e.layerId : active);
  const objectOf = (e: NewEntity): NewObject => ({
    geometry: geometryOf(e as unknown as EditGeometry) as unknown as EditGeometry,
    ...(e.color !== undefined && { color: e.color }),
    ...(e.lineWeight !== undefined && { lineWeight: e.lineWeight }),
    attrs: { ...e.attrs },
    ...(e.label !== undefined && { label: e.label }),
  });
  const hidden = new Set<string>();
  let ids: number[];
  try {
    ids = doc.transact('Yapıştır', () => {
      const out: number[] = [];
      for (let i = 0; i < moved.length; ) {
        const layerId = layerOf(moved[i]);
        let j = i + 1;
        while (j < moved.length && layerOf(moved[j]) === layerId) j++;
        const r = entitiesCreate.execute({ doc }, { layerId, objects: moved.slice(i, j).map(objectOf) });
        if (r.status !== 'completed') throw new PasteRefused('error' in r ? r.error.message : 'Yapıştırılamadı.');
        if (r.warnings.some((w) => w.code === 'layer_hidden')) hidden.add(layerId);
        out.push(...r.output.ids);
        i = j;
      }
      const bySymbol = new Map<string, string[]>();
      moved.forEach((e, k) => {
        const uid = e.symbol ? doc.uidOf(out[k]) : undefined;
        if (e.symbol && uid) bySymbol.set(e.symbol, [...(bySymbol.get(e.symbol) ?? []), uid]);
      });
      for (const [symbol, uids] of bySymbol) {
        const r = entitiesSet.execute({ doc }, { uids, symbol, operation: 'symbol' });
        if (r.status !== 'completed') throw new PasteRefused('error' in r ? r.error.message : 'Yapıştırılamadı.');
      }
      return out;
    });
  } catch (e) {
    if (!(e instanceof PasteRefused)) throw e;
    log.warn(e.message);
    return [];
  }
  for (const id of hidden) log.warn(`“${doc.layers.get(id)?.name ?? id}” katmanı gizli; yapıştırılan nesneler görünmeyecek.`);
  log.success(`${ids.length} nesne yapıştırıldı.`);
  return ids;
}
