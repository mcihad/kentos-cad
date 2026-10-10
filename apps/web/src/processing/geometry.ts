import type { Entity } from '../model/entities';
import type { ExprGeometry } from '../model/expression/expression';
import type { Bounds, Vec2 } from '../model/geometry';
import { CoreStore, cornerTexts, type ExprColumnData, type ExprWorldTable } from '../wasm/core';
import { packEntities } from '../wasm/pack';

/**
 * The geometry processing tools ask for, from the Rust geometry store
 * (docs/adr/0008, S4). A run packs the objects it reads into a store of its
 * own, in the page and in the processing worker alike, so both run the same
 * code, and tools ask it by id: expressions evaluated with the objects'
 * geometry values (`$alan`, `$merkez_y`, `$genişlik` …; docs/adr/0100), corner
 * numbering, the texts beside numbered corners, edge-length labels. The
 * dialog asks the drawing's store the host keeps (the viewport's). This only
 * packs and reads: no coordinate is computed here.
 */

/**
 * What the drawing's own geometry store answers for processing (the "visible"
 * scope, the dialog's previews); as an ExprGeometry it evaluates an expression
 * over objects by id, reading their geometry values from its shapes.
 */
export interface DocumentGeometry extends ExprGeometry {
  /** Ids of objects on every layer whose box overlaps `r`, in the document's order. */
  inBox(r: Bounds): readonly number[];
  /** Geometry values of these objects for expressions, one record each (`measuredAt`). */
  measures(ids: readonly number[]): Float64Array;
}

/** How corners are walked and merged (numbering.ts): the core's part of the numbering options. */
export interface CornerWalk {
  dir: 'cw' | 'ccw';
  start: 'northwest' | 'north' | 'first' | 'point';
  point: Vec2 | null;
  /** Corners closer than this (m) are one point with one number. */
  tolerance: number;
  /** Merge corners shared by several shapes (and with existing points). */
  shared: boolean;
}

/** A numbered corner as the core finds it; numbering.ts names it. */
export interface CoreCorner {
  p: Vec2;
  /** Unit vector pointing away from the shape at this corner (for text placement). */
  out: Vec2;
  /** k ≥ 0: the run's k-th new number (made where it first appears); −1 − j: existing point j's. */
  ref: number;
}

/** An edge-length label, with the object it belongs to. */
export interface CoreEdgeLabel {
  id: number;
  /** Text anchor (baseline centre). */
  p: Vec2;
  /** Degrees counter-clockwise from east, always readable. */
  rotation: number;
  /** Edge length in metres (an arc's length for an arc edge). */
  length: number;
}

/**
 * The geometry a run asks for: the objects of its features inputs, by id. As
 * an ExprGeometry it evaluates the run's expressions over them.
 */
export interface RunGeometry extends ExprGeometry {
  /** Geometry values of these objects for expressions, one record each (`measuredAt`). */
  measures(ids: readonly number[]): Float64Array;
  /**
   * Corner numbering of these objects in the given order (polygons: outer
   * ring, then holes; polylines; other kinds have no corners); `existing`:
   * numbered points a corner may take (with `shared`).
   */
  numberCorners(ids: readonly number[], walk: CornerWalk, existing: readonly Vec2[]): CoreCorner[];
  /** Where the texts beside numbered corners go, for texts of `chars` characters and `height`. */
  cornerTexts(corners: readonly { p: Vec2; out: Vec2 }[], texts: readonly string[], height: number, font: string): Vec2[];
  /** Edge-length labels of these objects in order (lines, polylines, polygons); an edge two shapes share once when `shared`. */
  edgeLengths(ids: readonly number[], height: number, minLength: number, side: 'outside' | 'inside', shared: boolean): { labels: CoreEdgeLabel[]; skipped: number };
  /**
   * Which inputs stand in the relation to which references (docs/adr/0200 §1): (input position, reference position)
   * pairs, inputs first, then references in their order; an object is never paired with itself. Ayrık is asked as
   * Kesişen and turned round by the caller.
   */
  relatePairs(inputs: readonly number[], references: readonly number[], relation: Exclude<SpatialRelation, 'disjoint'>, within: number): [number, number][];
  /**
   * Each input's nearest targets (docs/adr/0215 §2.1): `k` of them (0: all) within `max` (Infinity: no bound), by
   * distance then the targets' order; an object is never its own target.
   */
  nearest(inputs: readonly number[], targets: readonly number[], k: number, max: number, measure: ProximityMeasure): NearestFound[];
  /** The areas' neighbours (docs/adr/0215 §2.2), both ways round, in the list's order. */
  neighbors(ids: readonly number[], tolerance: number, corners: boolean, overlaps: boolean): NeighborFound[];
}

