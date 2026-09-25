import { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import { LayerStore, type LayerStyle } from '../model/layers';
import type {
  Anchor,
  Color,
  DataDefined,
  FillLayer,
  LayerRenderer,
  LibraryAsset,
  LibrarySymbol,
  LineLayer,
  LineWave,
  MarkerLayer,
  MarkerPlacement,
  MarkerSymbol,
  Rule,
  ShapeName,
  SizeUnit,
  Symbol,
  SymbolRef,
  SymbolSet,
  SymbolType,
} from '../model/style';
import type { CanvasPalette } from '../render/color';
import type { Gen } from '../wasm/calls/harness';
import { entity } from '../wasm/calls/sets/p5-entities';

/**
 * Test support for the style compiler (docs/adr/0008 “Stil derleyicisi”):
 * random symbols of every layer type with fixed and data-defined values
 * (valid, broken and odd expressions), renderers of every kind, image
 * assets and objects with the attributes the expressions read, for the
 * comparison with the TypeScript it replaced and the frozen cases
 * (fixtures/style/v1/cases.json, scripts/fixtures/record-style.test.ts).
 * Sizes stay in the range a drawing uses, so a case draws at most a few
 * thousand markers.
 */

const NUMBERS = ['Kat', '[Kat] * 0.5', 'Genişlik / 2', '$alan / 100', 'yuvarla($uzunluk / 10, 1)', '$ölçek / 500', '$sıra', 'eğer($alan > 100, 3, 1)', 'sayı(Parsel) % 7', 'Yok', '-1', '0', 'Kat /', '$köşe', '($x - $y) % 5', "'2.5'"];
const COLORS = ["eğer(Nitelik = 'Arsa', '#C04020', '#2060A0')", 'Renk', "'ink'", "'#11223344'", "'kırmızı'", "' #aabbcc '", "'fg-dim'", 'Nitelik', '(('];
const TEXTS = ['Parsel', "'E=' || Kat", '$sıra', "Ada || '/' || Parsel", 'büyük(Nitelik)', 'yuvarla($alan, 2)', '$katman', '$tür', 'Yok', "''", '((', "doldur(Parsel, 4, '0')"];
const BOOLS = ['$alan > 50', "Nitelik = 'Arsa'", 'Kat >= 3', 'boş(Yok)', 'Yok', '$sıra % 2 = 0', 'sayı(Parsel) > 40', '1 +', "Nitelik != 'Tarla' ve $uzunluk > 20"];

const FIXED_COLORS: Color[] = ['#1A2B3C', '#FF000080', 'ink', 'paper', 'fg', 'fg-dim', '#abc', '#2E7D32', '#E0457B'];
const UNITS: (SizeUnit | undefined)[] = [undefined, 'mm', 'mm', 'px', 'm'];
const SHAPES: ShapeName[] = ['circle', 'ring', 'square', 'rectangle', 'diamond', 'triangle', 'pentagon', 'hexagon', 'octagon', 'star', 'cross', 'x', 'line', 'arrow', 'arrowhead', 'chevron', 'semicircle', 'quartercircle', 'gear', 'arc'];
const ANCHORS: Anchor[] = ['center', 'top', 'bottom', 'left', 'right', 'top-left', 'top-right', 'bottom-left', 'bottom-right'];
const PLACEMENTS: MarkerPlacement[] = ['interval', 'interval', 'vertex', 'innerVertex', 'first', 'last', 'center', 'segmentCenter'];
const RINGS = [undefined, 'all', 'exterior', 'interior'] as const;

/** Image assets of the library: SVG drawings with colour parameters and a raster; "yok" is missing. */
export const ASSETS: readonly LibraryAsset[] = [
  { kind: 'asset', id: 'agac', name: 'Ağaç', path: ['Test'], format: 'svg', data: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 30"><circle cx="10" cy="10" r="8" fill="param(fill) #2E7D32" stroke="param(stroke)"/><path d="M10 18V30" stroke="currentColor"/></svg>', width: 20, height: 30 },
  { kind: 'asset', id: 'isaret', name: 'İşaret', path: ['Test'], format: 'svg', data: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 12"><rect width="24" height="12" fill="currentColor"/></svg>', width: 24, height: 12 },
  { kind: 'asset', id: 'doku', name: 'Doku', path: ['Test'], format: 'png', data: 'data:image/png;base64,iVBORw0KGgo=', width: 16, height: 8 },
];
const ASSET_IDS = ['agac', 'isaret', 'doku', 'yok'];

/** A dark theme's canvas colours (the core never sees them: the page colours the batches). */
export const PALETTE: CanvasPalette = {
  background: [0.08, 0.1, 0.13, 1],
  fg: '#E4EAF0',
  fgDim: '#A9B4C0',
  ink: '#FFFFFF',
  paper: '#151B22',
  gridMinor: [1, 1, 1, 0.05],
  gridMajor: [1, 1, 1, 0.11],
  accent: '#F2B632',
  snap: '#6FD08C',
  danger: '#EF6B61',
  label: '#C7D0DA',
  labelHalo: '#151B22',
  font: 'system-ui, sans-serif',
};

/** Width and height of the assets that exist (what the page gives the core). */
export const ASSET_SIZES: Record<string, readonly [number, number]> = Object.fromEntries(ASSETS.map((a) => [a.id, [a.width, a.height] as const]));

/** A field that is there only sometimes. */
function opt<K extends string, T>(g: Gen, p: number, key: K, value: () => T): { [k in K]?: T } {
  return (g.chance(p) ? { [key]: value() } : {}) as { [k in K]?: T };
}

/** A fixed value, or an expression with or without a fallback. */
function dd<T>(g: Gen, fixed: () => T, exprs: readonly string[], p = 0.3): DataDefined<T> {
  if (!g.chance(p)) return fixed();
  return { expr: g.pick(exprs), ...opt(g, 0.5, 'fallback', fixed) };
}

const color = (g: Gen) => dd(g, () => g.pick(FIXED_COLORS), COLORS);
const enabled = (g: Gen) => dd(g, () => g.chance(0.85), BOOLS, 0.4);
const dash = (g: Gen) => (g.chance(0.6) ? null : g.pick([[], [3, 1.5], [5, 1.2, 0.6, 1.2], [0.6, 1.2], [2, 0], [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]]));

function base(g: Gen, id: string) {
  return { id, ...opt(g, 0.15, 'enabled', () => enabled(g)), ...opt(g, 0.2, 'opacity', () => g.pick([0, 0.25, 0.5, 1])), ...opt(g, 0.5, 'unit', () => g.pick(UNITS)) };
}

export function markerLayer(g: Gen, id: string): MarkerLayer {
  const common = {
    ...base(g, id),
    size: dd(g, () => g.pick([0, 0.5, 2, 3.5, 6, -1]), NUMBERS),
    ...opt(g, 0.3, 'rotation', () => dd(g, () => g.num(-360, 360), NUMBERS)),
    ...opt(g, 0.3, 'offset', () => [g.num(-3, 3), g.num(-3, 3)] as const),
    ...opt(g, 0.3, 'anchor', () => g.pick(ANCHORS)),
  };
  switch (g.int(0, 5)) {
    case 0:
    case 1:
    case 2:
      return {
        ...common,
        type: 'shape',
        shape: g.pick(SHAPES),
        ...opt(g, 0.3, 'height', () => g.pick([0, 1, 4])),
        ...opt(g, 0.7, 'fill', () => (g.chance(0.2) ? null : color(g))),
        ...opt(g, 0.5, 'stroke', () => (g.chance(0.2) ? null : color(g))),
        ...opt(g, 0.4, 'strokeWidth', () => g.pick([0, 0.2, 0.5])),
        ...opt(g, 0.2, 'hole', () => g.pick([0, 0.4, 0.95, 1.5])),
        ...opt(g, 0.2, 'teeth', () => g.pick([0, 3, 8, 12.5])),
        ...opt(g, 0.2, 'teethDepth', () => g.pick([0.1, 0.3])),
        ...opt(g, 0.2, 'sweep', () => g.pick([0, 90, 180, 400])),
      };
    case 3:
      return { ...common, type: 'svg', asset: g.pick(ASSET_IDS), ...opt(g, 0.5, 'fill', () => color(g)), ...opt(g, 0.3, 'stroke', () => color(g)) };
    case 4:
      return {
        ...common,
        type: 'text',
        text: dd(g, () => g.pick(['Ada', 'SEG', '', 'YAPI YASAKLI', 'Çınar']), TEXTS, 0.5),
        ...opt(g, 0.4, 'font', () => g.pick(['ui', 'sans', 'narrow', 'serif', 'mono'] as const)),
        ...opt(g, 0.3, 'weight', () => g.pick([400, 700, 900] as const)),
        ...opt(g, 0.2, 'italic', () => g.chance(0.5)),
        ...opt(g, 0.5, 'color', () => color(g)),
        ...opt(g, 0.3, 'halo', () => (g.chance(0.3) ? null : { color: g.pick(FIXED_COLORS), width: g.pick([0.3, 1]) })),
      };
    default:
      return { ...common, type: 'raster', asset: g.pick(ASSET_IDS) };
  }
}

export function markerSymbol(g: Gen): MarkerSymbol {
  return { type: 'marker', layers: Array.from({ length: g.int(0, 3) }, (_, i) => markerLayer(g, `m${i}`)) };
}

function wave(g: Gen): LineWave {
  return {
    shape: g.pick(['sine', 'zigzag', 'square'] as const),
    length: g.pick([0.5, 2, 5]),
    amplitude: g.pick([0.3, 1, -0.5]),
    ...opt(g, 0.4, 'spacing', () => g.pick([0, 3, 8])),
    ...opt(g, 0.4, 'connect', () => g.chance(0.5)),
    ...opt(g, 0.4, 'offsetAlong', () => g.pick([0, 1.5, -2])),
  };
}

export function lineLayer(g: Gen, id: string): LineLayer {
  if (g.chance(0.6))
    return {
      ...base(g, id),
      type: 'simpleLine',
      color: color(g),
      width: dd(g, () => g.pick([0, 0.13, 0.5, 1.2]), NUMBERS),
      ...opt(g, 0.4, 'dash', () => dash(g)),
      ...opt(g, 0.2, 'dashOffset', () => g.pick([0, 1, 2.5])),
      ...opt(g, 0.3, 'cap', () => g.pick(['butt', 'round', 'square'] as const)),
      ...opt(g, 0.3, 'join', () => g.pick(['miter', 'round', 'bevel'] as const)),
      ...opt(g, 0.35, 'offset', () => dd(g, () => g.pick([0, 0.8, -1.5, 3]), NUMBERS)),
      ...opt(g, 0.3, 'rings', () => g.pick(RINGS)),
      ...opt(g, 0.2, 'wave', () => wave(g)),
      ...opt(g, 0.15, 'blur', () => g.pick([0, 0.8])),
      ...opt(g, 0.15, 'shift', () => [g.num(-1, 1), g.num(-1, 1)] as const),
    };
  return {
    ...base(g, id),
    type: 'markerLine',
    marker: markerSymbol(g),
    placement: g.pick(PLACEMENTS),
    ...opt(g, 0.7, 'interval', () => g.pick([0, 0.5, 3, 7.5, 20])),
    ...opt(g, 0.4, 'offsetAlong', () => g.pick([0, 1, 4, -2])),
    ...opt(g, 0.3, 'offset', () => dd(g, () => g.pick([0, 1, -2]), NUMBERS)),
    ...opt(g, 0.5, 'rotate', () => g.chance(0.7)),
    ...opt(g, 0.3, 'rings', () => g.pick(RINGS)),
    ...opt(g, 0.2, 'group', () => ({ count: g.pick([0, 1, 2, 3]), spacing: g.pick([0, 0.8, 2]) })),
  };
}

export function fillLayer(g: Gen, id: string): FillLayer {
  switch (g.int(0, 7)) {
    case 0:
      return { ...base(g, id), type: 'simpleFill', color: color(g) };
    case 1:
      return {
        ...base(g, id),
        type: 'hatchFill',
        angle: g.pick([0, 45, 90, 137.5, -30]),
        spacing: g.pick([0, 1, 2.5, 6]),
        width: g.pick([0, 0.18, 0.5]),
        color: color(g),
        ...opt(g, 0.3, 'offset', () => g.pick([0, 0.5])),
        ...opt(g, 0.3, 'dash', () => dash(g)),
        ...opt(g, 0.2, 'dashOffset', () => g.pick([0, 1])),
      };
    case 2:
      return {
        ...base(g, id),
        type: 'patternFill',
        marker: markerSymbol(g),
        spacingX: g.pick([0, 2, 5]),
        spacingY: g.pick([0, 2, 5, 8]),
        ...opt(g, 0.4, 'stagger', () => g.chance(0.5)),
        ...opt(g, 0.3, 'angle', () => g.pick([0, 30, 90])),
        ...opt(g, 0.3, 'offset', () => [g.num(-1, 1), g.num(-1, 1)] as const),
        ...opt(g, 0.3, 'jitter', () => g.pick([0, 0.5, 1])),
        ...opt(g, 0.3, 'coverage', () => g.pick([0, 0.4, 1])),
        ...opt(g, 0.3, 'seed', () => g.int(0, 99)),
      };
    case 3:
      return { ...base(g, id), type: 'imageFill', asset: g.pick(ASSET_IDS), tileSize: g.pick([0, 4, 10]), ...opt(g, 0.3, 'angle', () => g.pick([0, 45])) };
    case 4:
      return { ...base(g, id), type: 'centroidMarker', marker: markerSymbol(g), ...opt(g, 0.6, 'position', () => g.pick(['pointOnSurface', 'centroid'] as const)) };
    default:
      return lineLayer(g, id);
  }
}

/** A symbol of any type (or the one asked for), with up to four layers. */
export function randomSymbol(g: Gen, type: SymbolType = g.pick(['marker', 'line', 'fill'] as const)): Symbol {
  const n = g.int(0, 4);
  if (type === 'marker') return { type, layers: Array.from({ length: n }, (_, i) => markerLayer(g, `m${i}`)) };
  if (type === 'line') return { type, layers: Array.from({ length: n }, (_, i) => lineLayer(g, `l${i}`)) };
  return { type, layers: Array.from({ length: n }, (_, i) => fillLayer(g, `f${i}`)) };
}

// ── Renderers ──────────────────────────────────────────────────────────

/** A library reference (sometimes to a symbol the library lacks) or a symbol written in place. */
function ref(g: Gen, refs: readonly string[], type: SymbolType): SymbolRef {
  if (g.chance(0.5) && refs.length) return { ref: g.chance(0.1) ? 'yok' : g.pick(refs) };
  return randomSymbol(g, g.chance(0.8) ? type : undefined);
}

export function randomSet(g: Gen, refs: readonly string[]): SymbolSet {
  return { ...opt(g, 0.6, 'marker', () => ref(g, refs, 'marker')), ...opt(g, 0.7, 'line', () => ref(g, refs, 'line')), ...opt(g, 0.6, 'fill', () => ref(g, refs, 'fill')) };
}

function rules(g: Gen, refs: readonly string[], depth: number): Rule[] {
  return Array.from({ length: g.int(1, 4) }, (_, i) => ({
    id: `r${depth}.${i}`,
    label: `Kural ${i + 1}`,
    ...opt(g, 0.6, 'filter', () => (g.chance(0.1) ? '' : g.pick(BOOLS))),
    ...opt(g, 0.2, 'isElse', () => g.chance(0.8)),
    ...opt(g, 0.25, 'minScale', () => g.pick([0, 500, 1000, 2500])),
    ...opt(g, 0.25, 'maxScale', () => g.pick([200, 1000, 5000])),
    ...opt(g, 0.8, 'symbols', () => randomSet(g, refs)),
    ...opt(g, depth < 2 ? 0.3 : 0, 'children', () => rules(g, refs, depth + 1)),
    ...opt(g, 0.15, 'enabled', () => g.chance(0.3)),
  }));
}

export function randomRenderer(g: Gen, refs: readonly string[]): LayerRenderer {
  switch (g.int(0, 3)) {
    case 0:
      return { type: 'single', symbols: randomSet(g, refs) };
    case 1:
      return {
        type: 'categorized',
        expr: g.pick(['Nitelik', 'Kat', ...TEXTS]),
        categories: ['Arsa', 'Bahçe', 'Tarla', '', '3', '12'].filter(() => g.chance(0.6)).map((value) => ({ value, label: value || '(boş)', symbols: randomSet(g, refs), ...opt(g, 0.15, 'enabled', () => false) })),
        ...opt(g, 0.5, 'other', () => randomSet(g, refs)),
      };
    case 2: {
      const cuts = [-5, 0, 2, 3.5, 10, 50, 500].filter(() => g.chance(0.6));
      return { type: 'graduated', expr: g.pick(NUMBERS), classes: cuts.slice(1).map((max, i) => ({ min: cuts[i], max, label: `${cuts[i]}–${max}`, symbols: randomSet(g, refs) })) };
    }
    default:
      return { type: 'rules', rules: rules(g, refs, 0) };
  }
}

/** A layer's own look: colour, line type and weight, fill, point symbol, and sometimes a renderer. */
export function randomLayerStyle(g: Gen, refs: readonly string[]): LayerStyle {
  return {
    color: g.pick(FIXED_COLORS),
    lineType: g.pick(['continuous', 'dashed', 'dashdot', 'dotted'] as const),
    lineWeight: g.pick([0, 0.18, 0.35, 0.7]),
    ...opt(g, 0.4, 'fill', () => g.pick(['#7FB2E526', '#FFE08A80'])),
    ...opt(g, 0.4, 'point', () => ({ symbol: g.pick(['ring', 'cross', 'triangle'] as const), size: g.pick([4, 7, 10]) })),
    ...opt(g, 0.65, 'renderer', () => randomRenderer(g, refs)),
  };
}

// ── Objects ────────────────────────────────────────────────────────────

/** An object of any kind with the attributes the expressions read, sometimes its own colour or symbol. */
export function randomObject(g: Gen, id: number, layerId: string, refs: readonly string[], kind?: Entity['kind']): Entity {
  const e = entity(g, kind);
  const attrs: Record<string, string> = {
    ...opt(g, 0.7, 'Kat', () => g.pick(['1', '3', '4', '12', '', 'x'])),
    ...opt(g, 0.7, 'Nitelik', () => g.pick(['Arsa', 'Bahçe', 'Tarla', ''])),
    ...opt(g, 0.5, 'Genişlik', () => g.pick(['7', '12.5', '-3', '', 'geniş'])),
    ...opt(g, 0.4, 'Renk', () => g.pick(['#aa0000', 'ink', 'bad', '#12345678'])),
    ...opt(g, 0.7, 'Parsel', () => String(g.int(1, 99))),
    ...opt(g, 0.6, 'Ada', () => String(g.int(1, 999))),
  };
  const { color: _c, symbol: _s, ...rest } = e as Entity & { color?: string; symbol?: string };
  return { ...rest, id, layerId, attrs, ...opt(g, 0.2, 'color', () => g.pick(FIXED_COLORS)), ...opt(g, 0.15, 'symbol', () => (g.chance(0.1) ? 'yok' : g.pick(refs))) } as Entity;
}

// ── Layers ─────────────────────────────────────────────────────────────

/** A library of four random symbols and three of the system's, by id. */
export function randomLibrary(g: Gen, system: readonly LibrarySymbol[]): Map<string, Symbol> {
  const library = new Map<string, Symbol>();
  for (let k = 0; k < 4; k++) library.set(`s${k}`, randomSymbol(g));
  for (let k = 0; k < 3; k++) {
    const s = g.pick(system);
    library.set(s.id, s.symbol);
  }
  return library;
}

/** A drawing of one styled layer ("k"), as a layer build sees it. */
export function layerDocument(name: string, style: LayerStyle, entities: readonly NewEntity[]): CadDocument {
  const doc = new CadDocument({ name: 'Stil', layers: new LayerStore([{ id: 'k', name, style }], 'k'), origin: { x: 0, y: 0 } });
  doc.load([...entities]);
  return doc;
}

/** A styled layer to build: its drawing, and the view's origin, plot scale and clip box. */
export interface LayerScene {
  readonly doc: CadDocument;
  readonly name: string;
  readonly style: LayerStyle;
  readonly origin: Vec2;
  readonly plotScale: number;
  readonly clip?: Bounds;
}

export function layerScene(g: Gen, refs: readonly string[], maxObjects = 14): LayerScene {
  g.frame();
  const style = randomLayerStyle(g, refs);
  const name = g.pick(['Parsel sınırı', 'Yapı']);
  const doc = layerDocument(
    name,
    style,
    Array.from({ length: g.int(0, maxObjects) }, (_, i) => randomObject(g, i + 1, 'k', refs)),
  );
  const near = doc.size ? doc.bounds() : null;
  const origin: Vec2 = near && g.chance(0.7) ? { x: Math.round(near.minX), y: Math.round(near.minY) } : { x: 0, y: 0 };
  const plotScale = g.pick([500, 1000, 2000, 5000]);
  const c = near ? { x: g.num(near.minX, near.maxX), y: g.num(near.minY, near.maxY) } : { x: 0, y: 0 };
  const w = g.pick([20, 200, 5000]);
  const clip: Bounds | undefined = g.chance(0.6) ? { minX: c.x - w, minY: c.y - w, maxX: c.x + w, maxY: c.y + w } : undefined;
  return { doc, name, style: doc.layers.get('k')!.style, origin, plotScale, ...(clip ? { clip } : {}) };
}
