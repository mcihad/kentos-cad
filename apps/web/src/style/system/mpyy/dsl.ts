import type {
  Anchor,
  CentroidMarker,
  Color,
  FillLayer,
  HatchFill,
  ImageFill,
  LibraryCategory,
  LibraryItem,
  LineLayer,
  LineWave,
  MarkerLayer,
  MarkerLine,
  MarkerPlacement,
  MarkerSymbol,
  PatternFill,
  ShapeMarker,
  ShapeName,
  SimpleFill,
  SimpleLine,
  SvgMarker,
  Symbol,
  TextMarker,
} from '../../../model/style';

/**
 * Building blocks for the MPYY system library (Mekânsal Planlar Yapım
 * Yönetmeliği, EK-1a … EK-1e). A legend item is written as a list of
 * layers made by the helpers below; every length is paper millimetres as
 * printed in EK-1e (at the project's plot scale), every colour the
 * legend's RGB. Layer ids are given by position when the item is made.
 *
 *   const s = sheet('uip', ['Sınırlar', 'İdari sınırlar'], 'idari');
 *   s.line('koy-siniri', 'Köy sınırı', dashDots(BLACK, 0.7, { dash: 10, gap: 2, dots: 3, dot: 1, dotGap: 1 }), { ref: 'EK-1d s.1; EK-1e s.14' });
 */

// ── Colours ────────────────────────────────────────────────────────────

const hex2 = (v: number) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0').toUpperCase();

/**
 * Black and white follow the CAD convention (colour 7): the legend's black
 * is ink (black on paper, white on a dark screen) and its white is the
 * paper (the sheet showing through), so the symbols read on either theme
 * and print as the regulation shows them.
 */
export const BLACK: Color = 'ink';
export const WHITE: Color = 'paper';

/** "#RRGGBB" from the legend's R/G/B; `opacity` 0–1 adds alpha ("%50 şeffaf" is 0.5). Opaque black and white become BLACK and WHITE. */
export function rgb(r: number, g: number, b: number, opacity = 1): Color {
  if (opacity >= 1 && r === 0 && g === 0 && b === 0) return BLACK;
  if (opacity >= 1 && r === 255 && g === 255 && b === 255) return WHITE;
  return `#${hex2(r)}${hex2(g)}${hex2(b)}${opacity < 1 ? hex2(opacity * 255) : ''}`;
}

// ── Layers (ids are filled in when the item is made) ──────────────────

/** A layer without its id (given by position when the item is made); distributes over unions. */
type Draft<T> = T extends unknown ? Omit<T, 'id'> & { id?: string } : never;
type Many<T> = T | readonly T[];

// Fills

export const solid = (color: Color, opacity?: number): Draft<SimpleFill> => ({ type: 'simpleFill', color, opacity });

export interface HatchOptions {
  offset?: number;
  dash?: readonly number[];
  dashOffset?: number;
  opacity?: number;
}

/** Parallel lines: angle in degrees counter-clockwise from east (0 horizontal, 90 vertical, 45 rising to the right). */
export const hatch = (angle: number, spacing: number, width: number, color: Color = BLACK, o: HatchOptions = {}): Draft<HatchFill> => ({
  type: 'hatchFill',
  angle,
  spacing,
  width,
  color,
  offset: o.offset,
  dash: o.dash,
  dashOffset: o.dashOffset,
  opacity: o.opacity,
});

/** Two line families crossing (45°/135° by default). */
export const crossHatch = (spacing: number, width: number, color: Color = BLACK, angles: readonly [number, number] = [45, 135]): Draft<HatchFill>[] => angles.map((a) => hatch(a, spacing, width, color));

/** Vertical lines every `sx`, horizontal every `sy`. */
export const grid = (sx: number, sy: number, width: number, color: Color = BLACK): Draft<HatchFill>[] => [hatch(90, sx, width, color), hatch(0, sy, width, color)];

/**
 * Lines in groups: `count` lines `gap` apart (centre to centre), the group
 * repeating every `period` (paired or tripled hatches).
 */
