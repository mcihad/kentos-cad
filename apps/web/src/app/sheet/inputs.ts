import type { Capabilities } from '../../contracts/generated/sheet/Capabilities';
import type { CoordinateInput } from '../../contracts/generated/sheet/CoordinateInput';
import type { CrsInfo } from '../../contracts/generated/sheet/CrsInfo';
import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { FeatureInput } from '../../contracts/generated/sheet/FeatureInput';
import type { Item } from '../../contracts/generated/sheet/Item';
import type { LegendEntryInput } from '../../contracts/generated/sheet/LegendEntryInput';
import type { LegendInput } from '../../contracts/generated/sheet/LegendInput';
import type { MapLayers } from '../../contracts/generated/sheet/MapLayers';
import type { RenderInputs } from '../../contracts/generated/sheet/RenderInputs';
import type { RenderMode } from '../../contracts/generated/sheet/RenderMode';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import type { TableInput } from '../../contracts/generated/sheet/TableInput';
import type { CrsDef } from '../../geo/crs';
import { foldTurkish } from '../../core/text';
import type { Entity } from '../../model/entities';
import type { Bounds } from '../../model/geometry';
import type { Symbol } from '../../model/style';
import { legendOf } from '../../style/legend';
import type { AppContext } from '../context';

/**
 * What the engine needs of the project to draw a sheet (docs/sheet/design.md
 * §8 `RenderInputs`): the project's name, the user, today, the coordinate
 * system (with its transverse Mercator parameters, from which the engine
 * works out the meridian convergence itself), what the project offers, the
 * rows of the layer tables, the coordinate lists' points and the legends'
 * entries. The engine filters, sorts, lays out and writes them; this only
 * reads the drawing. “In the map” is asked of the drawing's index with the
 * ground each map frame covers, which the engine gives (`MapPrim.extent`).
 */

/**
 * The ellipsoids of geo/crs.ts by name: semi-major axis and inverse
 * flattening, as their definitions give them (GRS80: IUGG 1979; WGS 84: NIMA
 * TR8350.2; International 1924: Hayford). geo/crs.ts names the ellipsoid of
 * every system and keeps no axes; this table belongs there on merge.
 */
const ELLIPSOID: Record<CrsDef['ellipsoid'], { semiMajor: number; inverseFlattening: number }> = {
  GRS80: { semiMajor: 6_378_137, inverseFlattening: 298.257_222_101 },
  WGS84: { semiMajor: 6_378_137, inverseFlattening: 298.257_223_563 },
  'International 1924': { semiMajor: 6_378_388, inverseFlattening: 297 },
};

export function crsInfo(crs: CrsDef): CrsInfo {
  const tm = crs.projection === 'Transverse Mercator' || crs.projection === 'UTM';
  if (!tm || crs.centralMeridian === undefined) return { name: crs.name };
  return {
    name: crs.name,
    tm: { centralMeridian: crs.centralMeridian, scaleFactor: crs.scaleFactor ?? 1, falseEasting: crs.falseEasting ?? 0, falseNorthing: crs.falseNorthing ?? 0, ...ELLIPSOID[crs.ellipsoid] },
  };
}