/** How a distance is measured (docs/adr/0215 §2.1): edge to edge or centre to centre. */
export type ProximityMeasure = 'edges' | 'centers';

/** A target found for an input: their places in the two lists, the distance, the two nearest points and the bearing from the first to the second (radians, clockwise from north; NaN at 0 apart). */
export interface NearestFound {
  readonly input: number;
  readonly target: number;
  readonly d: number;
  readonly a: Vec2;
  readonly b: Vec2;
  readonly bearing: number;
}

/** A neighbour of an area: their places, how they neighbour, the shared boundary (m) and the overlapping area (m²). */
export interface NeighborFound {
  readonly area: number;
  readonly neighbor: number;
  readonly kind: 'edge' | 'corner' | 'overlap';
  readonly length: number;
  readonly overlap: number;
}

const NEAREST_STRIDE = 8;
const NEIGHBOR_STRIDE = 5;
const NEIGHBOR_KIND = ['edge', 'corner', 'overlap'] as const;

/** The core's records read back (`store::proximity`'s strides). */
export function nearestFound(r: Float64Array): NearestFound[] {
  const out: NearestFound[] = [];
  for (let k = 0; k + NEAREST_STRIDE <= r.length; k += NEAREST_STRIDE)
    out.push({ input: r[k], target: r[k + 1], d: r[k + 2], a: { x: r[k + 3], y: r[k + 4] }, b: { x: r[k + 5], y: r[k + 6] }, bearing: r[k + 7] });
  return out;
}

export function neighborFound(r: Float64Array): NeighborFound[] {
  const out: NeighborFound[] = [];
  for (let k = 0; k + NEIGHBOR_STRIDE <= r.length; k += NEIGHBOR_STRIDE)
    out.push({ area: r[k], neighbor: r[k + 1], kind: NEIGHBOR_KIND[r[k + 2]] ?? 'edge', length: r[k + 3], overlap: r[k + 4] });
  return out;
}

/** The relations of the queries (docs/adr/0200 §1), as the tools' values and the shared cases write them. */
export type SpatialRelation = 'intersects' | 'contains' | 'within' | 'disjoint' | 'near' | 'centerIn';

/** The core's relation codes (crates/shared/geometry-core/src/ops/spatial_query.rs `Relation::ALL`). */
const RELATION_CODE: Record<SpatialRelation, number> = { intersects: 0, contains: 1, within: 2, disjoint: 3, near: 4, centerIn: 5 };

/** The core's start codes (crates/shared/geometry-core/src/processing/numbering.rs `StartCorner::from_code`). */
const START: Record<CornerWalk['start'], number> = { northwest: 0, north: 1, first: 2, point: 3 };
/** Numbers per corner (`store::processing::CORNER_STRIDE`) and per edge label (`EDGE_LABEL_STRIDE`). */
const CORNER_STRIDE = 5;
const LABEL_STRIDE = 5;

/**
 * A geometry store of its own for a list of objects (a run's inputs, or the
 * objects a preview reads): packed on the first question, freed by `dispose`.
 */
export class ObjectStore implements RunGeometry, DocumentGeometry {
  private readonly objects: readonly Entity[];
  private store: CoreStore | null = null;

  constructor(objects: Iterable<Entity>) {
    this.objects = [...objects];
  }

  private get core(): CoreStore {
    if (!this.store) {
      this.store = new CoreStore();
      const p = packEntities(this.objects);
      this.store.putPacked(p.nums, p.strings);
    }
    return this.store;
  }

  inBox(r: Bounds): number[] {
    return Array.from(this.core.inBox(r.minX, r.minY, r.maxX, r.maxY));
  }

  measures(ids: readonly number[]): Float64Array {
    return this.core.measures(Float64Array.from(ids));
  }

