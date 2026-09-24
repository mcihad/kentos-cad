import { BLACK, WHITE, circle, framed, groupedHatch, label, pattern, shape, stipple, svg, text, type FrameOptions } from '../dsl';
import { pic, type PictogramName } from '../pictograms';

/**
 * Shared parts of the NİP (EK-1ç) legend. Lengths are paper mm at 1/5000.
 * Symbols are 7 mm (EK-1a AÇIKLAMA 6: "Nazım İmar Planı için 7 mm"); EK-1e
 * gives no frame or caption sizes for NİP, so frame lines and letter
 * faces follow the EK-1ç drawings: thin frames ≈0.25 mm, heavy ones
 * ≈0.45 mm; Arial Black, Arial bold or Times bold as drawn. Frames are
 * filled with the paper colour and captions have a paper halo, as the
 * legend shows them on white, so codes stay legible over the hatch (the
 * UİP convention).
 */

/** Symbol (frame) size. */
export const S = 7;
/** Caption under a frame (bold Times, font size). */
export const CAPTION = 2.6;

type Markers = ReturnType<typeof framed>;
type MarkerDraft = Markers[number];
type TextValue = Parameters<typeof framed>[0];

/**
 * Markers at the area's inside point, each in its own layer: the markers
 * of one centroid layer share a drawing level and markers of the same look
 * are batched across objects, so a paper-filled frame of one symbol could
 * cover the letters of another; separate layers keep the order.
 */
export const stack = (markers: MarkerDraft | readonly MarkerDraft[]) => (Array.isArray(markers) ? (markers as readonly MarkerDraft[]) : [markers as MarkerDraft]).map((m) => label(m));

/** A caption under a frame of height `frameHeight` (bold Times), with a paper halo over the hatch. */
export const captionMark = (value: TextValue, size = CAPTION, frameHeight = S, font: 'serif' | 'sans' = 'serif', weight: 400 | 700 = 700) =>
  text(value, size, { font, weight, offset: [0, -frameHeight / 2 - size * 0.85], halo: { color: WHITE, width: 0.3 } });

/**
 * A marker's offset is turned with the marker: the offset that puts a
 * marker turned `deg` degrees at (x, y) of the unturned symbol.
 */
export const turned = (x: number, y: number, deg: number): [number, number] => {
  const r = (-deg * Math.PI) / 180;
  return [x * Math.cos(r) - y * Math.sin(r), x * Math.sin(r) + y * Math.cos(r)];
};

/** Advance of a code in em: capitals ≈ 0.72 (Arial/Times bold), narrow ones ≈ 0.3; Arial Black ≈ 18 % wider. */
export const em = (t: string, black = false) => [...t].reduce((s, c) => s + ('Iİ1/().,:'.includes(c) ? 0.3 : c === 'M' ? 0.86 : c === ' ' ? 0.28 : 0.72), 0) * (black ? 1.18 : 1);

export interface BoxOptions {
  font?: 'sans' | 'serif';
  weight?: 400 | 700 | 900;
  /** Heavy frame line (the legend's Arial Black codes). */
  heavy?: boolean;
  frame?: FrameOptions['frame'];
  /** Largest letter size (font size, mm). */
  max?: number;
  caption?: string;
  captionSize?: number;
  size?: number;
}

/** A code in a frame as marker layers, the letters fitted to the frame. */
export function boxMarks(code: string, o: BoxOptions = {}): Markers {
  const size = o.size ?? S;
  const weight = o.weight ?? 700;
  const textSize = Math.min(o.max ?? size * 0.52, (size * 0.78) / em(code, weight >= 900 && o.font !== 'serif'));
  return [
    ...framed(code, { size, frame: o.frame, strokeWidth: o.heavy ? 0.45 : 0.25, font: o.font ?? 'sans', weight, textSize, background: WHITE }),
    ...(o.caption ? [captionMark(o.caption, o.captionSize ?? CAPTION, size)] : []),
  ];
}

/** A code in a frame at the area's inside point. */
export const box = (code: string, o: BoxOptions = {}) => stack(boxMarks(code, o));

export interface PictureOptions {
  size?: number;
  heavy?: boolean;
  captionSize?: number;
}

