import type { ProcessingTool } from '../../types';
import { densify, linesToPolygons, polygonsToLines, substring } from './paths';
import { pointsToPath } from './pointsToPath';
import { bearingLine, tableEllipse, xyLine } from './tables';
import { toPoints } from './toPoints';
import { vertices } from './vertices';

/** Geometri dönüştürme (docs/adr/0218): points from objects, lines from points and tables, pieces, rings and areas, densified edges. */
export const CONVERT_TOOLS: readonly ProcessingTool[] = [toPoints, vertices, pointsToPath, bearingLine, xyLine, tableEllipse, substring, polygonsToLines, linesToPolygons, densify];
