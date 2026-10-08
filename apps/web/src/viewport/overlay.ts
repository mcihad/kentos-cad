import type { CrosshairSize } from '../app/state';
import { pieceText, type BlockPiece } from '../model/blocks';
import type { CadDocument } from '../model/document';
import { polygonRing, type TextRun } from '../model/entities';
import type { DimensionLook } from '../model/annotationStyles';
import type { DrawingFont } from '../model/projectSettings';
import { dist, type Vec2 } from '../model/geometry';
import { dimensionMeasure, type DimensionLayout } from '../model/geom/dimension';
import { fillTemplate } from '../model/ops/labelText';
import type { CoreEdge } from '../model/ops/topologyRules';
import type { ProblemMark } from '../model/selection';
import { resolveColor, type CanvasPalette } from '../render/color';
import { faceFont, leanOf, valueFont, type Face } from '../render/drawingFaces';
import type { ToolCursor } from '../tools/Tool';
import type { Camera, ViewTransform } from './Camera';
import { leaderLayout } from '../model/geom/leader';
import type { TrackHit } from './objectTracking';
import { SNAP_LABEL, type SnapHit } from './picking';
import { DEFAULT_LABELS, DIMENSION_PREFIX, DIMENSION_UNIT, LABEL, LABEL_STRIDE, type GripSet } from './storeRecords';

// Text that is part of the drawing (text objects, dimension values, labels) is drawn in the project's typeface
// (CanvasPalette.drawingFont), whatever the interface's is; the overlay's own marks (snap names, scale bar,
// north arrow) use the interface typeface (CanvasPalette.font).

/** Screen-space annotation layer drawn with Canvas2D above the GPU canvas. */


function haloText(g: CanvasRenderingContext2D, text: string, x: number, y: number, fill: string, halo: string): void {
  g.lineJoin = 'round';
  g.lineWidth = 3;
  g.strokeStyle = halo;
  g.strokeText(text, x, y);
  g.fillStyle = fill;
  g.fillText(text, x, y);
}

/**
 * Widths of label texts in the font last set on the context, kept between frames: a pan measures each
 * text once, not at every frame. Forgets everything when it grows large or the font changes.
 */
const widths = new Map<string, number>();
let widthsFont = '';
function textWidth(g: CanvasRenderingContext2D, text: string): number {
  if (g.font !== widthsFont || widths.size > 20000) {
    widths.clear();
    widthsFont = g.font;
  }
  let w = widths.get(text);
  if (w === undefined) widths.set(text, (w = g.measureText(text).width));
  return w;
}

/**
 * Where labels already are on screen, in 8 px cells: a label that would
 * cover one is not drawn. Coarse on purpose (a cell or two of slack is
 * spacing between labels); the text's halo is inside it.
 */
class LabelRoom {
  private static readonly CELL = 8;
  private readonly cols: number;
  private readonly rows: number;
  private readonly taken: Uint8Array;

  constructor(width: number, height: number) {
    this.cols = Math.max(1, Math.ceil(width / LabelRoom.CELL));
    this.rows = Math.max(1, Math.ceil(height / LabelRoom.CELL));
    this.taken = new Uint8Array(this.cols * this.rows);
  }

  /** Takes the box if nothing is there yet; false (and nothing taken) when a label is in the way. */
  claim(x0: number, y0: number, x1: number, y1: number): boolean {
    const C = LabelRoom.CELL;
    const c0 = Math.max(0, Math.floor(x0 / C));
    const c1 = Math.min(this.cols - 1, Math.floor(x1 / C));
    const r0 = Math.max(0, Math.floor(y0 / C));
    const r1 = Math.min(this.rows - 1, Math.floor(y1 / C));
    // Wholly off screen: nothing to keep apart from (the store sends only labels near the view).
    if (c0 > c1 || r0 > r1) return true;
    for (let r = r0; r <= r1; r++) for (let c = c0; c <= c1; c++) if (this.taken[r * this.cols + c]) return false;
    for (let r = r0; r <= r1; r++) this.taken.fill(1, r * this.cols + c0, r * this.cols + c1 + 1);
    return true;
  }
}

/**
 * A text's mask (docs/adr/0145), in the text's frame on screen (the baseline's start at 0, 0): its box a tenth of
 * its height wider all round, 1.15 of the height over the baseline and 0.23 under (the store's `TextPlace::mask`),
 * `width` pixels along, in the sheet's colour. Nothing when `width` is 0 (no mask).
 */
/**
 * A text from where its baseline starts, at `s` on the screen: turned `rotation` degrees, `px` high, its letters
 * `factor` wide, over a mask `mask` px wide (none at 0; docs/adr/0145). Text objects, block texts, leaders' notes.
 */
function baselineText(g: CanvasRenderingContext2D, pal: CanvasPalette, s: Vec2, rotation: number, px: number, factor: number, mask: number, text: string, face: Face = {}): void {
  g.save();
  g.translate(s.x, s.y);
  g.rotate((-rotation * Math.PI) / 180);
  // A slant leans the mask and the letters from the baseline (docs/adr/0183 §2).
  const lean = leanOf(face);
  if (lean) g.transform(1, 0, -lean, 1, 0, 0);
  maskText(g, mask, px, pal.paper);
  if (factor !== 1) g.scale(factor, 1);
  g.font = faceFont(face, px, pal.drawingFont, 'italic 400');
  g.textAlign = 'left';
  g.textBaseline = 'alphabetic';
  haloText(g, text, 0, 0, pal.label, pal.labelHalo);
  g.restore();
}

/**
 * A table's cell (docs/adr/0184 §2): its words from where their baseline starts, upright in the table's face and colour,
 * bold when it is a heading row's; no mask, no halo but the drawing's own.
 */
