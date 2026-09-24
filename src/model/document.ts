import { Emitter } from '../core/emitter';
import { Signal } from '../core/signal';
import { entityBounds, type Entity, type NewEntity } from './entities';
import type { CrsDef } from '../geo/crs';
import { ProjectSettings, type ProjectSettingsData } from './projectSettings';
import { emptyBounds, isEmptyBounds, type Bounds, type Vec2 } from './geometry';
import type { LayerInit, LayerStyle } from './layers';
import { LayerStore } from './layers';
import type { ProjectStyles } from './style';

type Op =
  | { type: 'add'; entity: Entity }
  | { type: 'remove'; entity: Entity }
  | { type: 'update'; before: Entity; after: Entity }
  /** A layer's look (colour, line type, renderer …) is project data: undoable like objects. */
  | { type: 'layerStyle'; layerId: string; before: LayerStyle; after: LayerStyle };

interface Transaction {
  label: string;
  ops: Op[];
}

interface DocumentEvents {
  /** Entities on these layers changed geometry or membership. */
  changed: { layerIds: Set<string> };
  /** Only attributes changed (no geometry rebuild needed). */
  attrs: { ids: number[] };
  /**
   * Which objects an applied change touched (edit, undo, redo, rollback or an
   * external change) and whether a layer's style changed. Cloud sync
   * (app/cloud/sync.ts) diffs exactly these; `external` marks changes that
   * came from the server and must not be sent back.
   */
  touched: { ids: number[]; layerStyles: boolean; external: boolean };
}

/** The project's metadata another editor may have changed (applied by `applyExternal`). */
export interface ExternalMeta {
  name?: string;
  settings?: ProjectSettingsData;
  layers?: readonly LayerInit[];
  activeLayer?: string;
  styles?: ProjectStyles;
}

/** Everything a drawing file holds (see model/snapshot.ts for its versioned form). */
export interface DocumentContent {
  name: string;
  settings: ProjectSettingsData;
  origin: Vec2;
  homeView: Bounds | null;
  layers: readonly LayerInit[];
  activeLayer: string;
  entities: readonly Entity[];
  styles: ProjectStyles;
}

/**
 * The drawing. Coordinates are stored in float64 world units; `origin` is a
 * local anchor near the data so the GPU can work in float32 without jitter
 * (TM coordinates reach 4 500 000 m).
 */
export class CadDocument {
  readonly events = new Emitter<DocumentEvents>();
  readonly name: Signal<string>;
  readonly dirty = new Signal(false);
  readonly canUndo = new Signal(false);
  readonly canRedo = new Signal(false);
  /** Project-scoped settings (CRS, units, plot scale) — saved with the file. */
  readonly settings: ProjectSettings;
  readonly layers: LayerStore;
  /** Symbols and assets that belong to this project (docs/STYLE.md §5), saved with the file. */
  readonly styles = new Signal<ProjectStyles>({ items: [], categories: [] });
  /** Where the view opens (the project's start extent); all objects when unset. */
  homeView: Bounds | null = null;

  private entities = new Map<number, Entity>();
  private anchor: Vec2;
  /**
   * Counts every change to what a saved file holds. A save records the
   * revision it wrote and clears `dirty` only if nothing changed meanwhile
   * (an edit made during a slow save stays unsaved; CLAUDE.md §21.3).
   */
  private edits = 0;
  /** While positive, changes are not edits (another editor's changes arriving: `applyExternal`). */
  private quiet = 0;
  private external = false;
  private nextId = 1;
  private undoStack: Transaction[] = [];
  private redoStack: Transaction[] = [];
  private pending: Transaction | null = null;
  /** Open group (see beginGroup): committed transactions join it instead of the undo stack. */
  private group: Transaction | null = null;

  constructor(opts: { name: string; layers: LayerStore; origin: Vec2; settings?: Partial<ProjectSettingsData> }) {
    this.name = new Signal(opts.name);
    this.layers = opts.layers;
    this.anchor = opts.origin;
    this.settings = new ProjectSettings(opts.settings);
    // Project settings, the name, the project's styles and the layer tree
    // (visibility, locks, names) are part of the file: changing them is an edit.
    this.settings.changed.subscribe(() => this.markEdited());
    this.name.subscribe(() => this.markEdited());
    this.styles.subscribe(() => this.markEdited());
    this.layers.events.on('structure', () => this.markEdited());
    this.layers.events.on('state', () => this.markEdited());
  }

