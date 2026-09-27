import type { PreviewGeometry } from '../../render/symbolPreview';
import type { Anchor, FillLayer, LineLayer, MarkerLayer, MarkerPlacement, ShapeName, SizeUnit, Symbol, TextMarker } from '../../model/style';
import type { GeometryClass } from '../../style/geometry';

/**
 * The symbol designer's model (docs/STYLE.md §7, docs/adr/0094), without
 * the page: what each symbol layer type is called and how a new one starts,
 * a layer's one-line summary, form patches turned into layers, and the
 * layer list's edits (add, move, duplicate, remove, switch off) on a symbol
 * as plain JSON. fixtures/style/v1/designer.json holds it; the desktop's
 * designer (crates/native/style/src/designer.rs) checks the same file.
 */

export type AnyLayer = FillLayer | LineLayer | MarkerLayer;
export type LayerType = AnyLayer['type'];
export type SymbolKind = Symbol['type'];

export const LAYER_LABEL: Record<LayerType, string> = {
  simpleFill: 'Dolgu',
  hatchFill: 'Tarama',
  patternFill: 'Desen',
  imageFill: 'Görüntü dolgusu',
  centroidMarker: 'İç noktada işaret',
  simpleLine: 'Çizgi',
  markerLine: 'Çizgi boyunca işaret',
  shape: 'Şekil',
  svg: 'SVG çizimi',
  text: 'Yazı',
  raster: 'Görüntü',
};

export const LAYER_TYPES: Record<SymbolKind, LayerType[]> = {
  fill: ['simpleFill', 'hatchFill', 'patternFill', 'imageFill', 'simpleLine', 'markerLine', 'centroidMarker'],
  line: ['simpleLine', 'markerLine'],
  marker: ['shape', 'text', 'svg', 'raster'],
};

/** Layers that hold a marker symbol of their own (edited as child rows). */
export const hasMarker = (l: AnyLayer | undefined): l is Extract<AnyLayer, { marker: unknown }> =>
  !!l && (l.type === 'markerLine' || l.type === 'patternFill' || l.type === 'centroidMarker');

const dot = (id: string): MarkerLayer => ({ id, type: 'shape', shape: 'circle', size: 1, fill: 'ink' });

export function newLayer(type: LayerType, id: string, context: SymbolKind): AnyLayer {
  switch (type) {
    case 'simpleFill':
      return { id, type, color: '#C9D6E3' };
    case 'hatchFill':
      return { id, type, angle: 45, spacing: 2, width: 0.2, color: 'ink' };
    case 'patternFill':
      return { id, type, spacingX: 3, spacingY: 3, marker: { type: 'marker', layers: [dot('0')] } };
    case 'imageFill':
      return { id, type, asset: '', tileSize: 5 };
    case 'centroidMarker':
      return { id, type, marker: { type: 'marker', layers: [{ id: '0', type: 'text', text: { expr: 'etiket', fallback: 'A' }, size: 3, font: 'sans', weight: 700, color: 'ink' }] } };
    case 'simpleLine':
      return { id, type, color: 'ink', width: context === 'fill' ? 0.25 : 0.35 };
    case 'markerLine':
      return { id, type, placement: 'interval', interval: 6, offsetAlong: 3, rotate: true, marker: { type: 'marker', layers: [dot('0')] } };
    case 'shape':
      return { id, type, shape: 'circle', size: 3, fill: 'ink', stroke: null, strokeWidth: 0.2 };
    case 'svg':
      return { id, type, asset: '', size: 5, fill: 'ink' };
    case 'text':
      return { id, type, text: 'A', size: 3, font: 'sans', weight: 700, color: 'ink' };
    case 'raster':
      return { id, type, asset: '', size: 5 };
  }
}

// ── Option lists ───────────────────────────────────────────────────────

export const UNITS: { value: SizeUnit; label: string }[] = [
  { value: 'mm', label: 'Kâğıt mm' },
  { value: 'px', label: 'Ekran px' },
  { value: 'm', label: 'Harita m' },
];