function tableCell(g: CanvasRenderingContext2D, pal: CanvasPalette, s: Vec2, rotation: number, px: number, words: string, face: Face, bold: boolean, color: string): void {
  g.save();
  g.translate(s.x, s.y);
  g.rotate((-rotation * Math.PI) / 180);
  const lean = leanOf(face);
  if (lean) g.transform(1, 0, -lean, 1, 0, 0);
  g.font = faceFont(face, px, pal.drawingFont, bold ? '600' : '400', { bold });
  g.textAlign = 'left';
  g.textBaseline = 'alphabetic';
  haloText(g, words, 0, 0, color, pal.labelHalo);
  g.restore();
}

/**
 * A text's mask from `x0` (where it starts) `width` along, a tenth of its height wider all round (docs/adr/0145); a
 * dimension's value's (`along`) wider on its sides only, so that it leaves its dimension line and an arc length's
 * symbol, 0.12h off its box, in view (docs/adr/0147).
 */
function maskText(g: CanvasRenderingContext2D, width: number, px: number, paper: string, x0 = 0, along = false): void {
  if (!(width > 0)) return;
  const m = px * 0.1;
  const v = along ? 0 : m;
  g.save();
  g.fillStyle = paper;
  g.fillRect(x0 - m, -px * 1.15 - v, width + 2 * m, px * 1.38 + 2 * v);
  g.restore();
}

/**
 * A dimension's value centred on its baseline (the context turned to it), over its mask when it has one or stands on
 * its line (docs/adr/0147, 0183 §3); in its own typeface when it has one.
 */
function dimensionValue(g: CanvasRenderingContext2D, pal: CanvasPalette, px: number, text: string, color: string, mask: boolean, font?: DrawingFont): void {
  g.font = valueFont(font, px, pal.drawingFont);
  g.textAlign = 'center';
  g.textBaseline = 'alphabetic';
  if (mask) {
    const w = textWidth(g, text);
    maskText(g, w, px, pal.paper, -w / 2, true);
  }
  haloText(g, text, 0, 0, color, pal.labelHalo);
}

/**
 * One line of a multi-line text (docs/adr/0182 §3), run by run from where its baseline starts (`at`): upright at 400
 * (600 bold, italic when italic), raised 0.4 and lowered 0.15 of its height at 0.6 of it, in the run's colour (the
 * label's without one), underlined 0.12 of its height under its baseline; its letters `widthFactor` wide. A text's
 * face (docs/adr/0183 §2): its typeface, its bold and italic added to the runs', its letters leaning.
 */
export function paragraphLine(
  g: CanvasRenderingContext2D,
  pal: CanvasPalette,
  cam: { worldToScreen(p: Vec2): Vec2; scale: number },
  text: string,
  runs: readonly TextRun[] | undefined,
  [start, end]: readonly [number, number],
  at: Vec2,
  rotation: number,
  height: number,
  widthFactor: number,
  face: Face = {},
): void {
  const letters = Array.from(text);
  const px = height * cam.scale;
  if (px < 1) return;
  const s = cam.worldToScreen(at);
  const runAt = (i: number) => runs?.find((r) => r.start <= i && i < r.end);
  g.save();
  g.translate(s.x, s.y);
  g.rotate((-rotation * Math.PI) / 180);
  const lean = leanOf(face);
  if (lean) g.transform(1, 0, -lean, 1, 0, 0);
  if (widthFactor !== 1) g.scale(widthFactor, 1);
  g.textAlign = 'left';
  g.textBaseline = 'alphabetic';
  let x = 0;
  for (let i = start; i < Math.min(end, letters.length); ) {
    const run = runAt(i);
    let j = i + 1;
    while (j < Math.min(end, letters.length) && runAt(j) === run) j++;
    const words = letters.slice(i, j).join('');
    const size = run?.script ? px * 0.6 : px;
    const lift = run?.script === 'super' ? px * 0.4 : run?.script === 'sub' ? -px * 0.15 : 0;
    g.font = faceFont(face, size, pal.drawingFont, `${run?.italic ? 'italic ' : ''}${run?.bold ? 600 : 400}`, run);
    const color = run?.color ? resolveColor(run.color, pal) : pal.label;
    const w = g.measureText(words).width;
    if (words.trim()) haloText(g, words, x, -lift, color, pal.labelHalo);
    if (run?.underline) {
      const [top, thick] = [size * 0.12 - lift, Math.max(1, size * 0.06)];
      g.fillStyle = pal.labelHalo;
      g.fillRect(x - 1.5, top - 1.5, w + 3, thick + 3);
      g.fillStyle = color;
      g.fillRect(x, top, w, thick);
    }
    x += w;
    i = j;
  }
  g.restore();
}

/**
 * A multi-line text's mask (docs/adr/0182 §3): the box from `at` `w` along its turn and `h` up, metres, in the paper's
 * colour; a leaning text's leans from its corner (docs/adr/0183 §2).
 */
export function paragraphMask(g: CanvasRenderingContext2D, pal: CanvasPalette, cam: { worldToScreen(p: Vec2): Vec2; scale: number }, at: Vec2, rotation: number, w: number, h: number, lean = 0): void {
  const s = cam.worldToScreen(at);
  g.save();
  g.translate(s.x, s.y);
  g.rotate((-rotation * Math.PI) / 180);
  if (lean) g.transform(1, 0, -lean, 1, 0, 0);
  g.fillStyle = pal.paper;
  g.fillRect(0, -h * cam.scale, w * cam.scale, h * cam.scale);
  g.restore();
}

/** A multi-line text's records as the core gives them (`textLines`, the store's labels): its mask, then its lines; in its face. */
export function paragraphRecords(
  g: CanvasRenderingContext2D,
  pal: CanvasPalette,
  cam: { worldToScreen(p: Vec2): Vec2; scale: number },
  records: ArrayLike<number>,
  text: string,
  runs: readonly TextRun[] | undefined,
  face: Face = {},
): void {
  for (let i = 0; i + LABEL_STRIDE <= records.length; i += LABEL_STRIDE) {
    const at = { x: records[i + 2], y: records[i + 3] };
    if (records[i + 1] === LABEL.paragraphMask) paragraphMask(g, pal, cam, at, records[i + 4], records[i + 5], records[i + 6], leanOf(face));
    else paragraphLine(g, pal, cam, text, runs, [records[i + 7], records[i + 8]], at, records[i + 4], records[i + 5], records[i + 6], face);
  }
}

