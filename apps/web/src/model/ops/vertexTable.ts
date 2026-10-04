import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { Elevated } from './elevation';

/**
 * Köşe tablosu's computations (docs/adr/0172 §7): the core's `ops::vertex_table`, one for both platforms, on an
 * object's paths with their elevations (`elevatedPaths`' order). A write gives the new paths or why it is refused; the
 * table says the reason in its own words (ui/bottom/vertexEdit.ts). The independent reference is
 * scripts/fixtures/vertex_table_cases.py.
 */

/** The object's kind: its paths say the rest. */
export type VertexKind = 'line' | 'polyline' | 'polygon';

/**
 * A vertex as the table shows it: its path and place in it, where it is, its elevation, and the edge leaving it. What
 * a vertex has not (an elevation, an edge, an arc) is left out.
 */
export interface VertexRow {
  path: number;
  index: number;
  p: Vec2;
  z?: number | null;
  /** The edge's chord to the next vertex (none past an open path's end). */
  chord?: number | null;
  /** The edge's signed radius: plus turns left (counter-clockwise); none for a straight edge. */
  radius?: number | null;
}

/** Why a write is refused (`least`: half the chord, the shortest radius). */
export type VertexRefusal =
  | { why: 'missing' }
  | { why: 'ontoNeighbour' }
  | { why: 'noEdge' }
  | { why: 'lineArc' }
  | { why: 'noChord' }
  | { why: 'radiusBelow'; least: number }
  | { why: 'lineEnds' }
  | { why: 'pathMin' }
  | { why: 'ringMin' };

/** The object after a write: its kind (a line given a vertex is a polyline) and paths, or why not. */
export type VertexAnswer = { edited: { kind: VertexKind; paths: Elevated[] } } | { refusal: VertexRefusal };

/** The rows: every vertex of every path, in order. */
export const vertexTableRows = op<(paths: readonly Elevated[]) => VertexRow[]>('vertexTableRows');

/** Vertex `index` of path `path` moved to `to`; arcs keep their bulges. Not onto a neighbouring vertex. */
export const vertexTableMove = op<(kind: VertexKind, paths: readonly Elevated[], path: number, index: number, to: Vec2) => VertexAnswer>('vertexTableMove');

/** Vertex `index` of path `path` given the elevation `z` (null removes it). */
export const vertexTableZ = op<(kind: VertexKind, paths: readonly Elevated[], path: number, index: number, z: number | null) => VertexAnswer>('vertexTableZ');

/**
 * The edge leaving vertex `index` given the signed radius (null or 0: straight); an arc keeps its size. Shorter than
 * half the chord by at most `slack` metres is half a circle; by more, refused.
 */
export const vertexTableRadius = op<(kind: VertexKind, paths: readonly Elevated[], path: number, index: number, radius: number | null, slack: number) => VertexAnswer>('vertexTableRadius');

/** A vertex at `at` (elevation `z`) after vertex `after` of path `path`; the edge it splits is straight on both sides. */
export const vertexTableInsert = op<(kind: VertexKind, paths: readonly Elevated[], path: number, after: number, at: Vec2, z: number | null) => VertexAnswer>('vertexTableInsert');

/** The vertices `at` (each `[path, index]`) removed in one write; a polyline keeps two, a ring three, a line both ends. */
export const vertexTableRemove = op<(kind: VertexKind, paths: readonly Elevated[], at: readonly (readonly [number, number])[]) => VertexAnswer>('vertexTableRemove');
