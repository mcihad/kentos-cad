import type { Color } from '../../../../model/style';
import { BLACK, PLAN_LEVELS, ROOT, WHITE, circle, dotGrid, framed, label, pattern, rgb, shape, sheet, stipple, svg, text, type FrameOptions, type PlanLevel, type Sheet } from '../dsl';
import { pic, type PictogramName } from '../pictograms';

/**
 * Shared parts of EK-1a, Ortak gösterimler. The common legend is used by
 * every plan level, so each item is made once per level with that level's
 * EK-1e numbers (the sheets under ortak/ loop over `LEVELS`). Symbol sizes
 * follow EK-1a AÇIKLAMA 6: the frame is 5 mm in ÇDP, 7 mm in NİP and
 * 10 mm in UİP; EK-1e gives no other size for them.
 */

export type Level = Exclude<PlanLevel, 'msp'>;
export const LEVELS: readonly Level[] = ['uip', 'nip', 'cdp'];
export const LEVEL_NAME: Record<Level, string> = { uip: 'UİP', nip: 'NİP', cdp: 'ÇDP' };

/** EK-1a AÇIKLAMA 6: "Sembol büyüklükleri: ÇDP 5 mm, NİP 7 mm, UİP 10 mm." */
export const SYMBOL: Record<Level, number> = { uip: 10, nip: 7, cdp: 5 };

export const RED = rgb(255, 0, 0);
/** The planning boundaries' blue (EK-1e RGB 0/92/230). */
export const BLUE = rgb(0, 92, 230);

const TOP = 'Ortak gösterimler';

/**
 * One legend section for every level: `build` fills the sheet of each
 * level. Path is below "Ortak gösterimler"; slugs start with "ortak-".
 */
export function sections(path: readonly string[], slug: string, order: number, build: (s: Sheet, lv: Level) => void, levels: readonly Level[] = LEVELS): Sheet[] {
  return levels.map((lv) => {
    const s = sheet(lv, [TOP, ...path], `ortak-${slug}`, order);
    build(s, lv);
    return s;
  });
}

/**
 * An empty section that only declares a category (its order and
 * description), so "Ortak gösterimler" and its groups keep the legend's
 * order in the library tree.
 */
export function heading(path: readonly string[], order: number, description?: string): Sheet[] {
  return LEVELS.map((lv) => {
    const s = sheet(lv, path.length ? [TOP, ...path] : [TOP], `ortak-h${order}`, order);
    return { ...s, category: { path: [ROOT, PLAN_LEVELS[lv].name, TOP, ...path], order, description } };
  });
}

// ── Point symbols ──────────────────────────────────────────────────────

type Markers = ReturnType<typeof framed>;
type MarkerDraft = Markers[number];
type TextValue = Parameters<typeof framed>[0];

/**
 * Markers at the area's inside point, each in its own layer: markers of
 * the same look are batched across objects, so a paper-filled frame of one
 * symbol could otherwise be drawn over the letters of another.
 */
export const stack = (markers: MarkerDraft | readonly MarkerDraft[]) => (Array.isArray(markers) ? (markers as readonly MarkerDraft[]) : [markers as MarkerDraft]).map((m) => label(m));

/** Frame line: thin in the legend (≈0.3 mm at 10 mm), heavier where the legend draws it so. */
const frameLine = (lv: Level, heavy = false) => SYMBOL[lv] * (heavy ? 0.06 : 0.03);

/** A caption under a frame (the legend's bold Times), with a paper halo over hatches. */
export const caption = (lv: Level, value: TextValue, o: { size?: number; font?: 'serif' | 'sans' | 'narrow' } = {}) => {
  const size = o.size ?? SYMBOL[lv] * 0.38;
  return text(value, size, { font: o.font ?? 'serif', weight: 700, offset: [0, -SYMBOL[lv] / 2 - size * 0.85], halo: { color: WHITE, width: size * 0.1 } });
};

export interface CodeOptions {
  /** Letter height as a share of the frame (default from the letter count). */
  scale?: number;
  font?: FrameOptions['font'];
  weight?: FrameOptions['weight'];
  heavy?: boolean;
  caption?: string;
}

/** Letters in the level's square frame ("TGB", "OSB"), paper behind them. */
export function codeMarks(lv: Level, code: TextValue, o: CodeOptions = {}): Markers {
  const size = SYMBOL[lv];
  const n = typeof code === 'string' ? code.length : 3;
  const scale = o.scale ?? Math.min(0.45, 0.95 / Math.max(n, 1));
  return [
    ...framed(code, { size, strokeWidth: frameLine(lv, o.heavy), textSize: size * scale, font: o.font ?? 'sans', weight: o.weight ?? 900, background: WHITE }),
    ...(o.caption ? [caption(lv, o.caption)] : []),
  ];
}

/** Code frame at the area's inside point. */
export const code = (lv: Level, value: TextValue, o: CodeOptions = {}) => stack(codeMarks(lv, value, o));

export interface PictoOptions {
  /** Pictogram width as a share of the frame (default 0.8). */
  scale?: number;
  caption?: string;
  frame?: boolean;
  heavy?: boolean;
}

