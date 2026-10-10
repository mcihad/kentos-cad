import { DisposableStore } from '../core/disposable';
import type { BlockPiece } from '../model/blocks';
import type { CadDocument } from '../model/document';
import type { Entity, EntityGeometry } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import type { Affine } from '../model/geom/affine';
import type { Edge } from '../model/geom/intersect';
import type { LayerNode, LayerSnap, LayerStore } from '../model/layers';
import { transformedFrom } from '../model/ops/transform';
import type { ExtendResult, TrimResult } from '../model/ops/trim';
import type { ExprTable } from '../model/expression/expression';
import type { LabelTexts } from '../model/ops/labelText';
import type { LayerTime } from '../contracts/generated/LayerTime';
import type { LayerFilter } from '../contracts/generated/LayerFilter';
import { IdMarks } from '../model/idMarks';
import { compileFilter, filterPasses, filterPassesIn, type CompiledFilter } from '../model/layerFilter';
import type { TimeWindow } from '../model/time';
import { CoreStore, op, type CoreStyleProgram, type ExprColumnData, type ExprWorldTable, type StyledOut } from '../wasm/core';
import { packEntities } from '../wasm/pack';
import { LABEL_DEFAULTS_JSON, labelLayers, layerTexts, objectTexts } from '../model/labelTexts';
import { readGrips, type GripSet } from './storeRecords';

export type SnapKind =
  | 'endpoint'
  | 'midpoint'
  | 'center'
  | 'node'
  | 'quadrant'
  | 'intersection'
  | 'perpendicular'
  | 'tangent'
  | 'nearest'
  | 'centroid'
  | 'extension'
  | 'parallel'
  | 'grid';

/** How Çokgenle seç selects (docs/adr/0187 §2), in the store's order: İçindekiler, Kesişenler, Dışındakiler. */
export const POLYGON_MODES = ['inside', 'crossing', 'outside'] as const;
export type PolygonMode = (typeof POLYGON_MODES)[number];

export interface SnapHit {
  kind: SnapKind;
  point: Vec2;
  entityId: number;
}

export const SNAP_LABEL: Record<SnapKind, string> = {
  endpoint: 'Uç nokta',
  midpoint: 'Orta nokta',
  center: 'Merkez',
  node: 'Nokta',
  quadrant: 'Çeyrek',
  intersection: 'Kesişim',
  perpendicular: 'Dik',
  tangent: 'Teğet',
  nearest: 'En yakın',
  centroid: 'Ağırlık merkezi',
  extension: 'Uzantı',
  parallel: 'Paralel',
  grid: 'Karelaj',
};

/** The core's snap kinds by bit number (crates/shared/geometry-core/src/store/snap.rs). */
export const SNAP_BITS: readonly SnapKind[] = ['endpoint', 'midpoint', 'center', 'node', 'quadrant', 'intersection', 'perpendicular', 'tangent', 'nearest', 'centroid', 'extension', 'parallel', 'grid'];

/** Where an acquired end goes on (docs/adr/0163 §1): a straight edge's line beyond the end, or the rest of an arc's circle. */
export type Extension = { kind: 'line'; end: Vec2; dir: Vec2 } | { kind: 'arc'; c: Vec2; r: number; a0: number; sweep: number };

/** What a snap takes besides the drawing (docs/adr/0163 §1–§3). */
export interface SnapExtras {
  extensions?: readonly Extension[];
  /** Acquired directions, unit. */
  parallels?: readonly Vec2[];
  /** The object being drawn: an open path, one bulge per segment. */
  draft?: { pts: readonly Vec2[]; bulges?: readonly number[] } | null;
  /** Karelaj's spacings along x and y, metres. */
  grid?: readonly [number, number] | null;
}

/** The kinds as the core's bits. */
export function snapMask(kinds: ReadonlySet<SnapKind>): number {
  let mask = 0;
  for (let i = 0; i < SNAP_BITS.length; i++) if (kinds.has(SNAP_BITS[i])) mask |= 1 << i;
  return mask;
}

/** Extensions as the core's records. */
export function extensionRecords(list: readonly Extension[]): Float64Array {
  return Float64Array.from(list.flatMap((x) => (x.kind === 'line' ? [0, x.end.x, x.end.y, x.dir.x, x.dir.y] : [1, x.c.x, x.c.y, x.r, x.a0, x.sweep])));
}

/** The core's extension records read back. */
export function readExtensions(f: ArrayLike<number>): Extension[] {
  const out: Extension[] = [];
  for (let i = 0; i < f.length; ) {
    if (f[i] === 0) {
      out.push({ kind: 'line', end: { x: f[i + 1], y: f[i + 2] }, dir: { x: f[i + 3], y: f[i + 4] } });
      i += 5;
    } else {
      out.push({ kind: 'arc', c: { x: f[i + 1], y: f[i + 2] }, r: f[i + 3], a0: f[i + 4], sweep: f[i + 5] });
      i += 6;
    }
  }
  return out;
}

/**
 * How far `p` lies along an acquired extension from its end (a line's distance beyond the end, an arc's length around
 * the rest of the circle) when it lies on the extension within 1 µm; the snap's tag and a typed distance read it
 * (docs/adr/0163 §2).
 */
/** Why a ring cannot select (Çokgenle seç, docs/adr/0187 §2): fewer than three corners, no area, crossing itself; null when it can. */
export const ringProblem = op<(ring: readonly Vec2[]) => string | null>('selectionRingProblem');