/** A pictogram in a frame with the legend's caption under it, as marker layers. */
export function pictureMarks(name: PictogramName, caption?: string, o: PictureOptions = {}): Markers {
  const size = o.size ?? S;
  return [
    ...framed('', { size, strokeWidth: o.heavy ? 0.45 : 0.25, background: WHITE }),
    svg(pic(name), size * 0.88, { fill: BLACK }),
    ...(caption ? [captionMark(caption, o.captionSize ?? CAPTION, size)] : []),
  ];
}

/** A pictogram in a frame at the area's inside point. */
export const picture = (name: PictogramName, caption?: string, o: PictureOptions = {}) => stack(pictureMarks(name, caption, o));

/**
 * The afet frame (EK-1ç s.7-8): a thin outer square, a heavier inner
 * square about 1 mm inside with both diagonals, the code as a caption.
 */
export function crossBoxMarks(caption: string, size = S): Markers {
  const inner = size - 1.4;
  return [
    shape('square', size, { fill: WHITE, stroke: BLACK, strokeWidth: 0.2 }),
    shape('square', inner, { stroke: BLACK, strokeWidth: 0.35 }),
    shape('x', inner * Math.SQRT2, { stroke: BLACK, strokeWidth: 0.25 }),
    ...(caption ? [captionMark(caption, CAPTION, size)] : []),
  ];
}

/**
 * A filled toothed disc ("içi dolu dişli"), `d` outside: twelve teeth about
 * a fifth of the radius deep, as the legend's wheel (EK-1ç s.4 measures
 * 12 teeth, 0.22 of the radius deep; its teeth are rounded by the raster).
 */
export const gear = (d: number): MarkerDraft => ({ ...shape('gear', d, { fill: BLACK }), teeth: 12, teethDepth: 0.22 });

/** Hatch lines in pairs 1 mm apart, the pairs `period` apart ("1 mm tarama çiftleri … çiftler arası mesafe"). */
export const pairs = (angle: number, period: number, width = 0.2) => groupedHatch(angle, period, 2, 1, width);

/** "Karolaj merkezlerinde noktalama": dots of the pen width at the centres of a square grid. */
export const gridDots = (spacing: number, dot: number) => pattern(circle(dot, { fill: BLACK }), spacing, spacing);

/**
 * "Serbest noktalama" (park, yeşil alanlar): random dots of the pen width.
 * EK-1e gives no density; the EK-1ç samples show about 20 dots per cm²
 * (one per 2 × 2 mm cell, 80 % of the cells).
 */
export const freeDots = (dot: number, seed = 1) => stipple(2, dot, BLACK, 0.8, seed);

/**
 * A speckled disc of diameter `d` ("… mm çapında serbest noktalama"): the
 * legend's round blob of random specks, one SVG mark (42 dots inside a
 * circle filling 96 % of the drawing's box).
 */
export const speckle = (d: number): MarkerDraft => svg(pic('benekli-daire'), d / 0.96, { fill: BLACK });

/**
 * "Dik tarama çiftleri" (limanlar, iskele, istasyonlar): vertical strips
 * between line pairs `width` apart, strips `gap` apart, filled with
 * parallel lines 1 mm apart at 330° (30° below the horizontal, falling to
 * the right) that run from one line of the pair to the other. All one
 * pattern of 0.2 mm lines so rails and rungs stay aligned: the rungs are
 * short line markers, one per 1 mm band along the strip.
 */
export function strips(width: number, gap: number, line = 0.2) {
  // Rungs 1 mm apart across their direction are 1/cos 30° apart along the vertical rails.
  const pitch = 1 / Math.cos(Math.PI / 6);
  const rung = width / Math.cos(Math.PI / 6);
  // A rung falls width·tan 30° along the strip, more than one band: each band holds one rung that reaches into its neighbours.
  return pattern(
    [
      shape('line', pitch, { stroke: BLACK, strokeWidth: line, rotation: 90, offset: [-width / 2, 0] }),
      shape('line', pitch, { stroke: BLACK, strokeWidth: line, rotation: 90, offset: [width / 2, 0] }),
      shape('line', rung, { stroke: BLACK, strokeWidth: line, rotation: -30 }),
    ],
    width + gap,
    pitch,
  );
}