/** Today as ISO 8601 (`@tarih`), in the user's own day: the engine never reads a clock. */
export function today(now = new Date()): string {
  const p = (n: number) => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())}`;
}

/** An attribute's text as the engine's value: a plain decimal number is a number (a table's decimals apply to it). */
export function attrValue(text: string): string | number {
  return /^-?\d+(\.\d+)?$/.test(text.trim()) ? Number(text) : text;
}

/** A layer a table or a list names: by its id, else by its name (Turkish letters folded). */
function layerOf(ctx: AppContext, ref: string): string | null {
  const layers = ctx.doc.layers;
  if (layers.get(ref)?.type === 'layer') return ref;
  const want = foldTurkish(ref);
  return layers.leaves().find((l) => foldTurkish(l.name) === want)?.id ?? null;
}

/** The items of a sheet and of its master page (both are drawn, both get their inputs). */
function itemsOf(book: SheetBook, sheet: Sheet): Item[] {
  const master = sheet.master ? book.masters.find((m) => m.id === sheet.master) : undefined;
  return [...(master?.items ?? []), ...sheet.items];
}

/** The ground each map frame covers, by item id (from a list drawn before, or none yet). */
export function extentsOf(list: DisplayList | null): Map<string, Bounds> {
  const out = new Map<string, Bounds>();
  for (const p of list?.prims ?? []) if (p.type === 'map' && p.extent) out.set(p.item, { minX: p.extent[0], minY: p.extent[1], maxX: p.extent[2], maxY: p.extent[3] });
  return out;
}

export interface InputsOptions {
  readonly mode: RenderMode;
  readonly capabilities: Capabilities;
  /** The ground of each map (`extentsOf` a list drawn before); a map not in it counts nothing as in it. */
  readonly extents: ReadonlyMap<string, Bounds>;
  /** A legend symbol's picture key, made and kept by the caller (the painter and the SVG ask for it by that key). */
  legendSymbol(symbol: Symbol, size: { width: number; height: number }): string | null;
  /** The pictures whose bytes this device has (the preflight's `missing_asset`). */
  readonly assets?: readonly string[];
  /** The export's resolution (the preflight's pictures). */
  readonly dpi?: number;
}

export function renderInputs(ctx: AppContext, book: SheetBook, sheet: Sheet, o: InputsOptions): RenderInputs {
  const { doc } = ctx;
  const items = itemsOf(book, sheet);
  const crs = doc.crs.value;
  const inside = (map: string | undefined) => {
    const b = map ? o.extents.get(map) : undefined;
    return b ? new Set(ctx.view.inBox(b)) : null;
  };
  const tables: TableInput[] = [];
  const coordinates: CoordinateInput[] = [];
  const legends: LegendInput[] = [];
  for (const it of items) {
    const k = it.kind;
    if (k.type === 'table' && k.source.type === 'layer') {
      const layer = layerOf(ctx, k.source.layer);
      if (!layer) continue;
      const shown = inside(k.source.onlyInMap ?? undefined);
      const features: FeatureInput[] = doc.byLayer(layer).map((e) => {
        const m = ctx.view.measure([e.id]);
        return {
          id: String(e.id),
          attributes: Object.entries(e.attrs ?? {}).map(([name, value]) => ({ name, value: attrValue(value) })),
          ...(m.area > 0 ? { area: m.area } : {}),
          ...(m.length > 0 ? { length: m.length } : {}),
          inMap: shown ? shown.has(e.id) : true,
          inAtlas: false,
        };
      });
      tables.push({ item: it.id, features });
    } else if (k.type === 'coordinateList') {
      const list = k.source.type === 'selection' ? [...ctx.selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e) : (() => {
        const layer = layerOf(ctx, k.source.layer);
        return layer ? doc.byLayer(layer) : [];
      })();
      coordinates.push(coordinatesOf(ctx, it.id, list));
    } else if (k.type === 'legend') {
      const map = k.map ? items.find((m) => m.id === k.map) : undefined;
      const layers: MapLayers = map?.kind.type === 'map' ? map.kind.layers : { type: 'all' };
      legends.push({ item: it.id, entries: legendEntries(ctx, layers, inside(k.map ?? undefined), k.symbol, o) });
    }
  }
  return {
    mode: o.mode,
    project: { name: doc.name.value, user: ctx.cloud.me.value?.user.displayName ?? '', date: today(), crsName: crs.name },
    capabilities: o.capabilities,
    crs: crsInfo(crs),
    maps: [],
    legends,
    tables,
    coordinates,
    ...(o.assets ? { assets: [...o.assets] } : {}),
    ...(o.dpi ? { dpi: o.dpi } : {}),
  };
}

/** A coordinate list's points: one closed figure's corners (with its area), or the points and vertices of all. */
function coordinatesOf(ctx: AppContext, item: string, list: readonly Entity[]): CoordinateInput {
  const one = list.length === 1 ? list[0] : null;
  if (one && one.kind === 'polygon') {
    const area = ctx.view.measure([one.id]).area;
    return { item, points: one.pts.map((p) => ({ x: p.x, y: p.y })), closed: true, ...(area > 0 ? { area } : {}) };
  }
  const points = list.flatMap((e) => {
    if (e.kind === 'point') return [{ ...(e.label ? { name: e.label } : {}), x: e.p.x, y: e.p.y, ...(e.z !== undefined ? { z: e.z } : {}) }];
    if (e.kind === 'polygon' || e.kind === 'polyline') return e.pts.map((p) => ({ x: p.x, y: p.y }));
    if (e.kind === 'line') return [{ x: e.a.x, y: e.a.y }, { x: e.b.x, y: e.b.y }];
    return [];
  });
  return { item, points, closed: false };
}

/** A legend's rows: the layers its map shows, as the legend window lists them (style/legend.ts), each symbol a small picture. */
function legendEntries(ctx: AppContext, layers: MapLayers, shown: Set<number> | null, box: { width: number; height: number }, o: InputsOptions): LegendEntryInput[] {
  const { doc, styles } = ctx;
  const leaves = doc.layers.leaves().filter((l) => l.type === 'layer' && doc.layers.isVisible(l.id) && (layers.type !== 'list' || layers.layers.includes(l.id)));
  const groups = legendOf(
    leaves.map((l) => ({ id: l.id, name: l.name, style: l.style })),
    { entities: (id) => doc.byLayer(id), symbol: (r) => styles.library.symbol(r), itemName: (id) => styles.library.get(id)?.name },
  );
  const out: LegendEntryInput[] = [];
  for (const g of groups) {
    const parent = doc.layers.parentOf(g.layerId);
    const inMap = shown ? doc.byLayer(g.layerId).some((e) => shown.has(e.id)) : true;
    for (const entry of g.entries) {
      const key = entry.symbol ? o.legendSymbol(entry.symbol, box) : null;
      out.push({
        layer: g.layerId,
        label: g.entries.length > 1 ? entry.label : g.layerName,
        ...(g.entries.length > 1 ? { group: g.layerName } : parent && parent.type === 'group' ? { group: parent.name } : {}),
        symbol: key ? { type: 'image', asset: key } : { type: 'patch' },
        inMap,
        inAtlas: false,
      });
    }
  }
  return out;
}
