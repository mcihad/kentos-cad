/**
 * The scale a drawing's paper-mm symbol sizes are compiled at (docs/STYLE.md
 * §6): the project's plot scale, or with screen-sized symbols (Görünüm →
 * Semboller → Ekranda sabit) the view's own scale denominator, rounded to
 * quarter-octave steps so a zoom does not rebuild every layer at every
 * wheel tick. Pure, so fixtures/style/v1/batches.json pins it for both
 * platforms.
 */

/** Metres of paper per CSS pixel at 96 dpi (0.26458 mm). */
export const METRES_PER_PX = 0.00026458;

/** `size`: the symbol size mode; `plotScale`: the project's; `pxPerM`: the view's CSS px per metre. */
export function symbolScaleOf(size: 'plot' | 'screen', plotScale: number, pxPerM: number): number {
  if (size !== 'screen') return plotScale;
  const denominator = 1 / (pxPerM * METRES_PER_PX);
  return 2 ** (Math.round(Math.log2(Math.max(denominator, 1)) * 4) / 4);
}
