import type { DrawingFont } from '../../contracts/generated/DrawingFont';
import { pieceText, type BlockPiece } from '../../model/blocks';
import type { CadDocument } from '../../model/document';
import type { TextRun } from '../../model/entities';
import { dimensionMeasure, type DimensionLayout } from '../../model/geom/dimension';
import { resolveColor, type CanvasPalette } from '../../render/color';
import { DRAWING_FAMILY, type Face } from '../../render/drawingFaces';
import type { DimensionLook } from '../../model/annotationStyles';
import type { LabelStyle } from '../../contracts/generated/LabelStyle';
import { LABEL_STATE, labelStyleOf } from '../../viewport/placedLabels';
import { DIMENSION_PREFIX, DIMENSION_UNIT, LABEL, LABEL_STRIDE } from '../../viewport/storeRecords';
import type { VecPath } from './mapVectors';

/**
 * A map frame's texts for the PDF (docs/sheet/design.md §9a): what the map
 * frames write over the drawing (viewport/overlay.ts `drawLabels`: text
 * objects, dimension values, leaders' notes, block texts, and the objects'
 * labels as the label engine placed them, docs/adr/0212), as texts on the ground with
 * their sizes on the paper. A label's size is CSS pixels on the paper (as on
 * the screen at 100 %): 25.4/96 mm each; the map frames' pictures draw them
 * the same size, so a vector map and its picture agree.
 */

export interface VecText {
  readonly text: string;
  /** Where it is anchored on the ground (x east, y north, metres). */
  readonly x: number;
  readonly y: number;
  /** Font size in paper mm. */
  readonly size: number;
  /** Degrees, counter-clockwise on the paper. */
  readonly rotation: number;
  readonly align: 'left' | 'center';
  readonly baseline: 'alphabetic' | 'middle';
  /** A drawing typeface (`DRAWING_FONTS` id), its weight and whether it leans. */
  readonly font: { readonly family: DrawingFont; readonly weight: number; readonly italic: boolean };
  readonly color: string;
  /** The halo round its letters (the paper's colour), paper mm wide. */
  readonly halo: { readonly color: string; readonly width: number } | null;
  /** The letters' width factor (a text object's). */
  readonly widthFactor: number;
  /** The drawing layer it belongs to. */
  readonly layer: string;
}

export interface LabelInput {
  readonly doc: CadDocument;
  readonly palette: CanvasPalette;
  readonly font: DrawingFont;
  /** The label records of the map's box (the geometry store's `labels`, at `pxPerM`). */
  readonly spots: Float64Array;
  /** The texts the label records name (`ShownLabels.texts`). */
  readonly texts: readonly string[];
  /** CSS px per ground metre on the paper: 96 px per inch over the scale. */
  readonly pxPerM: number;
  /** The map's box on the ground (its west and north edges, and its size in CSS px) for the thinning. */
  readonly box: { readonly minX: number; readonly maxY: number; readonly width: number; readonly height: number };
  /** A dimension's value as drawn, in its look's writing (docs/adr/0183 §3). */
  dimensionText(l: Pick<DimensionLayout, 'prefix' | 'unit' | 'value'>, look: DimensionLook): string;
  pieces(block: string): readonly BlockPiece[] | null;
  /** A text's width in CSS px in a canvas font (`500 10.0px Barlow, …`). */
  measure(font: string, text: string): number;
}

/** CSS px on the paper in mm. */
export const PX_MM = 25.4 / 96;
/** The halo's width as the map frames stroke it: 3 px. */
const HALO_PX = 3;