export const extensionAlong = op<(x: Extension, p: Vec2) => number | null>('extensionAlong');

/** The point `d` along an acquired extension from its end; null before the end or past an arc's remainder. */
export const extensionAt = op<(x: Extension, d: number) => Vec2 | null>('extensionAt');

type LayerRow = { id: string; visible: boolean; locked: boolean; pickInterior: boolean; snapKinds?: number };

/**
 * A layer's own snapping as the store's kinds (docs/adr/0163 §4): none when off; Uç nokta brings Çeyrek with it, as
 * the settings do. The desktop's is `layer_snap_mask` (crates/native/interaction/src/spatial.rs).
 */
/** A window's labels (docs/adr/0212 §3.8): the records, and the texts their lines and curved labels name by index. */
export interface ShownLabels {
  records: Float64Array;
  texts: string[];
}

/** A label under a point: its object, class, state, and its frame's middle and angle (degrees). */
export type LabelHit = { id: number; cls: number; state: number; at: Vec2; angle: number; w: number; h: number };

/** `labels`' and `labelsShown`'s flags (docs/adr/0212 §4). */
export const LABELS_UNPLACED = 1;
export const LABELS_HIDDEN = 2;
export const LABELS_KEEP = 4;

export function layerSnapMask(snap: LayerSnap): number {
  if (snap.off) return 0;
  const kinds = new Set(snap.kinds as SnapKind[]);
  if (kinds.has('endpoint')) kinds.add('quadrant');
  return snapMask(kinds);
}

/** Every layer node with its flags resolved through its ancestors, as the store reads them. */
/** The objects' ids as the store takes them: a loop (`Float64Array.from` with a mapping is several times slower). */
function idsOf(list: readonly Entity[]): Float64Array {
  const out = new Float64Array(list.length);
  for (let i = 0; i < list.length; i++) out[i] = list[i].id;
  return out;
}

export function layerTable(layers: LayerStore): LayerRow[] {
  const out: LayerRow[] = [];
  const walk = (nodes: readonly LayerNode[]) => {
    for (const n of nodes) {
      out.push({
        id: n.id,
        visible: layers.isVisible(n.id),
        locked: layers.isLocked(n.id),
        pickInterior: n.style.pickInterior !== false,
        ...(n.snap ? { snapKinds: layerSnapMask(n.snap) } : {}),
      });
      walk(n.children);
    }
  };
  walk(layers.tree);
  return out;
}


/**
 * Spatial queries: picking, object snap, window selection, boundaries,
 * what the overlay draws (labels, grips), what tools preview (trim,
 * extend, ghosts) or total, and what the layer builders draw. The
 * Rust geometry store answers them (docs/adr/0008, S1: an R-tree and the
 * rules that were here, in the document's order); this keeps its copy of
 * the objects in step. Removals go at once; puts wait for the next query
 * and go packed (../wasm/pack.ts) in the order they happened, so new
 * objects land in the document's order. A reload (`reset`) sends
 * everything again, in a task of its own right after.
 */
export class PickIndex {
  private readonly store = new CoreStore();
  private readonly doc: CadDocument;
  private readonly d = new DisposableStore();
  /** Objects put since the last query, in the order they were first touched. */
  private readonly pending = new Set<number>();
  private reload = true;
  private layersDirty = true;
  /** The block definitions changed since they were sent (docs/adr/0144). */
  private blocksDirty = true;
  /** Blocks' pieces as the store numbers them, by block id, until the definitions change. */
  private readonly pieceTables = new Map<string, readonly BlockPiece[] | null>();
  /** The document's `linksVersion` the store's text-labelled objects were sent at (-1: not since a reload). */
  private sentLinks = -1;
  private timer: ReturnType<typeof setTimeout> | undefined;
  /** The temporal layers' time settings the objects' times were made with (docs/adr/0210 §6), as JSON by layer id. */
  private sentRules = new Map<string, string>();
  private rules = new Map<string, LayerTime>();
  /** Whether an object has had a time since the store was emptied: a put object off a temporal layer loses its own. */
  private timed = false;
  /**
   * The layers' filters the objects' marks were made with (docs/adr/0211 §3), by layer id: the object last seen and,
   * once it was asked, its JSON (an undo gives back a copy of the same filter); compiled.
   */
  private sentFilters = new Map<string, { filter: LayerFilter; key?: string }>();
  /** The label engine's layers as last sent (docs/adr/0212 §3.1), and each layer's texts' setting the texts were made with. */
  private sentLabelLayers = '';
  private sentTexts = new Map<string, string>();
  private filters = new Map<string, { compiled: CompiledFilter | null; error: string | null }>();
  /** The objects their layer's filter leaves out (the store holds the same marks for its queries). */
  private readonly left = new IdMarks();
  /** Each filtered layer's objects that pass, and all of them (Katmanlar's counts). */
  private readonly counts = new Map<string, { passed: number; total: number }>();
  /** Objects went since the counts were made. */
  private recount = false;

  constructor(doc: CadDocument) {
    this.doc = doc;
    this.store.setLabelDefaults(LABEL_DEFAULTS_JSON);
    // Text boxes (picking, window selection, extents) are measured in the project's drawing typeface.
    this.store.setFont(doc.settings.drawingFont.value);
    this.d.add(doc.settings.drawingFont.subscribe((f) => this.store.setFont(f)));
    this.d.add(doc.events.on('touched', ({ ids }) => this.touch(ids)));
    this.d.add(
      doc.events.on('reset', () => {
        this.reload = true;
        this.pending.clear();
        this.soon();
      }),
    );
    this.d.add(doc.layers.events.on('structure', () => (this.layersDirty = true)));
    this.d.add(doc.layers.events.on('state', () => (this.layersDirty = true)));
    this.d.add(
      doc.blocks.subscribe(() => {
        this.blocksDirty = true;
        this.pieceTables.clear();
      }),
    );
    this.soon();
  }