/**
 * Entity labels and text. Which ones a frame draws and where comes from the
 * geometry store (`labels`: visible layer, box in view, text size on
 * screen, the LabelStyle's scale range and smallest feature; see
 * ./storeRecords); size, template and colour from the layer's LabelStyle,
 * so this function knows nothing about specific layers.
 *
 * Labels (parcel numbers, point names, contour heights) are thinned where
 * they would overlap on screen: the first one keeps its place and one that
 * would cover it is left out of this view (as GIS labelling does), so a
 * zoomed-out map stays readable and draws fewer letters. Text objects and
 * dimension values are part of the drawing and are always drawn.
 */
/**
 * A view zoomed `k` times about the world point `a`, `a` staying where it is on screen (docs/adr/0205 §5): a grown
 * text is drawn through it, its positions and sizes `k` times its own about its anchor.
 */
function grownView(view: ViewTransform, a: Vec2, k: number): ViewTransform {
  const as = view.worldToScreen(a);
  return {
    scale: view.scale * k,
    width: view.width,
    height: view.height,
    worldToScreen: (p) => {
      const s = view.worldToScreen(p);
      return { x: as.x + (s.x - as.x) * k, y: as.y + (s.y - as.y) * k };
    },
    screenToWorld: (p) => view.screenToWorld({ x: as.x + (p.x - as.x) / k, y: as.y + (p.y - as.y) / k }),
  };
}

/**
 * A grown leader's arrowhead (docs/adr/0205 §5, §7): its areas solid and its lines, `k` times its own about its tip,
 * in its colour, over the one the scene draws as it is.
 */
function grownArrowhead(g: CanvasRenderingContext2D, view: ViewTransform, e: Extract<ReturnType<CadDocument['get']>, { kind: 'leader' }>, k: number, color: string): void {
  const l = leaderLayout(e);
  if (!l) return;
  const tip = e.pts[0];
  const at = (p: Vec2) => view.worldToScreen({ x: tip.x + (p.x - tip.x) * k, y: tip.y + (p.y - tip.y) * k });
  g.save();
  g.fillStyle = color;
  g.strokeStyle = color;
  g.lineWidth = 1.2;
  for (const ring of l.head.fills) {
    g.beginPath();
    ring.forEach((p, j) => (j ? g.lineTo(at(p).x, at(p).y) : g.moveTo(at(p).x, at(p).y)));
    g.closePath();
    g.fill();
  }
  for (const line of l.head.lines) {
    g.beginPath();
    line.pts.forEach((p, j) => (j ? g.lineTo(at(p).x, at(p).y) : g.moveTo(at(p).x, at(p).y)));
    if (line.closed) g.closePath();
    g.stroke();
  }
  g.restore();
}

/**
 * The drawing's text in the view. `stride` is the records' length: `LABEL_STRIDE`, or `LABEL_SHOWN_STRIDE` for the
 * store's `labelsShown` (docs/adr/0205 §5), whose records carry their factor and anchor: a grown one is drawn through a
 * view zoomed about its anchor.
 */
