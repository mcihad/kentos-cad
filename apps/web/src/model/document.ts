import type { MigrationSource } from '../contracts/generated/MigrationSource';
import { Emitter } from '../core/emitter';
import { Signal } from '../core/signal';
import { isUuid, uuidv7 } from '../core/uuid';
import { entityBounds, type DrawingEntity, type Entity, type NewEntity } from './entities';
import type { CrsDef } from '../geo/crs';
import { ProjectSettings, type ProjectSettingsData } from './projectSettings';
import { emptyBounds, isEmptyBounds, type Bounds, type Vec2 } from './geometry';
import type { LayerInit, LayerStyle } from './layers';
import { LayerStore } from './layers';
import { sameJson } from './sameJson';
import type { ProjectStyles } from './style';

type Op =
  | { type: 'add'; entity: DrawingEntity }
  | { type: 'remove'; entity: DrawingEntity }
  | { type: 'update'; before: DrawingEntity; after: DrawingEntity }
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
  /**
   * Objects were bulk-loaded or the whole drawing replaced (`load`,
   * `replaceWith`); no `touched` follows. Copies of the objects (the
   * geometry store, docs/adr/0008) rebuild from scratch.
   */
  reset: undefined;
}

/** The project's metadata another editor may have changed (applied by `applyExternal`). */
export interface ExternalMeta {
  name?: string;
  settings?: ProjectSettingsData;
  layers?: readonly LayerInit[];
  activeLayer?: string;
  styles?: ProjectStyles;
}

/**
 * Everything a drawing file holds (see model/snapshot.ts for its versioned
 * form). Objects come with their slots; those without a persistent id get
 * one when the drawing takes them (`replaceWith`). A drawing from a v1 file
 * or a v2 file brings the project's id and the migration source when it has
 * them (docs/adr/0014, docs/specs/kcad-v2.md §6.8); anything else has none.
 */
export interface DocumentContent {
  name: string;
  settings: ProjectSettingsData;
  origin: Vec2;
  homeView: Bounds | null;
  layers: readonly LayerInit[];
  activeLayer: string;
  entities: readonly Entity[];
  styles: ProjectStyles;
  projectId?: string | null;
  migratedFrom?: MigrationSource | null;
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
  /**
   * The project's persistent id, when it has one: derived from a v1 file or
   * read from a v2 file (docs/adr/0014). Set only with the whole drawing
   * (`replaceWith`); not an edit. A v2 save writes it.
   */
  projectId: string | null = null;
  /** The v1 file this drawing was migrated from; every v2 save keeps it (docs/specs/kcad-v2.md §6.8). */
  migratedFrom: MigrationSource | null = null;

  private entities = new Map<number, DrawingEntity>();
  /**
   * Persistent id → slot (docs/adr/0014), kept up to date by every change,
   * undo, redo and rollback: the boundary (files, the server, Python, AI)
   * names objects by persistent id, the rest of the app by slot.
   */
  private uids = new Map<string, number>();
  /** The persistent id of a new object (UUIDv7; tests give their own maker). */
  private readonly newUid: () => string;
  /**
   * Each layer's objects in document order (`byLayer`), kept up to date by
   * every change so rebuilding a layer does not walk the whole drawing.
   * An object moved to another layer keeps its place in the document, which
   * may lie anywhere among that layer's objects: such a layer is read again
   * in document order the next time it is asked for (`reordered`).
   */
  private layerIndex = new Map<string, Map<number, DrawingEntity>>();
  private reordered = new Set<string>();
  /**
   * Each slot's place in the document (larger is later), kept after the
   * object is removed: an object put back by undo, redo or a rolled-back
   * transaction returns to its place, not to the end (docs/adr/0020). Slots
   * are never given twice, so a slot that comes back is the same object.
   */
  private places = new Map<number, number>();
  private nextPlace = 1;
  /** The place of the object last appended; one put back before it makes the document unsorted. */
  private tailPlace = 0;
  private unsorted = false;
  /** While the document itself sets a layer style (an op), the layer store's event is not an edit of its own. */
  private applyingStyle = false;
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

