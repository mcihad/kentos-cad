import type { Color } from '../../../../model/style';
import { BLACK, WHITE, along, dotGrid, framed, hatch, label, pattern, rgb, shape, stipple, stroke, svg, text, type FrameOptions } from '../dsl';
import { pic, type PictogramName } from '../pictograms';

/**
 * Shared parts of the UİP (EK-1d) legend. Lengths are paper mm at 1/1000.
 * EK-1e gives no size for the legend's frames and captions; they are
 * measured from EK-1d (printed about 1:1: the 4 mm ticaret grid measures
 * 4.2 mm): code frames are 10 mm squares with a thin (≈0.3 mm) line,
 * captions under them bold Times of about 2.4 mm capital height (3.8 mm
 * font size). Frames
 * are filled with the paper colour, as the legend shows them on white, so
 * the code stays legible over the area's hatch. "Yazı
 * boyutu" is the font size (em): the 5 mm text of the yapı düzeni circles
 * measures 3.4–3.6 mm capital height, Arial's 0.72 em.
 */

export const RED = rgb(255, 0, 0);
/** Koruma kuşakları and the other hazard belts. */
export const BELT_FILL = rgb(245, 122, 122);

/** Code frame side and line. */
export const FRAME = 10;
export const FRAME_LINE = 0.3;
/** Caption under a frame (bold Times). */
export const CAPTION = 3.8;

type Markers = ReturnType<typeof framed>;
type TextValue = Parameters<typeof framed>[0];

/**
 * A caption under a frame of height `frameHeight` (bold Times by default).
 * It has a paper halo: on the map it sits on the area's hatch, where the
 * legend has white paper.
 */
export const captionMark = (value: TextValue, size = CAPTION, frameHeight = FRAME, font: 'serif' | 'sans' = 'serif') =>
  text(value, size, { font, weight: 700, offset: [0, -frameHeight / 2 - size * 0.85], halo: { color: WHITE, width: 0.4 } });

/** A code in a 10 mm frame, as marker layers (point items); `caption` goes under it. */
export function codeMarks(code: TextValue, textSize = 5, o: FrameOptions = {}): Markers {
  const { caption, captionSize, ...rest } = o;
  const h = rest.frame === 'rect' ? (rest.height ?? (rest.size ?? FRAME) * 0.6) : (rest.size ?? FRAME);
  return [
    ...framed(code, { size: FRAME, strokeWidth: FRAME_LINE, textSize, font: 'sans', weight: 700, background: WHITE, ...rest }),
    ...(caption ? [captionMark(caption, captionSize ?? CAPTION, h)] : []),
  ];
}

/** A code in a 10 mm frame at the area's inside point. */
export const code = (code: TextValue, textSize = 5, o: FrameOptions = {}) => label(codeMarks(code, textSize, o));

export interface PictoOptions {
  /** Pictogram width (default 0.8 of the frame). */
  size?: number;
  caption?: string;
  captionSize?: number;
  captionFont?: 'serif' | 'sans';
  frame?: FrameOptions['frame'];
  /** Frame width (height too unless `height`). */
  frameSize?: number;
  height?: number;
  strokeWidth?: number;
  offset?: readonly [number, number];
}

/** A pictogram in a frame (and its caption), as marker layers. */
export function pictoMarks(name: PictogramName, o: PictoOptions = {}): Markers {
  const size = o.frameSize ?? FRAME;
  const h = o.frame === 'rect' ? (o.height ?? size * 0.6) : size;
  return [
    ...framed('', { size, height: o.height, frame: o.frame ?? 'square', strokeWidth: o.strokeWidth ?? FRAME_LINE, background: WHITE }),
    svg(pic(name), o.size ?? size * 0.8, { fill: BLACK, offset: o.offset }),
    ...(o.caption ? [captionMark(o.caption, o.captionSize ?? CAPTION, h, o.captionFont)] : []),
  ];
}

/** A pictogram in a frame at the area's inside point. */
export const picto = (name: PictogramName, o: PictoOptions = {}) => label(pictoMarks(name, o));

/**
 * The double square with a cross (koruma kuşakları, SEG, tescilli bina;
 * EK-1d s.6–9): a 10 mm outer square, an inner square 1 mm inside with its
 * two diagonals, and the code as a caption. Koruma kuşakları draw the outer
 * line thin and the inner one heavier; tescilli bina the other way round.
 */
export function crossBoxMarks(caption: TextValue, o: { outer?: number; inner?: number; captionFont?: 'serif' | 'sans'; captionSize?: number } = {}): Markers {
  const cs = o.captionSize ?? CAPTION;
  return [
    shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: o.outer ?? 0.2 }),
    shape('square', FRAME - 2, { stroke: BLACK, strokeWidth: o.inner ?? 0.4 }),
    shape('x', (FRAME - 2) * Math.SQRT2, { stroke: BLACK, strokeWidth: 0.25 }),
    ...(caption ? [captionMark(caption, cs, FRAME, o.captionFont)] : []),
  ];
}

/**
 * "karolaj merkezlerinde noktalama": dots of the pen width at the centres
 * of a square grid.
 */
export const gridDots = (spacing: number, dot: number) => dotGrid(spacing, spacing, dot, BLACK);

/**
 * "Serbest noktalama" (park, yeşil alanlar): random dots of the pen width.
 * EK-1e gives no density; the legend shows about 20 dots per cm² (one per
 * 2.2 mm cell).
 */
