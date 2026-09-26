import { op } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import type { Edge } from '../geom/intersect';

/** Decomposes an entity into primitive edges (points and text have none); computed by the geometry core (docs/adr/0008). */
export const entityEdges = op<(e: Entity) => Edge[]>('entityEdges');

export const edgeLength = op<(e: Edge) => number>('edgeLength');

/** The edge of `e` nearest to p: a polyline's clicked segment, a circle itself; null for a point or text (docs/adr/0032). */
export const nearestEdge = op<(e: Entity, p: Vec2) => Edge | null>('nearestEdge');