  evaluateExpression(source: string, ids: Float64Array, texts: string, textLens: Int32Array, numbers: Float64Array, scale: number, want: number): ExprColumnData {
    return this.core.evaluateExpression(source, ids, texts, textLens, numbers, scale, want);
  }

  /** The same in a context, with the layers its calls to other objects look at (docs/adr/0214); their objects are here too. */
  evaluateExpressionIn(source: string, context: string, ids: Float64Array, texts: string, textLens: Int32Array, numbers: Float64Array, scale: number, want: number, world?: ExprWorldTable): ExprColumnData {
    return this.core.evaluateExpressionIn(source, context, ids, texts, textLens, numbers, scale, want, world);
  }

  numberCorners(ids: readonly number[], walk: CornerWalk, existing: readonly Vec2[]): CoreCorner[] {
    const xy = new Float64Array(existing.length * 2);
    existing.forEach((p, i) => xy.set([p.x, p.y], 2 * i));
    const r = this.core.numberCorners(Float64Array.from(ids), walk.dir === 'ccw', START[walk.start], walk.point, walk.tolerance, walk.shared, xy);
    const out: CoreCorner[] = [];
    for (let k = 0; k < r.length; k += CORNER_STRIDE) out.push({ p: { x: r[k], y: r[k + 1] }, out: { x: r[k + 2], y: r[k + 3] }, ref: r[k + 4] });
    return out;
  }

  cornerTexts(corners: readonly { p: Vec2; out: Vec2 }[], texts: readonly string[], height: number, font: string): Vec2[] {
    return textAnchors(corners, texts, height, font);
  }

  edgeLengths(ids: readonly number[], height: number, minLength: number, side: 'outside' | 'inside', shared: boolean): { labels: CoreEdgeLabel[]; skipped: number } {
    const r = this.core.edgeLengths(Float64Array.from(ids), height, minLength, side === 'inside', shared);
    const labels: CoreEdgeLabel[] = [];
    for (let k = 1; k < r.length; k += LABEL_STRIDE) labels.push({ id: r[k], p: { x: r[k + 1], y: r[k + 2] }, rotation: r[k + 3], length: r[k + 4] });
    return { labels, skipped: r[0] ?? 0 };
  }

  relatePairs(inputs: readonly number[], references: readonly number[], relation: Exclude<SpatialRelation, 'disjoint'>, within: number): [number, number][] {
    const r = this.core.relatePairs(Float64Array.from(inputs), Float64Array.from(references), RELATION_CODE[relation], within);
    const out: [number, number][] = [];
    for (let k = 0; k + 1 < r.length; k += 2) out.push([r[k], r[k + 1]]);
    return out;
  }

  nearest(inputs: readonly number[], targets: readonly number[], k: number, max: number, measure: ProximityMeasure): NearestFound[] {
    return nearestFound(this.core.nearest(Float64Array.from(inputs), Float64Array.from(targets), k, max, measure === 'centers' ? 1 : 0));
  }

  neighbors(ids: readonly number[], tolerance: number, corners: boolean, overlaps: boolean): NeighborFound[] {
    return neighborFound(this.core.neighbors(Float64Array.from(ids), tolerance, corners, overlaps));
  }

  /** Frees the Rust side; the store is built again if asked afterwards. */
  dispose(): void {
    this.store?.dispose();
    this.store = null;
  }
}

/** The texts beside numbered corners, placed by the core in one call. */
function textAnchors(corners: readonly { p: Vec2; out: Vec2 }[], texts: readonly string[], height: number, font: string): Vec2[] {
  const packed = new Float64Array(corners.length * 4);
  corners.forEach((c, i) => packed.set([c.p.x, c.p.y, c.out.x, c.out.y], 4 * i));
  const r = cornerTexts(packed, texts, height, font);
  const out: Vec2[] = [];
  for (let k = 0; k + 1 < r.length; k += 2) out.push({ x: r[k], y: r[k + 1] });
  return out;
}

/** Runs `fn` against a store of these objects, freed afterwards. */
export function withObjects<T>(objects: Iterable<Entity>, fn: (s: ObjectStore) => T): T {
  const s = new ObjectStore(objects);
  try {
    return fn(s);
  } finally {
    s.dispose();
  }
}
