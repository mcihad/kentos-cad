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
import type { LabelTexts, LabelWanted } from '../model/ops/labelText';
import { CoreStore, op, type CoreStyleProgram, type ExprColumnData } from '../wasm/core';
import { packEntities } from '../wasm/pack';
import { DEFAULT_LABELS, labelRule, readGrips, type GripSet } from './storeRecords';

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

type LayerRow = { id: string; visible: boolean; locked: boolean; pickInterior: boolean; label?: ReturnType<typeof labelRule>; snapKinds?: number };

/**
 * A layer's own snapping as the store's kinds (docs/adr/0163 §4): none when off; Uç nokta brings Çeyrek with it, as
 * the settings do. The desktop's is `layer_snap_mask` (crates/native/interaction/src/spatial.rs).
 */
export function layerSnapMask(snap: LayerSnap): number {
  if (snap.off) return 0;
  const kinds = new Set(snap.kinds as SnapKind[]);
  if (kinds.has('endpoint')) kinds.add('quadrant');
  return snapMask(kinds);
}

/** Every layer node with its flags resolved through its ancestors, as the store reads them. */
export function layerTable(layers: LayerStore): LayerRow[] {
  const out: LayerRow[] = [];
  const walk = (nodes: readonly LayerNode[]) => {
    for (const n of nodes) {
      const label = n.style.label ? labelRule(n.style.label) : undefined;
      out.push({
        id: n.id,
        visible: layers.isVisible(n.id),
        locked: layers.isLocked(n.id),
        pickInterior: n.style.pickInterior !== false,
        ...(label ? { label } : {}),
        ...(n.snap ? { snapKinds: layerSnapMask(n.snap) } : {}),
      });
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
  /** The block definitions changed since they were sent (docs/adr/0144). */
  private blocksDirty = true;
  /** Blocks' pieces as the store numbers them, by block id, until the definitions change. */
  private readonly pieceTables = new Map<string, readonly BlockPiece[] | null>();
  /** The document's `linksVersion` the store's text-labelled objects were sent at (-1: not since a reload). */
  private sentLinks = -1;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(doc: CadDocument) {
    this.doc = doc;
    this.store.setLabelDefaults(LABEL_DEFAULTS);
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
      }
    }
    if (gone.length) this.store.remove(Float64Array.from(gone));
  }

  /** Brings the store up to date before a query. */
  private sync(): void {
    // The definitions first: the inserts put next are placed with them.
    if (this.blocksDirty) {
      this.blocksDirty = false;
      this.store.setBlocks(JSON.stringify(this.doc.blocks.value));
    }
    if (this.reload) {
      this.reload = false;
      this.pending.clear();
      this.store.clear();
      this.sentLinks = -1;
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
    // The objects whose label a text writes show none of their own (docs/adr/0175 §4).
    if (this.sentLinks !== this.doc.linksVersion) {
      this.sentLinks = this.doc.linksVersion;
      this.store.setTextLabelled(Float64Array.from(this.doc.textLabelled()));
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

  /**
   * Etiketleri yazıya çevir (docs/adr/0175 §1): the texts `wanted` labels make at 1:`scale`, each object's label
   * placed by the store as the drawing's are, the template filled and the text measured in the drawing's typeface; a
   * text's `item` is its place in `wanted`.
   */
  labelTexts(wanted: readonly LabelWanted[], scale: number, thin: boolean): LabelTexts {
    this.sync();
    return JSON.parse(this.store.labelTexts(JSON.stringify(wanted), scale, thin)) as LabelTexts;
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

  /** A layer through the style engine, next to its geometry (render/styledLayer.ts); `pieces`: every insert's pieces' sets. */
  styled(program: CoreStyleProgram, ids: readonly number[], objects: Int32Array, pieces: Int32Array, table: ExprTable, clip: Bounds | null, origin: Vec2, plotScale: number, screen = false): { json: string; data: Float32Array } {
    this.sync();
    return this.store.buildStyled(program, Float64Array.from(ids), objects, pieces, table, clip, origin, plotScale, screen);
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
