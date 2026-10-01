import { op } from '../../wasm/core';
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
