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
  /** The interface typeface (Uygulama ayarları → Yazı tipi), for the overlay's own marks. */
  font: string;
  /** The drawing's typeface (Proje ayarları → Çizim yazı tipi): text objects, dimension values, labels. */
  drawingFont: string;
  /** Görünüm kipleri's rule for a colour the drawing names (`modedPalette`, docs/adr/0195 §1); none: as it is. */
  shown?: (hex: string) => string;
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
    font: v('--font-ui', 'system-ui, sans-serif'),
    drawingFont: v('--font-drawing', 'Barlow, system-ui, sans-serif'),
  };
}

/** Theme tokens a layer or entity colour may use instead of a hex value. */
export const THEME_COLORS = ['fg', 'fg-dim', 'ink'] as const;

export function resolveColor(color: string, palette: CanvasPalette): string {
  if (color === 'fg') return palette.fg;
  if (color === 'fg-dim') return palette.fgDim;
  if (color === 'ink') return palette.ink;
  if (color === 'paper') return palette.paper;
  return palette.shown ? palette.shown(color) : color;
}

/** Görünüm kipleri's colour mode (`graphics.colorMode`, docs/adr/0195 §1). */
export type ColorMode = 'color' | 'mono' | 'gray';

/** How the page colours a layer (docs/adr/0195 §2): the colour mode, and whether fills are drawn opaque (Saydamlık off). */
export interface ViewColors {
  mode: ColorMode;
  opaque: boolean;
}

/**
 * `c` as the mode draws it, its alpha kept (the desktop's `view_rgba`): in one colour the palette's ink, in gray its
 * brightness `0.2126 r + 0.7152 g + 0.0722 b` over the sRGB values (Rec. 709).
 */
export function viewRgba(c: RGBA, mode: ColorMode, palette: Pick<CanvasPalette, 'ink'>): RGBA {
  if (mode === 'mono') {
    const ink = parseHex(palette.ink);
    return [ink[0], ink[1], ink[2], c[3]];
  }
  if (mode === 'gray') {
    const y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    return [y, y, y, c[3]];
  }
  return c;
}

/**
 * A resolved colour's text as the mode draws it (a text's colour, an SVG symbol's parameter; the desktop's
 * `view_hex`): `#RRGGBB`, with its alpha byte when it is not opaque; a colour that does not read stays as it is.
 */
export function viewHex(hex: string, mode: ColorMode, palette: Pick<CanvasPalette, 'ink'>): string {
  if (mode === 'color') return hex;
  const c = viewRgba(parseHex(hex), mode, palette);
  if (c.some((v) => Number.isNaN(v))) return hex;
  const byte = (v: number) => Math.min(255, Math.max(0, Math.round(v * 255))).toString(16).toUpperCase().padStart(2, '0');
  return `#${byte(c[0])}${byte(c[1])}${byte(c[2])}${c[3] < 1 ? byte(c[3]) : ''}`;
}

/** A fill's alpha as Saydamlık draws it: off, whatever shows at all is opaque. */
export const viewAlpha = (a: number, opaque: boolean) => (opaque && a > 0 ? 1 : a);

/** Görünüm kipleri as the drawing area reads them from the settings (docs/adr/0195 §1). */
export interface ViewModes {
  colorMode: ColorMode;
  fills: boolean;
  areaEdges: boolean;
  transparency: boolean;
}

/** The selection and hover highlight's colour (`graphics.highlightColor`, docs/adr/0195 §1). */
export type HighlightColor = 'accent' | 'orange' | 'red' | 'green' | 'cyan' | 'magenta';

/** The named highlight colours (the desktop's `highlight_hex`). */
export const HIGHLIGHT_COLORS: Readonly<Record<Exclude<HighlightColor, 'accent'>, string>> = {
  orange: '#F5A623',
  red: '#E5484D',
  green: '#30A46C',
  cyan: '#05A2C2',
  magenta: '#D6409F',
};

/** The highlight's colour: the theme's accent, or a named one. */
export const highlightHex = (c: HighlightColor, accent: string): string => (c === 'accent' ? accent : (HIGHLIGHT_COLORS[c] ?? accent));

/**
 * The palette the drawing's texts are drawn with in a colour mode (docs/adr/0195 §1): their inks (label, fg, fg-dim)
 * and every colour they name as the mode shows them; the halo and the paper keep the ground's colours.
 */
export function modedPalette(pal: CanvasPalette, mode: ColorMode): CanvasPalette {
  if (mode === 'color') return pal;
  const shown = (hex: string) => viewHex(hex, mode, pal);
  return { ...pal, label: shown(pal.label), fg: shown(pal.fg), fgDim: shown(pal.fgDim), ink: shown(pal.ink), shown };
}