  /** Local anchor near the data: the GPU works in float32 relative to it. */
  get origin(): Vec2 {
    return this.anchor;
  }

  /** The current revision of the file's content (see `markSaved`). */
  get revision(): number {
    return this.edits;
  }

  private markEdited(): void {
    if (this.quiet) return;
    this.edits++;
    this.dirty.set(true);
  }

  /** The drawing has changes a save has not written (cloud sync restoring a device draft). */
  markUnsaved(): void {
    this.markEdited();
  }

  /** A save of `revision` succeeded: the drawing is clean unless it changed since. */
  markSaved(revision: number): void {
    if (revision === this.edits) this.dirty.set(false);
  }

  /**
   * Assigned CRS. Changing it re-labels coordinates; it does not reproject
   * (reprojection is an explicit, undoable geo/transform operation).
   */
  get crs(): Signal<CrsDef> {
    return this.settings.crs;
  }

  get size(): number {
    return this.entities.size;
  }

  get(id: number): Entity | undefined {
    return this.entities.get(id);
  }

  all(): IterableIterator<Entity> {
    return this.entities.values();
  }

  byLayer(layerId: string): Entity[] {
    const out: Entity[] = [];
    for (const e of this.entities.values()) if (e.layerId === layerId) out.push(e);
    return out;
  }

  countByLayer(): Map<string, number> {
    const m = new Map<string, number>();
    for (const e of this.entities.values()) m.set(e.layerId, (m.get(e.layerId) ?? 0) + 1);
    return m;
  }

  bounds(ids?: Iterable<number>): Bounds | null {
    const b = emptyBounds();
    const list = ids ? [...ids].map((id) => this.entities.get(id)).filter((e): e is Entity => !!e) : this.entities.values();
    for (const e of list) {
      const eb = entityBounds(e);
      b.minX = Math.min(b.minX, eb.minX);
      b.minY = Math.min(b.minY, eb.minY);
      b.maxX = Math.max(b.maxX, eb.maxX);
      b.maxY = Math.max(b.maxY, eb.maxY);
    }
    return isEmptyBounds(b) ? null : b;
  }

  // ── Editing (all edits are undoable) ────────────────────────────────

  /**
   * Groups several edits into one undo step, all or nothing: if `fn`
   * throws, what it already changed is reverted (newest first) and nothing
   * is recorded, marked dirty or kept for undo; the error goes on to the
   * caller. A transaction inside a transaction joins the outer one as a
   * savepoint: its failure reverts only its own edits, and the outer one
   * reverts everything if the error reaches it. Entity ids are not reused.
   */
  transact<T>(label: string, fn: () => T): T {
    const outer = this.pending;
    const tx = outer ?? { label, ops: [] };
    const mark = tx.ops.length;
    if (!outer) this.pending = tx;
    let done = false;
    try {
      const result = fn();
      done = true;
      return result;
    } finally {
      if (!done) this.rollback(tx, mark);
      if (!outer) {
        this.pending = null;
        if (done && tx.ops.length) this.commit(tx);
      }
    }
  }

  /** Reverts the ops a transaction applied after `mark`, newest first, and forgets them. */
  private rollback(tx: Transaction, mark: number): void {
    const undone = tx.ops.splice(mark);
    if (undone.length) this.applyAll(undone.reverse().map(invert));
  }

  /**
   * Groups everything committed until `end()` into one undo step, across
   * awaits (a processing model runs several tools, each applying its own
   * transaction). `cancel()` reverts what the group did and records
   * nothing. A group inside a group joins the outer one.
   */
  beginGroup(label: string): { end(): void; cancel(): void } {
    if (this.group) return { end: () => {}, cancel: () => {} };
    const g: Transaction = { label, ops: [] };
    this.group = g;
    const close = () => {
      if (this.group === g) this.group = null;
    };
    return {
      end: () => {
        close();
        if (g.ops.length) this.commit(g);
      },
      cancel: () => {
        close();
        if (g.ops.length) this.applyAll([...g.ops].reverse().map(invert));
      },
    };
  }

  add(init: NewEntity): Entity {
    const entity = { ...init, id: this.nextId++ } as Entity;
    this.record({ type: 'add', entity }, 'Ekle');
    return entity;
  }

