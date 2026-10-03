import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import { paintList, type PaintSources } from './painter';
import type { PaperColors } from './paperPainter';

/**
 * A sheet or a template as a small picture (the Paftalar list, the gallery's
 * cards and details): its own plan from the engine painted small (design
 * §11: “canlı küçük resim”), the map frames' content as small as the
 * picture (a template's maps have no place: their grey box), letters too
 * small to read left out, the paper's edge round it.
 */

/** The paper's size fitted in a `box` × `boxHeight` CSS px area (a square by default), `fill` of it at most. */
export function thumbSize(paper: { width: number; height: number }, box: number, fill = 0.92, boxHeight = box): { width: number; height: number; scale: number } {
  const scale = fill * Math.min(box / paper.width, boxHeight / paper.height);
  return { width: Math.max(1, Math.round(paper.width * scale)), height: Math.max(1, Math.round(paper.height * scale)), scale };
}

/** Paints a plan filling a canvas whose CSS size is `width` × `height` (device pixels `dpr` times). */
export function paintThumb(g: CanvasRenderingContext2D, width: number, height: number, dpr: number, list: DisplayList, src: PaintSources, c: PaperColors): void {
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.clearRect(0, 0, width * dpr, height * dpr);
  paintList(g, list, { scale: (width * dpr) / list.size.width, x: 0, y: 0, minLinePx: 0.5, minTextPx: 2.5, placeholderFont: c.font }, src);
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.strokeStyle = c.paperEdge;
  g.lineWidth = 1 / dpr;
  g.strokeRect(0.5 / dpr, 0.5 / dpr, width - 1 / dpr, height - 1 / dpr);
}
