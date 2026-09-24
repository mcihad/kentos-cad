import { DisposableStore } from '../core/disposable';
import type { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import type { Affine } from '../model/geom/affine';
import type { Edge } from '../model/geom/intersect';
import type { LayerNode, LayerStore } from '../model/layers';
import type { ExtendResult, TrimResult } from '../model/ops/trim';
import { CoreStore } from '../wasm/core';
import { packEntities } from '../wasm/pack';
import { DEFAULT_LABELS, labelRule, readGrips, type GripSet } from './storeRecords';

export type SnapKind = 'endpoint' | 'midpoint' | 'center' | 'node' | 'quadrant' | 'intersection' | 'perpendicular' | 'tangent' | 'nearest';

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
};

/** The core's snap kinds by bit number (crates/shared/geometry-core/src/store/snap.rs). */
const SNAP_BITS: readonly SnapKind[] = ['endpoint', 'midpoint', 'center', 'node', 'quadrant', 'intersection', 'perpendicular', 'tangent', 'nearest'];

type LayerRow = { id: string; visible: boolean; locked: boolean; pickInterior: boolean; label?: ReturnType<typeof labelRule> };

/** Every layer node with its flags resolved through its ancestors, as the store reads them. */
export function layerTable(layers: LayerStore): LayerRow[] {
  const out: LayerRow[] = [];
  const walk = (nodes: readonly LayerNode[]) => {
    for (const n of nodes) {
      const label = n.style.label ? labelRule(n.style.label) : undefined;
      out.push({ id: n.id, visible: layers.isVisible(n.id), locked: layers.isLocked(n.id), pickInterior: n.style.pickInterior !== false, ...(label ? { label } : {}) });
      walk(n.children);
    }
  };
  walk(layers.tree);
  return out;
}

/** The label defaults by kind, as the store reads them. */
const LABEL_DEFAULTS = JSON.stringify(Object.fromEntries(Object.entries(DEFAULT_LABELS).map(([kind, st]) => [kind, labelRule(st)])));

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
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(doc: CadDocument) {
    this.doc = doc;
    this.store.setLabelDefaults(LABEL_DEFAULTS);
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
      }
    }
    if (gone.length) this.store.remove(Float64Array.from(gone));
  }

  /** Brings the store up to date before a query. */
  private sync(): void {
    if (this.reload) {
      this.reload = false;
      this.pending.clear();
      this.store.clear();
      const p = packEntities(this.doc.all());
      this.store.putPacked(p.nums, p.strings);
    } else if (this.pending.size) {
      const list: Entity[] = [];
      for (const id of this.pending) {
        const e = this.doc.get(id);
        if (e) list.push(e);
      }
      this.pending.clear();
      const p = packEntities(list);
      this.store.putPacked(p.nums, p.strings);
    }
    if (this.layersDirty) {
      this.layersDirty = false;
      this.store.setLayers(JSON.stringify(layerTable(this.doc.layers)));
    }
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
  labels(view: Bounds, scale: number, editingId: number | null): Float64Array {
    this.sync();
    return this.store.labels(view.minX, view.minY, view.maxX, view.maxY, scale, editingId);
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
    let mask = 0;
    for (let i = 0; i < SNAP_BITS.length; i++) if (kinds.has(SNAP_BITS[i])) mask |= 1 << i;
    const r = this.store.snap(p.x, p.y, tol, mask, from);
    return r.length ? { kind: SNAP_BITS[r[0]], point: { x: r[1], y: r[2] }, entityId: r[3] } : null;
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

  /** Their geometry values for expressions (style/geometry.ts `measuredAt`). */
  measures(ids: readonly number[]): Float64Array {
    this.sync();
    return this.store.measures(Float64Array.from(ids));
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
}