export const SHAPES: { value: ShapeName; label: string }[] = [
  { value: 'circle', label: 'Daire' },
  { value: 'ring', label: 'Halka (noktalı)' },
  { value: 'square', label: 'Kare' },
  { value: 'rectangle', label: 'Dikdörtgen' },
  { value: 'diamond', label: 'Baklava' },
  { value: 'triangle', label: 'Üçgen' },
  { value: 'pentagon', label: 'Beşgen' },
  { value: 'hexagon', label: 'Altıgen' },
  { value: 'octagon', label: 'Sekizgen' },
  { value: 'star', label: 'Yıldız' },
  { value: 'cross', label: 'Artı' },
  { value: 'x', label: 'Çarpı' },
  { value: 'line', label: 'Çizgi' },
  { value: 'arrow', label: 'Ok' },
  { value: 'arrowhead', label: 'Ok ucu' },
  { value: 'chevron', label: 'Açık ok ucu (V)' },
  { value: 'semicircle', label: 'Yarım daire' },
  { value: 'quartercircle', label: 'Çeyrek daire' },
  { value: 'gear', label: 'Dişli' },
  { value: 'arc', label: 'Yay (açık)' },
];

/** Shapes drawn as lines only: they have no fill and no hole. */
export const OPEN_SHAPES: ReadonlySet<ShapeName> = new Set<ShapeName>(['cross', 'x', 'line', 'arrow', 'chevron', 'arc']);

export const PLACEMENTS: { value: MarkerPlacement; label: string }[] = [
  { value: 'interval', label: 'Aralıklı' },
  { value: 'vertex', label: 'Her köşede' },
  { value: 'innerVertex', label: 'İç köşelerde' },
  { value: 'first', label: 'Başta' },
  { value: 'last', label: 'Sonda' },
  { value: 'center', label: 'Ortada' },
  { value: 'segmentCenter', label: 'Kenar ortalarında' },
];

export const ANCHORS: { value: Anchor; label: string }[] = [
  { value: 'center', label: 'Orta' },
  { value: 'top', label: 'Üst' },
  { value: 'bottom', label: 'Alt' },
  { value: 'left', label: 'Sol' },
  { value: 'right', label: 'Sağ' },
  { value: 'top-left', label: 'Sol üst' },
  { value: 'top-right', label: 'Sağ üst' },
  { value: 'bottom-left', label: 'Sol alt' },
  { value: 'bottom-right', label: 'Sağ alt' },
];

export const FONTS: { value: NonNullable<TextMarker['font']>; label: string }[] = [
  { value: 'sans', label: 'Arial' },
  { value: 'narrow', label: 'Arial Narrow' },
  { value: 'serif', label: 'Times' },
  { value: 'ui', label: 'Arayüz yazısı' },
  { value: 'mono', label: 'Eş aralıklı' },
];

export const WEIGHTS: { value: string; label: string }[] = [
  { value: '400', label: 'Normal' },
  { value: '500', label: 'Orta' },
  { value: '600', label: 'Yarı kalın' },
  { value: '700', label: 'Kalın' },
  { value: '900', label: 'Siyah (Arial Black)' },
];

export const CAPS: { value: 'butt' | 'round' | 'square'; label: string }[] = [
  { value: 'butt', label: 'Düz' },
  { value: 'round', label: 'Yuvarlak' },
  { value: 'square', label: 'Kare' },
];

export const RINGS: { value: 'all' | 'exterior' | 'interior'; label: string }[] = [
  { value: 'all', label: 'Hepsi' },
  { value: 'exterior', label: 'Yalnızca dış sınır' },
  { value: 'interior', label: 'Yalnızca adalar' },
];