export function drawLabels(
  g: CanvasRenderingContext2D,
  doc: CadDocument,
  view: ViewTransform,
  pal: CanvasPalette,
  spots: Float64Array,
  dimensionText: (l: Pick<DimensionLayout, 'prefix' | 'unit' | 'value'>, look: DimensionLook) => string,
  pieces: (block: string) => readonly BlockPiece[] | null = () => null,
  stride: number = LABEL_STRIDE,
): void {
  const layers = doc.layers;
  const ink = { fg: pal.fg, 'fg-dim': pal.fgDim, label: pal.label } as const;
  const room = new LabelRoom(view.width, view.height);
  g.save();
  g.textAlign = 'center';
  g.textBaseline = 'middle';
  for (let i = 0; i < spots.length; i += stride) {
    const e = doc.get(spots[i]);
    if (!e) continue;
    const what = spots[i + 1];
    const x = spots[i + 2];
    const y = spots[i + 3];
    // A grown record: its factor and anchor after the record (docs/adr/0205 §5).
    const k = stride > LABEL_STRIDE ? spots[i + LABEL_STRIDE] : 1;
    const cam = k === 1 ? view : grownView(view, { x: spots[i + LABEL_STRIDE + 1], y: spots[i + LABEL_STRIDE + 2] }, k);
    if (what === LABEL.dimension && e.kind === 'dimension') {
      const s = cam.worldToScreen({ x, y });
      g.save();
      g.translate(s.x, s.y);
      g.rotate((-spots[i + 4] * Math.PI) / 180);
      // Its value's own colour when its look names one (docs/adr/0205 §6), else the object's.
      const color = e.textColor ?? e.color ?? layers.get(e.layerId)?.style.color;
      const measured = { value: spots[i + 5], unit: DIMENSION_UNIT[spots[i + 6]] ?? 'length', prefix: DIMENSION_PREFIX[spots[i + 7]] ?? '' };
      dimensionValue(g, pal, e.height * cam.scale, e.text || dimensionText(measured, e), !color || color === 'fg' || color === 'fg-dim' ? pal.label : resolveColor(color, pal), spots[i + 8] === 1, e.font);
      g.restore();
      continue;
    }
    // x, y where the text's baseline starts (its point moved by its alignment); its width factor and mask (docs/adr/0145).
    if (what === LABEL.text && e.kind === 'text') {
      baselineText(g, pal, cam.worldToScreen({ x, y }), spots[i + 4], e.height * cam.scale, spots[i + 5], spots[i + 6] * cam.scale, e.text, e);
      continue;
    }
    // A multi-line text: its mask, then its lines (docs/adr/0182 §3); a block's own too. Its face (docs/adr/0183 §2).
    if (what === LABEL.paragraphMask) {
      const owner = e.kind === 'text' ? e : e.kind === 'insert' ? pieces(e.block)?.[spots[i + 7]] : undefined;
      paragraphMask(g, pal, cam, { x, y }, spots[i + 4], spots[i + 5], spots[i + 6], owner?.kind === 'text' ? leanOf(owner) : 0);
      continue;
    }
    if (what === LABEL.line && e.kind === 'text') {
      paragraphLine(g, pal, cam, e.text, e.runs, [spots[i + 7], spots[i + 8]], { x, y }, spots[i + 4], spots[i + 5], spots[i + 6], e);
      continue;
    }
    if (what === LABEL.pieceLine && e.kind === 'insert') {
      const piece = pieces(e.block)?.[spots[i + 6]];
      if (piece?.kind === 'text') paragraphLine(g, pal, cam, piece.text, piece.runs, [spots[i + 7], spots[i + 8]], { x, y }, spots[i + 4], spots[i + 5], piece.widthFactor ?? 1, piece);
      continue;
    }
    // A table's cell: its words in the table's face and colour (docs/adr/0184 §2).
    if (what === LABEL.cell && e.kind === 'table') {
      const words = e.cells[spots[i + 5]]?.[spots[i + 6]];
      if (words) {
        const color = e.color ?? layers.get(e.layerId)?.style.color;
        const ink = !color || color === 'fg' || color === 'fg-dim' ? pal.label : resolveColor(color, pal);
        tableCell(g, pal, cam.worldToScreen({ x, y }), spots[i + 4], e.height * cam.scale, words, e, spots[i + 7] === 1, ink);
      }
      continue;
    }
    // A leader's note, as a text (docs/adr/0146 §5); grown, its arrowhead with it (docs/adr/0205 §5).
    if (what === LABEL.leader && e.kind === 'leader') {
      if (k > 1) {
        const color = e.color ?? layers.get(e.layerId)?.style.color;
        grownArrowhead(g, view, e, k, !color || color === 'fg' || color === 'fg-dim' ? pal.label : resolveColor(color, pal));
      }
      if (e.text) baselineText(g, pal, cam.worldToScreen({ x, y }), spots[i + 4], e.height * cam.scale, 1, spots[i + 6] * cam.scale, e.text);
      continue;
    }
    // A block's texts, dimension values and leaders' notes (docs/adr/0144), as its own objects draw theirs.
    if ((what === LABEL.pieceText || what === LABEL.pieceDimension || what === LABEL.pieceLeader) && e.kind === 'insert') {
      const piece = pieces(e.block)?.[spots[i + 6]];
      const s = cam.worldToScreen({ x, y });
      if (what === LABEL.pieceText && piece?.kind === 'text') {
        // An attribute's piece shows the insert's value, else its default (docs/adr/0144 §7).
        const text = pieceText(piece, e.attrs);
        if (!text) continue;
        baselineText(g, pal, s, spots[i + 4], spots[i + 5] * cam.scale, spots[i + 7], spots[i + 8] * cam.scale, text, piece);
      } else if (what === LABEL.pieceLeader && piece?.kind === 'leader' && piece.text) {
        // A leader's note (docs/adr/0146 §5), as a text piece's.
        baselineText(g, pal, s, spots[i + 4], spots[i + 5] * cam.scale, 1, spots[i + 8] * cam.scale, piece.text);
      } else if (what === LABEL.pieceDimension && piece?.kind === 'dimension') {
        g.save();
        g.translate(s.x, s.y);
        g.rotate((-spots[i + 4] * Math.PI) / 180);
        const color = piece.textColor ?? e.color ?? layers.get(e.layerId)?.style.color;
        const measured = { value: spots[i + 5], ...dimensionMeasure(piece.style, piece.angle) };
        dimensionValue(g, pal, spots[i + 7] * cam.scale, piece.text || dimensionText(measured, piece), !color || color === 'fg' || color === 'fg-dim' ? pal.label : resolveColor(color, pal), spots[i + 8] === 1, piece.font);
        g.restore();
      }
      continue;
    }
    const st = layers.get(e.layerId)?.style.label ?? DEFAULT_LABELS[e.kind];
    if (!st || !e.label) continue;
    const size = Math.min(st.maxSize ?? st.size, st.size + (st.grow ?? 0) * cam.scale);
    const text = fillTemplate(st.template, e.label);
    const color = ink[st.ink ?? 'label'];
    g.font = `${st.weight ?? 500} ${size.toFixed(1)}px ${pal.drawingFont}`;

    const w = textWidth(g, text);
    switch (what) {
      case LABEL.center: {
        const s = cam.worldToScreen({ x, y });
        if (!room.claim(s.x - w / 2, s.y - size / 2, s.x + w / 2, s.y + size / 2)) break;
        haloText(g, text, s.x, s.y, color, pal.labelHalo);
        break;
      }
      case LABEL.corner: {
        const tl = cam.worldToScreen({ x, y });
        if (!room.claim(tl.x + 8, tl.y + 14 - size / 2, tl.x + 8 + w, tl.y + 14 + size / 2)) break;
        g.textAlign = 'left';
        haloText(g, text, tl.x + 8, tl.y + 14, color, pal.labelHalo);
        g.textAlign = 'center';
        break;
      }
      case LABEL.beside: {
        const s = cam.worldToScreen({ x, y });
        if (!room.claim(s.x + 7, s.y - 7 - size / 2, s.x + 7 + w, s.y - 7 + size / 2)) break;
        g.textAlign = 'left';
        haloText(g, text, s.x + 7, s.y - 7, color, pal.labelHalo);
        g.textAlign = 'center';
        break;
      }
      case LABEL.along: {
        // Placed a third of the way along, kept upright.
        const a = cam.worldToScreen({ x, y });
        const c = cam.worldToScreen({ x: spots[i + 4], y: spots[i + 5] });
        let ang = Math.atan2(c.y - a.y, c.x - a.x);
        if (ang > Math.PI / 2 || ang < -Math.PI / 2) ang += Math.PI;
        // The rotated text's box on screen.
        const mx = (a.x + c.x) / 2;
        const my = (a.y + c.y) / 2;
        const hx = (Math.abs(Math.cos(ang)) * w + Math.abs(Math.sin(ang)) * size) / 2;
        const hy = (Math.abs(Math.sin(ang)) * w + Math.abs(Math.cos(ang)) * size) / 2;
        if (!room.claim(mx - hx, my - hy, mx + hx, my + hy)) break;
        g.save();
        g.translate((a.x + c.x) / 2, (a.y + c.y) / 2);
        g.rotate(ang);
        haloText(g, text, 0, 0, color, pal.labelHalo);
        g.restore();
        break;
      }
    }
  }
  g.restore();
}

