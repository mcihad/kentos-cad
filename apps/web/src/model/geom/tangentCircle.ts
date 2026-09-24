import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { Edge } from './intersect';

/** Circles tangent to edges, computed by the geometry core (docs/adr/0008). */

/**
 * Circle of radius r tangent to two edges ("Teğet, teğet, yarıçap"). Lines
 * count as infinite, arcs as their full circle. The centre lies on a
 * parallel of each at distance r (lines: ±r, circles: R ± r); among the
 * crossings of those, the one whose tangent points are nearest to the
 * picked points wins.
 */
export const tangentTangentRadius = op<(e1: Edge, pick1: Vec2, e2: Edge, pick2: Vec2, r: number) => { c: Vec2; r: number } | null>('tangentTangentRadius');

/**
 * Circle tangent to three edges ("Teğet, teğet, teğet"; Apollonius).
 * Each edge gives one equation in the centre and radius: a line (taken
 * as infinite) keeps the centre at ±r from it, a circle at R + r, R − r
 * or r − R from its centre. Every side combination is solved (three
 * lines exactly, anything with a circle by Newton's method started at the
 * picked points); the circle whose tangent points lie nearest the picks
 * wins.
 */
export const tangentTangentTangent = op<(edges: readonly [Edge, Edge, Edge], picks: readonly [Vec2, Vec2, Vec2]) => { c: Vec2; r: number } | null>('tangentTangentTangent');
