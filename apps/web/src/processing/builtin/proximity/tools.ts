import type { ProcessingTool } from '../../types';
import { proximityHub } from './hub';
import { proximityMatrix } from './matrix';
import { proximityNearest } from './nearest';
import { proximityNeighbors } from './neighbors';
import { proximityShortestLine } from './shortestLine';

/** Yakınlık (docs/adr/0215): the five tools, in the toolbox's order. */
export const PROXIMITY_TOOLS: readonly ProcessingTool[] = [proximityNearest, proximityMatrix, proximityHub, proximityNeighbors, proximityShortestLine];
