import type { Color, MarkerLayer, Symbol, SymbolLayer } from '../model/style';

/**
 * The thematic renderers' value rules on the page (docs/adr/0213 §2), the
 * style core's (`style/thematic.rs`) for the window and the legend: a ramp's
 * colour at a share, a value's share and step of 256, a size from it, a
 * value's class among breaks, and a symbol with its main colour or its size
 * changed (§2.10). The legend's rows rest on them; fixtures/style/v1/legend.json
 * holds the desktop's (crates/native/style/src/legend.rs) to the same.
 */

const HEX = /^#([0-9a-f]{6}|[0-9a-f]{8})$/i;

/** `#RRGGBB` or `#RRGGBBAA` as red, green, blue, alpha; null for anything else. */
export function parseRgba(c: string): [number, number, number, number] | null {
  const s = c.trim();
  if (!HEX.test(s)) return null;
  const v = [1, 3, 5, 7].map((i) => (i < s.length ? parseInt(s.slice(i, i + 2), 16) : 255));
  return [v[0], v[1], v[2], v[3]];
}

/** `#RRGGBB` (opaque) or `#RRGGBBAA`. */
export function hexOf(c: readonly number[]): string {
  const h = (v: number) => v.toString(16).padStart(2, '0').toUpperCase();
  return `#${h(c[0])}${h(c[1])}${h(c[2])}${c[3] === 255 ? '' : h(c[3])}`;
}

/** The ramp's colour at `t` (0–1): equally spaced stops, channels mixed straight, halves rounded up. */
export function rampAt(stops: readonly Color[], t: number): string {
  const s = stops.map(parseRgba).filter((c): c is [number, number, number, number] => !!c);
  if (!s.length) return '#000000';
  if (s.length === 1) return hexOf(s[0]);
  const u = Number.isNaN(t) ? 0 : Math.min(1, Math.max(0, t));
  const at = u * (s.length - 1);
  const k = Math.min(Math.floor(at), s.length - 2);
  const f = at - k;
  const [a, b] = [s[k], s[k + 1]];
  return hexOf(a.map((v, i) => Math.min(255, Math.max(0, Math.round(v + (b[i] - v) * f)))));
}

/** A value's share of [min, max], clipped; 0 when the range is empty. */
export function shareOf(v: number, min: number, max: number): number {
  if (!(max - min > 0)) return 0;
  return Math.min(1, Math.max(0, (v - min) / (max - min)));
}

/** The share's step of 256. */
export function stepOf(t: number): number {
  return Math.round(255 * Math.min(1, Math.max(0, t)));
}

export const EXPONENTS = { area: 0.5, radius: 1, flannery: 0.57 } as const;

/** Orantılı sembol's size at share `t`: `min + (max − min) · tᵉ`. */
export function sizeAt(t: number, min: number, max: number, e: number): number {
  const u = Math.min(1, Math.max(0, t));
  return min + (max - min) * (e === 1 ? u : u ** e);
}

/** A value's class among ascending breaks: how many it reaches. */
export function classOf(v: number, breaks: readonly number[]): number {
  return breaks.filter((b) => v >= b).length;
}

const OPEN = new Set(['cross', 'x', 'line', 'arrow', 'chevron', 'arc']);

function recolorMarker(m: MarkerLayer, c: string): MarkerLayer {
  switch (m.type) {
    case 'shape':
      return OPEN.has(m.shape) ? { ...m, stroke: c } : { ...m, fill: c };
    case 'svg':
      return { ...m, fill: c };
    case 'text':
      return { ...m, color: c };
    default:
      return m;
  }
}

const recolorMarkers = (layers: readonly MarkerLayer[], c: string) => layers.map((m) => recolorMarker(m, c));

/** The symbol with its main colour `c` (docs/adr/0213 §2.10): fills (not edges), lines, marks. */
export function withColor(symbol: Symbol, c: string): Symbol {
  const layers = symbol.layers.map((l: SymbolLayer): SymbolLayer => {
    if (symbol.type === 'fill') {
      switch (l.type) {
        case 'simpleFill':
        case 'hatchFill':
        case 'gradientFill':
          return { ...l, color: c };
        case 'patternFill':
        case 'centroidMarker':
          return { ...l, marker: { ...l.marker, layers: recolorMarkers(l.marker.layers, c) } };
        default:
          return l;
      }
    }
    if (symbol.type === 'line') {
      if (l.type === 'simpleLine') return { ...l, color: c };
      if (l.type === 'markerLine') return { ...l, marker: { ...l.marker, layers: recolorMarkers(l.marker.layers, c) } };
      return l;
    }
    return l.type === 'shape' || l.type === 'svg' || l.type === 'text' ? recolorMarker(l, c) : l;
  });
  return { ...symbol, layers } as Symbol;
}