export const WAVES: { value: 'none' | 'sine' | 'zigzag' | 'square'; label: string }[] = [
  { value: 'none', label: 'Düz çizgi' },
  { value: 'sine', label: 'Dalga (sinüs)' },
  { value: 'zigzag', label: 'Zikzak' },
  { value: 'square', label: 'Kare dalga' },
];

export const POSITIONS: { value: 'pointOnSurface' | 'centroid'; label: string }[] = [
  { value: 'pointOnSurface', label: 'Alanın içinde (her zaman)' },
  { value: 'centroid', label: 'Ağırlık merkezi' },
];

// ── Summaries ──────────────────────────────────────────────────────────

const num = (v: unknown) => (typeof v === 'number' ? String(Math.round(v * 100) / 100) : 'ƒ');
const unitOf = (l: AnyLayer) => (l.unit === 'px' ? 'px' : l.unit === 'm' ? 'm' : 'mm');

/** The suffix a count takes as Turkish reads it: 2'li, 3'lü, 6'lı, 9'lu, 10'lu, 20'li. */
export function countSuffix(n: number): string {
  const ones = ['', 'li', 'li', 'lü', 'lü', 'li', 'lı', 'li', 'li', 'lu'];
  const tens = ['lü', 'lu', 'li', 'lu', 'lı', 'li', 'lı', 'li', 'li', 'lı'];
  const k = Math.abs(Math.trunc(n));
  if (k % 10) return ones[k % 10];
  if (k % 100) return tens[(k % 100) / 10];
  return k % 1000 ? 'lü' : 'li';
}

/** A layer's line in the list: its colour, size or placement at a glance. */
export function summary(l: AnyLayer): string {
  const u = unitOf(l);
  switch (l.type) {
    case 'simpleFill':
      return typeof l.color === 'string' ? l.color : 'ifadeden renk';
    case 'hatchFill':
      return `${l.angle}° · ${num(l.spacing)} ${u} aralık`;
    case 'patternFill':
      return `${num(l.spacingX)} × ${num(l.spacingY)} ${u}${l.stagger ? ', şaşırtmalı' : ''}${l.jitter ? ', dağınık' : ''}`;
    case 'imageFill':
      return l.asset ? `${num(l.tileSize)} ${u} döşeme` : 'çizim seçilmedi';
    case 'centroidMarker':
      return l.position === 'centroid' ? 'ağırlık merkezinde' : 'alanın içinde';
    case 'simpleLine':
      return `${num(l.width)} ${u}${l.dash?.length ? ', kesikli' : ''}${l.offset ? (typeof l.offset === 'number' ? `, ${num(l.offset)} ${u} kaydırılmış` : ', ifadeyle kaydırılmış') : ''}${l.wave ? ', dalgalı' : ''}`;
    case 'markerLine':
      return l.placement === 'interval'
        ? `her ${num(l.interval)} ${u}${l.group && l.group.count > 1 ? `, ${l.group.count}'${countSuffix(l.group.count)}` : ''}`
        : (PLACEMENTS.find((p) => p.value === l.placement)?.label ?? '');
    case 'shape':
      return `${SHAPES.find((s) => s.value === l.shape)?.label ?? l.shape} · ${num(l.size)} ${u}`;
    case 'svg':
    case 'raster':
      return l.asset ? `${num(l.size)} ${u}` : 'çizim seçilmedi';
    case 'text':
      return typeof l.text === 'string' ? `“${l.text}”` : 'öznitelikten';
  }
}

// ── Form patches ───────────────────────────────────────────────────────

export type Patch = Partial<AnyLayer> & Record<string, unknown>;

