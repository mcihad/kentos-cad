import type { FeatureFeed } from '../contracts/generated/FeatureFeed';
import type { MigrationSource } from '../contracts/generated/MigrationSource';
import type { ServiceLayer } from '../contracts/generated/ServiceLayer';
import { Emitter } from '../core/emitter';
import { Signal } from '../core/signal';
import { isUuid, uuidv7 } from '../core/uuid';
import { blockFaultMessage, blockUses, definitionsFault, nameKey, type BlockDefinition } from './blocks';
import { entityBounds, type DrawingEntity, type Entity, type NewEntity } from './entities';
import type { CrsDef } from '../geo/crs';
import { ProjectSettings, type ProjectSettingsData } from './projectSettings';
import { emptyBounds, isEmptyBounds, type Bounds, type Vec2 } from './geometry';
import type { LayerInit, LayerNode, LayerStyle } from './layers';
import { fieldsProblem, type LayerField } from './layerFields';
import { LayerStore } from './layers';
import { followHatches, tiesOf } from './hatchTies';
import { followLinks } from './linkedTexts';
import { sameJson } from './sameJson';
import { canonical, feedProblem, serviceProblem } from './serviceRules';
import type { ProjectStyles } from './style';

type Op =
  | { type: 'add'; entity: DrawingEntity }
  | { type: 'remove'; entity: DrawingEntity }
  | { type: 'update'; before: DrawingEntity; after: DrawingEntity }
  /** A layer's look (colour, line type, renderer …) is project data: undoable like objects. */
  | { type: 'layerStyle'; layerId: string; before: LayerStyle; after: LayerStyle }
  /** A layer's fields (docs/adr/0199 §3): undoable like its look. */
  | { type: 'layerFields'; layerId: string; before: LayerField[]; after: LayerField[] }
  /** A layer's name, map service and source (docs/adr/0208 §2, §10): undoable like its look. */
  | { type: 'layerService'; layerId: string; before: Served; after: Served }
  /**
   * A layer or a group taken out of the tree with everything under it
   * (`removeLayer`), and its inverse: `node` is a copy as it was (children,
   * flags, style), `parent` and `index` its place (null parent: the top).
   */
  | ({ type: 'layerRemove' } & LayerPlace)
  | ({ type: 'layerAdd' } & LayerPlace)
  /**
   * The active layer an edit set (`addLayer` with `activate`): applied only
   * while the active layer is still `before`, so undo and redo leave a layer
   * made active by hand in between as it is.
   */
  | { type: 'layerActive'; before: string; after: string }
  /**
   * A block definition added at `index` of the list (docs/adr/0144), and its
   * inverse, which takes it out; a definition changed in place, by its id.
   */
  | { type: 'blockAdd'; index: number; block: BlockDefinition }
  | { type: 'blockRemove'; index: number; block: BlockDefinition }
  | { type: 'blockUpdate'; before: BlockDefinition; after: BlockDefinition };

/** What a layer is drawn from or took its objects from (docs/adr/0208): none, a service, or a source. */
export interface ServedBy {
  service?: ServiceLayer;
  feed?: FeatureFeed;
}

/** A service layer as its undo step keeps it: its name too. */
type Served = ServedBy & { name: string };

/** A layer tree node as a history step keeps it, and its place: the parent group (null: the top) and the index there. */
interface LayerPlace {
  node: LayerNode;
  parent: string | null;
  index: number;
}

/** Whether an op changes the layer tree rather than an object. */
const isLayerTreeOp = (o: Op): o is Extract<Op, { type: 'layerRemove' | 'layerAdd' }> => o.type === 'layerRemove' || o.type === 'layerAdd';

/** Whether an op is about layers (their tree, style or the active one) rather than an object. */
const isLayerOp = (o: Op): o is Extract<Op, { type: 'layerStyle' | 'layerFields' | 'layerService' | 'layerRemove' | 'layerAdd' | 'layerActive' }> =>
  o.type === 'layerStyle' || o.type === 'layerFields' || o.type === 'layerService' || o.type === 'layerActive' || isLayerTreeOp(o);

/** Whether an op changes the block definitions. */
const isBlockOp = (o: Op): o is Extract<Op, { type: 'blockAdd' | 'blockRemove' | 'blockUpdate' }> =>
  o.type === 'blockAdd' || o.type === 'blockRemove' || o.type === 'blockUpdate';

/** Whether an op adds, removes or changes an object. */
const isObjectOp = (o: Op): o is Extract<Op, { type: 'add' | 'remove' | 'update' }> => o.type === 'add' || o.type === 'remove' || o.type === 'update';

