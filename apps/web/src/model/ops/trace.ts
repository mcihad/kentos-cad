import { CoreTraceGraph, op, writeArgs } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * İzle and Zincir (docs/adr/0161): the core's `ops::trace` and `ops::join::chain`, one for both platforms. A traced
 * way goes along visible line work cut at every meeting point, the shortest by length; its corners are the line work's
 * own vertices and the crossings where it turns, its arcs keep their circles. The independent reference is
 * scripts/fixtures/trace_cases.py.
 */

/** A traced way: its corners (the given ends first and last), one bulge per edge, its length. */
export interface Traced {
  pts: Vec2[];
  bulges: number[];
  length: number;
}

/** The kinds İzle follows (ellipses and curves only have an approximate outline, docs/adr/0149). */
export const tracedKind = (e: Entity): boolean => e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon' || e.kind === 'arc' || e.kind === 'circle';

/** The way from `a` to `b` along `lines`, the graph built for this call alone (tests; tools keep a `TraceGraph`). */
export const tracePath = op<(lines: readonly Entity[], a: Vec2, b: Vec2) => Traced | null>('tracePath');

/** The point of `lines` nearest to `p` within `reach` metres (tests; tools keep a `TraceGraph`). */
export const traceNearest = op<(lines: readonly Entity[], p: Vec2, reach: number) => Vec2 | null>('traceNearest');

/** The graph of line work kept in the core: built once per view, asked on every pointer move. */
export interface TraceGraph {
  /** The shortest way from `a` to `b` along the line work, or null. */
  path(a: Vec2, b: Vec2): Traced | null;
  /** The point of the line work nearest to `p` within `reach` metres (a vertex bit for bit), or null. */
  nearest(p: Vec2, reach: number): Vec2 | null;
  free(): void;
}

/** The graph of the line work İzle follows among `entities`. */
export function traceGraph(entities: readonly Entity[]): TraceGraph {
  const core = CoreTraceGraph.ofEntities(writeArgs(entities));
  return {
    path: (a, b) => core.path(a.x, a.y, b.x, b.y) as Traced | null,
    nearest: (p, reach) => core.nearest(p.x, p.y, reach) as Vec2 | null,
    free: () => core.free(),
  };
}

/** An object Zincir may walk through: its shape and whether its layer is locked. */
export interface ChainObject {
  shape: Entity;
  locked?: boolean;
}

/** The chain from the seed: its members from one end to the other, whether a locked object stopped it, whether it closed. */
export interface ChainFound {
  members: number[];
  locked: boolean;
  closed: boolean;
}

/** Zincir (docs/adr/0161 §2): the objects joined end to end with `objects[seed]`, within `tol`. */
export const joinChain = op<(objects: readonly ChainObject[], seed: number, tol: number) => ChainFound>('joinChain');