  dispose(): void {
    clearTimeout(this.timer);
    this.d.dispose();
    this.store.dispose();
  }

  /**
   * Sends a reloaded drawing right after the change instead of at the first
   * pointer move (a large one takes a tenth of a second or more).
   */
  private soon(): void {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.sync(), 0);
  }

  private touch(ids: readonly number[]): void {
    if (this.reload) return; // the next query sends everything anyway
    const gone: number[] = [];
    for (const id of ids) {
      if (this.doc.get(id)) this.pending.add(id);
      else {
        this.pending.delete(id);
        gone.push(id);
        this.left.delete(id);
      }
    }
    if (gone.length) {
      this.store.remove(Float64Array.from(gone));
      if (this.filters.size) this.recount = true;
    }
  }

  /** Brings the store up to date before a query. */
  private sync(): void {
    // The definitions first: the inserts put next are placed with them.
    if (this.blocksDirty) {
      this.blocksDirty = false;
      this.store.setBlocks(JSON.stringify(this.doc.blocks.value));
    }
    let put: Entity[] = [];
    const reloaded = this.reload;
    if (this.reload) {
      this.reload = false;
      this.pending.clear();
      this.store.clear();
      this.sentTexts.clear();
      this.sentLinks = -1;
      this.sentRules.clear();
      this.timed = false;
      this.sentFilters.clear();
      this.filters.clear();
      this.left.clear();
      this.counts.clear();
      const p = packEntities(this.doc.all());
      this.store.putPacked(p.nums, p.strings);
    } else if (this.pending.size) {
      for (const id of this.pending) {
        const e = this.doc.get(id);
        if (e) put.push(e);
      }
      this.pending.clear();
      const p = packEntities(put);
      this.store.putPacked(p.nums, p.strings);
    }
    const layersChanged = this.layersDirty || reloaded;
    if (this.layersDirty) {
      this.layersDirty = false;
      this.store.setLayers(JSON.stringify(layerTable(this.doc.layers)));
    }
    if (layersChanged || put.length) this.syncTimes(put, layersChanged);
    if (layersChanged || put.length || this.recount) this.syncFilters(put, layersChanged);
    if (layersChanged || put.length) this.syncLabels(reloaded ? [...this.doc.all()] : put, layersChanged);
    // The objects whose label a text writes show none of their own (docs/adr/0175 §4).
    if (this.sentLinks !== this.doc.linksVersion) {
      this.sentLinks = this.doc.linksVersion;
      this.store.setTextLabelled(Float64Array.from(this.doc.textLabelled()));
    }
  }

  /**
   * The label engine's inputs (docs/adr/0212 §3.1): the layers' labelling when it changed; the texts of a layer whose
   * texts' setting changed (its classes' texts, conditions and templates; its name, for `$katman`) whole, then those of
   * the objects just put; the objects' pins (a put object lost its own).
   */
  private syncLabels(put: readonly Entity[], layersChanged: boolean): void {
    const layers = this.doc.layers;
    const whole: string[] = [];
    if (layersChanged) {
      const table = labelLayers(layers);
      if (table !== this.sentLabelLayers) {
        this.sentLabelLayers = table;
        this.store.setLabelLayers(table);
      }
      const seen = new Set<string>();
      for (const l of layers.leaves()) {
        if (l.service) continue;
        seen.add(l.id);
        const key = JSON.stringify([l.name, l.style.label ?? null, l.style.labels?.mode ?? null, l.style.labels?.classes ?? null]);
        if (this.sentTexts.get(l.id) !== key) {
          if (this.sentTexts.has(l.id)) whole.push(l.id);
          this.sentTexts.set(l.id, key);
        }
      }
      for (const id of [...this.sentTexts.keys()]) if (!seen.has(id)) this.sentTexts.delete(id);
    }
    const name = (id: string) => layers.get(id)?.name ?? id;
    const send = (list: readonly Entity[], id: string) => {
      if (!list.length) return;
      const t = objectTexts(list, layerTexts(layers.get(id)), name, this.store);
      this.store.setObjectLabels(t.ids, t.from, t.classes, t.texts, t.lens, t.zs);
    };
    for (const id of whole) send(this.doc.byLayer(id), id);
    if (put.length) {
      const done = new Set(whole);
      const byLayer = new Map<string, Entity[]>();
      for (const e of put) {
        if (done.has(e.layerId)) continue;
        const list = byLayer.get(e.layerId);
        if (list) list.push(e);
        else byLayer.set(e.layerId, [e]);
      }
      for (const [id, list] of byLayer) send(list, id);
      const pins: [number, unknown][] = [];
      for (const e of put) if (e.labelPins?.length) pins.push([e.id, e.labelPins]);
      if (pins.length) this.store.setLabelPins(JSON.stringify(pins));
    }
  }

  /**
   * The temporal layers' objects' times (docs/adr/0210 §6): a layer whose time setting changed (or that has one since
   * the store was emptied) whole, then the objects just put that are on a temporal layer, each from its start and end
   * attributes; an object put off a temporal layer loses its time.
   */
  private syncTimes(put: readonly Entity[], layersChanged: boolean): void {
    const whole: string[] = [];
    if (layersChanged) {
      const next = new Map<string, LayerTime>();
      for (const l of this.doc.layers.leaves()) if (l.time && !l.service) next.set(l.id, l.time);
      const keys = new Map([...next].map(([id, t]) => [id, JSON.stringify(t)]));
      for (const [id, key] of keys) if (this.sentRules.get(id) !== key) whole.push(id);
      // A setting taken away: the layer's objects lose their times.
      for (const id of this.sentRules.keys()) if (!next.has(id)) this.untime(this.doc.byLayer(id));
      this.rules = next;
      this.sentRules = keys;
    }
    for (const id of whole) this.time(this.doc.byLayer(id), this.rules.get(id)!);
    if (!put.length) return;
    const done = new Set(whole);
    const byLayer = new Map<string, Entity[]>();
    const off: Entity[] = [];
    for (const e of put) {
      if (done.has(e.layerId)) continue;
      if (this.rules.has(e.layerId)) {
        const list = byLayer.get(e.layerId);
        if (list) list.push(e);
        else byLayer.set(e.layerId, [e]);
      } else if (this.timed) off.push(e);
    }
    for (const [id, list] of byLayer) this.time(list, this.rules.get(id)!);
    this.untime(off);
  }

  /**
   * The layers' filters' marks (docs/adr/0211 §3): a layer whose filter changed (or that has one since the store was
   * emptied) whole, then the objects just put that are on a filtered layer; an object put off one is let in again.
   * The counts follow when anything moved.
   */
  private syncFilters(put: readonly Entity[], layersChanged: boolean): void {
    const whole: string[] = [];
    if (layersChanged) {
      const next = new Map<string, LayerFilter>();
      for (const l of this.doc.layers.leaves()) if (l.filter && !l.service) next.set(l.id, l.filter);
      for (const [id, f] of next) {
        const was = this.sentFilters.get(id);
        if (was?.filter === f) continue;
        if (!was) {
          whole.push(id);
          this.sentFilters.set(id, { filter: f });
          continue;
        }
        const key = JSON.stringify(f);
        if ((was.key ??= JSON.stringify(was.filter)) !== key) whole.push(id);
        this.sentFilters.set(id, { filter: f, key });
      }
      // A filter taken away: the layer's objects come in again.
      for (const id of [...this.sentFilters.keys()])
        if (!next.has(id)) {
          this.letIn(this.doc.byLayer(id));
          this.filters.delete(id);
          this.counts.delete(id);
          this.sentFilters.delete(id);
        }
      for (const id of whole) {
        const r = compileFilter(next.get(id)!);
        this.filters.set(id, r.ok ? { compiled: r.filter, error: null } : { compiled: null, error: r.error });
      }
    }
    for (const id of whole) this.markWhole(id);
    if (put.length) {
      const done = new Set(whole);
      const byLayer = new Map<string, Entity[]>();
      const off: Entity[] = [];
      for (const e of put) {
        if (done.has(e.layerId)) continue;
        if (this.filters.has(e.layerId)) {
          const list = byLayer.get(e.layerId);
          if (list) list.push(e);
          else byLayer.set(e.layerId, [e]);
        } else if (this.left.has(e.id)) off.push(e);
      }
      for (const [id, list] of byLayer) this.mark(list, id);
      this.letIn(off);
    }
    // A whole layer asked counted itself; objects put or gone count the filtered layers again.
    if (put.length || this.recount) {
      this.recount = false;
      for (const id of this.filters.keys()) {
        const list = this.doc.byLayer(id);
        let n = 0;
        for (const e of list) if (!this.left.has(e.id)) n++;
        this.counts.set(id, { passed: n, total: list.length });
      }
    }
  }

  /**
   * A filtered layer's objects marked whole, and counted (docs/adr/0211 §6): a list is turned into the objects' ids
   * once (the document's index of persistent ids) and the condition is asked only of the objects it names; a condition
   * that does not compile lets none through.
   */
  private markWhole(layerId: string): void {
    const f = this.filters.get(layerId);
    if (!f) return;
    const list = this.doc.byLayer(layerId);
    const pass = f.compiled ? filterPassesIn(this.doc, f.compiled, list, (id) => this.doc.layers.get(id)?.name ?? id, this.store) : null;
    const out = new Uint8Array(list.length);
    let n = 0;
    for (let i = 0; i < list.length; i++)
      if (pass?.[i]) {
        this.left.delete(list[i].id);
        n++;
      } else {
        out[i] = 1;
        this.left.add(list[i].id);
      }
    this.store.setFiltered(idsOf(list), out);
    this.counts.set(layerId, { passed: n, total: list.length });
  }

  /** The marks of `list`, objects of filtered layer `layerId` just put, into the store; a condition that does not compile lets none through. */
  private mark(list: readonly Entity[], layerId: string): void {
    const f = this.filters.get(layerId);
    if (!f || !list.length) return;
    const pass = f.compiled ? filterPasses(f.compiled, list, (id) => this.doc.layers.get(id)?.name ?? id, this.store) : list.map(() => false);
    const out = new Uint8Array(list.length);
    for (let i = 0; i < list.length; i++) {
      out[i] = pass[i] ? 0 : 1;
      if (pass[i]) this.left.delete(list[i].id);
      else this.left.add(list[i].id);
    }
    this.store.setFiltered(idsOf(list), out);
  }

  private letIn(list: readonly Entity[]): void {
    if (!list.length) return;
    for (const e of list) this.left.delete(e.id);
    this.store.setFiltered(idsOf(list), new Uint8Array(list.length));
  }

  /** Whether the object passes its layer's filter (always without one, docs/adr/0211 §3). */
  filterShown(id: number): boolean {
    this.sync();
    return !this.left.has(id);
  }

  /** A filtered layer's objects that pass its filter, and all of them; null without a filter. */
  filterCounts(layerId: string): { passed: number; total: number } | null {
    this.sync();
    return this.counts.get(layerId) ?? null;
  }

  /** Why a layer's filter lets nothing through: its condition does not compile. */
  filterError(layerId: string): string | null {
    this.sync();
    return this.filters.get(layerId)?.error ?? null;
  }

  /** For each of `ids`, whether the view shows it: it passes its layer's filter and shows at the slider's window. */
  viewShown(ids: readonly number[]): Uint8Array {
    this.sync();
    return this.store.viewMask(new Float64Array(ids));
  }

  /** The objects' times read from their values straight into the store, in one call: nothing crosses back. */
  private time(list: readonly Entity[], rule: LayerTime): void {
    if (!list.length) return;
    const end = rule.end ?? null;
    const lens = new Int32Array(list.length * 2);
    const ids = new Float64Array(list.length);
    const texts: string[] = [];
    for (let i = 0; i < list.length; i++) {
      const { id, attrs } = list[i];
      const s = attrs[rule.start];
      const e = end === null ? undefined : attrs[end];
      ids[i] = id;
      lens[2 * i] = s === undefined ? -1 : s.length;
      lens[2 * i + 1] = e === undefined ? -1 : e.length;
      if (s !== undefined) texts.push(s);
      if (e !== undefined) texts.push(e);
    }
    this.store.setLayerTimes(ids, end !== null, !!rule.cumulative, texts.join(''), lens);
    this.timed = true;
  }

  private untime(list: readonly Entity[]): void {
    if (!list.length) return;
    const times = new Float64Array(list.length * 3).fill(-1);
    this.store.setTimes(idsOf(list), times);
  }

  /** The time slider's window (docs/adr/0210 §5): queries leave out the objects it does not show; null ends the filter. */
  setTimeWindow(w: TimeWindow | null): void {
    if (!w) this.store.setTimeWindow(0, 0, 0);
    else if (w.kind === 'instant') this.store.setTimeWindow(1, w.a, w.a);
    else this.store.setTimeWindow(2, w.a, w.b);
  }

  /** For each of `ids`, whether it shows at the slider's window (the layer builder's filter). */
  timeShown(ids: readonly number[]): Uint8Array {
    this.sync();
    return this.store.timeMask(Float64Array.from(ids));
  }

  /** How many objects have a time and the extent of their starts and ends (the slider's range). */
  timeSummary(): { count: number; extent: [number, number] | null } {
    this.sync();
    const [count, lo, hi] = this.store.timeSummary();
    return { count, extent: Number.isNaN(lo) ? null : [lo, hi] };
  }

  private entities(ids: Float64Array): Entity[] {
    const out: Entity[] = [];
    for (const id of ids) {
      const e = this.doc.get(id);
      if (e) out.push(e);
    }
    return out;
  }

  /** Ids in the store's order, which must be the document's (picking.test.ts checks it through edits). */
  ids(): number[] {
    this.sync();
    return Array.from(this.store.ids());
  }

  /**
   * What the overlay draws in `view` at `scale` px/m, in the document's
   * order (LABEL_STRIDE numbers per record, ./storeRecords); `editingId` is
   * left out (the inline editor draws it).
   */
  labels(view: Bounds, scale: number, editingId: number | null, flags = 0): ShownLabels {
    this.sync();
    const records = this.store.labels(view.minX, view.minY, view.maxX, view.maxY, scale, editingId, flags);
    return { records, texts: this.store.placedTexts() };
  }

  /**
   * The same as the view shows them under `size` (docs/adr/0205 §5): `LABEL_SHOWN_STRIDE` numbers a record. `flags`
   * (docs/adr/0212 §4): `LABELS_UNPLACED` the labels with no free place too, `LABELS_HIDDEN` the hidden ones,
   * `LABELS_KEEP` the main view's, kept for `labelAt`.
   */
  labelsShown(view: Bounds, scale: number, editingId: number | null, size: 'legible' | 'true' | 'screen', plotScale: number, flags = 0): ShownLabels {
    this.sync();
    const records = this.store.labelsShown(view.minX, view.minY, view.maxX, view.maxY, scale, editingId, size, plotScale, flags);
    return { records, texts: this.store.placedTexts() };
  }

  /** The label under `p` among the main view's last labels, within `tol` px; `all` counts the unplaced and hidden. */
  labelAt(p: Vec2, tol: number, all = false): LabelHit | null {
    return this.store.labelAt(p.x, p.y, tol, all);
  }

  /** The labels whose middle is in the box `a`–`b` among the main view's last labels (Etiketi sabitle's window). */
  labelsIn(a: Vec2, b: Vec2, all = false): LabelHit[] {
    return this.store.labelsIn(a.x, a.y, b.x, b.y, all);
  }

  /** Where an object's labels are pinned from: its anchor (world), null for none. */
  labelAnchor(id: number): Vec2 | null {
    this.sync();
    return this.store.labelAnchor(id);
  }

  /**
   * Etiketleri yazıya çevir (docs/adr/0212 §4): the labels of the objects `ids` as the label engine places their window
   * at 1:`scale` (as the sheet does), written as objects; `every` writes the unplaced ones too. A text's `item` is its
   * object's place in `ids`.
   */
  labelTexts(ids: readonly number[], scale: number, every: boolean): LabelTexts {
    this.sync();
    return JSON.parse(this.store.labelTexts(Float64Array.from(ids), scale, every)) as LabelTexts;
  }

  /** Grips of these objects (unknown ids left out), in the given order. */
  grips(ids: Iterable<number>): GripSet[] {
    this.sync();
    return readGrips(this.store.grips(Float64Array.from(ids)));
  }

  /** Visible entities whose bounds overlap `r`. */
  overlapping(r: Bounds, exceptId?: number): Entity[] {
    this.sync();
    return this.entities(this.store.overlapping(r.minX, r.minY, r.maxX, r.maxY, exceptId));
  }

  /**
   * Picks the most specific entity: points and edges first, then the
   * smallest polygon containing the cursor (building before parcel before
   * block). Layers with `pickInterior: false` are edge-pick only.
   */
  hit(p: Vec2, tol: number): Entity | null {
    this.sync();
    const id = this.store.hit(p.x, p.y, tol);
    return id === undefined ? null : (this.doc.get(id) ?? null);
  }

  /** Every visible object a click at `p` could mean, the most specific first (Sıradakini seç, docs/adr/0187 §1); the first is `hit`'s. */
  hits(p: Vec2, tol: number): Entity[] {
    this.sync();
    return this.entities(this.store.hits(p.x, p.y, tol));
  }

  /** Edge-only pick (targets for trim, extend, offset and fillet): the nearest edge `filter` accepts. */
  hitEdge(p: Vec2, tol: number, filter?: (e: Entity) => boolean): Entity | null {
    this.sync();
    const hits = this.store.hitEdge(p.x, p.y, tol);
    for (let i = 0; i < hits.length; i += 2) {
      const e = this.doc.get(hits[i]);
      if (e && (!filter || filter(e))) return e;
    }
    return null;
  }

  /**
   * Smallest visible closed shape (polygon, circle, closed spline) containing
   * `p`, as a ring — the boundary a hatch fills.
   */
  enclosing(p: Vec2): { entity: Entity; ring: Vec2[] } | null {
    this.sync();
    const r = this.store.enclosing(p.x, p.y);
    const entity = r.length ? this.doc.get(r[0]) : undefined;
    if (!entity) return null;
    const ring: Vec2[] = [];
    for (let i = 1; i + 1 < r.length; i += 2) ring.push({ x: r[i], y: r[i + 1] });
    return { entity, ring };
  }

  /** Edges of every visible entity overlapping `r` — boundaries for trim/extend. */
  edgesIn(r: Bounds, exceptId?: number): Edge[] {
    this.sync();
    const f = this.store.edgesIn(r.minX, r.minY, r.maxX, r.maxY, exceptId);
    const out: Edge[] = [];
    for (let i = 0; i < f.length; ) {
      if (f[i] === 0) {
        out.push({ kind: 'seg', a: { x: f[i + 1], y: f[i + 2] }, b: { x: f[i + 3], y: f[i + 4] } });
        i += 5;
      } else {
        out.push({ kind: 'arc', c: { x: f[i + 1], y: f[i + 2] }, r: f[i + 3], a0: f[i + 4], sweep: f[i + 5] });
        i += 6;
      }
    }
    return out;
  }

  snap(p: Vec2, tol: number, kinds: ReadonlySet<SnapKind>, from: Vec2 | null = null): SnapHit | null {
    this.sync();
    const r = this.store.snap(p.x, p.y, tol, snapMask(kinds), from);
    return r.length ? { kind: SNAP_BITS[r[0]], point: { x: r[1], y: r[2] }, entityId: r[3] } : null;
  }

  /**
   * `snap` with what the drawing does not hold (docs/adr/0163): acquired extensions and parallels, the object being
   * drawn, Karelaj. A hit on none of the drawing's objects has `entityId` −1.
   */
  snapEx(p: Vec2, tol: number, kinds: ReadonlySet<SnapKind>, from: Vec2 | null, extras: SnapExtras): SnapHit | null {
    this.sync();
    const draft = extras.draft;
    const r = this.store.snapEx(
      p.x,
      p.y,
      tol,
      snapMask(kinds),
      from,
      extensionRecords(extras.extensions ?? []),
      Float64Array.from((extras.parallels ?? []).flatMap((u) => [u.x, u.y])),
      Float64Array.from((draft?.pts ?? []).flatMap((q) => [q.x, q.y])),
      Float64Array.from(draft?.bulges ?? []),
      extras.grid?.[0] ?? 0,
      extras.grid?.[1] ?? 0,
    );
    return r.length ? { kind: SNAP_BITS[r[0]], point: { x: r[1], y: r[2] }, entityId: r[3] } : null;
  }

  /** The extensions of object `id`'s edges ending at `at` (Uzantı's acquisition, docs/adr/0163 §2). */
  extensionsAt(id: number, at: Vec2): Extension[] {
    this.sync();
    return readExtensions(this.store.extensionsAt(id, at.x, at.y));
  }

  /** The direction of the straight edge nearest `p` within `tol`, unit (Paralel's acquisition). */
  directionAt(p: Vec2, tol: number): Vec2 | null {
    this.sync();
    const u = this.store.directionAt(p.x, p.y, tol);
    return u.length ? { x: u[0], y: u[1] } : null;
  }

  /**
   * Trim `target` at `at` (`trimEntity`) against the chosen boundaries or,
   * when none are chosen, every visible edge in `view`; the store hands the
   * trim only the edges near the target, which gives the same result.
   */
  trim(target: Entity, at: Vec2, view: Bounds, chosen: ReadonlySet<number> | null): TrimResult {
    this.sync();
    return this.store.trimPreview(JSON.stringify(target), at.x, at.y, target.id, chosen ? Float64Array.from(chosen) : null, view.minX, view.minY, view.maxX, view.maxY) as TrimResult;
  }

  /** Extend the end of `target` nearest `at` (`extendEntity`), boundaries as in `trim`. */
  extend(target: Entity, at: Vec2, view: Bounds, chosen: ReadonlySet<number> | null): ExtendResult {
    this.sync();
    return this.store.extendPreview(JSON.stringify(target), at.x, at.y, target.id, chosen ? Float64Array.from(chosen) : null, view.minX, view.minY, view.maxX, view.maxY) as ExtendResult;
  }

  /**
   * Outlines of objects moved by each affine, for ghosts: at most `limit` +
   * 1 objects, counted as the modify tools counted them (paths as
   * ../tools/preview.ts `strokePaths` draws them).
   */
  ghosts(ids: readonly number[], affines: readonly Affine[], limit: number): Float64Array {
    this.sync();
    return this.store.transformOutlines(Float64Array.from(ids), Float64Array.from(affines.flat()), limit);
  }

  /**
   * Outlines of a block placed as an insert would place it (docs/adr/0144): Blok ekle's ghost, and at the
   * origin with scale 1 the Bloklar panel's picture. Empty for a block the drawing does not define.
   */
  blockOutlines(block: string, p: Vec2, scale = 1, rotation = 0, mirror = false): Float64Array {
    this.sync();
    return this.store.insertOutlines(block, p.x, p.y, scale, rotation, mirror);
  }

  /**
   * A drawing point in the own coordinates of the definition the insert `id` places (docs/adr/0144): the
   * Bloklar panel's new base point shown on that insert. Null when `id` is not an insert of a known block.
   */
  insertLocal(id: number, p: Vec2): Vec2 | null {
    this.sync();
    const at = this.store.insertLocal(id, p.x, p.y);
    return at.length === 2 ? { x: at[0], y: at[1] } : null;
  }

  /**
   * `transformEntities(list, affines)` done by the store on its own copies
   * (move, copy, rotate, scale, mirror, arrays): only the new geometry
   * comes back, packed, so no object crosses as JSON (docs/adr/0008). The
   * list's objects are the drawing's; they keep their other fields.
   */
  transformEntities<E extends Entity>(list: readonly E[], affines: readonly Affine[]): E[] {
    this.sync();
    const ids = Float64Array.from(list, (e) => e.id);
    return transformedFrom(list, ids, affines.length, this.store.transformPacked(ids, Float64Array.from(affines.flat())));
  }

  /** Outlines of objects stretched by a window and (dx, dy) (`stretchEntity`), for ghosts. */
  stretchGhosts(ids: readonly number[], window: Bounds, dx: number, dy: number): Float64Array {
    this.sync();
    return this.store.stretchOutlines(Float64Array.from(ids), window.minX, window.minY, window.maxX, window.maxY, dx, dy);
  }

  /** The box around these objects (all of them without `ids`), as `CadDocument.bounds` gives it; null when empty. */
  extent(ids?: Iterable<number>): Bounds | null {
    this.sync();
    const b = this.store.extent(ids ? Float64Array.from(ids) : null);
    return b.length ? { minX: b[0], minY: b[1], maxX: b[2], maxY: b[3] } : null;
  }

  /** Total length (polygons' perimeters left out) and total area, summed in the given order. */
  measure(ids: Iterable<number>): { length: number; area: number } {
    this.sync();
    const [length, area] = this.store.measure(Float64Array.from(ids));
    return { length, area };
  }

  /** What these objects draw, one record each (style/geometry.ts `DrawnReader`): the layer builders' geometry. */
  drawn(ids: readonly number[], oriented: boolean, clip?: Bounds): Float64Array {
    this.sync();
    return this.store.drawn(Float64Array.from(ids), oriented, clip ?? null);
  }

  /** Their geometry values for expressions (model/expression/expressionLib.ts `measuredAt`). */
  measures(ids: readonly number[]): Float64Array {
    this.sync();
    return this.store.measures(Float64Array.from(ids));
  }

  /** An expression over these objects, their geometry values read from the store's shapes (model/expression/expression.ts `ExprGeometry`). */
  evaluateExpression(source: string, ids: Float64Array, texts: string, textLens: Int32Array, numbers: Float64Array, scale: number, want: number): ExprColumnData {
    this.sync();
    return this.store.evaluateExpression(source, ids, texts, textLens, numbers, scale, want);
  }

  /** The same in a context, with the layers its calls to other objects look at (docs/adr/0214). */
  evaluateExpressionIn(source: string, context: string, ids: Float64Array, texts: string, textLens: Int32Array, numbers: Float64Array, scale: number, want: number, world?: ExprWorldTable): ExprColumnData {
    this.sync();
    return this.store.evaluateExpressionIn(source, context, ids, texts, textLens, numbers, scale, want, world);
  }

  /** A layer through the style engine, next to its geometry (render/styledLayer.ts); `pieces`: every insert's pieces' sets. */
  styled(program: CoreStyleProgram, ids: readonly number[], objects: Int32Array, pieces: Int32Array, table: ExprTable, clip: Bounds | null, origin: Vec2, plotScale: number, screen = false, view = { fills: true, areaEdges: true }, frame: { pxPerM: number; picture: string } | null = null): StyledOut {
    this.sync();
    return this.store.buildStyled(program, Float64Array.from(ids), objects, pieces, table, clip, origin, plotScale, screen, view, frame);
  }

  /**
   * A block's pieces as the store numbers them in `GROUP` records and piece
   * labels (docs/adr/0144): each its kind and fields relative to the base
   * point, its own colour and line weight when it has them. Null for a block
   * the drawing does not define. Kept until the definitions change.
   */
  blockPieces(block: string): readonly BlockPiece[] | null {
    this.sync();
    let table = this.pieceTables.get(block);
    if (table === undefined) {
      const json = this.store.blockPieces(block);
      table = json === undefined ? null : (JSON.parse(json) as BlockPiece[]);
      this.pieceTables.set(block, table);
    }
    return table;
  }

  /**
   * Patlat of an insert (docs/adr/0144 §3): its definition's objects one
   * level open, placed as the insert places them, each with its own fields
   * (the insert's colour and line weight when it has none, the insert's layer
   * when its own is the block's); or why not.
   */
  explodeInsert(e: Entity): { pieces: (EntityGeometry & Record<string, unknown>)[] } | { error: string } {
    this.sync();
    return JSON.parse(this.store.explodeInsert(JSON.stringify(e))) as { pieces: (EntityGeometry & Record<string, unknown>)[] } | { error: string };
  }

  /** An insert's pieces as placed (the drawing's coordinates), or null for any other object. */
  insertPieces(id: number): readonly BlockPiece[] | null {
    this.sync();
    const json = this.store.insertPieces(id);
    return json === undefined ? null : (JSON.parse(json) as BlockPiece[]);
  }

  /** Ids of objects on every layer whose box overlaps `r`, in the document's order (the processing tools' "visible" scope). */
  inBox(r: Bounds): number[] {
    this.sync();
    return Array.from(this.store.inBox(r.minX, r.minY, r.maxX, r.maxY));
  }

  /** Window (fully inside) or crossing (touching) selection. */
  inRect(r: Bounds, crossing: boolean): number[] {
    this.sync();
    return Array.from(this.store.inRect(r.minX, r.minY, r.maxX, r.maxY, crossing));
  }

  /**
   * The visible closed shapes around `p` with their areas, smallest first:
   * parcel, block, district (İçeren alanı seç, docs/adr/0141).
   */
  containing(p: Vec2): { entity: Entity; area: number }[] {
    this.sync();
    const r = this.store.containing(p.x, p.y);
    const out: { entity: Entity; area: number }[] = [];
    for (let i = 0; i + 1 < r.length; i += 2) {
      const entity = this.doc.get(r[i]);
      if (entity) out.push({ entity, area: r[i + 1] });
    }
    return out;
  }

  /**
   * The visible areas with a hole around `p`, each with the hole's part and place, the smallest hole first (Deliği
   * sil, Deliği doldur; docs/adr/0173 §5).
   */
  holesAt(p: Vec2): { entity: Entity; part: number; hole: number }[] {
    this.sync();
    const r = this.store.holesAt(p.x, p.y);
    const out: { entity: Entity; part: number; hole: number }[] = [];
    for (let i = 0; i + 2 < r.length; i += 3) {
      const entity = this.doc.get(r[i]);
      if (entity) out.push({ entity, part: r[i + 1], hole: r[i + 2] });
    }
    return out;
  }

  /** What the fence (an open path) crosses; a point within `tol` counts (Çitle seç). */
  inFence(fence: readonly Vec2[], tol: number): number[] {
    this.sync();
    return Array.from(this.store.inFence(Float64Array.from(fence.flatMap((p) => [p.x, p.y])), tol));
  }

  /** What lies wholly inside the circle, or also what it touches when `crossing` (Daireyle seç). */
  inCircle(c: Vec2, r: number, crossing: boolean): number[] {
    this.sync();
    return Array.from(this.store.inCircle(c.x, c.y, r, crossing));
  }

  /** Çokgenle seç (docs/adr/0187 §2): what lies wholly inside the ring, touches it too, or does not touch it at all. */
  inPolygon(ring: readonly Vec2[], mode: PolygonMode): number[] {
    this.sync();
    return Array.from(this.store.inPolygon(Float64Array.from(ring.flatMap((p) => [p.x, p.y])), POLYGON_MODES.indexOf(mode)));
  }

  /** The visible objects lying far from the rest of the drawing (Kapsam denetimi). */
  extentOutliers(): number[] {
    this.sync();
    return Array.from(this.store.extentOutliers());
  }

  /** Genel bakış (docs/adr/0181 §3): the extent of what the visible layers hold; null for nothing. */
  overviewExtent(): Bounds | null {
    this.sync();
    const e = this.store.overviewExtent();
    return e.length === 4 ? { minX: e[0], minY: e[1], maxX: e[2], maxY: e[3] } : null;
  }

  /** The overview's picture (RGBA, ⌊width·dpr + 0.5⌋ × ⌊height·dpr + 0.5⌋), each layer in `colors` (`#RRGGBB` by id); empty for nothing. */
  overviewPicture(width: number, height: number, dpr: number, colors: Record<string, string>): Uint8Array {
    this.sync();
    return this.store.overviewPicture(width, height, dpr, JSON.stringify(colors));
  }
}
