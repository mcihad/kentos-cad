import type { DrawingFont } from '../../contracts/generated/DrawingFont';
import { pieceText, type BlockPiece } from '../../model/blocks';
import type { CadDocument } from '../../model/document';
import type { TextRun } from '../../model/entities';
import { dimensionMeasure, type DimensionLayout } from '../../model/geom/dimension';
import { fillTemplate } from '../../model/ops/labelText';
import { resolveColor, type CanvasPalette } from '../../render/color';
import { DRAWING_FAMILY, type Face } from '../../render/drawingFaces';
import type { DimensionLook } from '../../model/annotationStyles';
import { DEFAULT_LABELS, DIMENSION_PREFIX, DIMENSION_UNIT, LABEL, LABEL_STRIDE } from '../../viewport/storeRecords';
import type { VecPath } from './mapVectors';

/**
 * A map frame's texts for the PDF (docs/sheet/design.md §9a): what the map
 * frames write over the drawing (viewport/overlay.ts `drawLabels`: text
 * objects, dimension values, leaders' notes, block texts, and the layers'
 * labels thinned where they would overlap), as texts on the ground with
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

/** Where labels already are, in 8 px cells (the map frames' own thinning, viewport/overlay.ts `LabelRoom`). */
class Room {
  private static readonly CELL = 8;
  private readonly cols: number;
  private readonly rows: number;
  private readonly taken: Uint8Array;
  constructor(width: number, height: number) {
    this.cols = Math.max(1, Math.ceil(width / Room.CELL));
    this.rows = Math.max(1, Math.ceil(height / Room.CELL));
    this.taken = new Uint8Array(this.cols * this.rows);
  }
  claim(x0: number, y0: number, x1: number, y1: number): boolean {
    const C = Room.CELL;
    const c0 = Math.max(0, Math.floor(x0 / C));
    const c1 = Math.min(this.cols - 1, Math.floor(x1 / C));
    const r0 = Math.max(0, Math.floor(y0 / C));
    const r1 = Math.min(this.rows - 1, Math.floor(y1 / C));
    if (c0 > c1 || r0 > r1) return true;
    for (let r = r0; r <= r1; r++) for (let c = c0; c <= c1; c++) if (this.taken[r * this.cols + c]) return false;
    for (let r = r0; r <= r1; r++) this.taken.fill(1, r * this.cols + c0, r * this.cols + c1 + 1);
    return true;
  }
}

/** The texts the map frames write, and the masks under text objects (as paths in the paper's colour). */
export function mapTexts(o: LabelInput): { texts: VecText[]; masks: { layer: string; path: VecPath }[] } {
  const { doc, palette: pal, spots, pxPerM } = o;
  const texts: VecText[] = [];
  const masks: { layer: string; path: VecPath }[] = [];
  const room = new Room(o.box.width, o.box.height);
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
  // Ground metres to the paper's CSS px (y down from the map's north-west corner), and back.
  const sx = (x: number) => (x - o.box.minX) * pxPerM;
  const sy = (y: number) => (o.box.maxY - y) * pxPerM;
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
  for (let i = 0; i < spots.length; i += LABEL_STRIDE) {
    const e = doc.get(spots[i]);
    if (!e) continue;
    const layer = e.layerId;
    const what = spots[i + 1];
    const x = spots[i + 2];
    const y = spots[i + 3];
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
    // A layer's label (parcel number, point name …): thinned where it would cover another.
    const st = doc.layers.get(e.layerId)?.style.label ?? DEFAULT_LABELS[e.kind];
    if (!st || !e.label) continue;
    const size = Math.min(st.maxSize ?? st.size, st.size + (st.grow ?? 0) * pxPerM);
    const t = fillTemplate(st.template, e.label);
    const color = ink[st.ink ?? 'label'];
    const weight = st.weight ?? 500;
    const w = o.measure(css(weight, size), t);
    const at = (dx: number, dy: number) => ({ x: x + dx / pxPerM, y: y - dy / pxPerM });
    const font = { family: o.font, weight, italic: false };
    switch (what) {
      case LABEL.center:
        if (room.claim(sx(x) - w / 2, sy(y) - size / 2, sx(x) + w / 2, sy(y) + size / 2)) text(layer, { text: t, x, y, size: size * PX_MM, rotation: 0, align: 'center', baseline: 'middle', font, color });
        break;
      case LABEL.corner:
        if (room.claim(sx(x) + 8, sy(y) + 14 - size / 2, sx(x) + 8 + w, sy(y) + 14 + size / 2)) text(layer, { text: t, ...at(8, 14), size: size * PX_MM, rotation: 0, align: 'left', baseline: 'middle', font, color });
        break;
      case LABEL.beside:
        if (room.claim(sx(x) + 7, sy(y) - 7 - size / 2, sx(x) + 7 + w, sy(y) - 7 + size / 2)) text(layer, { text: t, ...at(7, -7), size: size * PX_MM, rotation: 0, align: 'left', baseline: 'middle', font, color });
        break;
      case LABEL.along: {
        // A third of the way along, kept upright (on the paper, y up).
        const cx = spots[i + 4];
        const cy = spots[i + 5];
        let ang = Math.atan2(cy - y, cx - x);
        if (ang > Math.PI / 2 || ang < -Math.PI / 2) ang += Math.PI;
        const mx = (sx(x) + sx(cx)) / 2;
        const my = (sy(y) + sy(cy)) / 2;
        const hx = (Math.abs(Math.cos(ang)) * w + Math.abs(Math.sin(ang)) * size) / 2;
        const hy = (Math.abs(Math.sin(ang)) * w + Math.abs(Math.cos(ang)) * size) / 2;
        if (room.claim(mx - hx, my - hy, mx + hx, my + hy)) text(layer, { text: t, x: (x + cx) / 2, y: (y + cy) / 2, size: size * PX_MM, rotation: (ang * 180) / Math.PI, align: 'center', baseline: 'middle', font, color });
        break;
      }
    }
  }
  return { texts, masks };
}