  /**
   * Adds many objects as one change (a file import): each is its own
   * undoable op, but listeners hear one event for all of them, not one
   * per object (the layer panel recounts every object on each event).
   */
  addMany(inits: readonly NewEntity[], label = 'Ekle'): Entity[] {
    const entities = inits.map((init) => ({ ...init, id: this.nextId++ }) as Entity);
    if (!entities.length) return entities;
    const ops: Op[] = entities.map((entity) => ({ type: 'add', entity }));
    if (this.pending) {
      // One push per op: a spread of 10⁵ arguments overflows the stack.
      for (const op of ops) this.pending.ops.push(op);
      this.applyAll(ops);
    } else {
      this.applyAll(ops);
      this.commit({ label, ops });
    }
    return entities;
  }

  remove(ids: Iterable<number>): void {
    this.transact('Sil', () => {
      for (const id of ids) {
        const entity = this.entities.get(id);
        if (entity) this.record({ type: 'remove', entity }, 'Sil');
      }
    });
  }

  /** Changes a layer's style as one undoable step (the Layers panel, the layer style window). */
  setLayerStyle(layerId: string, patch: Partial<LayerStyle>, label = 'Katman stili'): void {
    const node = this.layers.get(layerId);
    if (!node) return;
    const before = structuredClone(node.style);
    const after = { ...before, ...structuredClone(patch) };
    for (const k of Object.keys(after) as (keyof LayerStyle)[]) if (after[k] === undefined) delete after[k];
    if (JSON.stringify(before) === JSON.stringify(after)) return;
    this.record({ type: 'layerStyle', layerId, before, after }, label);
  }

  update(id: number, patch: Partial<Entity>): void {
    const before = this.entities.get(id);
    if (!before) return;
    const after = { ...before, ...patch, id } as Entity;
    // Holes belong to polygons only: trimming or breaking one opens it into a polyline.
    if (after.kind !== 'polygon' && 'holes' in after) delete (after as { holes?: unknown }).holes;
    this.record({ type: 'update', before, after }, 'Değiştir');
  }

  /** Bulk load without history (file open, sample data). */
  load(list: NewEntity[]): void {
    for (const init of list) {
      const entity = { ...init, id: this.nextId++ } as Entity;
      this.entities.set(entity.id, entity);
    }
    this.undoStack = [];
    this.redoStack = [];
    this.syncHistory();
    this.events.emit('changed', { layerIds: new Set(list.map((e) => e.layerId)) });
  }

  /**
   * Replaces the whole drawing with one read from a file: objects, layer
   * tree, settings, styles, anchor and start view. No history is kept and
   * the result is clean. Refused while an edit or a group is open.
   */
  replaceWith(data: DocumentContent): void {
    if (this.pending || this.group) throw new Error('Açık bir düzenleme varken çizim değiştirilemez.');
    const touched = new Set([...this.entities.values()].map((e) => e.layerId));
    this.entities = new Map(data.entities.map((e) => [e.id, e]));
    this.nextId = data.entities.reduce((m, e) => Math.max(m, e.id), 0) + 1;
    this.anchor = { ...data.origin };
    this.homeView = data.homeView;
    this.layers.reset(data.layers, data.activeLayer);
    this.settings.assign(data.settings);
    this.name.set(data.name);
    this.styles.set(data.styles);
    this.undoStack = [];
    this.redoStack = [];
    this.syncHistory();
    for (const e of data.entities) touched.add(e.layerId);
    for (const l of this.layers.leaves()) touched.add(l.id);
    this.events.emit('changed', { layerIds: touched });
    this.edits++;
    this.dirty.set(false);
  }

  /** A fresh object id (for objects that arrive from elsewhere, see `applyExternal`). */
  allocateId(): number {
    return this.nextId++;
  }

  /** Whether an edit or a group is open (external changes wait until it closes). */
  get busy(): boolean {
    return !!(this.pending || this.group);
  }

  /**
   * Applies changes that another editor already saved: no undo step, not an
   * unsaved edit. Undo steps touching these objects are dropped, so undo can
   * never silently revert someone else's change (CLAUDE.md §15). Objects are
   * put with their id (`allocateId` for new ones). Refused while an edit is open.
   */
  applyExternal(changes: { put?: readonly Entity[]; remove?: readonly number[]; meta?: ExternalMeta }): void {
    if (this.busy) throw new Error('Açık bir düzenleme varken dışarıdan gelen değişiklik uygulanamaz.');
    const ops: Op[] = [];
    for (const e of changes.put ?? []) {
      const before = this.entities.get(e.id);
      ops.push(before ? { type: 'update', before, after: e } : { type: 'add', entity: e });
      if (e.id >= this.nextId) this.nextId = e.id + 1;
    }
    for (const id of changes.remove ?? []) {
      const entity = this.entities.get(id);
      if (entity) ops.push({ type: 'remove', entity });
    }
    this.quiet++;
    this.external = true;
    try {
      if (ops.length) this.applyAll(ops);
      const m = changes.meta;
      if (m?.layers) this.layers.reset(m.layers, m.activeLayer ?? this.layers.active.value);
      else if (m?.activeLayer) this.layers.reset(this.layers.tree, m.activeLayer);
      if (m?.settings) this.settings.assign(m.settings);
      if (m?.name !== undefined) this.name.set(m.name);
      if (m?.styles) this.styles.set(m.styles);
    } finally {
      this.external = false;
      this.quiet--;
    }
    this.forgetHistoryOf(new Set(ops.map((o) => (o.type === 'update' ? o.after.id : o.type === 'layerStyle' ? -1 : o.entity.id))));
  }

