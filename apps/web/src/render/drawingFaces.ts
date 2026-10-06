/**
 * The drawing's typefaces as CSS families (docs/adr/0055) and the faces a text of its own asks for (docs/adr/0183
 * §2): a text with a typeface is drawn in it upright at 400 (600 bold, its italic face when italic; the browser leans
 * a family without one); a text without one keeps the project's typeface, a one-line text italic. A canvas does not
 * ask for a face by itself: `wantFace` loads one the first time a text needs it and tells who listens when it is in,
 * so the overlay draws again.
 */
import type { DrawingFont } from '../model/projectSettings';

const FALLBACK = 'system-ui, sans-serif';

/** Each typeface's CSS family, with what stands in while it loads. */
export const DRAWING_FAMILY: Readonly<Record<DrawingFont, string>> = {
  barlow: `Barlow, ${FALLBACK}`,
  arimo: `Arimo, Arial, ${FALLBACK}`,
  overpass: `Overpass, ${FALLBACK}`,
  quicksand: `Quicksand, ${FALLBACK}`,
  'architects-daughter': `'Architects Daughter', ${FALLBACK}`,
  'courier-prime': `'Courier Prime', 'Courier New', monospace`,
  'plex-mono': `'IBM Plex Mono', ui-monospace, monospace`,
};

/** What of a text's face the drawing reads. */
export interface Face {
  font?: DrawingFont;
  bold?: boolean;
  italic?: boolean;
  oblique?: number;
}

/**
 * The CSS font a text is drawn in at `px`: its own typeface (bold 600, italic) when it has one, else `project` (the
 * palette's drawing family) in the look a text always had, `legacy` (`italic 400`, a multi-line text's run weight…).
 */
export function faceFont(f: Face, px: number, project: string, legacy: string, run?: { bold?: boolean; italic?: boolean }): string {
  if (f.font === undefined) return `${legacy} ${px.toFixed(1)}px ${project}`;
  const italic = f.italic || run?.italic;
  const bold = f.bold || run?.bold;
  const font = `${italic ? 'italic ' : ''}${bold ? 600 : 400} ${px.toFixed(1)}px ${DRAWING_FAMILY[f.font]}`;
  wantFace(font);
  return font;
}

/** A dimension's value's CSS font at `px` (docs/adr/0183 §3): 500 in its own typeface, else in `project`. */
export function valueFont(font: DrawingFont | undefined, px: number, project: string): string {
  if (font === undefined) return `500 ${px.toFixed(1)}px ${project}`;
  const css = `500 ${px.toFixed(1)}px ${DRAWING_FAMILY[font]}`;
  wantFace(css);
  return css;
}

/** The tangent of a text's slant: how far its letters' tops move along per unit up; 0 without a typeface or a slant. */
export const leanOf = (f: Face): number => (f.font !== undefined && f.oblique !== undefined ? Math.tan((f.oblique * Math.PI) / 180) : 0);

const asked = new Set<string>();
const listeners = new Set<() => void>();

/** Loads `font` (a CSS font) the first time it is asked for; who listens hears when it is in. Nothing in a test. */
export function wantFace(font: string): void {
  if (asked.has(font)) return;
  asked.add(font);
  const fonts = typeof document === 'undefined' ? undefined : document.fonts;
  if (!fonts || fonts.check(font)) return;
  fonts
    .load(font, 'Ağİ')
    .then(() => {
      for (const l of listeners) l();
    })
    .catch(() => undefined);
}

/** Listens for faces coming in; returns the way to stop. */
export function onFaceLoaded(listener: () => void): () => void {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}