/**
 * Grip squares on selected entities (their grips from the geometry store).
 * Grips closer than 9 px on screen are thinned so dense polylines
 * (contours) stay readable.
 */
export function drawGrips(g: CanvasRenderingContext2D, sets: readonly GripSet[], cam: Camera, pal: CanvasPalette, hot: { id: number; index: number } | null = null): void {
  if (sets.length > 150) return;
  g.save();
  g.fillStyle = pal.accent;
  g.strokeStyle = pal.labelHalo;
  g.lineWidth = 1;
  for (const set of sets) {
    let last: Vec2 | null = null;
    for (let i = 0; i < set.points.length; i++) {
      if (hot && hot.id === set.id && hot.index === i) continue;
      const s = cam.worldToScreen(set.points[i]);
      const x = Math.round(s.x);
      const y = Math.round(s.y);
      if (set.segments[i] >= 0) {
        // Mid grips (add a vertex / bend an arc): small hollow diamonds, hidden on short segments.
        if (!midGripVisible(set, i, cam)) continue;
        g.save();
        g.fillStyle = pal.labelHalo;
        g.strokeStyle = pal.accent;
        g.beginPath();
        g.moveTo(x, y - 4);
        g.lineTo(x + 4, y);
        g.lineTo(x, y + 4);
        g.lineTo(x - 4, y);
        g.closePath();
        g.fill();
        g.stroke();
        g.restore();
        continue;
      }
      if (last && Math.abs(s.x - last.x) < 9 && Math.abs(s.y - last.y) < 9) continue;
      last = s;
      g.fillRect(x - 3, y - 3, 6, 6);
      g.strokeRect(x - 3.5, y - 3.5, 7, 7);
    }
  }
  // The grip being edited ("sıcak tutamaç") is drawn larger in ink colour.
  const hp = hot ? sets.find((set) => set.id === hot.id)?.points[hot.index] : undefined;
  if (hp) {
    const s = cam.worldToScreen(hp);
    g.fillStyle = pal.fg;
    g.strokeStyle = pal.accent;
    g.lineWidth = 1.5;
    g.fillRect(Math.round(s.x) - 4, Math.round(s.y) - 4, 8, 8);
    g.strokeRect(Math.round(s.x) - 4.5, Math.round(s.y) - 4.5, 9, 9);
  }
  g.restore();
}

/**
 * The vertices Köşe tablosu's selected rows name (docs/adr/0172 §3): an accent ring round each grip, on a halo so it
 * shows on any drawing; at most 2 000 (a selection of a contour's every row stays light).
 */
export function drawMarkedVertices(g: CanvasRenderingContext2D, pts: readonly Vec2[], cam: Camera, pal: CanvasPalette): void {
  if (!pts.length) return;
  g.save();
  const ring = (width: number, color: string) => {
    g.lineWidth = width;
    g.strokeStyle = color;
    g.beginPath();
    for (const p of pts.slice(0, 2000)) {
      const s = cam.worldToScreen(p);
      const [x, y] = [Math.round(s.x) + 0.5, Math.round(s.y) + 0.5];
      g.moveTo(x + 8, y);
      g.arc(x, y, 8, 0, Math.PI * 2);
    }
    g.stroke();
  };
  ring(4, pal.labelHalo);
  ring(2, pal.accent);
  g.restore();
}

/**
 * The place Koordinata git marked (docs/adr/0178 §6): an accent ring with four ticks and a dot in it, on a halo so it
 * shows on any drawing, its coordinates written below it on the right.
 */
export function drawSearchMark(g: CanvasRenderingContext2D, p: Vec2, label: string, cam: Camera, pal: CanvasPalette): void {
  const s = cam.worldToScreen(p);
  const [x, y] = [Math.round(s.x) + 0.5, Math.round(s.y) + 0.5];
  g.save();
  const shape = (width: number, color: string) => {
    g.lineWidth = width;
    g.strokeStyle = color;
    g.beginPath();
    g.moveTo(x + 8, y);
    g.arc(x, y, 8, 0, Math.PI * 2);
    for (const [dx, dy] of [[0, -1], [1, 0], [0, 1], [-1, 0]]) {
      g.moveTo(x + dx * 8, y + dy * 8);
      g.lineTo(x + dx * 15, y + dy * 15);
    }
    g.stroke();
  };
  shape(4, pal.labelHalo);
  shape(2, pal.accent);
  g.fillStyle = pal.accent;
  g.beginPath();
  g.arc(x, y, 2, 0, Math.PI * 2);
  g.fill();
  g.font = `600 11px ${pal.font}`;
  g.textBaseline = 'top';
  haloText(g, label, x + 12, y + 12, pal.accent, pal.labelHalo);
  g.restore();
}