  /** Drops the undo and redo steps that touch any of these objects. */
  forgetHistoryOf(ids: ReadonlySet<number>): void {
    if (!ids.size) return;
    const touches = (tx: Transaction) =>
      tx.ops.some((o) => (o.type === 'update' ? ids.has(o.before.id) : o.type === 'layerStyle' ? false : ids.has(o.entity.id)));
    this.undoStack = this.undoStack.filter((tx) => !touches(tx));
    this.redoStack = this.redoStack.filter((tx) => !touches(tx));
    this.syncHistory();
  }

  undo(): string | null {
    const tx = this.undoStack.pop();
    if (!tx) return null;
    this.applyAll([...tx.ops].reverse().map(invert));
    this.redoStack.push(tx);
    this.syncHistory();
    this.markEdited();
    return tx.label;
  }

  redo(): string | null {
    const tx = this.redoStack.pop();
    if (!tx) return null;
    this.applyAll(tx.ops);
    this.undoStack.push(tx);
    this.syncHistory();
    this.markEdited();
    return tx.label;
  }

  private record(op: Op, label: string): void {
    if (this.pending) {
      this.pending.ops.push(op);
      this.applyAll([op]);
    } else {
      this.applyAll([op]);
      this.commit({ label, ops: [op] });
    }
  }

  private commit(tx: Transaction): void {
    if (this.group && tx !== this.group) {
      for (const op of tx.ops) this.group.ops.push(op);
      return;
    }
    this.undoStack.push(tx);
    if (this.undoStack.length > 200) this.undoStack.shift();
    this.redoStack = [];
    this.markEdited();
    this.syncHistory();
  }

  private applyAll(ops: Op[]): void {
    const layerIds = new Set<string>();
    const attrIds: number[] = [];
    const touched: number[] = [];
    let layerStyles = false;
    for (const op of ops) {
      if (op.type === 'layerStyle') {
        this.layers.replaceStyle(op.layerId, op.after);
        layerStyles = true;
        continue;
      }
      touched.push(op.type === 'update' ? op.after.id : op.entity.id);
      if (op.type === 'add') {
        this.entities.set(op.entity.id, op.entity);
        layerIds.add(op.entity.layerId);
      } else if (op.type === 'remove') {
        this.entities.delete(op.entity.id);
        layerIds.add(op.entity.layerId);
      } else {
        this.entities.set(op.after.id, op.after);
        if (geometryChanged(op.before, op.after)) {
          layerIds.add(op.before.layerId);
          layerIds.add(op.after.layerId);
        } else attrIds.push(op.after.id);
      }
    }
    if (layerIds.size) this.events.emit('changed', { layerIds });
    if (attrIds.length) this.events.emit('attrs', { ids: attrIds });
    if (touched.length || layerStyles) this.events.emit('touched', { ids: touched, layerStyles, external: this.external });
  }

  private syncHistory(): void {
    this.canUndo.set(this.undoStack.length > 0);
    this.canRedo.set(this.redoStack.length > 0);
  }
}

function invert(op: Op): Op {
  if (op.type === 'layerStyle') return { ...op, before: op.after, after: op.before };
  if (op.type === 'add') return { type: 'remove', entity: op.entity };
  if (op.type === 'remove') return { type: 'add', entity: op.entity };
  return { type: 'update', before: op.after, after: op.before };
}

function geometryChanged(a: Entity, b: Entity): boolean {
  if (a.layerId !== b.layerId || a.color !== b.color || a.label !== b.label) return true;
  const { attrs: _a, ...ga } = a;
  const { attrs: _b, ...gb } = b;
  return JSON.stringify(ga) !== JSON.stringify(gb);
}
