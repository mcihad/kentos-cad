import type { LabelStyle } from '../contracts/generated/LabelStyle';
import type { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import { resolveColor, type CanvasPalette } from '../render/color';
import type { ViewTransform } from './Camera';
import { DEFAULT_LABELS, LABEL } from './storeRecords';

/**
 * The label engine's labels on the overlay (docs/adr/0212 §3.8): each label's frame record, then its lines, letters
 * and callout, as the geometry store placed them. The look is the label's class's: the letters' colour, weight and
 * slant, the halo, the background (or a contour's mask), the shadow and the callout's line. Unplaced labels (only
 * asked for when Yerleşmeyen etiketleri göster is on) are red, hidden ones (Etiketi gizle's Göster) faint, pinned ones
 * outlined when Sabit etiketleri vurgula is on. The desktop's is `labels.rs`.
 */

/** A frame's state bits (the core's `engine::PINNED` …). */
export const LABEL_STATE = { pinned: 1, unplaced: 2, hidden: 4, overlapping: 8, curved: 16, outside: 32, masked: 64 } as const;

/** The style a label record's class names: its layer's rule's, its layer's single label, else its kind's default. */
export function labelStyleOf(doc: CadDocument, e: Entity, cls: number): LabelStyle | undefined {
  const node = doc.layers.get(e.layerId);
  const labels = node?.style.labels;
  if (labels?.mode === 'rules') return labels.classes?.[cls]?.style;
  return node?.style.label ?? DEFAULT_LABELS[e.kind];
}

export interface PlacedOptions {
  /** Sabit etiketleri vurgula (`graphics.pinnedLabels`). */
  pinned?: boolean;
}

const RAD = Math.PI / 180;

/** A shape behind a label: in its frame, `w` × `h` px round its middle. */
function backdrop(g: CanvasRenderingContext2D, shape: 'rect' | 'round' | 'ellipse', w: number, h: number): void {
  g.beginPath();
  if (shape === 'ellipse') g.ellipse(0, 0, (w / 2) * Math.SQRT2, (h / 2) * Math.SQRT2, 0, 0, Math.PI * 2);
  else if (shape === 'round') g.roundRect(-w / 2, -h / 2, w, h, Math.min(h / 2, 6));
  else g.rect(-w / 2, -h / 2, w, h);
}

/**
 * Draws the label whose frame record is at `i` and returns the index past its last record. `cam` is the view the
 * records are drawn through; `texts` the records' texts.
 */
export function drawPlacedLabel(
  g: CanvasRenderingContext2D,
  doc: CadDocument,
  cam: ViewTransform,
  pal: CanvasPalette,
  records: Float64Array,
  texts: readonly string[],
  i: number,
  stride: number,
  options: PlacedOptions,
): number {
  const id = records[i];
  let end = i + stride;
  while (end < records.length && records[end] === id && records[end + 1] > LABEL.placed && records[end + 1] <= LABEL.placedCallout) end += stride;
  const e = doc.get(id);
  const st = e && labelStyleOf(doc, e, records[i + 7]);
  if (!e || !st) return end;
  const state = records[i + 8];
  const s = cam.worldToScreen({ x: records[i + 2], y: records[i + 3] });
  const angle = records[i + 4] * RAD;
  const w = records[i + 5];
  const h = records[i + 6];
  const ink = { fg: pal.fg, 'fg-dim': pal.fgDim, label: pal.label } as const;
  const unplaced = (state & LABEL_STATE.unplaced) !== 0;
  const color = unplaced ? pal.danger : st.color ? resolveColor(st.color, pal) : ink[st.ink ?? 'label'];
  const haloWidth = st.halo ? st.halo.width : 1.5;
  const haloColor = st.halo?.color ? resolveColor(st.halo.color, pal) : pal.labelHalo;
  const masked = (state & LABEL_STATE.masked) !== 0;
  const bg = st.background ?? (masked ? { shape: 'rect' as const, fill: 'paper', padding: 1 } : undefined);
  const pad = bg ? (bg.padding ?? 2) : 0;
  const curved = (state & LABEL_STATE.curved) !== 0;
  const letters = new Map<number, string[]>();
  const lettersOf = (t: number) => {
    let l = letters.get(t);
    if (!l) letters.set(t, (l = Array.from(texts[t] ?? '')));
    return l;
  };
  const font = (size: number) => `${st.italic ? 'italic ' : ''}${st.weight ?? 500} ${size.toFixed(1)}px ${pal.drawingFont}`;

  /** The label's backdrops and letters, `dx`, `dy` px off, in `fill` (a shadow) or its own colours. */
  const paint = (dx: number, dy: number, fill: string | null) => {
    // The backdrop: the frame's box, or a curved label's every letter's.
    if (bg && (bg.fill || bg.stroke || fill)) {
      const boxes: [number, number, number, number, number][] = [];
      if (!curved) boxes.push([s.x, s.y, angle, w + 2 * pad, h + 2 * pad]);
      else
        for (let k = i + stride; k < end; k += stride)
          if (records[k + 1] === LABEL.placedLetter) {
            const p = cam.worldToScreen({ x: records[k + 2], y: records[k + 3] });
            boxes.push([p.x, p.y, records[k + 4] * RAD, records[k + 8] + 2 * pad, records[k + 5] + 2 * pad]);
          }
      for (const [x, y, a, bw, bh] of boxes) {
        g.save();
        g.translate(x + dx, y + dy);
        g.rotate(-a);
        backdrop(g, bg.shape, bw, bh);
        if (fill) {
          g.fillStyle = fill;
          g.fill();
        } else {
          if (bg.fill) {
            g.fillStyle = resolveColor(bg.fill, pal);
            g.fill();
          }
          if (bg.stroke) {
            g.strokeStyle = resolveColor(bg.stroke, pal);
            g.lineWidth = 1;
            g.stroke();
          }
        }
        g.restore();
      }
    }
    for (let k = i + stride; k < end; k += stride) {
      const what = records[k + 1];
      if (what !== LABEL.placedLine && what !== LABEL.placedLetter) continue;
      const p = cam.worldToScreen({ x: records[k + 2], y: records[k + 3] });
      const text = what === LABEL.placedLine ? (texts[records[k + 6]] ?? '') : (lettersOf(records[k + 6])[records[k + 7]] ?? '');
      if (!text) continue;
      g.save();
      g.translate(p.x + dx, p.y + dy);
      g.rotate(-records[k + 4] * RAD);
      g.font = font(records[k + 5]);
      if (!fill && haloWidth > 0 && !bg?.fill) {
        g.lineJoin = 'round';
        g.lineWidth = haloWidth * 2;
        g.strokeStyle = haloColor;
        g.strokeText(text, 0, 0);
      }
      g.fillStyle = fill ?? color;
      g.fillText(text, 0, 0);
      g.restore();
    }
  };

  g.save();
  g.textAlign = 'center';
  g.textBaseline = 'middle';
  if (state & LABEL_STATE.hidden) g.globalAlpha = 0.35;
  // The callout under the label.
  for (let k = i + stride; k < end; k += stride) {
    if (records[k + 1] !== LABEL.placedCallout) continue;
    const a = cam.worldToScreen({ x: records[k + 2], y: records[k + 3] });
    const b = cam.worldToScreen({ x: records[k + 4], y: records[k + 5] });
    g.beginPath();
    g.moveTo(a.x, a.y);
    if (st.callout?.kind === 'manhattan') g.lineTo(b.x, a.y);
    g.lineTo(b.x, b.y);
    g.strokeStyle = st.callout?.color ? resolveColor(st.callout.color, pal) : color;
    g.lineWidth = st.callout?.width ?? 1;
    g.stroke();
  }
  if (st.shadow && !unplaced) {
    const alpha = g.globalAlpha;
    g.globalAlpha = alpha * (st.shadow.opacity ?? 0.5);
    paint(st.shadow.dx, -st.shadow.dy, st.shadow.color ? resolveColor(st.shadow.color, pal) : '#000000');
    g.globalAlpha = alpha;
  }
  paint(0, 0, null);
  if (options.pinned && state & LABEL_STATE.pinned) {
    g.translate(s.x, s.y);
    g.rotate(-angle);
    g.setLineDash([3, 2]);
    g.strokeStyle = pal.accent;
    g.lineWidth = 1;
    g.strokeRect(-w / 2 - 3, -h / 2 - 3, w + 6, h + 6);
  }
  g.restore();
  return end;
}