export const groupedHatch = (angle: number, period: number, count: number, gap: number, width: number, color: Color = BLACK, o: HatchOptions = {}): Draft<HatchFill>[] =>
  Array.from({ length: count }, (_, i) => hatch(angle, period, width, color, { ...o, offset: (o.offset ?? 0) + i * gap }));

export interface PatternOptions {
  stagger?: boolean;
  angle?: number;
  offset?: readonly [number, number];
  jitter?: number;
  coverage?: number;
  seed?: number;
  opacity?: number;
}

/** A marker repeated on a grid `sx` × `sy`. */
export const pattern = (marker: Many<Draft<MarkerLayer>>, sx: number, sy: number, o: PatternOptions = {}): Draft<PatternFill> => ({
  type: 'patternFill',
  marker: markerSym(marker),
  spacingX: sx,
  spacingY: sy,
  stagger: o.stagger,
  angle: o.angle,
  offset: o.offset,
  jitter: o.jitter,
  coverage: o.coverage,
  seed: o.seed,
  opacity: o.opacity,
});

/** Filled dots (or rings with `hollow`) of diameter `size` on a grid. */
export const dotGrid = (sx: number, sy: number, size: number, color: Color = BLACK, o: PatternOptions & { hollow?: boolean; strokeWidth?: number } = {}): Draft<PatternFill> =>
  pattern(o.hollow ? circle(size, { stroke: color, strokeWidth: o.strokeWidth ?? 0.15 }) : circle(size, { fill: color }), sx, sy, o);

/**
 * Random dots (sand, "serbest noktalama"): about one dot of diameter `size`
 * per `cell` × `cell`, `coverage` of the cells used.
 */
export const stipple = (cell: number, size: number, color: Color = BLACK, coverage = 0.8, seed = 1): Draft<PatternFill> => pattern(circle(size, { fill: color }), cell, cell, { jitter: 1, coverage, seed });

/** An image asset repeated, `tile` mm wide. */
export const imageFill = (asset: string, tile: number, angle?: number): Draft<ImageFill> => ({ type: 'imageFill', asset, tileSize: tile, angle });

/** The area's boundary line (positive offset moves it into the area). */
export const edge = (color: Color, width: number, o: StrokeOptions = {}): Draft<SimpleLine> => stroke(color, width, o);

/** Markers or text at the area's inside point. */
export const label = (marker: Many<Draft<MarkerLayer>>, position?: CentroidMarker['position']): Draft<CentroidMarker> => ({ type: 'centroidMarker', marker: markerSym(marker), position });

// Lines

export interface StrokeOptions {
  dash?: readonly number[];
  dashOffset?: number;
  offset?: number;
  cap?: SimpleLine['cap'];
  join?: SimpleLine['join'];
  wave?: LineWave;
  opacity?: number;
  rings?: SimpleLine['rings'];
}

export const stroke = (color: Color, width: number, o: StrokeOptions = {}): Draft<SimpleLine> => ({
  type: 'simpleLine',
  color,
  width,
  dash: o.dash,
  dashOffset: o.dashOffset,
  offset: o.offset,
  cap: o.cap,
  join: o.join,
  wave: o.wave,
  opacity: o.opacity,
  rings: o.rings,
});

export interface AlongOptions {
  /** Distance of the first place from the start (default half the interval). */
  offsetAlong?: number;
  /** Perpendicular shift: + left of the drawing direction (on areas: inside). */
  offset?: number;
  /** Turn with the line (default true). */
  rotate?: boolean;
  placement?: MarkerPlacement;
  group?: { count: number; spacing: number };
  rings?: MarkerLine['rings'];
}

/** Markers every `interval` along the line. */
export const along = (marker: Many<Draft<MarkerLayer>>, interval: number, o: AlongOptions = {}): Draft<MarkerLine> => ({
  type: 'markerLine',
  marker: markerSym(marker),
  placement: o.placement ?? 'interval',
  interval,
  offsetAlong: o.offsetAlong ?? interval / 2,
  offset: o.offset,
  rotate: o.rotate ?? true,
  group: o.group,
  rings: o.rings,
});

