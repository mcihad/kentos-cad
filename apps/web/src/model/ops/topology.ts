import { op } from '../../wasm/core';
import type { ArcGeom } from '../geom/arc';
import type { Vec2 } from '../geometry';

/**
 * Topolojik temizlik (docs/adr/0148): the core's `ops::topology`. Line work and area outlines put right within a
 * tolerance: ends that almost meet meet, vertices that almost coincide coincide (Uçlar, Köşeler), and a free end
 * that stops short of a line or runs past one is extended or trimmed, or else moved onto the nearest line (Uzat,
 * Buda). A vertex never moves further than the tolerance and joins on a vertex that is there; fixed objects and
 * points never move. The independent reference is scripts/fixtures/topology_cases.py.
 */

/** A path of an object with its bulges and its vertices' elevations. */
export interface TopoPath {
  pts: Vec2[];
  bulges?: number[];
  closed: boolean;
  zs: (number | null)[];
}

/** An object as the cleanup takes it: an arc is two vertices and a bulge; `edges` are a boundary only. */
export interface TopoObject {
  kind: 'line' | 'polyline' | 'arc' | 'area' | 'point' | 'edges';
  fixed: boolean;
  paths: TopoPath[];
}

/** The four works (docs/adr/0148 §3). */
export interface TopoWorks {
  ends: boolean;
  vertices: boolean;
  extend: boolean;
  trim: boolean;
}

export interface TopoChange {
  kind: 'end' | 'vertex' | 'extended' | 'trimmed' | 'edge';
  from: Vec2;
  to: Vec2;
  object: number;
}

export interface TopoResult {
  /** Each changed object: its index in the input, its new paths. */
  changed: { object: number; paths: TopoPath[] }[];
  changes: TopoChange[];
  counts: { ends: number; vertices: number; extended: number; trimmed: number; edges: number };
  maxShift: number;
}

/** Topolojik temizlik over `objects` (in the drawing's order); throws on a tolerance below 1 µm. */
export const topologyClean = op<(objects: TopoObject[], tolerance: number, works: TopoWorks) => TopoResult>('topologyClean');

/**
 * An arc as the cleanup takes it (docs/adr/0148 §2): its two ends counter-clockwise and the bulge between them, so a
 * moved end keeps its angle; null for a whole turn (a boundary, as a circle is). The core's, so the web and the
 * desktop hand the cleanup the same bits.
 */
export const topologyArcPath = op<(arc: ArcGeom) => TopoPath | null>('topologyArcPath');

/** The arc a cleaned arc's path stands for, counter-clockwise from its first vertex, angles in [0, 2π); null when it is no arc. */
export const topologyPathArc = op<(path: TopoPath) => ArcGeom | null>('topologyPathArc');
