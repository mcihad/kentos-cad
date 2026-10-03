/**
 * Where the paper lies on the screen (docs/sheet/design.md §11): a scale in
 * CSS pixels per millimetre of paper and the screen position of the paper's
 * top left corner. Fitting the page, the real size (a millimetre of paper is
 * a millimetre on a 96 dpi screen), zooming about a point and the zoom shown
 * as a percentage of the real size. Pure arithmetic: the workspace keeps one
 * per sheet and the painters draw with it.
 */

/** CSS pixels per millimetre at real size: CSS has 96 px to the inch. */
export const PX_PER_MM = 96 / 25.4;

/** Zoom limits, as a share of real size: 2 % (an A0 whole in a small window) to 6400 %. */
export const MIN_ZOOM = 0.02;
export const MAX_ZOOM = 64;

export interface PaperViewport {
  /** CSS pixels per millimetre of paper. */
  readonly scale: number;
  /** Screen position (CSS px, in the drawing area) of the paper's top left corner. */
  readonly x: number;
  readonly y: number;
}

export interface Size {
  readonly width: number;
  readonly height: number;
}

const clampScale = (s: number) => Math.min(MAX_ZOOM * PX_PER_MM, Math.max(MIN_ZOOM * PX_PER_MM, s));

/** The whole page in the area, `pad` CSS px clear on every side, centred. */
export function fitPage(paper: Size, area: Size, pad = 28): PaperViewport {
  const w = Math.max(1, area.width - 2 * pad);
  const h = Math.max(1, area.height - 2 * pad);
  const scale = clampScale(Math.min(w / paper.width, h / paper.height));
  return { scale, x: (area.width - paper.width * scale) / 2, y: (area.height - paper.height * scale) / 2 };
}

/** Real size, the paper point under the area's centre kept there (the page's centre when none is given). */
export function realSize(paper: Size, area: Size, view?: PaperViewport): PaperViewport {
  const at = { x: area.width / 2, y: area.height / 2 };
  const centre = view ? toPaper(view, at) : { x: paper.width / 2, y: paper.height / 2 };
  return { scale: PX_PER_MM, x: at.x - centre.x * PX_PER_MM, y: at.y - centre.y * PX_PER_MM };
}

/** Zoomed by `factor` about a screen point, which stays over the same paper point. */
export function zoomAt(view: PaperViewport, factor: number, at: { x: number; y: number }): PaperViewport {
  const scale = clampScale(view.scale * factor);
  const k = scale / view.scale;
  return { scale, x: at.x - (at.x - view.x) * k, y: at.y - (at.y - view.y) * k };
}

export const panBy = (view: PaperViewport, dx: number, dy: number): PaperViewport => ({ scale: view.scale, x: view.x + dx, y: view.y + dy });

/** A screen point (CSS px in the area) as a paper point (mm from the top left corner). */
export const toPaper = (view: PaperViewport, p: { x: number; y: number }) => ({ x: (p.x - view.x) / view.scale, y: (p.y - view.y) / view.scale });

/** A paper point (mm) on the screen (CSS px). */
export const toScreen = (view: PaperViewport, p: { x: number; y: number }) => ({ x: view.x + p.x * view.scale, y: view.y + p.y * view.scale });

/** The zoom as a percentage of real size, whole (“%65”). */
export const zoomPercent = (view: PaperViewport): number => Math.round((view.scale / PX_PER_MM) * 100);

/** A wheel's turn as a zoom factor: a notch (100 px) is about 1.2×, a trackpad's small steps zoom smoothly. */
export const wheelFactor = (deltaY: number, deltaMode: number): number => Math.pow(1.2, -(deltaMode === 1 ? deltaY * 33 : deltaMode === 2 ? deltaY * 400 : deltaY) / 100);

/** The steps the zoom buttons and keys go through, as shares of real size. */
export const ZOOM_STEPS: readonly number[] = [0.05, 0.1, 0.125, 0.25, 0.33, 0.5, 0.67, 0.75, 1, 1.5, 2, 3, 4, 6, 8, 12, 16, 32, 64];

/** The next step up (1) or down (−1) from a zoom given as a share of real size. */
export function nextStep(zoom: number, dir: 1 | -1): number {
  const eps = 1e-6;
  if (dir > 0) return ZOOM_STEPS.find((s) => s > zoom + eps) ?? MAX_ZOOM;
  return [...ZOOM_STEPS].reverse().find((s) => s < zoom - eps) ?? MIN_ZOOM;
}