/**
 * A dashed line with dots in its gaps (köy sınırı, mahalle sınırı):
 * `dash` long, then `gap`, `dots` round dots of diameter `dot` spaced
 * `dotGap` apart (edge to edge), `gap` again. The dots are markers, so they
 * stay round whatever the cap.
 */
export function dashDots(color: Color, width: number, o: { dash: number; gap: number; dots: number; dot: number; dotGap: number; dotColor?: Color }): Draft<LineLayer>[] {
  const run = o.dots * o.dot + (o.dots - 1) * o.dotGap;
  const period = o.dash + 2 * o.gap + run;
  return [
    stroke(color, width, { dash: [o.dash, period - o.dash] }),
    along(circle(o.dot, { fill: o.dotColor ?? color }), period, { offsetAlong: o.dash + o.gap + run / 2, rotate: true, group: o.dots > 1 ? { count: o.dots, spacing: o.dot + o.dotGap } : undefined }),
  ];
}

/**
 * Markers sitting in the gaps of a dashed line, the line stopping at them:
 * dash `dash`, then a gap of `gap` with the marker at its middle.
 */
export function dashMarkers(color: Color, width: number, dash: number, gap: number, marker: Many<Draft<MarkerLayer>>, o: { rotate?: boolean; offset?: number } = {}): Draft<LineLayer>[] {
  return [stroke(color, width, { dash: [dash, gap] }), along(marker, dash + gap, { offsetAlong: dash + gap / 2, rotate: o.rotate ?? true, offset: o.offset })];
}

/** Two parallel lines `gap` apart (centre to centre). */
export const double = (color: Color, width: number, gap: number, o: StrokeOptions = {}): Draft<SimpleLine>[] => [stroke(color, width, { ...o, offset: gap / 2 }), stroke(color, width, { ...o, offset: -gap / 2 })];

/**
 * Short strokes across the line (ticks), `length` long, every `interval`:
 * standing on the left side (+1; on areas: inside), the right (-1) or
 * centred (0), at `angle` degrees to the line (90: square to it).
 */
export function ticks(color: Color, width: number, length: number, interval: number, side: -1 | 0 | 1 = 1, o: { offsetAlong?: number; angle?: number } = {}): Draft<MarkerLine> {
  // A marker's offset is in its own turned frame: half a length along the tick puts its foot on the line.
  return along(shape('line', length, { stroke: color, strokeWidth: width, rotation: o.angle ?? 90, offset: [(side * length) / 2, 0] }), interval, { offsetAlong: o.offsetAlong, rotate: true });
}

// Markers

export interface ShapeOptions {
  fill?: Color | null;
  stroke?: Color | null;
  strokeWidth?: number;
  height?: number;
  rotation?: number;
  offset?: readonly [number, number];
  anchor?: Anchor;
  opacity?: number;
  /** Round hole (share of the radius), gear teeth and depth, arc opening (degrees). */
  hole?: number;
  teeth?: number;
  teethDepth?: number;
  sweep?: number;
}

export const shape = (name: ShapeName, size: number, o: ShapeOptions = {}): Draft<ShapeMarker> => ({
  type: 'shape',
  shape: name,
  size,
  height: o.height,
  fill: o.fill ?? null,
  stroke: o.stroke ?? null,
  strokeWidth: o.strokeWidth,
  rotation: o.rotation,
  offset: o.offset,
  anchor: o.anchor,
  opacity: o.opacity,
  hole: o.hole,
  teeth: o.teeth,
  teethDepth: o.teethDepth,
  sweep: o.sweep,
});

export const circle = (size: number, o: ShapeOptions = {}): Draft<ShapeMarker> => shape('circle', size, o);

