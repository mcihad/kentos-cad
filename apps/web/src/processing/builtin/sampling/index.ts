import type { ProcessingTool } from '../../types';
import { alongLines } from './along';
import { grid } from './grid';
import { randomPoints, regularPoints } from './points';
import { subdivide } from './subdivide';

/** Örnekleme (docs/adr/0220): grids, regular and random points, points along lines, subdivided objects. */
export const SAMPLE_TOOLS: readonly ProcessingTool[] = [grid, regularPoints, randomPoints, alongLines, subdivide];
