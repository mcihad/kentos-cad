import type { RGBA } from './types';

/** "#RRGGBB" or "#RRGGBBAA" → linear-ish 0..1 RGBA (sRGB values, no conversion). */
export function parseHex(hex: string, alphaMul = 1): RGBA {
  const h = hex.replace('#', '');
  const full = h.length === 3 ? h.split('').map((c) => c + c).join('') : h;
  const r = parseInt(full.slice(0, 2), 16) / 255;
  const g = parseInt(full.slice(2, 4), 16) / 255;
  const b = parseInt(full.slice(4, 6), 16) / 255;
  const a = full.length >= 8 ? parseInt(full.slice(6, 8), 16) / 255 : 1;
  return [r, g, b, a * alphaMul];
}

export const withAlpha = (c: RGBA, a: number): RGBA => [c[0], c[1], c[2], a];

/** Canvas colours derived from CSS custom properties, refreshed on theme change. */
export interface CanvasPalette {
  background: RGBA;
  fg: string;
  fgDim: string;
  /** CAD colour 7 ("siyah"): black on light backgrounds, white on dark. */
  ink: string;
  /** The sheet itself: white on paper and the light theme, the canvas colour on dark (knockouts, "white" insides). */
  paper: string;
  gridMinor: RGBA;
  gridMajor: RGBA;
  accent: string;
  snap: string;
  /** Parts about to be removed (trim preview). */
  danger: string;
  label: string;
  labelHalo: string;
}

export function readCanvasPalette(el: Element = document.documentElement): CanvasPalette {
  const cs = getComputedStyle(el);
  const v = (name: string, fallback: string) => cs.getPropertyValue(name).trim() || fallback;
  return {
    background: parseHex(v('--canvas-bg', '#151B22')),
    fg: v('--canvas-fg', '#E4EAF0'),
    fgDim: v('--canvas-fg-dim', '#A9B4C0'),
    ink: v('--canvas-ink', '#FFFFFF'),
    paper: v('--canvas-bg', '#151B22'),
    gridMinor: parseHex(v('--canvas-grid-minor', '#FFFFFF0D')),
    gridMajor: parseHex(v('--canvas-grid-major', '#FFFFFF1C')),
    accent: v('--canvas-accent', '#F2B632'),
    snap: v('--canvas-snap', '#6FD08C'),
    danger: v('--c-danger', '#EF6B61'),
    label: v('--canvas-label', '#C7D0DA'),
    labelHalo: v('--canvas-bg', '#151B22'),
  };
}

/** Theme tokens a layer or entity colour may use instead of a hex value. */
export const THEME_COLORS = ['fg', 'fg-dim', 'ink'] as const;

export function resolveColor(color: string, palette: CanvasPalette): string {
  if (color === 'fg') return palette.fg;
  if (color === 'fg-dim') return palette.fgDim;
  if (color === 'ink') return palette.ink;
  if (color === 'paper') return palette.paper;
  return color;
}
