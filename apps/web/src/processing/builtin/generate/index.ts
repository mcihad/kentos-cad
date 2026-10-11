import type { ProcessingTool } from '../../types';
import { bounding, concaveHull } from './bounding';
import { pole } from './pole';
import { delaunay, voronoi } from './triangulation';

/** Geometri üretimi (docs/adr/0219): cells and triangles of points, bounding geometries, concave hulls, poles. */
export const GENERATE_TOOLS: readonly ProcessingTool[] = [voronoi, delaunay, bounding, concaveHull, pole];
