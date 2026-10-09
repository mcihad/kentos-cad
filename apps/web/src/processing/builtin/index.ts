import type { ProcessingTool } from '../types';
import { calculateField } from './calculateField';
import { edgeLengths } from './edgeLengths';
import { geometryBuffer } from './geometry/buffer';
import { geometryClip } from './geometry/clip';
import { geometryDissolve } from './geometry/dissolve';
import { geometryDifference, geometryIntersection, geometrySymDifference, geometryUnion } from './geometry/overlay';
import { geometryReproject } from './geometry/reproject';
import { geometrySimplify } from './geometry/simplify';
import { geometryRepair, geometryValidity } from './geometry/validity';
import { infoFromEnclosing } from './infoFromEnclosing';
import { infoFromInside } from './infoFromInside';
import { joinByField } from './joinByField';
import { networkClosestFacility } from './network/closestFacility';
import { networkOdMatrix } from './network/odMatrix';
import { networkServiceAreas } from './network/serviceAreas';
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
  geometryBuffer,
  geometryClip,
  geometryDissolve,
  geometryIntersection,
  geometryDifference,
  geometrySymDifference,
  geometryUnion,
  geometryValidity,
  geometryRepair,
  geometrySimplify,
  geometryReproject,
  networkClosestFacility,
  networkOdMatrix,
  networkServiceAreas,
];