/** Turns a form patch (with the helper keys offsetX, jitterPct …) into a layer. */
export function applyPatch(l: AnyLayer, patch: Patch): AnyLayer {
  const p: Record<string, unknown> = { ...patch };
  const cur = l as unknown as Record<string, unknown>;
  const offset = (cur.offset as readonly [number, number] | undefined) ?? [0, 0];
  if ('shiftX' in p || 'shiftY' in p) {
    const cs = (cur.shift as readonly [number, number] | undefined) ?? [0, 0];
    const next: [number, number] = [Number(p.shiftX ?? cs[0]), Number(p.shiftY ?? cs[1])];
    p.shift = next[0] || next[1] ? next : undefined;
    delete p.shiftX;
    delete p.shiftY;
  }
  if ('offsetX' in p || 'offsetY' in p) {
    p.offset = [p.offsetX ?? offset[0], p.offsetY ?? offset[1]];
    delete p.offsetX;
    delete p.offsetY;
  }
  for (const [from, to] of [['holePct', 'hole'], ['teethDepthPct', 'teethDepth']] as const)
    if (from in p) {
      p[to] = Number(p[from]) / 100;
      delete p[from];
    }
  if ('jitterPct' in p) {
    p.jitter = Number(p.jitterPct) / 100;
    delete p.jitterPct;
  }
  if ('coveragePct' in p) {
    p.coverage = Number(p.coveragePct) / 100;
    delete p.coveragePct;
  }
  if ('groupCount' in p || 'groupSpacing' in p) {
    const g = (cur.group as { count: number; spacing: number } | undefined) ?? { count: 1, spacing: 1 };
    const count = Math.max(1, Math.round(Number(p.groupCount ?? g.count)));
    p.group = count > 1 ? { count, spacing: Number(p.groupSpacing ?? g.spacing) } : undefined;
    delete p.groupCount;
    delete p.groupSpacing;
  }
  if ('haloWidth' in p) {
    const halo = cur.halo as { color: string; width: number } | null | undefined;
    p.halo = halo ? { ...halo, width: Number(p.haloWidth) } : halo;
    delete p.haloWidth;
  }
  if (p.dash === null) p.dash = undefined;
  return { ...l, ...p } as AnyLayer;
}

// ── The layer list ─────────────────────────────────────────────────────

/** Where a layer is: [i] a symbol layer, [i, j] layer j of the marker of layer i. */
export type LayerPath = readonly [number] | readonly [number, number];

const layersOf = (s: Symbol) => s.layers as readonly AnyLayer[];

export function layerAt(s: Symbol, p: LayerPath): AnyLayer | undefined {
  const top = layersOf(s)[p[0]];
  if (p.length === 1 || !top) return top;
  return hasMarker(top) ? (top.marker.layers[p[1]] as AnyLayer | undefined) : undefined;
}

/** The symbol with the layer at `p` replaced (the marker of its parent for child rows). */
export function putLayer(s: Symbol, p: LayerPath, l: AnyLayer): Symbol {
  if (p.length === 1) return { ...s, layers: layersOf(s).map((x, i) => (i === p[0] ? l : x)) } as Symbol;
  const parent = layersOf(s)[p[0]];
  if (!hasMarker(parent)) return s;
  const marker = { ...parent.marker, layers: parent.marker.layers.map((x, j) => (j === p[1] ? (l as MarkerLayer) : x)) };
  return putLayer(s, [p[0]], { ...parent, marker } as AnyLayer);
}

/** The list a layer lives in: its siblings and a symbol with them replaced. */
function siblings(s: Symbol, p: LayerPath): { list: AnyLayer[]; index: number; set: (list: AnyLayer[]) => Symbol } {
  if (p.length === 1) return { list: [...layersOf(s)], index: p[0], set: (list) => ({ ...s, layers: list }) as Symbol };
  const parent = layersOf(s)[p[0]];
  const list = hasMarker(parent) ? [...(parent.marker.layers as AnyLayer[])] : [];
  return { list, index: p[1], set: (l) => (hasMarker(parent) ? putLayer(s, [p[0]], { ...parent, marker: { ...parent.marker, layers: l as MarkerLayer[] } } as AnyLayer) : s) };
}

/** A layer id not yet in `list`: its length, or the next free number after it. */
export function uid(list: readonly AnyLayer[]): string {
  const used = new Set(list.map((l) => l.id));
  let i = list.length;
  while (used.has(String(i))) i++;
  return String(i);
}