  constructor(opts: { name: string; layers: LayerStore; origin: Vec2; settings?: Partial<ProjectSettingsData>; newUid?: () => string }) {
    this.name = new Signal(opts.name);
    this.newUid = opts.newUid ?? uuidv7;
    this.layers = opts.layers;
    this.anchor = opts.origin;
    this.settings = new ProjectSettings(opts.settings);
    // Project settings, the name, the project's styles and the layer tree
    // (visibility, locks, names) are part of the file: changing them is an edit.
    this.settings.changed.subscribe(() => this.markEdited());
    this.name.subscribe(() => this.markEdited());
    this.styles.subscribe(() => this.markEdited());
    this.layers.events.on('structure', () => this.markEdited());
    // A layer style set by an op is an edit when its step commits, not when applied (ADR 0003).
    this.layers.events.on('state', () => {
      if (!this.applyingStyle) this.markEdited();
    });
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

  /**
   * The object in a slot. Every object of the drawing has its persistent id
   * (`DrawingEntity`); the reading methods give out the general `Entity`,
   * which the app's code takes everywhere, and `uidOf` / `byUid` name it.
   */
  get(id: number): Entity | undefined {
    return this.entities.get(id);
  }

  /** The persistent id of the object in a slot (docs/adr/0014), if the slot holds one. */
  uidOf(id: number): string | undefined {
    return this.entities.get(id)?.uid;
  }

  /** The object with this persistent id, if it is in the drawing. */
  byUid(uid: string): DrawingEntity | undefined {
    const slot = this.uids.get(uid);
    return slot === undefined ? undefined : this.entities.get(slot);
  }

  /** The slot of the object with this persistent id, if it is in the drawing. */
  slotOf(uid: string): number | undefined {
    return this.uids.get(uid);
  }

  all(): IterableIterator<Entity> {
    return this.entities.values();
  }

  /** The objects on a layer, in document order. */
  byLayer(layerId: string): Entity[] {
    if (this.reordered.size) this.reorder();
    const members = this.layerIndex.get(layerId);
    return members ? [...members.values()] : [];
  }

  /** Object count per layer; layers without objects are left out. */
  countByLayer(): Map<string, number> {
    const m = new Map<string, number>();
    for (const [id, members] of this.layerIndex) if (members.size) m.set(id, members.size);
    return m;
  }

  bounds(ids?: Iterable<number>): Bounds | null {
    const b = emptyBounds();
    const list = ids ? [...ids].map((id) => this.entities.get(id)).filter((e): e is DrawingEntity => !!e) : this.entities.values();
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
    // A group ends once: a second end or cancel does nothing (docs/adr/0020).
    let closed = false;
    const close = () => {
      if (closed) return false;
      closed = true;
      if (this.group === g) this.group = null;
      return true;
    };
    return {
      end: () => {
        if (close() && g.ops.length) this.commit(g);
      },
      cancel: () => {
        if (close() && g.ops.length) this.applyAll([...g.ops].reverse().map(invert));
      },
    };
  }

  /**
   * Adds a new object: a new slot and a new persistent id, whatever `init`
   * carries (a copy spread from another object brings that one's; ADR 0014).
   */
  add(init: NewEntity): DrawingEntity {
    const entity = { ...init, id: this.nextId++, uid: this.newUid() } as DrawingEntity;
    this.record({ type: 'add', entity }, 'Ekle');
    return entity;
  }

  /**
   * Adds many objects as one change (a file import, a copy, a paste): each
   * is its own undoable op, but listeners hear one event for all of them,
   * not one per object (the layer panel, the geometry store and cloud sync
   * each take every event). Inside a transaction they join it. Every one is
   * a new object, with a new slot and persistent id, as with `add`.
   */
  addMany(inits: readonly NewEntity[], label = 'Ekle'): DrawingEntity[] {
    const entities = inits.map((init) => ({ ...init, id: this.nextId++, uid: this.newUid() }) as DrawingEntity);
    this.recordMany(entities.map((entity) => ({ type: 'add', entity })), label);
    return entities;
  }

  /** Removes objects as one change (one event for all of them); unknown ids are skipped. */
  remove(ids: Iterable<number>): void {
    const ops: Op[] = [];
    const seen = new Set<number>();
    for (const id of ids) {
      const entity = this.entities.get(id);
      if (!entity || seen.has(id)) continue;
      seen.add(id);
      ops.push({ type: 'remove', entity });
    }
    this.recordMany(ops, 'Sil');
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

  /** Changes an object; it keeps its slot and persistent id, whatever the patch holds. */
  update(id: number, patch: Partial<Entity>): void {
    const op = updateOp(this.entities.get(id), patch);
    if (op) this.record(op, 'Değiştir');
  }

  /**
   * Replaces an object's content, keeping its slot and persistent id: the
   * same object, changed in shape or even kind (the part of a trimmed line
   * that remains, a circle broken into an arc, a line that takes a vertex
   * and becomes a polyline; ADR 0014). Unlike `update` nothing is merged:
   * a field `init` lacks is gone afterwards. Undoable; an unknown id is skipped.
   */
  replace(id: number, init: NewEntity, label = 'Değiştir'): void {
    const before = this.entities.get(id);
    if (!before) return;
    const after = { ...init, id, uid: before.uid } as DrawingEntity;
    // Nothing changes: not an edit (docs/adr/0020).
    if (!sameJson(before, after)) this.record({ type: 'update', before, after }, label);
  }

  /**
   * Changes many objects as one change (move, stretch, a symbol or a layer
   * for the selection): the same ops as `update` after `update`, but
   * listeners hear one event for all of them. Inside a transaction they
   * join it. Returns how many objects changed; unknown ids are skipped.
   */
  updateMany(patches: readonly (Partial<Entity> & { id: number })[], label = 'Değiştir'): number {
    const ops: Op[] = [];
    // An id given twice changes what its first patch made, as a second `update` would.
    const latest = new Map<number, DrawingEntity>();
    for (const patch of patches) {
      const op = updateOp(latest.get(patch.id) ?? this.entities.get(patch.id), patch);
      if (!op) continue;
      latest.set(op.after.id, op.after);
      ops.push(op);
    }
    this.recordMany(ops, label);
    return ops.length;
  }

  /** Bulk load without history (sample data), as new objects: new slots and persistent ids. */
  load(list: NewEntity[]): void {
    for (const init of list) this.put({ ...init, id: this.nextId++, uid: this.newUid() } as DrawingEntity);
    this.undoStack = [];
    this.redoStack = [];
    this.syncHistory();
    this.events.emit('reset', undefined);
    this.events.emit('changed', { layerIds: new Set(list.map((e) => e.layerId)) });
  }

  /**
   * Replaces the whole drawing with one read from a file: objects, layer
   * tree, settings, styles, anchor and start view. No history is kept and
   * the result is clean. Refused while an edit or a group is open. The
   * drawing takes the objects themselves; one without a persistent id gets a
   * new one (a cloud project's until ADR 0014 slice 3, generated data), a v1
   * file's come derived from the file (`attachV1Identities`). An id that is
   * not a UUID or is given twice refuses the whole drawing, before anything
   * changes.
   */
  replaceWith(data: DocumentContent): void {
    if (this.pending || this.group) throw new Error('Açık bir düzenleme varken çizim değiştirilemez.');
    const given = new Set<string>();
    for (const e of data.entities) {
      if (e.uid === undefined) continue;
      if (!isUuid(e.uid)) throw new Error(`Nesne ${e.id}: kalıcı kimlik “${e.uid}” küçük harfli, tireli bir UUID değil; çizim açılmadı.`);
      if (given.has(e.uid)) throw new Error(`Nesne ${e.id}: kalıcı kimlik ${e.uid} iki nesnede birden var; çizim açılmadı.`);
      given.add(e.uid);
    }
    for (const e of data.entities) e.uid ??= this.newUid();
    const entities = data.entities as readonly DrawingEntity[];
    const touched = new Set([...this.entities.values()].map((e) => e.layerId));
    this.entities = new Map(entities.map((e) => [e.id, e]));
    this.uids = new Map(entities.map((e) => [e.uid, e.id]));
    this.places = new Map(entities.map((e, i) => [e.id, i + 1]));
    this.nextPlace = entities.length + 1;
    this.tailPlace = entities.length;
    this.unsorted = false;
    this.layerIndex.clear();
    this.reordered.clear();
    for (const e of this.entities.values()) this.members(e.layerId).set(e.id, e);
    this.nextId = data.entities.reduce((m, e) => Math.max(m, e.id), 0) + 1;
    this.anchor = { ...data.origin };
    this.homeView = data.homeView;
    this.projectId = data.projectId ?? null;
    this.migratedFrom = data.migratedFrom ?? null;
    this.layers.reset(data.layers, data.activeLayer);
    this.settings.assign(data.settings);
    this.name.set(data.name);
    this.styles.set(data.styles);
    this.undoStack = [];
    this.redoStack = [];
    this.syncHistory();
    for (const e of data.entities) touched.add(e.layerId);
    for (const l of this.layers.leaves()) touched.add(l.id);
    this.events.emit('reset', undefined);
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
   * put with their id (`allocateId` for new ones). One already in the drawing
   * keeps its persistent id; a new one gets a new id (how the server's ids
   * become persistent ids is ADR 0014's slice 3). Refused while an edit is open.
   */
  applyExternal(changes: { put?: readonly Entity[]; remove?: readonly number[]; meta?: ExternalMeta }): void {
    if (this.busy) throw new Error('Açık bir düzenleme varken dışarıdan gelen değişiklik uygulanamaz.');
    const ops: Op[] = [];
    for (const e of changes.put ?? []) {
      const before = this.entities.get(e.id);
      if (before) ops.push({ type: 'update', before, after: (e.uid === before.uid ? e : { ...e, uid: before.uid }) as DrawingEntity });
      else ops.push({ type: 'add', entity: { ...e, uid: this.newUid() } as DrawingEntity });
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

  /**
   * Reverts the last step and returns its name. Nothing happens while an
   * edit or a group is open (a model running): undoing the step before it
   * would lose that step's redo when the group ends (docs/adr/0020).
   */
  undo(): string | null {
    if (this.busy) return null;
    const tx = this.undoStack.pop();
    if (!tx) return null;
    this.applyAll([...tx.ops].reverse().map(invert));
    this.redoStack.push(tx);
    this.syncHistory();
    this.markEdited();
    return tx.label;
  }

  redo(): string | null {
    if (this.busy) return null;
    const tx = this.redoStack.pop();
    if (!tx) return null;
    this.applyAll(tx.ops);
    this.undoStack.push(tx);
    this.syncHistory();
    this.markEdited();
    return tx.label;
  }

  private record(op: Op, label: string): void {
    this.recordMany([op], label);
  }

  /** Applies ops as one change: one event of each kind, one undo step (or into the open transaction). */
  private recordMany(ops: Op[], label: string): void {
    if (!ops.length) return;
    if (this.pending) {
      // One push per op: a spread of 10⁵ arguments overflows the stack.
      for (const op of ops) this.pending.ops.push(op);
      this.applyAll(ops);
    } else {
      this.applyAll(ops);
      this.commit({ label, ops });
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
        this.applyingStyle = true;
        try {
          this.layers.replaceStyle(op.layerId, op.after);
        } finally {
          this.applyingStyle = false;
        }
        layerStyles = true;
        continue;
      }
      touched.push(op.type === 'update' ? op.after.id : op.entity.id);
      if (op.type === 'add') {
        this.put(op.entity);
        layerIds.add(op.entity.layerId);
      } else if (op.type === 'remove') {
        this.drop(op.entity.id);
        layerIds.add(op.entity.layerId);
      } else {
        this.put(op.after);
        if (geometryChanged(op.before, op.after)) {
          layerIds.add(op.before.layerId);
          layerIds.add(op.after.layerId);
        } else attrIds.push(op.after.id);
      }
    }
    if (this.unsorted) this.sortByPlace(layerIds);
    if (layerIds.size) this.events.emit('changed', { layerIds });
    if (attrIds.length) this.events.emit('attrs', { ids: attrIds });
    if (touched.length || layerStyles) this.events.emit('touched', { ids: touched, layerStyles, external: this.external });
  }

  /** Sets an object in the drawing, its persistent id's slot and its layer's index. */
  private put(e: DrawingEntity): void {
    const prev = this.entities.get(e.id);
    if (!prev) {
      let place = this.places.get(e.id);
      if (place === undefined) this.places.set(e.id, (place = this.nextPlace++));
      if (place < this.tailPlace) this.unsorted = true;
      else this.tailPlace = place;
    }
    this.entities.set(e.id, e);
    if (prev && prev.uid !== e.uid) this.uids.delete(prev.uid);
    this.uids.set(e.uid, e.id);
    if (prev && prev.layerId !== e.layerId) {
      this.layerIndex.get(prev.layerId)?.delete(e.id);
      this.reordered.add(e.layerId);
    }
    // A new id goes to the end of the document and so to the end of its layer; a known one keeps its place.
    this.members(e.layerId).set(e.id, e);
  }

  private drop(id: number): void {
    const e = this.entities.get(id);
    if (!e) return;
    this.entities.delete(id);
    this.uids.delete(e.uid);
    this.layerIndex.get(e.layerId)?.delete(id);
  }

  /**
   * Puts the document back in place order after objects returned to their
   * places (undo of a removal, a rolled-back transaction): one sort of the
   * drawing, and the returned objects' layers are read again in that order.
   */
  private sortByPlace(layerIds: ReadonlySet<string>): void {
    const place = (e: DrawingEntity) => this.places.get(e.id) ?? 0;
    this.entities = new Map([...this.entities.values()].sort((a, b) => place(a) - place(b)).map((e) => [e.id, e]));
    for (const id of layerIds) this.reordered.add(id);
    this.unsorted = false;
  }

  private members(layerId: string): Map<number, DrawingEntity> {
    let m = this.layerIndex.get(layerId);
    if (!m) this.layerIndex.set(layerId, (m = new Map()));
    return m;
  }

  /** Reads the layers objects moved into again in document order: one walk of the drawing for all of them. */
  private reorder(): void {
    const fresh = new Map<string, Map<number, DrawingEntity>>();
    for (const id of this.reordered) fresh.set(id, new Map());
    this.reordered.clear();
    for (const e of this.entities.values()) fresh.get(e.layerId)?.set(e.id, e);
    for (const [id, m] of fresh) this.layerIndex.set(id, m);
  }

  private syncHistory(): void {
    this.canUndo.set(this.undoStack.length > 0);
    this.canRedo.set(this.redoStack.length > 0);
  }
}

/**
 * `before` with `patch` over it, keeping its slot and persistent id; null
 * when nothing would change, which is not an edit (docs/adr/0020). Holes
 * belong to polygons and a hatch's islands: a polygon that trimming or
 * breaking opens into a polyline loses them, a hatch that moves keeps them.
 */
function updateOp(before: DrawingEntity | undefined, patch: Partial<Entity>): Extract<Op, { type: 'update' }> | null {
  if (!before) return null;
  const after = { ...before, ...patch, id: before.id, uid: before.uid } as DrawingEntity;
  if (after.kind !== 'polygon' && after.kind !== 'hatch' && 'holes' in after) delete (after as { holes?: unknown }).holes;
  return sameJson(before, after) ? null : { type: 'update', before, after };
}

function invert(op: Op): Op {
  if (op.type === 'layerStyle') return { ...op, before: op.after, after: op.before };
  if (op.type === 'add') return { type: 'remove', entity: op.entity };
  if (op.type === 'remove') return { type: 'add', entity: op.entity };
  return { type: 'update', before: op.after, after: op.before };
}

/** Whether an edit changed more than attributes: everything else compared as JSON would write it. */
function geometryChanged(a: Entity, b: Entity): boolean {
  if (a.layerId !== b.layerId || a.color !== b.color || a.label !== b.label) return true;
  return !sameJson(a, b, 'attrs');
}
