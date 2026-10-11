import { op } from '../../wasm/core';
import type { Entity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Doğrusal referanslama (docs/adr/0224): routes (lines and polylines with a measure, M, at their vertices) and the
 * places along them, computed by the geometry core (`ops::lrs`; the desktop's tools call the same functions), a list
 * a call: making routes from pieces, calibrating them by points, placing point and line events, locating objects
 * along them, overlaying line events, summing their measures up. An offset or a distance is positive to the right of
 * the measure direction. The independent reference is scripts/fixtures/lrs_cases.py.
 */

/** A part of a route: its vertices, its arcs' bulges and each vertex's measure, null where it has none. */
export interface RoutePart {
  pts: Vec2[];
  bulges?: number[] | null;
  ms: (number | null)[];
}

/** A route: its parts in walked order. */
export interface Route {
  parts: RoutePart[];
}

/**
 * An object as a route: a line's two ends and their measures, a polyline's own part and its parts, a vertex without a
 * measure null; none of another kind (the desktop's `kentos_native_application::geometry::route`). Km yaz and the
 * Doğrusal referans tools read routes through here.
 */
export function routeOf(e: Entity): Route | null {
  if (e.kind === 'line') return { parts: [{ pts: [e.a, e.b], ms: [e.ma ?? null, e.mb ?? null] }] };
  if (e.kind !== 'polyline') return null;
  const part = (q: { pts: RoutePart['pts']; bulges?: number[]; ms?: (number | null)[] }): RoutePart => ({
    pts: q.pts,
    ...(q.bulges && { bulges: q.bulges }),
    ms: q.ms ?? q.pts.map(() => null),
  });
  return { parts: [part(e), ...(e.parts ?? []).map(part)] };
}

/** Whether a route has a measure at any vertex. */
export const measured = (route: Route): boolean => route.parts.some((q) => q.ms.some((m) => m !== null));

/** Why a route gives no place. (A field the core has none of is left out, not null.) */
export type Fault =
  | { fault: 'noMeasures' }
  | { fault: 'outside'; least: number; most: number }
  | { fault: 'same' }
  | { fault: 'fewPoints' }
  | { fault: 'offset' }
  | { fault: 'noRoute' }
  | { fault: 'noMeasure' };

/** Rota oluştur's rules: the measures from the length, a field each or a from and to each. */
export interface CreateRules {
  method: 'length' | 'oneField' | 'twoFields';
  priority?: 'upperLeft' | 'lowerLeft' | 'upperRight' | 'lowerRight';
  factor?: number;
  start?: number;
  ignoreGaps?: boolean;
}

/** A piece of a route to be: its route id, path and field values. */
export interface Piece {
  id: string;
  pts: Vec2[];
  bulges?: number[];
  value?: number | null;
  from?: number | null;
  to?: number | null;
}

export type PieceFault = 'noId' | 'noValue' | 'noFromTo' | 'noPath';

export interface Created {
  routes: { id: string; parts: RoutePart[] }[];
  faults: { piece: number; why: PieceFault }[];
}

export const lrsCreate = op<(pieces: readonly Piece[], rules: CreateRules) => Created>('lrsCreate');

export interface CalibrateRules {
  radius: number;
  before?: boolean;
  after?: boolean;
  ignoreGaps?: boolean;
  /** Vertices put where the points fall (true when absent); false: only the route's own vertices take measures. */
  split?: boolean;
}

/** A calibration point: where it is and its measure. */
export interface Mark {
  p: Vec2;
  m: number;
}

export interface Calibrated {
  parts?: RoutePart[] | null;
  used: boolean[];
  fault?: Fault | null;
}

/** Each route calibrated by its own marks (`marks[i]` route `i`'s). */
export const lrsCalibrate = op<(routes: readonly Route[], marks: readonly (readonly Mark[])[], rules: CalibrateRules) => Calibrated[]>('lrsCalibrate');

/** An event to place: its route by its place in the list (null: no such route), a point's measure or a line's from and to, its offset. */
export interface EventIn {
  route: number | null;
  m?: number | null;
  from?: number | null;
  to?: number | null;
  offset?: number | null;
}

/** An event placed: a point, a line's parts, or why not. */
export interface EventOut {
  p?: Vec2 | null;
  parts?: RoutePart[] | null;
  fault?: Fault | null;
}

export const lrsEvents = op<(routes: readonly Route[], events: readonly EventIn[]) => EventOut[]>('lrsEvents');

/** An object located along a route: a point's measure and distance, a line's or an area's piece's from and to. */
export interface Located {
  route: number;
  m?: number | null;
  from?: number | null;
  to?: number | null;
  distance?: number | null;
}

/** Each object's places along the routes within the radius; only the nearest route unless `all`. */
export const lrsLocate = op<(routes: readonly Route[], features: readonly EntityGeometry[], radius: number, all: boolean) => Located[][]>('lrsLocate');

/** A line event: its route and measures. */
export interface LineEvent {
  route: string;
  from: number;
  to: number;
}

/** A piece of an overlay: its route, measures and the events of each side covering it. */
export interface Overlaid {
  route: string;
  from: number;
  to: number;
  a?: number | null;
  b?: number | null;
}

export const lrsOverlay = op<(a: readonly LineEvent[], b: readonly LineEvent[], union: boolean) => Overlaid[]>('lrsOverlay');

/** The first, last, least and most measure of a route and how many vertices have one. */
export interface Summary {
  first?: number | null;
  last?: number | null;
  least?: number | null;
  most?: number | null;
  count: number;
}

export const lrsSummaries = op<(routes: readonly Route[]) => Summary[]>('lrsSummaries');