/** An edge the core writes as points along it (arcs every 7.5° at most). */
function edgePoints(e: CoreEdge): Vec2[] {
  if (e.kind === 'seg') return [e.a, e.b];
  const n = Math.max(2, Math.ceil(Math.abs(e.sweep) / (Math.PI / 24)));
  return Array.from({ length: n + 1 }, (_, i) => {
    const a = e.a0 + (e.sweep * i) / n;
    return { x: e.c.x + Math.cos(a) * e.r, y: e.c.y + Math.sin(a) * e.r };
  });
}

/**
 * A topology finding over the drawing (docs/adr/0202 §5): its regions filled and outlined, its edges drawn bold, in the
 * danger colour on a halo; its place marked as Koordinata git marks one, the problem's name beside it.
 */
export function drawProblemMark(g: CanvasRenderingContext2D, m: ProblemMark, cam: Camera, pal: CanvasPalette): void {
  g.save();
  g.lineJoin = 'round';
  g.lineCap = 'round';
  if (m.regions.length) {
    g.beginPath();
    for (const a of m.regions) {
      for (const r of [a.outer, ...a.holes]) {
        polygonRing({ pts: [...r.pts], ...(r.bulges ? { bulges: [...r.bulges] } : {}) }).forEach((p, i) => {
          const s = cam.worldToScreen(p);
          i ? g.lineTo(s.x, s.y) : g.moveTo(s.x, s.y);
        });
        g.closePath();
      }
    }
    g.globalAlpha = 0.3;
    g.fillStyle = pal.danger;
    g.fill('evenodd');
    g.globalAlpha = 1;
    g.lineWidth = 3.5;
    g.strokeStyle = pal.labelHalo;
    g.stroke();
    g.lineWidth = 1.5;
    g.strokeStyle = pal.danger;
    g.stroke();
  }
  if (m.edges.length) {
    g.beginPath();
    for (const e of m.edges) {
      edgePoints(e).forEach((p, i) => {
        const s = cam.worldToScreen(p);
        i ? g.lineTo(s.x, s.y) : g.moveTo(s.x, s.y);
      });
    }
    g.lineWidth = 6;
    g.strokeStyle = pal.labelHalo;
    g.stroke();
    g.lineWidth = 3;
    g.strokeStyle = pal.danger;
    g.stroke();
  }
  g.restore();
  drawSearchMark(g, m.at, m.label, cam, { ...pal, accent: pal.danger });
}

/** The snap marker; `label` is what it says instead of its kind's name (“Uzantı 12.063 m”, docs/adr/0163 §2). */
export function drawSnap(g: CanvasRenderingContext2D, hit: SnapHit, cam: Camera, pal: CanvasPalette, label?: string): void {
  const s = cam.worldToScreen(hit.point);
  const x = Math.round(s.x) + 0.5;
  const y = Math.round(s.y) + 0.5;
  g.save();
  g.strokeStyle = pal.snap;
  g.lineWidth = 1.5;
  g.beginPath();
  switch (hit.kind) {
    case 'midpoint':
      g.moveTo(x, y - 6);
      g.lineTo(x + 6, y + 5);
      g.lineTo(x - 6, y + 5);
      g.closePath();
      break;
    case 'center':
    case 'node':
      g.arc(x, y, 5.5, 0, Math.PI * 2);
      break;
    case 'quadrant':
      g.moveTo(x, y - 6);
      g.lineTo(x + 6, y);
      g.lineTo(x, y + 6);
      g.lineTo(x - 6, y);
      g.closePath();
      break;
    case 'intersection':
      g.moveTo(x - 5, y - 5);
      g.lineTo(x + 5, y + 5);
      g.moveTo(x + 5, y - 5);
      g.lineTo(x - 5, y + 5);
      break;
    case 'perpendicular':
      g.moveTo(x - 6, y - 6);
      g.lineTo(x - 6, y + 5);
      g.lineTo(x + 6, y + 5);
      g.moveTo(x - 6, y);
      g.lineTo(x, y);
      g.lineTo(x, y + 5);
      break;
    case 'tangent':
      g.arc(x, y + 1, 4.5, 0, Math.PI * 2);
      g.moveTo(x - 6.5, y - 4.5);
      g.lineTo(x + 6.5, y - 4.5);
      break;
    case 'nearest':
      g.moveTo(x - 5, y - 5);
      g.lineTo(x + 5, y - 5);
      g.lineTo(x - 5, y + 5);
      g.lineTo(x + 5, y + 5);
      g.closePath();
      break;
    // docs/adr/0163 §1: a diamond with a dot; a plus on a dashed line; two slanted strokes; a small grid.
    case 'centroid':
      g.moveTo(x, y - 6);
      g.lineTo(x + 6, y);
      g.lineTo(x, y + 6);
      g.lineTo(x - 6, y);
      g.closePath();
      g.moveTo(x + 1.6, y);
      g.arc(x, y, 1.6, 0, Math.PI * 2);
      break;
    case 'extension':
      g.moveTo(x, y - 5);
      g.lineTo(x, y + 5);
      g.moveTo(x - 5, y);
      g.lineTo(x + 5, y);
      g.stroke();
      g.beginPath();
      g.setLineDash([3, 2.5]);
      g.moveTo(x - 9, y);
      g.lineTo(x + 9, y);
      break;
    case 'parallel':
      g.moveTo(x - 7, y + 6);
      g.lineTo(x - 1, y - 6);
      g.moveTo(x - 1, y + 6);
      g.lineTo(x + 5, y - 6);
      break;
    case 'grid':
      g.moveTo(x - 3, y - 5);
      g.lineTo(x - 3, y + 5);
      g.moveTo(x + 3, y - 5);
      g.lineTo(x + 3, y + 5);
      g.moveTo(x - 5, y - 3);
      g.lineTo(x + 5, y - 3);
      g.moveTo(x - 5, y + 3);
      g.lineTo(x + 5, y + 3);
      break;
    default:
      g.rect(x - 5, y - 5, 10, 10);
  }
  g.stroke();
  g.setLineDash([]);
  g.font = `500 10.5px ${pal.font}`;
  g.textBaseline = 'bottom';
  // Above-right, so it never collides with the tool's measurement tag (below-right).
  haloText(g, label ?? SNAP_LABEL[hit.kind], x + 9, y - 7, pal.snap, pal.labelHalo);
  g.restore();
}