/** The marker layer “Katman ekle” may also add into: the chosen child's parent, or the chosen layer that places markers. */
export function addParent(s: Symbol, selected: LayerPath): number | null {
  if (selected.length === 2) return selected[0];
  return hasMarker(layerAt(s, selected)) ? selected[0] : null;
}

export interface Edited {
  symbol: Symbol;
  selected: LayerPath;
}

/** A new layer at the end of the symbol, or of the marker of layer `parent`; it is chosen. */
export function addLayer(s: Symbol, type: LayerType, parent: number | null): Edited {
  if (parent === null) {
    const layer = newLayer(type, uid(layersOf(s)), s.type);
    return { symbol: { ...s, layers: [...layersOf(s), layer] } as Symbol, selected: [layersOf(s).length] };
  }
  const { list, set } = siblings(s, [parent, 0]);
  const layer = newLayer(type, uid(list), 'marker');
  return { symbol: set([...list, layer]), selected: [parent, list.length] };
}

/** Whether the chosen layer can move up (-1) or down (+1) its list. */
export function canMove(s: Symbol, selected: LayerPath, delta: number): boolean {
  const { list, index } = siblings(s, selected);
  const to = index + delta;
  return index < list.length && to >= 0 && to < list.length;
}

/** The chosen layer swapped with its neighbour; null when it is at that end. */
export function moveLayer(s: Symbol, selected: LayerPath, delta: number): Edited | null {
  if (!canMove(s, selected, delta)) return null;
  const { list, set, index } = siblings(s, selected);
  const to = index + delta;
  [list[index], list[to]] = [list[to], list[index]];
  return { symbol: set(list), selected: selected.length === 1 ? [to] : [selected[0], to] };
}

/** A copy of the chosen layer (with a new id) after it; the copy is chosen. */
export function duplicateLayer(s: Symbol, selected: LayerPath): Edited | null {
  const { list, set, index } = siblings(s, selected);
  if (!list[index]) return null;
  const copy = { ...structuredClone(list[index]), id: uid(list) };
  list.splice(index + 1, 0, copy);
  return { symbol: set(list), selected: selected.length === 1 ? [index + 1] : [selected[0], index + 1] };
}

/** Whether the chosen layer can go: a symbol keeps one layer at least (a marker may be left empty). */
export function canRemove(s: Symbol, selected: LayerPath): boolean {
  const { list, index } = siblings(s, selected);
  return !!list[index] && !(selected.length === 1 && list.length === 1);
}

/** The symbol without the chosen layer, and the layer chosen after it; null when it cannot go. */
export function removeLayer(s: Symbol, selected: LayerPath): Edited | null {
  if (!canRemove(s, selected)) return null;
  const { list, set, index } = siblings(s, selected);
  list.splice(index, 1);
  const next = Math.min(index, list.length - 1);
  return { symbol: set(list), selected: selected.length === 1 ? [Math.max(0, next)] : next < 0 ? [selected[0]] : [selected[0], next] };
}

/** A layer switched on (its `enabled` goes) or off (`enabled: false`). */
export function setEnabled(s: Symbol, p: LayerPath, on: boolean): Symbol {
  const l = layerAt(s, p);
  return l ? putLayer(s, p, { ...l, enabled: on ? undefined : false } as AnyLayer) : s;
}

// ── The window ─────────────────────────────────────────────────────────

export const KIND_TITLE: Record<SymbolKind, string> = { fill: 'Alan sembolü', line: 'Çizgi sembolü', marker: 'İşaret sembolü' };

export const GEOMETRIES: Record<SymbolKind, { value: PreviewGeometry; label: string }[]> = {
  fill: [
    { value: 'area', label: 'Alan' },
    { value: 'hole', label: 'Adalı alan' },
  ],
  line: [
    { value: 'line', label: 'Düz' },
    { value: 'bent', label: 'Kırık' },
    { value: 'area', label: 'Alan kenarı' },
  ],
  marker: [{ value: 'point', label: 'Nokta' }],
};

