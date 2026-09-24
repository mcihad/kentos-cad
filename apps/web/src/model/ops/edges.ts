import { op } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Edge } from '../geom/intersect';

/** Decomposes an entity into primitive edges (points and text have none); computed by the geometry core (docs/adr/0008). */
export const entityEdges = op<(e: Entity) => Edge[]>('entityEdges');

export const edgeLength = op<(e: Edge) => number>('edgeLength');
