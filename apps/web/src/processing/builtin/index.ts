import type { ProcessingTool } from '../types';
import { calculateField } from './calculateField';
import { edgeLengths } from './edgeLengths';
import { infoFromEnclosing } from './infoFromEnclosing';
import { infoFromInside } from './infoFromInside';
import { joinByField } from './joinByField';
import { selectByExpression } from './selectByExpression';
import { selectByLocation } from './selectByLocation';
import { summaryStatistics } from './summaryStatistics';
import { vertexNumbering } from './vertexNumbering';

/** Tools that ship with KentOS, registered at start-up (app/createApp). */
export const BUILTIN_TOOLS: readonly ProcessingTool[] = [
  vertexNumbering,
  edgeLengths,
  calculateField,
  selectByExpression,
  selectByLocation,
  infoFromInside,
  infoFromEnclosing,
  summaryStatistics,
  joinByField,
];