export interface Draft {
  name: string;
  path: string[];
  symbol: Symbol;
}

/** A new symbol of a kind: one layer of the first type it takes, named after the kind, in `path` (Sembollerim). */
export function newDraft(kind: SymbolKind, path?: readonly string[]): Draft {
  return { name: `Yeni ${KIND_TITLE[kind].toLocaleLowerCase('tr')}`, path: [...(path ?? ['Sembollerim'])], symbol: { type: kind, layers: [newLayer(LAYER_TYPES[kind][0], '0', kind)] } as Symbol };
}

/** What a layer style's slot with no symbol and no plain look starts editing from. */
export function defaultFor(cls: GeometryClass): Symbol {
  if (cls === 'fill') return { type: 'fill', layers: [{ id: '0', type: 'simpleFill', color: '#C9D6E3' }, { id: '1', type: 'simpleLine', color: 'ink', width: 0.2 }] };
  if (cls === 'line') return { type: 'line', layers: [{ id: '0', type: 'simpleLine', color: 'ink', width: 0.35 }] };
  return { type: 'marker', layers: [{ id: '0', type: 'shape', shape: 'circle', size: 2.4, fill: 'ink' }] };
}

/** The window's title: “Alan sembolü tasarımcısı”, or for a symbol inside a layer style “<slot>: alan sembolü”; • when changed. */
export function designerTitle(kind: SymbolKind, inline: string | null, dirty: boolean): string {
  const base = inline !== null ? `${inline}: ${KIND_TITLE[kind].toLocaleLowerCase('tr')}` : `${KIND_TITLE[kind]} tasarımcısı`;
  return `${base}${dirty ? ' •' : ''}`;
}

/** The name and category a symbol is saved under: blank ones get “Adsız sembol” in Sembollerim. */
export function savedAs(d: Draft): { name: string; path: string[] } {
  return { name: d.name.trim() || 'Adsız sembol', path: d.path.length ? d.path : ['Sembollerim'] };
}

/** The category field's text as a path: “Ana / Alt”, blanks dropped. */
export const pathOf = (text: string): string[] =>
  text
    .split('/')
    .map((s) => s.trim())
    .filter(Boolean);

/** The preview's scale, paper millimetres to CSS pixels: 1–40, a step is ×1.25, 1:1 is 96 dpi. */
export const ZOOM = { min: 1, max: 40, step: 1.25, start: 4, real: 96 / 25.4 } as const;

export const zoomed = (pxPerMm: number, factor: number): number => Math.min(ZOOM.max, Math.max(ZOOM.min, pxPerMm * factor));

export const zoomText = (pxPerMm: number): string => `1 mm = ${pxPerMm.toFixed(1)} px`;

/** The window's fixed words. */
export const DESIGNER_TEXTS = {
  layers: 'Katmanlar',
  add: 'Katman ekle',
  intoSymbol: 'Sembole',
  intoMarker: (parent: string) => `“${parent}” işaretine`,
  order: 'Listede üstteki katman önce, alttaki en son (en üstte) çizilir.',
  lastLayer: 'Sembolün en az bir katmanı olmalı.',
  onePoint: 'Örnek: tek nokta',
  pickLayer: 'Bir katman seçin.',
  inline: 'Bu sembol katman stilinin içindedir; kitaplığa yazılmaz.',
  inMarker: (parent: string) => `${parent} işaretinde`,
  notSaved: (first: string, more: number) => `Kaydedilemedi: ${first}${more > 0 ? ` (ve ${more} sorun daha)` : ''}`,
  saved: (name: string) => `“${name}” kaydedildi.`,
  cannotEdit: 'Bu sembol düzenlenemez: sistem sembollerinin kopyası düzenlenir.',
} as const;
