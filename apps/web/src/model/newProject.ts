import { crsBySrid, ELLIPSOIDS, workAreaCentre, type CrsDef } from '../geo/crs';
import { provinceByCode, type Province } from '../geo/provinces';
import type { DocumentContent } from './document';
import { tmForward } from './geom/geodesy';
import type { Bounds, Vec2 } from './geometry';
import { PROJECT_SETTINGS_DEFAULTS, type DrawingFont, type DrawingUnit, type Workspace } from './projectSettings';
import { CAD_ACTIVE_LAYER, cadLayers, STANDARD_ACTIVE_LAYER, standardLayers } from './standardLayers';

/**
 * An empty project (Dosya → Yeni proje): a CAD project's technical drawing
 * layers or the standard layer tree of a cadastral sheet, the chosen
 * coordinate system and plot scale, and the default units. Its local origin
 * is the zone's work-area centre (geo/crs.ts), since no object exists yet to
 * anchor it. It opens on its start view, which stays its home view
 * (`startView`, docs/adr/0165 §3).
 */

export interface NewProjectOptions {
  name: string;
  srid: number;
  /** Plot scale denominator (1:1000 → 1000). */
  plotScale: number;
  /** Project type (CBS when not given, docs/adr/0165). */
  workspace?: Workspace;
  /** Drawing typeface (Barlow when not given). */
  drawingFont?: DrawingFont;
  /** The province (plate code) whose centre it opens on (docs/adr/0165 §3); none: the zone's work area. */
  province?: number;
  /** A local project's drawing unit (docs/adr/0165 §2); metres when not given. */
  drawingUnit?: DrawingUnit;
}

export const NEW_PROJECT_NAME = 'Yeni proje';

export function newProjectContent(o: NewProjectOptions): DocumentContent {
  const crs = crsBySrid(o.srid);
  if (!crs) throw new Error(`EPSG:${o.srid} bu sürümde tanımlı değil. Listedeki sistemlerden birini seçin.`);
  return {
    name: o.name.trim() || NEW_PROJECT_NAME,
    settings: {
      ...PROJECT_SETTINGS_DEFAULTS,
      srid: crs.srid,
      plotScale: o.plotScale,
      workspace: o.workspace ?? PROJECT_SETTINGS_DEFAULTS.workspace,
      drawingFont: o.drawingFont ?? PROJECT_SETTINGS_DEFAULTS.drawingFont,
      // Only a local project has a unit of its own; metres are not written.
      ...(crs.kind === 'local' && o.drawingUnit && o.drawingUnit !== 'm' ? { drawingUnit: o.drawingUnit } : {}),
    },
    origin: workAreaCentre(crs),
    homeView: startView(crs, o.plotScale, o.province === undefined ? null : (provinceByCode(o.province) ?? null)),
    // A CAD project starts with technical drawing layers, any other with a cadastral sheet's (docs/adr/0165 §3).
    layers: o.workspace === 'cad' ? cadLayers() : standardLayers(o.plotScale),
    activeLayer: o.workspace === 'cad' ? CAD_ACTIVE_LAYER : STANDARD_ACTIVE_LAYER,
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

/** An A3 sheet across (landscape), metres on paper. */
const A3 = { w: 0.42, h: 0.297 };

/**
 * A latitude and longitude (degrees) in `crs`: a geographic system's longitude and latitude, a projected one's
 * grid point (east, north); null for the local system and where the projection cannot reach. A start view's, not a
 * datum transformation: the degrees are taken on the system's own ellipsoid (the desktop's `crs::project_lat_lon`).
 */
export function projectLatLon(crs: CrsDef, lat: number, lon: number): Vec2 | null {
  if (crs.kind === 'local') return null;
  if (crs.kind === 'geographic') return { x: lon, y: lat };
  if (crs.projection === 'Pseudo-Mercator') {
    const r = 6_378_137;
    const x = r * ((lon * Math.PI) / 180);
    const y = r * Math.log(Math.tan(Math.PI / 4 + (lat * Math.PI) / 360));
    return Number.isFinite(x) && Number.isFinite(y) ? { x, y } : null;
  }
  const e = crs.ellipsoid && ELLIPSOIDS[crs.ellipsoid];
  if (!e || crs.centralMeridian === undefined) return null;
  const tm = { centralMeridian: crs.centralMeridian, scaleFactor: crs.scaleFactor ?? 1, falseEasting: crs.falseEasting ?? 0, falseNorthing: crs.falseNorthing ?? 0, ...e };
  return tmForward(tm, lat, lon);
}

/**
 * Where a new project opens and comes back to, its home view (docs/adr/0165 §3): a local project on an A3 landscape
 * sheet at its scale with 0,0 at the bottom left, as AutoCAD's new drawing; any other on one sheet at its scale around
 * its province's centre, or around its zone's work area without one (the desktop's `new_project::start_view`).
 */
export function startView(crs: CrsDef, plotScale: number, province: Province | null): Bounds {
  if (crs.kind === 'local') return { minX: 0, minY: 0, maxX: A3.w * plotScale, maxY: A3.h * plotScale };
  const centre = (province && projectLatLon(crs, province.lat, province.lon)) ?? workAreaCentre(crs);
  return sheetAround(centre, plotScale, crs.unit);
}