const CROSSHAIR_ARM: Record<CrosshairSize, number> = { small: 16, medium: 40, full: 1e5 };

export function drawCrosshair(g: CanvasRenderingContext2D, at: Vec2, cursor: ToolCursor, pal: CanvasPalette, size: CrosshairSize): void {
  if (cursor === 'grab') return;
  const x = Math.round(at.x) + 0.5;
  const y = Math.round(at.y) + 0.5;
  const arm = size === 'full' ? CROSSHAIR_ARM.full : cursor === 'pick' ? Math.round(CROSSHAIR_ARM[size] * 0.55) : CROSSHAIR_ARM[size];
  const box = cursor === 'pick' ? 5 : 0;
  g.save();
  g.strokeStyle = pal.fg;
  g.globalAlpha = 0.85;
  g.lineWidth = 1;
  g.beginPath();
  g.moveTo(x - arm, y);
  g.lineTo(x - box, y);
  g.moveTo(x + box, y);
  g.lineTo(x + arm, y);
  g.moveTo(x, y - arm);
  g.lineTo(x, y - box);
  g.moveTo(x, y + box);
  g.lineTo(x, y + arm);
  if (box) g.rect(x - box, y - box, box * 2, box * 2);
  g.stroke();
  g.restore();
}

/** Alternating map scale bar, bottom-right (the toolbox usually sits left). */
export function drawScaleBar(g: CanvasRenderingContext2D, cam: Camera, pal: CanvasPalette): void {
  const targetPx = 120;
  const raw = targetPx / cam.scale;
  const p = Math.pow(10, Math.floor(Math.log10(raw)));
  const len = [1, 2, 5, 10].map((m) => m * p).reduce((best, v) => (Math.abs(v * cam.scale - targetPx) < Math.abs(best * cam.scale - targetPx) ? v : best));
  const px = len * cam.scale;
  const x0 = cam.width - 20 - px;
  const y0 = cam.height - 22;
  g.save();
  for (let i = 0; i < 4; i++) {
    g.fillStyle = i % 2 ? pal.labelHalo : pal.fg;
    g.fillRect(x0 + (px / 4) * i, y0, px / 4, 4);
  }
  g.strokeStyle = pal.fg;
  g.lineWidth = 1;
  g.strokeRect(x0 + 0.5, y0 + 0.5, px, 4);
  g.font = `500 10.5px ${pal.font}`;
  g.textBaseline = 'bottom';
  g.textAlign = 'left';
  haloText(g, '0', x0, y0 - 3, pal.label, pal.labelHalo);
  g.textAlign = 'right';
  const unit = len >= 1000 ? `${len / 1000} km` : len >= 1 ? `${len} m` : `${(len * 100).toPrecision(2)} cm`;
  haloText(g, unit, x0 + px, y0 - 3, pal.label, pal.labelHalo);
  g.restore();
}

/** Grid-north arrow with Turkish "K" (Kuzey), top-right. */
export function drawNorthArrow(g: CanvasRenderingContext2D, cam: Camera, pal: CanvasPalette): void {
  const x = cam.width - 30;
  const y = 22;
  g.save();
  g.fillStyle = pal.fg;
  g.strokeStyle = pal.fg;
  g.lineWidth = 1;
  g.beginPath();
  g.moveTo(x, y + 6);
  g.lineTo(x + 6, y + 28);
  g.lineTo(x, y + 23);
  g.closePath();
  g.fill();
  g.beginPath();
  g.moveTo(x, y + 6);
  g.lineTo(x - 6, y + 28);
  g.lineTo(x, y + 23);
  g.closePath();
  g.stroke();
  g.font = `600 11px ${pal.font}`;
  g.textAlign = 'center';
  g.textBaseline = 'bottom';
  haloText(g, 'K', x, y + 3, pal.fg, pal.labelHalo);
  g.restore();
}

/** The coordinate axes' icon: how far from the drawing area's left and bottom edges it stands, and its arms' length. */
const UCS_MARGIN = 30;
const UCS_ARM = 38;

/**
 * A CAD project's coordinate axes (docs/adr/0165 §5) in place of grid north and the scale bar: X to the right and Y
 * up from a small square, at the bottom left, or on the origin when 0,0 is on screen with room for the arms and their
 * names, as AutoCAD's UCS icon (the desktop's map_marks.rs).
 */
export function drawUcsIcon(g: CanvasRenderingContext2D, cam: Camera, pal: CanvasPalette): void {
  const s = cam.worldToScreen({ x: 0, y: 0 });
  const room = UCS_ARM + 16;
  const on = Number.isFinite(s.x) && Number.isFinite(s.y) && s.x >= UCS_MARGIN && s.x + room <= cam.width && s.y <= cam.height - UCS_MARGIN && s.y - room >= 0;
  const o = on ? s : { x: UCS_MARGIN, y: cam.height - UCS_MARGIN };
  g.save();
  g.strokeStyle = pal.fg;
  g.fillStyle = pal.fg;
  g.lineWidth = 1.5;
  g.beginPath();
  g.moveTo(o.x, o.y);
  g.lineTo(o.x + UCS_ARM, o.y);
  g.moveTo(o.x, o.y);
  g.lineTo(o.x, o.y - UCS_ARM);
  g.stroke();
  const head = (a: Vec2, b: Vec2, c: Vec2) => {
    g.beginPath();
    g.moveTo(a.x, a.y);
    g.lineTo(b.x, b.y);
    g.lineTo(c.x, c.y);
    g.closePath();
    g.fill();
  };
  head({ x: o.x + UCS_ARM + 2, y: o.y }, { x: o.x + UCS_ARM - 6, y: o.y - 3.5 }, { x: o.x + UCS_ARM - 6, y: o.y + 3.5 });
  head({ x: o.x, y: o.y - UCS_ARM - 2 }, { x: o.x - 3.5, y: o.y - UCS_ARM + 6 }, { x: o.x + 3.5, y: o.y - UCS_ARM + 6 });
  g.lineWidth = 1;
  g.strokeRect(o.x - 3, o.y - 3, 6, 6);
  g.font = `600 11px ${pal.font}`;
  g.textBaseline = 'bottom';
  g.textAlign = 'left';
  haloText(g, 'X', o.x + UCS_ARM + 6, o.y + 6, pal.fg, pal.labelHalo);
  g.textAlign = 'center';
  haloText(g, 'Y', o.x, o.y - UCS_ARM - 5, pal.fg, pal.labelHalo);
  g.restore();
}