/** A pictogram in the level's frame (and its caption under it). */
export function pictoMarks(lv: Level, name: PictogramName, o: PictoOptions = {}): Markers {
  const size = SYMBOL[lv];
  return [
    ...(o.frame === false ? [] : framed('', { size, strokeWidth: frameLine(lv, o.heavy), background: WHITE })),
    svg(pic(name), size * (o.scale ?? 0.8), { fill: BLACK }),
    ...(o.caption ? [caption(lv, o.caption)] : []),
  ];
}

export const picto = (lv: Level, name: PictogramName, o: PictoOptions = {}) => stack(pictoMarks(lv, name, o));

/**
 * The double square with the inner square's diagonals (EK-1a koruma
 * alanları: YSK), code as a caption.
 */
export function crossBoxMarks(lv: Level, cap: string): Markers {
  const size = SYMBOL[lv];
  const inner = size * 0.8;
  return [
    shape('square', size, { fill: WHITE, stroke: BLACK, strokeWidth: frameLine(lv) }),
    shape('square', inner, { stroke: BLACK, strokeWidth: frameLine(lv) }),
    shape('x', inner * Math.SQRT2, { stroke: BLACK, strokeWidth: frameLine(lv) }),
    ...(cap ? [caption(lv, cap)] : []),
  ];
}

// ── Line markers ───────────────────────────────────────────────────────

const r3 = (v: number) => Math.round(v * 1000) / 1000;

/**
 * A cog wheel (TGB, serbest bölge, OSB, endüstri bölgesi): `outer` across
 * the teeth, a round hole of `inner` (none: a solid wheel). The legend
 * draws 12 square teeth about 45% of the rim's depth; EK-1e gives only the
 * two diameters (a solid wheel takes the teeth of a 0.72 hole).
 */
export function gear(outer: number, inner: number | null, color: Color = BLACK): MarkerDraft {
  const hole = inner === null ? 0 : inner / outer;
  const teethDepth = r3(0.45 * (1 - (inner === null ? 0.72 : hole)));
  return { ...shape('gear', outer, { fill: color }), teeth: 12, teethDepth, ...(hole > 0 ? { hole: r3(hole) } : {}) };
}

/** A circle with a plus inside, the plus's arms along and across the line (⊕). */
export const circleCross = (d: number, width: number, color: Color) => [circle(d, { stroke: color, strokeWidth: width }), shape('cross', d, { stroke: color, strokeWidth: width })];

/**
 * An equilateral triangle of side `side` standing with its base on the
 * line, apex to the left of the drawing direction (the inside of areas).
 */
export const triangleOnLine = (side: number, o: { fill?: Color | null; stroke?: Color | null; strokeWidth?: number }) =>
  shape('triangle', side, { fill: o.fill ?? null, stroke: o.stroke ?? null, strokeWidth: o.strokeWidth, offset: [0, r3((side * Math.sqrt(3)) / 6)] });

// ── Fills ──────────────────────────────────────────────────────────────

/** "karolaj merkezlerinde noktalama": dots of the pen width at the centres of a square grid. */
export const gridDots = (spacing: number, dot: number, color: Color = BLACK) => dotGrid(spacing, spacing, dot, color);

/** "serbest noktalama": random dots of the pen width, about one per `cell`² mm. */
export const freeDots = (dot: number, cell = 2.2, color: Color = BLACK, seed = 11) => stipple(cell, dot, color, 0.95, seed);

/**
 * A straight stroke from (x1, y1) to (x2, y2) in a line marker's frame
 * (mm, x along the line, y to its left). A turned marker's offset is in
 * its own frame, so the midpoint is turned back by the stroke's angle.
 */
export function seg(x1: number, y1: number, x2: number, y2: number, width: number, color: Color = BLACK) {
  const a = Math.atan2(y2 - y1, x2 - x1);
  const mx = (x1 + x2) / 2;
  const my = (y1 + y2) / 2;
  const offset: [number, number] = [r3(Math.cos(a) * mx + Math.sin(a) * my), r3(-Math.sin(a) * mx + Math.cos(a) * my)];
  return shape('line', r3(Math.hypot(x2 - x1, y2 - y1)), { stroke: color, strokeWidth: width, rotation: r3((a * 180) / Math.PI), offset });
}

/**
 * "Dik tarama çiftleri" (gar, tersane, havaalanı): vertical strips `width`
 * wide, `gap` apart, bounded by vertical lines, filled inside only with
 * lines at `angle` (counter-clockwise from east) 1 mm apart. The engine
 * cannot clip a hatch to strips, so the strip is one pattern tile: its
 * two edges and three infill strokes (the tile is three infill spacings
 * tall, so the strokes join across tiles).
 */
export function stripHatch(width: number, gap: number, angle: number, line: number, spacing = 1, color: Color = BLACK) {
  const t = Math.abs(Math.cos((angle * Math.PI) / 180));
  const dy = spacing / t;
  const tileW = width + gap;
  const tileH = 3 * dy;
  const cx = width / 2 - tileW / 2;
  const marks = [
    shape('line', r3(tileH), { stroke: color, strokeWidth: line, rotation: 90, offset: [r3(-tileW / 2), 0] }),
    shape('line', r3(tileH), { stroke: color, strokeWidth: line, rotation: 90, offset: [r3(width - tileW / 2), 0] }),
    ...[-1, 0, 1].map((k) => shape('line', r3(width / t), { stroke: color, strokeWidth: line, rotation: angle, offset: [r3(cx), r3(k * dy)] })),
  ];
  return pattern(marks, tileW, r3(tileH));
}
