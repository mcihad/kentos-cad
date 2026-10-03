import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { PaintSources } from '../../render/sheet/painter';
import type { PaperColors } from '../../render/sheet/paperPainter';
import { paintThumb, thumbSize } from '../../render/sheet/thumbPainter';
import { h } from '../dom';

/**
 * A small paper picture as a canvas (the Paftalar list, the gallery): the
 * sheet's plan sized to fit a `box` × `boxHeight` px area, sharp at the
 * screen's pixel ratio, drawn with the workspace's colours (read once by
 * the caller from an element that has them). `paint` draws it again (a map
 * picture or a typeface arrived).
 */
export function thumbCanvas(list: DisplayList, box: number, src: PaintSources, colors: PaperColors, label: string, boxHeight = box): { canvas: HTMLCanvasElement; paint(): void } {
  const { width, height } = thumbSize(list.size, box, 0.92, boxHeight);
  const dpr = Math.max(1, Math.min(3, window.devicePixelRatio || 1));
  const canvas = h('canvas', { class: 'sheet-thumb', width: String(Math.round(width * dpr)), height: String(Math.round(height * dpr)), role: 'img', 'aria-label': label });
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;
  const paint = () => {
    const g = canvas.getContext('2d');
    if (g) paintThumb(g, width, height, dpr, list, src, colors);
  };
  paint();
  return { canvas, paint };
}