const MM_PER_PX = 25.4 / 96;
const convert = (v: number, from: string | undefined, to: 'mm' | 'px') => ((from ?? 'mm') === 'mm' && to === 'px' ? v / MM_PER_PX : from === 'px' && to === 'mm' ? v * MM_PER_PX : v);
const fixedOr = (v: unknown, d: number) => (typeof v === 'number' ? v : v && typeof v === 'object' && typeof (v as { fallback?: unknown }).fallback === 'number' ? (v as { fallback: number }).fallback : d);

/** A marker symbol's size in `unit` (its largest layer's; a layer without one is 2). */
export function markerSize(symbol: Symbol, unit: 'mm' | 'px'): number {
  return symbol.layers.reduce((m, l) => (l.type === 'shape' || l.type === 'svg' || l.type === 'text' || l.type === 'raster' ? Math.max(m, convert(fixedOr(l.size, 2), l.unit, unit)) : m), 0);
}

/** A line symbol's width in `unit` (its widest line's). */
export function lineWidth(symbol: Symbol, unit: 'mm' | 'px'): number {
  return symbol.layers.reduce((m, l) => (l.type === 'simpleLine' ? Math.max(m, convert(fixedOr(l.width, 0), l.unit, unit)) : m), 0);
}

function scaleMarker(m: MarkerLayer, k: number): MarkerLayer {
  const out = { ...m, size: fixedOr(m.size, 2) * k } as MarkerLayer;
  if (m.type === 'shape' && typeof m.height === 'number') (out as { height?: number }).height = m.height * k;
  if (m.offset) (out as { offset?: readonly [number, number] }).offset = [m.offset[0] * k, m.offset[1] * k];
  return out;
}

/** The symbol scaled to `size` in `unit` (docs/adr/0213 §2.2): a marker's layers, a line's widths. */
export function withSize(symbol: Symbol, size: number, unit: 'mm' | 'px'): Symbol {
  if (symbol.type === 'marker') {
    const now = markerSize(symbol, unit);
    if (!(now > 0)) return symbol;
    const k = size / now;
    return { ...symbol, layers: symbol.layers.map((l) => (l.type === 'shape' || l.type === 'svg' || l.type === 'text' || l.type === 'raster' ? scaleMarker(l, k) : l)) } as Symbol;
  }
  if (symbol.type === 'line') {
    const now = lineWidth(symbol, unit);
    if (!(now > 0)) return symbol;
    const k = size / now;
    return {
      ...symbol,
      layers: symbol.layers.map((l) => {
        if (l.type === 'simpleLine') return { ...l, width: fixedOr(l.width, 0) * k, ...(typeof l.offset === 'number' && { offset: l.offset * k }) };
        if (l.type === 'markerLine') return { ...l, marker: { ...l.marker, layers: l.marker.layers.map((m) => scaleMarker(m, k)) } };
        return l;
      }),
    } as Symbol;
  }
  return symbol;
}

// ── Ready-made colours ──────────────────────────────────────────────────

/** Heat maps' ramps (docs/adr/0213 §2.6): clear at nothing. */
export const HEAT_RAMPS: Record<string, { label: string; stops: readonly Color[] }> = {
  isi: { label: 'Isı', stops: ['#2B83BA00', '#2B83BA', '#ABDDA4', '#FFFFBF', '#FDAE61', '#D7191C'] },
  sariKirmizi: { label: 'Sarıdan kırmızıya', stops: ['#FFF5B800', '#FFF5B8', '#FDB863', '#E66101', '#A50F15'] },
  maviler: { label: 'Maviler', stops: ['#E3EEF900', '#9ECAE1', '#4292C6', '#08306B'] },
  griler: { label: 'Griler', stops: ['#F0F0F000', '#BDBDBD', '#737373', '#252525'] },
};

/** İki değişkenli renk's grids by their corners: low-low, high X, high Y, high-high (Joshua Stevens's schemes). */
export const BIVARIATE_SCHEMES: Record<string, { label: string; corners: readonly [Color, Color, Color, Color] }> = {
  pembeMavi: { label: 'Pembe – mavi', corners: ['#E8E8E8', '#5AC8C8', '#BE64AC', '#3B4994'] },
  yesilMor: { label: 'Yeşil – mor', corners: ['#E8E8E8', '#73AE80', '#6C83B5', '#2A5A5B'] },
  turuncuMavi: { label: 'Turuncu – mavi', corners: ['#E8E8E8', '#E4ACAC', '#ACE4E4', '#5A5A9A'] },
};

/** An n × n grid from its corners (row by row, Y's class the row, X's the column), mixed straight both ways. */
export function bivariateColors(corners: readonly [Color, Color, Color, Color], n: number): Color[] {
  const [a, b, c, d] = corners.map((x) => parseRgba(x) ?? [0, 0, 0, 255]);
  const out: Color[] = [];
  for (let j = 0; j < n; j++) {
    for (let i = 0; i < n; i++) {
      const u = n > 1 ? i / (n - 1) : 0;
      const w = n > 1 ? j / (n - 1) : 0;
      out.push(hexOf(a.map((v, k) => Math.round(v * (1 - u) * (1 - w) + b[k] * u * (1 - w) + c[k] * (1 - u) * w + d[k] * u * w))));
    }
  }
  return out;
}