/** The ids of a node and of every node under it. */
function nodeIds(node: LayerNode, out: string[] = []): string[] {
  out.push(node.id);
  for (const c of node.children) nodeIds(c, out);
  return out;
}

/** An edit the document refuses, with the reason for people; nothing was changed. */
export class Refusal extends Error {}

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
   * external change) and whether a layer's style changed. `uids` are their
   * persistent ids in the same order (a removed object can no longer be asked
   * for its own). Cloud sync (app/cloud/sync.ts) diffs exactly these, by
   * persistent id; `external` marks changes that came from the server and
   * must not be sent back.
   */
  touched: { ids: number[]; uids: string[]; layerStyles: boolean; external: boolean };
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
  /** Block definitions (docs/adr/0144); none when absent. */
  blocks?: readonly BlockDefinition[];
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
  /**
   * The block definitions in the drawing's order (docs/adr/0144): undoable
   * state, changed only by `addBlock`, `updateBlock`, `removeBlock` (and
   * undo, redo, a rolled back transaction), each time as a new list.
   */
  readonly blocks = new Signal<readonly BlockDefinition[]>([]);
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
  /**
   * The texts that write an object's label, by that object's persistent id (docs/adr/0175 §4): what a step that
   * changes the object updates (model/linkedTexts.ts).
   */
  private links = new Map<string, Set<number>>();
  /** The hatches whose region follows an object, by that object's persistent id (docs/adr/0186 §6). */
  private ties = new Map<string, Set<number>>();
  /** Moves whenever which objects have linked texts may have changed (`textLabelled`). */
  private linkEdits = 0;
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
  /** While the document itself changes a layer's style or the tree (an op), the layer store's event is not an edit of its own. */
  private applyingLayers = false;
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
    // A layer style or a tree change made by an op is an edit when its step commits, not when applied (ADR 0003).
    this.layers.events.on('structure', () => {
      if (!this.applyingLayers) this.markEdited();
    });
    this.layers.events.on('state', () => {
      if (!this.applyingLayers) this.markEdited();
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

  /** Whether a text writes the label of the object with this persistent id: that text is its label now (docs/adr/0175 §4). */
  hasLinkedText(uid: string): boolean {
    return !!this.links.get(uid)?.size;
  }

  /** The slots of the texts that write the label of the object with this persistent id, in slot order. */
  linkedTexts(uid: string): number[] {
    return [...(this.links.get(uid) ?? [])].sort((a, b) => a - b);
  }

  /**
   * The slots of the objects whose label a text writes, in slot order: the drawing, the sheet and Etiketleri yazıya
   * çevir leave their own labels out (docs/adr/0175 §4).
   */
  textLabelled(): number[] {
    const out: number[] = [];
    for (const uid of this.links.keys()) {
      const slot = this.uids.get(uid);
      if (slot !== undefined) out.push(slot);
    }
    return out.sort((a, b) => a - b);
  }

  /** Moves whenever `textLabelled` may answer otherwise. */
  get linksVersion(): number {
    return this.linkEdits;
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

  /**
   * Gives a layer its fields as one undo step “Alanlar” (docs/adr/0199 §3): the schema `fields` (empty: none), and the
   * keys `renames` moves on the layer's objects (old → new, all at once, so two may swap), their values kept; a deleted
   * field's values stay as attributes without a schema. Refused with nothing changed (`Refusal`, the desktop's
   * `set_layer_fields`): a group, fields with a problem (`fieldsProblem`), renames that would give an object two
   * attributes of one name. An unknown id changes nothing; returns whether anything changed.
   */
  setLayerFields(layerId: string, fields: readonly LayerField[], renames: readonly (readonly [string, string])[] = []): boolean {
    const node = this.layers.get(layerId);
    if (!node) return false;
    if (node.type === 'group') throw new Refusal(`“${node.name}” bir grup; alanlar yalnız katmanın olur.`);
    const problem = fieldsProblem(fields);
    if (problem) throw new Refusal(problem);
    const moved = new Map(renames.filter(([a, b]) => a !== b));
    const patches: (Partial<Entity> & { id: number })[] = [];
    if (moved.size)
      for (const e of this.byLayer(layerId)) {
        const keys = Object.keys(e.attrs).sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
        if (!keys.some((k) => moved.has(k))) continue;
        const next: Record<string, string> = {};
        for (const k of keys) {
          const key = moved.get(k) ?? k;
          if (key in next) throw new Refusal(`Yeniden adlandırma bir nesnede “${key}” adlı iki öznitelik yapıyor; alana başka bir ad verin.`);
          next[key] = e.attrs[k];
        }
        patches.push({ id: e.id, attrs: next });
      }
    const before = structuredClone(node.fields ?? []);
    const after = structuredClone([...fields]);
    if (sameJson(before, after) && !patches.length) return false;
    this.transact('Alanlar', () => {
      if (!sameJson(before, after)) this.record({ type: 'layerFields', layerId, before, after }, 'Alanlar');
      if (patches.length) this.updateMany(patches, 'Alanlar');
    });
    return true;
  }

  /**
   * Gives a layer its map service and its source as one undo step named `label` (docs/adr/0208 §2, §10): `next` is
   * what the layer is drawn from or took its objects from afterwards (an empty one: neither); `name`, a new name in the
   * same step. Refused with nothing changed (`Refusal`, the desktop's `set_layer_service`): a group, a blank name, both
   * at once, a service or a source with a problem (`serviceProblem`, `feedProblem`), a service on a layer that holds
   * objects. That a connection named is the project's is the command's to check (`cad.layers.service`). An unknown id
   * changes nothing; returns whether it changed.
   */
  setLayerService(layerId: string, next: ServedBy, label = 'Servis katmanı', name?: string): boolean {
    const node = this.layers.get(layerId);
    if (!node) return false;
    if (name !== undefined && !name.trim()) throw new Refusal('Katmanın adı boş olamaz; bir ad verin.');
    if (node.type === 'group') throw new Refusal(`“${node.name}” bir grup; servis ve veri kaynağı yalnız katmanın olur.`);
    if (next.service && next.feed) throw new Refusal(`“${node.name}” katmanı hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz.`);
    const problem = (next.service && serviceProblem(next.service)) || (next.feed && feedProblem(next.feed));
    if (problem) throw new Refusal(problem);
    if (next.service && this.layerIndex.get(layerId)?.size)
      throw new Refusal(`“${node.name}” katmanında nesne var; servis katmanı nesne tutmaz. Servisi yeni bir katmana ekleyin.`);
    const before: Served = structuredClone({ name: node.name, ...(node.service && { service: node.service }), ...(node.feed && { feed: node.feed }) });
    const after: Served = structuredClone({ name: name?.trim() ?? node.name, ...(next.service && { service: next.service }), ...(next.feed && { feed: next.feed }) });
    if (sameJson(canonical(before), canonical(after))) return false;
    this.record({ type: 'layerService', layerId, before, after }, label);
    return true;
  }

  /**
   * Adds a layer, or a group, as one undo step “Katman ekle” / “Grup ekle”
   * (into the open transaction or group, if one is, under its name): where
   * `LayerStore.add` puts it (into a group given as `parentId`, else that
   * layer's group, else the top; an unknown parent: the top; at the end),
   * the group it goes into opened as `add` opens it (view state: undo leaves
   * it open). With `activate` the new layer becomes the active one in the
   * same step. Undo takes it away again: the active layer it set goes back to
   * the one before; one made active by hand that goes with it gives way to
   * the first layer of the tree. Refused with a `Refusal`, nothing changed,
   * when an id of it is already in the tree. `index` and `label` place and name it otherwise (`cad.layers.service`,
   * docs/adr/0208 §15). Returns the node in the tree.
   */
  addLayer(init: LayerInit, parentId: string | null = null, opts: { activate?: boolean; index?: number; label?: string } = {}): LayerNode {
    const layers = this.layers;
    const taken = init.id !== undefined && layers.get(init.id) ? init.id : undefined;
    if (taken !== undefined) throw new Refusal(`“${taken}” kimlikli katman zaten var; katman eklenmedi.`);
    const node = layers.make(init);
    const again = nodeIds(node).find((id) => layers.get(id));
    if (again !== undefined) throw new Refusal(`“${again}” kimlikli katman zaten var; katman eklenmedi.`);
    const parent = layers.containerFor(parentId);
    const container = parent === null ? null : layers.get(parent);
    // `index` places it among the group's nodes (first on top); none or past the end: last, drawn under the rest.
    const length = container ? container.children.length : layers.tree.length;
    const index = opts.index === undefined ? length : Math.min(Math.max(0, opts.index), length);
    const label = opts.label ?? (node.type === 'group' ? 'Grup ekle' : 'Katman ekle');
    const before = layers.active.value;
    if (container) container.expanded = true;
    this.transact(label, () => {
      this.record({ type: 'layerAdd', node: structuredClone(node), parent, index }, label);
      if (opts.activate && node.type === 'layer' && before !== node.id) this.record({ type: 'layerActive', before, after: node.id }, label);
    });
    return layers.get(node.id)!;
  }

  /** The ids of the node now in the tree under this id and of every node under it; none when it is not in the tree. */
  private subtree(id: string): string[] {
    const node = this.layers.get(id);
    return node ? nodeIds(node) : [];
  }

  /**
   * Deletes a layer, or a group with everything under it, and the objects on
   * them, as one undo step “Katman sil” (into the open transaction or group,
   * if one is). Undo puts the node back in its place with its children,
   * flags and style, then the objects in their slots with their persistent
   * ids. Refused with a `Refusal` and nothing changed, in this order: the
   * last layer (or a group holding every layer), the active layer (or a
   * group holding it), a locked node (by itself or a group above it), a
   * group with a locked layer under it. Returns how many objects went; an
   * unknown id changes nothing and returns 0.
   */
  removeLayer(id: string): number {
    const layers = this.layers;
    const node = layers.get(id);
    const place = layers.placeOf(id);
    if (!node || !place) return 0;
    const refused = this.layerRemovalRefused(id);
    if (refused) throw new Refusal(refused);
    const kept = new Set(layers.leavesOf(id).map((l) => l.id));
    const gone = [...this.entities.values()].filter((e) => kept.has(e.layerId)).map((e) => e.id);
    this.transact('Katman sil', () => {
      this.remove(gone);
      this.record({ type: 'layerRemove', node: structuredClone(node), parent: place.parent, index: place.index }, 'Katman sil');
    });
    return gone.length;
  }

  /**
   * Why `removeLayer(id)` would refuse, in its words, or null when it would
   * not: the interface asks before it asks its own question.
   */
  layerRemovalRefused(id: string): string | null {
    const layers = this.layers;
    const node = layers.get(id);
    if (!node) return null;
    const group = node.type === 'group';
    const leaves = layers.leavesOf(id);
    const kept = new Set(leaves.map((l) => l.id));
    if (layers.leaves().every((l) => kept.has(l.id)))
      return group
        ? `“${node.name}” grubu çizimin bütün katmanlarını içeriyor; silinemez. Çizimde en az bir katman olmalı.`
        : `“${node.name}” çizimin son katmanı; silinemez. Çizimde en az bir katman olmalı.`;
    const active = layers.active.value;
    if (id === active) return `“${node.name}” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin.`;
    if (kept.has(active)) return `“${node.name}” grubu etkin katmanı (“${layers.get(active)?.name ?? active}”) içeriyor; silinemez. Önce grubun dışındaki bir katmanı etkinleştirin.`;
    if (layers.isLocked(id)) return `“${node.name}” ${group ? 'grubu' : 'katmanı'} kilitli; silinemez. Kilidi Katmanlar panelinden açın.`;
    const locked = leaves.find((l) => layers.isLocked(l.id));
    if (locked) return `“${node.name}” grubu kilitli bir katman (“${locked.name}”) içeriyor; silinemez. Kilidi Katmanlar panelinden açın.`;
    return null;
  }

  // ── Block definitions (docs/adr/0144; the desktop's domain/blocks.rs) ──

  /** The definition with this id. */
  block(id: string): BlockDefinition | undefined {
    return this.blocks.value.find((b) => b.id === id);
  }

  /** The definition with this name, compared as names are (Turkish case folded). */
  blockNamed(name: string): BlockDefinition | undefined {
    const key = nameKey(name);
    return this.blocks.value.find((b) => nameKey(b.name) === key);
  }

  /** How many inserts of the definition the drawing's own objects hold (not those inside other definitions). */
  blockUses(id: string): number {
    return blockUses(this.entities.values(), id);
  }

  /**
   * Adds a definition after the others: one undo step “Blok tanımla” (into
   * the open transaction or group, if one is). Refused with a `Refusal`,
   * nothing changed, when it breaks a block rule (its name or id taken, an
   * insert of an unknown block in it, a cycle, too deep). The drawing keeps
   * its own copy.
   */
  addBlock(block: BlockDefinition): void {
    const next = [...this.blocks.value, block];
    this.refuseBlocks(next);
    this.record({ type: 'blockAdd', index: this.blocks.value.length, block: structuredClone(block) }, 'Blok tanımla');
  }

  /**
   * Replaces the definition with the same id, in its place: one undo step
   * “Blok değiştir”; every insert of it shows the new one. False when no
   * definition has the id or nothing changes; refused as `addBlock` is.
   */
  updateBlock(block: BlockDefinition): boolean {
    const list = this.blocks.value;
    const index = list.findIndex((b) => b.id === block.id);
    if (index < 0 || sameJson(list[index], block)) return false;
    const next = list.map((b, i) => (i === index ? block : b));
    this.refuseBlocks(next);
    this.record({ type: 'blockUpdate', before: list[index], after: structuredClone(block) }, 'Blok değiştir');
    return true;
  }

  /**
   * Removes a definition no insert uses: one undo step “Blok sil”; undo puts
   * it back in its place. False when no definition has the id; refused with
   * a `Refusal` as `blockRemovalRefused` says.
   */
  removeBlock(id: string): boolean {
    const list = this.blocks.value;
    const index = list.findIndex((b) => b.id === id);
    if (index < 0) return false;
    const refused = this.blockRemovalRefused(id);
    if (refused) throw new Refusal(refused);
    this.record({ type: 'blockRemove', index, block: list[index] }, 'Blok sil');
    return true;
  }

  /** Why `removeBlock(id)` would refuse, or null: an insert of the definition in the drawing, or in another definition. */
  blockRemovalRefused(id: string): string | null {
    const block = this.block(id);
    if (!block) return null;
    const placed = this.blockUses(id);
    if (placed > 0) return `“${block.name}” bloğu çizimde ${placed} kez yerleştirilmiş; silinemez. Önce yerleştirmelerini silin.`;
    const holder = this.blocks.value.find((b) => b.id !== id && blockUses(b.entities, id) > 0);
    return holder ? `“${block.name}” bloğu “${holder.name}” bloğunun içinde kullanılıyor; silinemez.` : null;
  }

  /** Throws the refusal a list of definitions earns, in the documents' words. */
  private refuseBlocks(list: readonly BlockDefinition[]): void {
    const fault = definitionsFault(list);
    if (fault) throw new Refusal(blockFaultMessage(fault, (i) => list[i]?.name ?? ''));
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
   * new one (generated data), a v1 file's come derived from the file
   * (`attachV1Identities`), a cloud project's are the server's ids
   * (`readProject`, ADR 0014 slice 3). An id that is not a UUID or is given
   * twice refuses the whole drawing, before anything changes.
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
    this.links.clear();
    this.ties.clear();
    this.linkEdits++;
    for (const e of entities) this.link(e);
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
    this.settings.replace(data.settings);
    this.name.set(data.name);
    this.styles.set(data.styles);
    this.blocks.set(data.blocks ?? []);
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

  /**
   * The slot the next new object takes: every object added after reading it has this slot or a later one, as slots are
   * never reused (a group template's members find the objects a tool wrote by it, docs/adr/0176 §5).
   */
  get nextSlot(): number {
    return this.nextId;
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
   * unsaved edit. Objects are put with their slot (`allocateId` for new
   * ones). A new object takes the persistent id it carries (the server's id,
   * ADR 0014 slice 3) or gets a new one; one already in the drawing keeps its
   * own. Undo and redo steps touching these objects are dropped, by slot and
   * by persistent id, so undo can never silently revert someone else's
   * change (CLAUDE.md §15) nor bring back a second copy of an object that
   * arrived in another slot (docs/adr/0026). A persistent id that is not a
   * UUID, comes twice, belongs to another object of the drawing or would
   * change an object's own refuses the whole change before anything
   * happens. Refused while an edit is open.
   *
   * `blocks`: the drawing's block definitions become these, in this order
   * (docs/adr/0144 §5); a definition that did not change keeps its object.
   * A list that breaks a block rule refuses the whole change. Undo and redo
   * steps that change a definition that differs now, or that place or hold
   * one that is gone, are dropped.
   */
  applyExternal(changes: { put?: readonly Entity[]; remove?: readonly number[]; meta?: ExternalMeta; blocks?: readonly BlockDefinition[] }): void {
    if (this.busy) throw new Error('Açık bir düzenleme varken dışarıdan gelen değişiklik uygulanamaz.');
    const blocks = changes.blocks ? this.externalBlocks(changes.blocks) : null;
    const given = new Set<string>();
    for (const e of changes.put ?? []) {
      if (e.uid === undefined) continue;
      if (!isUuid(e.uid)) throw new Error(`Nesne ${e.id}: kalıcı kimlik “${e.uid}” küçük harfli, tireli bir UUID değil; değişiklik uygulanmadı.`);
      if (given.has(e.uid)) throw new Error(`Kalıcı kimlik ${e.uid} değişiklikte iki kez var; değişiklik uygulanmadı.`);
      given.add(e.uid);
      const before = this.entities.get(e.id);
      if (before && before.uid !== e.uid) throw new Error(`Nesne ${e.id}: kalıcı kimliği ${before.uid}, gelen ${e.uid}; bir nesnenin kimliği değişmez, değişiklik uygulanmadı.`);
      if (!before && this.uids.has(e.uid)) throw new Error(`Nesne ${e.id}: kalıcı kimlik ${e.uid} çizimde başka bir nesnenin; değişiklik uygulanmadı.`);
    }
    const ops: Op[] = [];
    for (const e of changes.put ?? []) {
      const before = this.entities.get(e.id);
      if (before) ops.push({ type: 'update', before, after: (e.uid === before.uid ? e : { ...e, uid: before.uid }) as DrawingEntity });
      else ops.push({ type: 'add', entity: { ...e, uid: e.uid ?? this.newUid() } as DrawingEntity });
      if (e.id >= this.nextId) this.nextId = e.id + 1;
    }
    for (const id of changes.remove ?? []) {
      const entity = this.entities.get(id);
      if (entity) ops.push({ type: 'remove', entity });
    }
    this.quiet++;
    this.external = true;
    try {
      // The definitions first: whoever hears of the objects places their inserts with them.
      if (blocks?.changed.size) this.blocks.set(blocks.list);
      if (ops.length) this.applyAll(ops);
      const m = changes.meta;
      if (m?.layers) this.layers.reset(m.layers, m.activeLayer ?? this.layers.active.value);
      else if (m?.activeLayer) this.layers.reset(this.layers.tree, m.activeLayer);
      // The server's settings are whole: a unit, a second system, a definition, datum choices, survey settings, layer
      // states, styles or topology rules it does not name are none (docs/adr/0165 §2, 0167 §1, 0168, 0169 §3, 0177 §4,
      // 0183 §1, 0202 §1).
      if (m?.settings)
        this.settings.assign({
          ...m.settings,
          drawingUnit: m.settings.drawingUnit ?? 'm',
          secondSrid: m.settings.secondSrid ?? null,
          customCrs: m.settings.customCrs ?? null,
          secondCustomCrs: m.settings.secondCustomCrs ?? null,
          datumTransforms: m.settings.datumTransforms ?? [],
          survey: m.settings.survey ?? null,
          layerStates: m.settings.layerStates ?? [],
          textStyles: m.settings.textStyles ?? [],
          dimensionStyles: m.settings.dimensionStyles ?? [],
          topology: m.settings.topology ?? null,
          annotation: m.settings.annotation ?? null,
          connections: m.settings.connections ?? [],
          networks: m.settings.networks ?? [],
        });
      if (m?.name !== undefined) this.name.set(m.name);
      if (m?.styles) this.styles.set(m.styles);
    } finally {
      this.external = false;
      this.quiet--;
    }
    const slots = new Set<number>();
    const uids = new Set<string>();
    for (const o of ops) {
      if (!isObjectOp(o)) continue;
      const e = o.type === 'update' ? o.after : o.entity;
      slots.add(e.id);
      uids.add(e.uid);
    }
    this.forgetHistoryOf(slots, uids);
    if (blocks?.changed.size) this.forgetBlockHistory(blocks.changed, blocks.gone);
    // Another editor's object on a layer a step added (or took away): undoing that step would take the
    // layer from under it, so the step goes.
    const onLayers = new Set((changes.put ?? []).map((e) => e.layerId));
    if (onLayers.size) {
      // The node as the step recorded it, and as it is now (a layer put into an added group since).
      const holds = (tx: Transaction) => tx.ops.some((o) => isLayerTreeOp(o) && [...nodeIds(o.node), ...this.subtree(o.node.id)].some((id) => onLayers.has(id)));
      this.undoStack = this.undoStack.filter((tx) => !holds(tx));
      this.redoStack = this.redoStack.filter((tx) => !holds(tx));
      this.syncHistory();
    }
    // Another editor's tree: a removed layer's recorded place may no longer fit it, so those steps go.
    if (changes.meta?.layers) {
      const touchesTree = (tx: Transaction) => tx.ops.some(isLayerTreeOp);
      this.undoStack = this.undoStack.filter((tx) => !touchesTree(tx));
      this.redoStack = this.redoStack.filter((tx) => !touchesTree(tx));
      this.syncHistory();
    }
  }

  /**
   * Block definitions from elsewhere, checked whole before anything
   * changes: the list the drawing takes (an unchanged definition keeps its
   * object), the ids whose definition differs now or is gone, and those gone.
   */
  private externalBlocks(list: readonly BlockDefinition[]): { list: BlockDefinition[]; changed: Set<string>; gone: Set<string> } {
    const fault = definitionsFault(list);
    if (fault) throw new Error(`Gelen blok tanımları kurala uymuyor (${blockFaultMessage(fault, (i) => list[i]?.name ?? '')}); değişiklik uygulanmadı.`);
    const before = new Map(this.blocks.value.map((b) => [b.id, b]));
    const changed = new Set<string>();
    const next = list.map((b) => {
      const was = before.get(b.id);
      if (was && sameJson(was, b)) return was;
      changed.add(b.id);
      return structuredClone(b);
    });
    const kept = new Set(next.map((b) => b.id));
    const gone = new Set([...before.keys()].filter((id) => !kept.has(id)));
    for (const id of gone) changed.add(id);
    return { list: next, changed, gone };
  }

  /**
   * Drops the undo and redo steps that change one of the `changed`
   * definitions (someone else's now), or that place or hold one of the
   * `gone` ones: undoing them would revert that change, or put back an
   * insert of a block the drawing no longer has.
   */
  private forgetBlockHistory(changed: ReadonlySet<string>, gone: ReadonlySet<string>): void {
    const places = (e: Entity) => e.kind === 'insert' && gone.has(e.block);
    const holds = (b: BlockDefinition) => gone.size > 0 && b.entities.some(places);
    const touches = (tx: Transaction) =>
      tx.ops.some((o) => {
        if (o.type === 'blockUpdate') return changed.has(o.after.id) || holds(o.before) || holds(o.after);
        if (o.type === 'blockAdd' || o.type === 'blockRemove') return changed.has(o.block.id) || holds(o.block);
        if (o.type === 'update') return places(o.before) || places(o.after);
        return (o.type === 'add' || o.type === 'remove') && places(o.entity);
      });
    this.undoStack = this.undoStack.filter((tx) => !touches(tx));
    this.redoStack = this.redoStack.filter((tx) => !touches(tx));
    this.syncHistory();
  }

  /**
   * Drops the undo and redo steps that touch any of these objects, named by
   * slot or by persistent id (an object deleted here that someone else
   * brought back sits in a new slot under the same id).
   */
  forgetHistoryOf(ids: ReadonlySet<number>, uids: ReadonlySet<string> = new Set()): void {
    if (!ids.size && !uids.size) return;
    const hit = (e: DrawingEntity) => ids.has(e.id) || uids.has(e.uid);
    const touches = (tx: Transaction) => tx.ops.some((o) => (o.type === 'update' ? hit(o.before) : isObjectOp(o) ? hit(o.entity) : false));
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

  /**
   * A finished step: into the open group, or onto the undo history as a new edit, with what keeps its linked texts
   * with their objects (model/linkedTexts.ts, docs/adr/0175 §4).
   */
  private commit(tx: Transaction): void {
    if (this.group && tx !== this.group) {
      for (const op of tx.ops) this.group.ops.push(op);
      return;
    }
    if (this.links.size) {
      const follow = followLinks(tx.ops.filter(isObjectOp), {
        get: (id) => this.entities.get(id),
        byUid: (uid) => this.byUid(uid),
        linkedTo: (uid) => this.links.get(uid) ?? [],
        layerLabel: (layerId) => this.layers.get(layerId)?.style.label,
        font: this.settings.drawingFont.value,
      });
      if (follow.length) {
        for (const op of follow) tx.ops.push(op);
        this.applyAll(follow);
      }
    }
    // After the linked texts: a text a hatch leaves open may have moved with its object (model/hatchTies.ts).
    if (this.ties.size) {
      const follow = followHatches(tx.ops.filter(isObjectOp), {
        get: (id) => this.entities.get(id),
        byUid: (uid) => this.byUid(uid),
        tiedTo: (uid) => this.ties.get(uid) ?? [],
        font: this.settings.drawingFont.value,
        blocks: () => this.blocks.value,
      });
      if (follow.length) {
        for (const op of follow) tx.ops.push(op);
        this.applyAll(follow);
      }
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
    const uids: string[] = [];
    let layerStyles = false;
    let blocks: readonly BlockDefinition[] | null = null;
    for (const op of ops) {
      // Definitions are found by id; the list is replaced, never changed in place (`blocks` hears it once).
      if (isBlockOp(op)) {
        const list: readonly BlockDefinition[] = blocks ?? this.blocks.value;
        if (op.type === 'blockAdd') blocks = [...list.slice(0, op.index), op.block, ...list.slice(op.index)];
        else if (op.type === 'blockRemove') blocks = list.filter((b) => b.id !== op.block.id);
        else blocks = list.map((b) => (b.id === op.after.id ? op.after : b));
        continue;
      }
      if (isLayerOp(op)) {
        this.applyingLayers = true;
        try {
          if (op.type === 'layerStyle') this.layers.replaceStyle(op.layerId, op.after);
          else if (op.type === 'layerFields') this.layers.replaceFields(op.layerId, op.after);
          else if (op.type === 'layerService') this.layers.replaceService(op.layerId, op.after.name, op.after.service, op.after.feed);
          else if (op.type === 'layerActive') {
            if (this.layers.active.value === op.before) this.layers.setActive(op.after);
          }
          // A node still holding objects stays: taking it away would leave them on no layer. (Its step's
          // own objects go before it; another editor's put on it drops the step, see applyExternal.)
          else if (op.type === 'layerRemove') {
            if (!this.subtree(op.node.id).some((id) => this.layerIndex.get(id)?.size)) this.layers.detach(op.node.id);
          } else this.layers.attach(op.node, op.parent, op.index);
        } finally {
          this.applyingLayers = false;
        }
        if (op.type === 'layerStyle') layerStyles = true;
        continue;
      }
      const e = op.type === 'update' ? op.after : op.entity;
      touched.push(e.id);
      uids.push(e.uid);
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
    if (blocks) this.blocks.set(blocks);
    if (layerIds.size) this.events.emit('changed', { layerIds });
    if (attrIds.length) this.events.emit('attrs', { ids: attrIds });
    if (touched.length || layerStyles) this.events.emit('touched', { ids: touched, uids, layerStyles, external: this.external });
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
    if (prev) this.unlink(prev);
    this.link(e);
    if (!prev && this.links.has(e.uid)) this.linkEdits++;
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
    this.unlink(e);
    if (this.links.has(e.uid)) this.linkEdits++;
    this.layerIndex.get(e.layerId)?.delete(id);
  }

  /** Notes a linked text under the object it writes the label of, an associative hatch under its objects. */
  private link(e: DrawingEntity): void {
    for (const of of tiesOf(e)) {
      let hatches = this.ties.get(of);
      if (!hatches) this.ties.set(of, (hatches = new Set()));
      hatches.add(e.id);
    }
    if (e.kind !== 'text' || e.labelOf === undefined) return;
    let texts = this.links.get(e.labelOf);
    if (!texts) {
      this.links.set(e.labelOf, (texts = new Set()));
      this.linkEdits++;
    }
    texts.add(e.id);
  }

  private unlink(e: DrawingEntity): void {
    for (const of of tiesOf(e)) {
      const hatches = this.ties.get(of);
      if (hatches?.delete(e.id) && !hatches.size) this.ties.delete(of);
    }
    if (e.kind !== 'text' || e.labelOf === undefined) return;
    const texts = this.links.get(e.labelOf);
    if (texts?.delete(e.id) && !texts.size) {
      this.links.delete(e.labelOf);
      this.linkEdits++;
    }
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
 * Parts belong to polygons, polylines and points (docs/adr/0143, 0174): an
 * object of another kind than before keeps only the parts the patch gives.
 */
function updateOp(before: DrawingEntity | undefined, patch: Partial<Entity>): Extract<Op, { type: 'update' }> | null {
  if (!before) return null;
  const after = { ...before, ...patch, id: before.id, uid: before.uid } as DrawingEntity;
  if (after.kind !== 'polygon' && after.kind !== 'hatch' && 'holes' in after) delete (after as { holes?: unknown }).holes;
  if (after.kind !== before.kind && !('parts' in patch) && 'parts' in after) delete (after as { parts?: unknown }).parts;
  if (after.kind !== 'polygon' && after.kind !== 'polyline' && after.kind !== 'point' && 'parts' in after) delete (after as { parts?: unknown }).parts;
  return sameJson(before, after) ? null : { type: 'update', before, after };
}

function invert(op: Op): Op {
  if (op.type === 'blockAdd') return { ...op, type: 'blockRemove' };
  if (op.type === 'blockRemove') return { ...op, type: 'blockAdd' };
  if (op.type === 'blockUpdate') return { ...op, before: op.after, after: op.before };
  if (op.type === 'layerStyle') return { ...op, before: op.after, after: op.before };
  if (op.type === 'layerFields') return { ...op, before: op.after, after: op.before };
  if (op.type === 'layerService') return { ...op, before: op.after, after: op.before };
  if (op.type === 'layerActive') return { ...op, before: op.after, after: op.before };
  if (op.type === 'layerRemove') return { ...op, type: 'layerAdd' };
  if (op.type === 'layerAdd') return { ...op, type: 'layerRemove' };
  if (op.type === 'add') return { type: 'remove', entity: op.entity };
  if (op.type === 'remove') return { type: 'add', entity: op.entity };
  return { type: 'update', before: op.after, after: op.before };
}

/** Whether an edit changed more than attributes: everything else compared as JSON would write it. */
function geometryChanged(a: Entity, b: Entity): boolean {
  if (a.layerId !== b.layerId || a.color !== b.color || a.label !== b.label) return true;
  return !sameJson(a, b, 'attrs');
}