export interface TextOptions {
  font?: TextMarker['font'];
  weight?: TextMarker['weight'];
  italic?: boolean;
  color?: Color;
  offset?: readonly [number, number];
  anchor?: Anchor;
  rotation?: number;
  halo?: TextMarker['halo'];
}

/** Text of letter height `size`; `value` may be an expression ({ expr: "'E=' || Emsal" }). */
export const text = (value: TextMarker['text'], size: number, o: TextOptions = {}): Draft<TextMarker> => ({
  type: 'text',
  text: value,
  size,
  font: o.font ?? 'sans',
  weight: o.weight ?? 700,
  italic: o.italic,
  color: o.color ?? BLACK,
  offset: o.offset,
  anchor: o.anchor,
  rotation: o.rotation,
  halo: o.halo,
});

export const svg = (asset: string, size: number, o: { fill?: Color; stroke?: Color; offset?: readonly [number, number]; rotation?: number } = {}): Draft<SvgMarker> => ({
  type: 'svg',
  asset,
  size,
  fill: o.fill,
  stroke: o.stroke,
  offset: o.offset,
  rotation: o.rotation,
});

export interface FrameOptions {
  /** Frame outline: square (the legends' usual box), a rectangle `w` × `h`, a circle, a double square, or none. */
  frame?: 'square' | 'rect' | 'circle' | 'double' | 'none';
  /** Frame width (and height for squares and circles). */
  size?: number;
  /** Rectangle height. */
  height?: number;
  strokeWidth?: number;
  color?: Color;
  /** Box background (null: see-through, the area shows). */
  background?: Color | null;
  textSize?: number;
  font?: TextMarker['font'];
  weight?: TextMarker['weight'];
  /** Text written under the frame (the legends' caption, e.g. "SÜA"). */
  caption?: string;
  captionSize?: number;
  captionFont?: TextMarker['font'];
  offset?: readonly [number, number];
}

/** A frame with a code inside ("T1", "BHA" …), as the legends' symbols. */
export function framed(code: TextMarker['text'], o: FrameOptions = {}): Draft<MarkerLayer>[] {
  const size = o.size ?? 5;
  const color = o.color ?? BLACK;
  const sw = o.strokeWidth ?? 0.2;
  const off = o.offset ?? [0, 0];
  const out: Draft<MarkerLayer>[] = [];
  const kind = o.frame ?? 'square';
  const bg = o.background ?? null;
  if (kind === 'square') out.push(shape('square', size, { fill: bg, stroke: color, strokeWidth: sw, offset: off }));
  else if (kind === 'rect') out.push(shape('rectangle', size, { height: o.height ?? size * 0.6, fill: bg, stroke: color, strokeWidth: sw, offset: off }));
  else if (kind === 'circle') out.push(circle(size, { fill: bg, stroke: color, strokeWidth: sw, offset: off }));
  else if (kind === 'double') {
    out.push(shape('square', size, { fill: bg, stroke: color, strokeWidth: sw, offset: off }));
    out.push(shape('square', size * 0.8, { stroke: color, strokeWidth: sw, offset: off }));
  }
  if (code !== '') out.push(text(code, o.textSize ?? size * 0.42, { font: o.font, weight: o.weight, color, offset: off }));
  if (o.caption) {
    const cs = o.captionSize ?? size * 0.4;
    const h = kind === 'rect' ? (o.height ?? size * 0.6) : size;
    out.push(text(o.caption, cs, { font: o.captionFont ?? 'serif', weight: 700, color, offset: [off[0], off[1] - h / 2 - cs * 0.85] }));
  }
  return out;
}

// ── Symbols ────────────────────────────────────────────────────────────

const flat = <T>(layers: readonly Many<T>[]): T[] => layers.flatMap((l) => (Array.isArray(l) ? (l as readonly T[]) : [l as T]));