export const freeDots = (dot = 0.4, seed = 7) => stipple(2.2, dot, BLACK, 0.95, seed);

/**
 * "6 mm çapında serbest noktalama 18 mm karolaj merkezlerinde (şaşırtmalı
 * sıra)" (turizm): a 6 mm blob of random speckles, 18 mm apart along a row,
 * rows staggered by half. Both annexes draw the rows half the row spacing
 * apart (9 mm), a diamond lattice. The blob is the legend's own bitmap
 * speckle, traced as the "benek" pictogram (its drawing spans 88 of the
 * 100 units, hence the marker size).
 */
export function speckleBlobs(diameter = 6, spacing = 18): ReturnType<typeof pattern> {
  return pattern(svg(pic('benek'), round(diameter / 0.88), { fill: BLACK }), spacing, spacing / 2, { stagger: true });
}

const round = (v: number) => Math.round(v * 1000) / 1000;

/**
 * Koruma kuşağı boundary (EK-1e s.110–116): 0.3 mm line, 7 mm dashes and a
 * 3 mm "çarpı" (strokes 3 mm long, 2.12 mm across) centred in each gap; the
 * gap (4.4 mm) is measured from the ≈1:1 drawing. The belt's code "is
 * written on the boundary at suitable intervals": every fourth dash, black,
 * standing on the line on the inner side (as SEG in EK-1d s.7). The text is
 * moved inward by its own offset, not the line's: an inset ring is shorter,
 * so places along it would drift off the dashes.
 */
export function beltEdge(codeText: string, color = RED, gap = 4.4): ReturnType<typeof stroke | typeof along>[] {
  const period = 7 + gap;
  const out: ReturnType<typeof stroke | typeof along>[] = [
    stroke(color, 0.3, { dash: [7, gap] }),
    along(shape('x', 3, { stroke: color, strokeWidth: 0.3 }), period, { offsetAlong: 7 + gap / 2 }),
  ];
  if (codeText) out.push(along(text(codeText, 2.5, { font: 'sans', weight: 700, offset: [0, 1.25] }), period * 4, { offsetAlong: 3.5 + period }));
  return out;
}

/**
 * The legend's "(………)" after a use name: the value of `field` in capitals
 * and parentheses under the frame, only when the field is filled.
 */
export const parenBelow = (field: string, frameHeight = FRAME) =>
  text({ expr: `eğer(boş([${field}]), '', '(' || büyük([${field}]) || ')')` }, 3, { font: 'sans', weight: 700, offset: [0, -frameHeight / 2 - 2.6], halo: { color: WHITE, width: 0.4 } });

/**
 * A straight stroke from (x1, y1) to (x2, y2) in a symbol's frame (mm):
 * a turned `line` marker. A turned marker's offset is in its own frame, so
 * the midpoint is turned back by the stroke's angle.
 */
export function seg(x1: number, y1: number, x2: number, y2: number, width: number, color: Color = BLACK) {
  const a = Math.atan2(y2 - y1, x2 - x1);
  const mx = (x1 + x2) / 2;
  const my = (y1 + y2) / 2;
  const r3 = (v: number) => Math.round(v * 1000) / 1000;
  const offset: [number, number] = [r3(Math.cos(a) * mx + Math.sin(a) * my), r3(-Math.sin(a) * mx + Math.cos(a) * my)];
  return shape('line', r3(Math.hypot(x2 - x1, y2 - y1)), { stroke: color, strokeWidth: width, rotation: r3((a * 180) / Math.PI), offset });
}

/**
 * Staggered dashed lines ("kesik şaşırtmalı çizgiler"): lines `spacing`
 * apart, every other line's dashes shifted by half a period.
 */
export const staggeredDashes = (angle: number, spacing: number, dash: number, gap: number, width: number, color: Color = BLACK) => [
  hatch(angle, spacing * 2, width, color, { dash: [dash, gap] }),
  hatch(angle, spacing * 2, width, color, { dash: [dash, gap], offset: spacing, dashOffset: (dash + gap) / 2 }),
];

/**
 * A length in metres (an expression over the object's fields) as paper mm
 * at the drawing's scale, for data-defined offsets and sizes: things drawn
 * at their real width (roads, canals) keep it at any plot scale.
 */
export const metresToMm = (metres: string) => `(${metres}) * 1000 / $ölçek`;

/**
 * A line `halfWidth` metres (an expression) to the left (`side` 1) or the
 * right (−1) of the drawn axis; `fallback` is the half width in metres
 * shown when the expression gives nothing.
 */
export function sideLine(color: Color, width: number, halfWidth: string, side: 1 | -1, fallback: number, o: Parameters<typeof stroke>[2] = {}) {
  return { ...stroke(color, width, { cap: 'butt', ...o }), offset: { expr: `${side < 0 ? '-' : ''}${metresToMm(halfWidth)}`, fallback: side * fallback } };
}

/** Two lines of `width` mm, `fullWidth` metres apart (an expression), centred on the axis. */
export const sidePair = (color: Color, width: number, fullWidth: string, fallback: number, o: Parameters<typeof stroke>[2] = {}) => [
  sideLine(color, width, `(${fullWidth}) / 2`, 1, fallback / 2, o),
  sideLine(color, width, `(${fullWidth}) / 2`, -1, fallback / 2, o),
];
