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
import { INTERPOLATION_TOOLS } from './interpolation/tools';
import { RASTER_OPS_TOOLS } from './rasterOps/tools';
import { RASTER_VECTOR_TOOLS } from './rasterVector/tools';
import { HYDROLOGY_TOOLS } from './hydrology/tools';
import { DISTANCE_TOOLS } from './distance/tools';
import { SUITABILITY_TOOLS } from './suitability/tools';
import { STATS_TOOLS } from './stats/tools';
import { REMOTE_TOOLS } from './remote/tools';
import { SURFACE_TOOLS } from './surface/tools';
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
  // Yüzey analizi (docs/adr/0231): the raster core's jobs in the page's analysis worker.
  ...SURFACE_TOOLS,
  // İnterpolasyon and Yoğunluk (docs/adr/0232): the raster core's point jobs, in the same worker.
  ...INTERPOLATION_TOOLS,
  // Raster işlemleri and Raster istatistiği (docs/adr/0233): the raster core's operation jobs, in the same worker.
  ...RASTER_OPS_TOOLS,
  // Raster ve vektör and Taranmış harita (docs/adr/0234): its operation and point jobs, Eğrilere kot ver the geometry core's.
  ...RASTER_VECTOR_TOOLS,
  // Hidroloji (docs/adr/0235): the raster core's operation job over the DEM in memory.
  ...HYDROLOGY_TOOLS,
  // Uzaklık ve maliyet (docs/adr/0236): the point job from objects, the operation job over the cost raster.
  ...DISTANCE_TOOLS,
  // Uygunluk analizi (docs/adr/0237): the operation job over the criteria, AHP's weights and ROC's counts with it.
  ...SUITABILITY_TOOLS,
  // Mekânsal istatistik (docs/adr/0238): the geometry core's runs over the objects' places.
  ...STATS_TOOLS,
  // Uzaktan algılama (docs/adr/0242): the operation job over the images, classes, references and dates.
  ...REMOTE_TOOLS,
];