/** The texts the map frames write, and the masks under text objects (as paths in the paper's colour). */
export function mapTexts(o: LabelInput): { texts: VecText[]; masks: { layer: string; path: VecPath }[] } {
  const { doc, palette: pal, spots, pxPerM } = o;
  const texts: VecText[] = [];
  const masks: { layer: string; path: VecPath }[] = [];
  const ink = { fg: pal.fg, 'fg-dim': pal.fgDim, label: pal.label } as const;
  const halo = { color: pal.labelHalo, width: HALO_PX * PX_MM };
  const css = (weight: number, px: number, italic = false, family: DrawingFont = o.font) =>
    `${italic ? 'italic ' : ''}${weight} ${px.toFixed(1)}px ${family === o.font ? pal.drawingFont : DRAWING_FAMILY[family]}`;
  /**
   * A text's typeface, weight and slant (docs/adr/0183 §2): its own typeface upright at 400 (600 bold, italic), the
   * run's bold and italic added; without one the project's, `legacy` (italic for a one-line text, as always).
   */
  const faceOf = (f: Face, legacy: { weight: number; italic: boolean }, run?: TextRun) =>
    f.font === undefined
      ? { family: o.font, weight: run?.bold ? 600 : legacy.weight, italic: legacy.italic || run?.italic === true }
      : { family: f.font, weight: f.bold || run?.bold ? 600 : 400, italic: f.italic === true || run?.italic === true };
  const mm = (groundM: number) => (groundM * pxPerM) * PX_MM;
  const text = (layer: string, t: Omit<VecText, 'layer' | 'halo' | 'widthFactor'> & { widthFactor?: number; halo?: VecText['halo'] }) => texts.push({ layer, halo, widthFactor: 1, ...t });
  /** A text's mask (docs/adr/0145) from its baseline start, `w` ground metres along, `h` high, turned `deg`. */
  const mask = (layer: string, x: number, y: number, deg: number, w: number, h: number, x0 = 0, along = false) => {
    if (!(w > 0)) return;
    const m = h * 0.1;
    const v = along ? 0 : m;
    const a = (deg * Math.PI) / 180;
    const c = Math.cos(a);
    const s = Math.sin(a);
    // The box in the text's frame (y up): from below the baseline to above the letters.
    const corners = [
      [x0 - m, -0.23 * h - v],
      [x0 + w + m, -0.23 * h - v],
      [x0 + w + m, 1.15 * h + v],
      [x0 - m, 1.15 * h + v],
    ];
    masks.push({ layer, path: { parts: [{ points: corners.flatMap(([u, t]) => [x + u * c - t * s, y + u * s + t * c]), closed: true }], fill: { color: pal.paper, opacity: 1, rule: 'nonzero' } } });
  };
  /** A box turned `deg` from (x, y), its corners `[along, up]` ground metres in its frame, filled with `color`. */
  const fill = (layer: string, x: number, y: number, deg: number, corners: readonly (readonly [number, number])[], color: string) => {
    const a = (deg * Math.PI) / 180;
    const [c, s] = [Math.cos(a), Math.sin(a)];
    masks.push({ layer, path: { parts: [{ points: corners.flatMap(([u, v]) => [x + u * c - v * s, y + u * s + v * c]), closed: true }], fill: { color, opacity: 1, rule: 'nonzero' } } });
  };
  /** One line of a multi-line text (docs/adr/0182 §3), run by run as the overlay draws it (`paragraphLine`). */
  const line = (layer: string, words: string, runs: readonly TextRun[] | undefined, [start, end]: readonly [number, number], x: number, y: number, deg: number, h: number, factor: number, face: Face = {}) => {
    const letters = Array.from(words);
    const a = (deg * Math.PI) / 180;
    const [c, s] = [Math.cos(a), Math.sin(a)];
    const runAt = (k: number) => runs?.find((r) => r.start <= k && k < r.end);
    const last = Math.min(end, letters.length);
    let along = 0;
    for (let k = start; k < last; ) {
      const run = runAt(k);
      let j = k + 1;
      while (j < last && runAt(j) === run) j++;
      const part = letters.slice(k, j).join('');
      const size = run?.script ? h * 0.6 : h;
      const lift = run?.script === 'super' ? h * 0.4 : run?.script === 'sub' ? -h * 0.15 : 0;
      const font = faceOf(face, { weight: 400, italic: false }, run);
      const w = (o.measure(css(font.weight, size * pxPerM, font.italic, font.family), part) / pxPerM) * factor;
      const color = run?.color ? resolveColor(run.color, pal) : pal.label;
      if (part.trim()) text(layer, { text: part, x: x + c * along - s * lift, y: y + s * along + c * lift, size: mm(size), rotation: deg, align: 'left', baseline: 'alphabetic', font, color, widthFactor: factor });
      if (run?.underline) fill(layer, x, y, deg, [[along, lift - 0.12 * size], [along + w, lift - 0.12 * size], [along + w, lift - 0.18 * size], [along, lift - 0.18 * size]], color);
      along += w;
      k = j;
    }
  };
  /**
   * An object's label as the label engine placed it (docs/adr/0212 §3.8): its backdrop (its background, or a contour's
   * mask), its lines or letters with its halo, its callout; records `from..to`.
   */
  const placed = (layer: string, st: LabelStyle, state: number, from: number, to: number) => {
    const color = st.color ? resolveColor(st.color, pal) : ink[st.ink ?? 'label'];
    const font = { family: o.font, weight: st.weight ?? 500, italic: st.italic === true };
    const own = st.halo ? (st.halo.width > 0 ? { color: st.halo.color ? resolveColor(st.halo.color, pal) : pal.labelHalo, width: st.halo.width * 2 * PX_MM } : null) : halo;
    const masked = (state & LABEL_STATE.masked) !== 0;
    const bg = st.background ?? (masked ? { shape: 'rect' as const, fill: 'paper', padding: 1 } : undefined);
    const pad = bg ? (bg.padding ?? 2) / pxPerM : 0;
    const box = (cx: number, cy: number, deg: number, w: number, h: number) => {
      if (!bg?.fill) return;
      const [hw, hh] = [w / 2 / pxPerM + pad, h / 2 / pxPerM + pad];
      fill(layer, cx, cy, deg, [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]], resolveColor(bg.fill, pal));
    };
    if (!(state & LABEL_STATE.curved)) box(spots[from + 2], spots[from + 3], spots[from + 4], spots[from + 5], spots[from + 6]);
    const letters = new Map<number, string[]>();
    for (let k = from + LABEL_STRIDE; k < to; k += LABEL_STRIDE) {
      const what = spots[k + 1];
      const [x, y, deg, px] = [spots[k + 2], spots[k + 3], spots[k + 4], spots[k + 5]];
      if (what === LABEL.placedCallout) {
        const width = (st.callout?.width ?? 1) * PX_MM;
        const c = st.callout?.color ? resolveColor(st.callout.color, pal) : color;
        const pts = st.callout?.kind === 'manhattan' ? [x, y, spots[k + 4], y, spots[k + 4], spots[k + 5]] : [x, y, spots[k + 4], spots[k + 5]];
        masks.push({ layer, path: { parts: [{ points: pts, closed: false }], stroke: { color: c, opacity: 1, width, dash: null, dashOffset: 0, cap: 'butt', join: 'miter' } } });
        continue;
      }
      let t = '';
      if (what === LABEL.placedLine) t = o.texts[spots[k + 6]] ?? '';
      else if (what === LABEL.placedLetter) {
        const n = spots[k + 6];
        let list = letters.get(n);
        if (!list) letters.set(n, (list = Array.from(o.texts[n] ?? '')));
        t = list[spots[k + 7]] ?? '';
        box(x, y, deg, spots[k + 8], px);
      }
      if (t.trim()) text(layer, { text: t, x, y, size: px * PX_MM, rotation: deg, align: 'center', baseline: 'middle', font, color, halo: bg?.fill ? null : own });
    }
  };
  for (let i = 0; i < spots.length; i += LABEL_STRIDE) {
    const e = doc.get(spots[i]);
    if (!e) continue;
    const layer = e.layerId;
    const what = spots[i + 1];
    const x = spots[i + 2];
    const y = spots[i + 3];
    if (what === LABEL.placed) {
      let end = i + LABEL_STRIDE;
      while (end < spots.length && spots[end] === spots[i] && spots[end + 1] > LABEL.placed && spots[end + 1] <= LABEL.placedCallout) end += LABEL_STRIDE;
      const st = labelStyleOf(doc, e, spots[i + 7]);
      const state = spots[i + 8];
      if (st && !(state & (LABEL_STATE.unplaced | LABEL_STATE.hidden))) placed(layer, st, state, i, end);
      i = end - LABEL_STRIDE;
      continue;
    }
    // A multi-line text: its mask, then its lines (docs/adr/0182 §3); a block's own too.
    if (what === LABEL.paragraphMask) {
      const [w, h] = [spots[i + 5], spots[i + 6]];
      fill(layer, x, y, spots[i + 4], [[0, 0], [w, 0], [w, h], [0, h]], pal.paper);
      continue;
    }
    if (what === LABEL.line && e.kind === 'text') {
      line(layer, e.text, e.runs, [spots[i + 7], spots[i + 8]], x, y, spots[i + 4], spots[i + 5], spots[i + 6], e);
      continue;
    }
    if (what === LABEL.pieceLine && e.kind === 'insert') {
      const piece = o.pieces(e.block)?.[spots[i + 6]];
      if (piece?.kind === 'text') line(layer, piece.text, piece.runs, [spots[i + 7], spots[i + 8]], x, y, spots[i + 4], spots[i + 5], piece.widthFactor ?? 1, piece);
      continue;
    }
    // A table's cell (docs/adr/0184 §2): its words in the table's face and colour, a heading row's bold.
    if (what === LABEL.cell && e.kind === 'table') {
      const words = e.cells[spots[i + 5]]?.[spots[i + 6]];
      if (!words) continue;
      const color = e.color ?? doc.layers.get(e.layerId)?.style.color;
      const font = faceOf(e, { weight: 400, italic: false }, spots[i + 7] === 1 ? { start: 0, end: 0, bold: true } : undefined);
      text(layer, { text: words, x, y, size: mm(e.height), rotation: spots[i + 4], align: 'left', baseline: 'alphabetic', font, color: !color || color === 'fg' || color === 'fg-dim' ? pal.label : resolveColor(color, pal) });
      continue;
    }
    if (what === LABEL.dimension && e.kind === 'dimension') {
      const color = e.color ?? doc.layers.get(e.layerId)?.style.color;
      const measured = { value: spots[i + 5], unit: DIMENSION_UNIT[spots[i + 6]] ?? 'length', prefix: DIMENSION_PREFIX[spots[i + 7]] ?? '' };
      const t = e.text || o.dimensionText(measured, e);
      const family = e.font ?? o.font;
      if (spots[i + 8] === 1) {
        const w = o.measure(css(500, e.height * pxPerM, false, family), t) / pxPerM;
        mask(layer, x, y, spots[i + 4], w, e.height, -w / 2, true);
      }
      text(layer, { text: t, x, y, size: mm(e.height), rotation: spots[i + 4], align: 'center', baseline: 'alphabetic', font: { family, weight: 500, italic: false }, color: !color || color === 'fg' || color === 'fg-dim' ? pal.label : resolveColor(color, pal) });
      continue;
    }
    if ((what === LABEL.text && e.kind === 'text') || (what === LABEL.leader && e.kind === 'leader' && e.text)) {
      const factor = what === LABEL.text ? spots[i + 5] : 1;
      mask(layer, x, y, spots[i + 4], spots[i + 6], e.height);
      const font = faceOf(e.kind === 'text' ? e : {}, { weight: 400, italic: true });
      text(layer, { text: e.text ?? '', x, y, size: mm(e.height), rotation: spots[i + 4], align: 'left', baseline: 'alphabetic', font, color: pal.label, widthFactor: factor });
      continue;
    }
    if ((what === LABEL.pieceText || what === LABEL.pieceDimension || what === LABEL.pieceLeader) && e.kind === 'insert') {
      const piece = o.pieces(e.block)?.[spots[i + 6]];
      if (what === LABEL.pieceText && piece?.kind === 'text') {
        const t = pieceText(piece, e.attrs);
        if (!t) continue;
        mask(layer, x, y, spots[i + 4], spots[i + 8], spots[i + 5]);
        text(layer, { text: t, x, y, size: mm(spots[i + 5]), rotation: spots[i + 4], align: 'left', baseline: 'alphabetic', font: faceOf(piece, { weight: 400, italic: true }), color: pal.label, widthFactor: spots[i + 7] });
      } else if (what === LABEL.pieceLeader && piece?.kind === 'leader' && piece.text) {
        mask(layer, x, y, spots[i + 4], spots[i + 8], spots[i + 5]);
        text(layer, { text: piece.text, x, y, size: mm(spots[i + 5]), rotation: spots[i + 4], align: 'left', baseline: 'alphabetic', font: { family: o.font, weight: 400, italic: true }, color: pal.label });
      } else if (what === LABEL.pieceDimension && piece?.kind === 'dimension') {
        const color = e.color ?? doc.layers.get(e.layerId)?.style.color;
        const t = piece.text || o.dimensionText({ value: spots[i + 5], ...dimensionMeasure(piece.style, piece.angle) }, piece);
        const family = piece.font ?? o.font;
        if (spots[i + 8] === 1) {
          const w = o.measure(css(500, spots[i + 7] * pxPerM, false, family), t) / pxPerM;
          mask(layer, x, y, spots[i + 4], w, spots[i + 7], -w / 2, true);
        }
        text(layer, { text: t, x, y, size: mm(spots[i + 7]), rotation: spots[i + 4], align: 'center', baseline: 'alphabetic', font: { family, weight: 500, italic: false }, color: !color || color === 'fg' || color === 'fg-dim' ? pal.label : resolveColor(color, pal) });
      }
      continue;
    }
  }
  return { texts, masks };
}
