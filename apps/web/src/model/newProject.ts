import { crsBySrid, workAreaCentre } from '../geo/crs';
import type { DocumentContent } from './document';
import type { Bounds } from './geometry';
import { PROJECT_SETTINGS_DEFAULTS, type DrawingFont, type Workspace } from './projectSettings';
import { STANDARD_ACTIVE_LAYER, standardLayers } from './standardLayers';

/**
 * An empty project (Dosya → Yeni proje): the standard layer tree, the chosen
 * coordinate system and plot scale, and the default units. Its local origin
 * is the zone's work-area centre (geo/crs.ts), since no object exists yet to
 * anchor it; it has no start view, so a saved file opens on its objects.
 */

export interface NewProjectOptions {
  name: string;
  srid: number;
  /** Plot scale denominator (1:1000 → 1000). */
  plotScale: number;
  /** Work mode (hybrid when not given). */
  workspace?: Workspace;
  /** Drawing typeface (Barlow when not given). */
  drawingFont?: DrawingFont;
}

export const NEW_PROJECT_NAME = 'Yeni proje';

export function newProjectContent(o: NewProjectOptions): DocumentContent {
  const crs = crsBySrid(o.srid);
  if (!crs) throw new Error(`EPSG:${o.srid} bu sürümde tanımlı değil. Listedeki sistemlerden birini seçin.`);
  return {
    name: o.name.trim() || NEW_PROJECT_NAME,
    settings: { ...PROJECT_SETTINGS_DEFAULTS, srid: crs.srid, plotScale: o.plotScale, workspace: o.workspace ?? PROJECT_SETTINGS_DEFAULTS.workspace, drawingFont: o.drawingFont ?? PROJECT_SETTINGS_DEFAULTS.drawingFont },
    origin: workAreaCentre(crs),
    homeView: null,
    layers: standardLayers(o.plotScale),
    activeLayer: STANDARD_ACTIVE_LAYER,
    entities: [],
    styles: { items: [], categories: [] },
  };
}

/**
 * One paper sheet (50 × 37.5 cm) at the plot scale around `p`: what a new,
 * empty project shows first. In a geographic system the sheet's metres are
 * turned into degrees roughly (a start view needs no precision).
 */
export function sheetAround(p: { x: number; y: number }, plotScale: number, unit: 'metre' | 'degree' = 'metre'): Bounds {
  const k = unit === 'degree' ? 1 / 111_320 : 1;
  const w = 0.25 * plotScale * k;
  const h = 0.1875 * plotScale * k;
  return { minX: p.x - w, minY: p.y - h, maxX: p.x + w, maxY: p.y + h };
}