/** Gives every layer (and the layers of nested marker symbols) its id by position. */
function withIds<T extends { id?: string }>(layers: readonly T[]): (T & { id: string })[] {
  return layers.map((l, i) => {
    const nested = l as { marker?: MarkerSymbol };
    const out = { ...l, id: l.id ?? String(i) } as T & { id: string };
    if (nested.marker) (out as { marker?: MarkerSymbol }).marker = { type: 'marker', layers: withIds(nested.marker.layers) };
    return out;
  });
}

export const markerSym = (layers: Many<Draft<MarkerLayer>>): MarkerSymbol => ({ type: 'marker', layers: withIds(flat([layers])) as MarkerLayer[] });

// ── Items ──────────────────────────────────────────────────────────────

export type PlanLevel = 'uip' | 'nip' | 'cdp' | 'msp';

export const ROOT = 'MPYY';

export const PLAN_LEVELS: Record<PlanLevel, { name: string; annex: string; order: number; scale: string }> = {
  uip: { name: 'Uygulama imar planı', annex: 'EK-1d', order: 1, scale: '1/1000' },
  nip: { name: 'Nazım imar planı', annex: 'EK-1ç', order: 2, scale: '1/5000' },
  cdp: { name: 'Çevre düzeni planı', annex: 'EK-1c', order: 3, scale: '1/25000 – 1/100000' },
  msp: { name: 'Mekânsal strateji planı', annex: 'EK-1e', order: 4, scale: '1/250000 ve küçük' },
};

export interface ItemMeta {
  /** Where it comes from: "EK-1d s.3; EK-1e s.57". */
  ref?: string;
  /** Anything a planner should know (placeholder text, an AÇIKLAMA, what is estimated). */
  note?: string;
  tags?: readonly string[];
}

/** One section of a plan level's legend; items are collected in `items` in legend order. */
export interface Sheet {
  readonly items: LibraryItem[];
  readonly category: LibraryCategory;
  area(id: string, name: string, layers: readonly Many<Draft<FillLayer>>[], meta?: ItemMeta): void;
  line(id: string, name: string, layers: readonly Many<Draft<LineLayer>>[], meta?: ItemMeta): void;
  point(id: string, name: string, layers: readonly Many<Draft<MarkerLayer>>[], meta?: ItemMeta): void;
}

/**
 * A legend section: `path` below the plan level (["Sınırlar", "İdari
 * sınırlar"]), `slug` for ids (mpyy.uip.idari.koy-siniri), `order` among
 * the level's sections.
 */
export function sheet(level: PlanLevel, path: readonly string[], slug: string, order = 0): Sheet {
  const items: LibraryItem[] = [];
  const full = [ROOT, PLAN_LEVELS[level].name, ...path];
  const add = (id: string, name: string, symbol: Symbol, meta: ItemMeta) => {
    items.push({ kind: 'symbol', id: `mpyy.${level}.${slug}.${id}`, name, path: full, symbol, description: meta.note, tags: meta.tags, reference: meta.ref });
  };
  return {
    items,
    category: { path: full, order },
    area: (id, name, layers, meta = {}) => add(id, name, { type: 'fill', layers: withIds(flat(layers)) as FillLayer[] }, meta),
    line: (id, name, layers, meta = {}) => add(id, name, { type: 'line', layers: withIds(flat(layers)) as LineLayer[] }, meta),
    point: (id, name, layers, meta = {}) => add(id, name, { type: 'marker', layers: withIds(flat(layers)) as MarkerLayer[] }, meta),
  };
}

/** Categories for the plan levels themselves. */
export const LEVEL_CATEGORIES: readonly LibraryCategory[] = [
  { path: [ROOT], order: 2, description: 'Mekânsal Planlar Yapım Yönetmeliği gösterimleri (EK-1a … EK-1e); ölçüler kâğıt milimetresidir' },
  ...(Object.keys(PLAN_LEVELS) as PlanLevel[]).map((k) => ({ path: [ROOT, PLAN_LEVELS[k].name], order: PLAN_LEVELS[k].order, description: `${PLAN_LEVELS[k].annex}, ${PLAN_LEVELS[k].scale}` })),
];
