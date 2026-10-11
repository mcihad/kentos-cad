import type { ProcessingTool } from '../../types';
import { densify, linesToPolygons, polygonsToLines, substring } from './paths';
import { pointsToPath } from './pointsToPath';
import { bearingLine, tableEllipse, tablePoints, xyLine } from './tables';
import { toPoints } from './toPoints';
import { vertices } from './vertices';

/** Geometri dönüştürme (docs/adr/0218, 0223 §5): points from objects and tables, lines from points and tables, pieces, rings and areas, densified edges. */
export const CONVERT_TOOLS: readonly ProcessingTool[] = [toPoints, vertices, pointsToPath, bearingLine, xyLine, tableEllipse, tablePoints, substring, polygonsToLines, linesToPolygons, densify];