/** How near (CSS px) the pointer must be to take a grip, and to be said to rest on one (the select tool's tag). */
export const GRIP_HIT_PX = 6;

/** A mid grip is offered only when its segment is long enough on screen to tell it from the vertices. */
export function midGripVisible(set: GripSet, index: number, cam: Camera): boolean {
  const seg = set.segments[index];
  if (!(seg >= 0)) return true;
  // The segment counts within its own ring: the first part's is at the start of the grips (docs/adr/0143).
  const ring = set.rings?.[index] ?? { from: 0, count: set.vertices };
  const a = cam.worldToScreen(set.points[ring.from + seg]);
  const b = cam.worldToScreen(set.points[ring.from + ((seg + 1) % ring.count)]);
  return Math.hypot(b.x - a.x, b.y - a.y) >= 28;
}

/**
 * Object tracking: acquired points as small crosses, the alignment line(s)
 * the cursor is locked to (dashed, through the whole view) and a tag with
 * the distance and angle from the tracked point.
 */
/**
 * What object tracking and the snap additions show (docs/adr/0085, 0163 §2): the acquired points, the acquired edges,
 * and the extensions or the parallel the snap lies on, dashed.
 */
export interface TrackingMarks {
  points: readonly Vec2[];
  edges: readonly { at: Vec2; dir: Vec2 }[];
  /** An extension from its end to the snap: the segment, or points around an arc. */
  paths: readonly (readonly Vec2[])[];
  /** The parallel's whole line through the last point. */
  lines: readonly { through: Vec2; dir: Vec2 }[];
}

export function drawObjectTracking(g: CanvasRenderingContext2D, marks: TrackingMarks, track: TrackHit | null, cam: Camera, pal: CanvasPalette, formatLength: (m: number) => string): void {
  if (!marks.points.length && !marks.edges.length) return;
  g.save();
  g.strokeStyle = pal.snap;
  // The snap additions' guides under the marks.
  if (marks.paths.length || marks.lines.length) {
    g.lineWidth = 1;
    g.globalAlpha = 0.85;
    g.setLineDash([3, 4]);
    for (const path of marks.paths) {
      g.beginPath();
      path.forEach((p, i) => {
        const s = cam.worldToScreen(p);
        if (i) g.lineTo(s.x, s.y);
        else g.moveTo(s.x, s.y);
      });
      g.stroke();
    }
    for (const l of marks.lines) {
      const o = cam.worldToScreen(l.through);
      g.beginPath();
      g.moveTo(o.x - l.dir.x * 1e4, o.y + l.dir.y * 1e4);
      g.lineTo(o.x + l.dir.x * 1e4, o.y - l.dir.y * 1e4);
      g.stroke();
    }
    g.setLineDash([]);
    g.globalAlpha = 1;
  }
  g.lineWidth = 1.5;
  // An acquired edge: two short strokes along it where it was rested on.
  for (const e of marks.edges) {
    const c = cam.worldToScreen(e.at);
    const [ux, uy] = [e.dir.x, -e.dir.y];
    const [nx, ny] = [-uy * 2.5, ux * 2.5];
    g.beginPath();
    for (const side of [-1, 1]) {
      const [ox, oy] = [c.x + nx * side, c.y + ny * side];
      g.moveTo(ox - ux * 5, oy - uy * 5);
      g.lineTo(ox + ux * 5, oy + uy * 5);
    }
    g.stroke();
  }
  for (const p of marks.points) {
    const s = cam.worldToScreen(p);
    const x = Math.round(s.x) + 0.5;
    const y = Math.round(s.y) + 0.5;
    g.beginPath();
    g.moveTo(x - 5, y);
    g.lineTo(x + 5, y);
    g.moveTo(x, y - 5);
    g.lineTo(x, y + 5);
    g.stroke();
  }
  if (track) {
    g.lineWidth = 1;
    g.globalAlpha = 0.85;
    g.setLineDash([3, 4]);
    for (const l of track.lines) {
      const o = cam.worldToScreen(l.origin);
      const r = (l.angle * Math.PI) / 180;
      g.beginPath();
      g.moveTo(o.x, o.y);
      g.lineTo(o.x + Math.cos(r) * 1e4, o.y - Math.sin(r) * 1e4);
      g.stroke();
    }
    g.setLineDash([]);
    g.globalAlpha = 1;
    const at = cam.worldToScreen(track.point);
    const l = track.lines[0];
    const text = track.lines.length > 1 ? 'İzleme: kesişim' : `İzleme ${formatLength(dist(l.origin, track.point))} < ${l.angle}°`;
    g.font = `500 10.5px ${pal.font}`;
    g.textBaseline = 'bottom';
    haloText(g, text, Math.round(at.x) + 9, Math.round(at.y) - 7, pal.snap, pal.labelHalo);
  }
  g.restore();
}
