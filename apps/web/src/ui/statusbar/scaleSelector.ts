import type { Axes } from '../../app/format';
import { CAD_SCALES, GIS_SCALES } from '../../model/newProjectWizard';

/**
 * The status bar's scale selector (docs/adr/0165 §5; StatusBar.ts, the desktop's screen_scale.rs): the scales it
 * offers and a typed scale's reading, apart from the DOM.
 */

/** A scale typed as 1:N, N, or N with dots between its digits: a whole number over 0 (the desktop's `typed_scale`). */
export function typedScale(text: string): number | null {
  const t = text.trim().replace(/^1:/, '').replace(/[.\s]/g, '');
  if (!/^\d+$/.test(t)) return null;
  const n = Number(t);
  return n > 0 && n < 1e9 ? n : null;
}

/** The scales offered: the project's type's (the new project wizard's), and a wider one for a map. */
export function offeredScales(axes: Axes): readonly number[] {
  return axes === 'cad' ? CAD_SCALES : [...GIS_SCALES, 100_000];
}
