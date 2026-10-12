import type { PreviewGeometry } from '../../render/symbolPreview';
import type { Anchor, Effect, EffectType, FillLayer, GeneratorStep, GeneratorStepKind, GeometryGenerator, LineLayer, MarkerLayer, MarkerPlacement, MarkerSymbolLayer, ShapeName, SizeUnit, Symbol, TextMarker, VertexChoice } from '../../model/style';
import { EFFECT_TYPES } from '../../model/style';
import type { GeometryClass } from '../../style/geometry';

/**
 * The symbol designer's model (docs/STYLE.md §7, docs/adr/0094), without
 * the page: what each symbol layer type is called and how a new one starts,
 * a layer's one-line summary, form patches turned into layers, and the
 * layer list's edits (add, move, duplicate, remove, switch off) on a symbol
 * as plain JSON. fixtures/style/v1/designer.json holds it; the desktop's
 * designer (crates/native/style/src/designer.rs) checks the same file.
 */

export type AnyLayer = FillLayer | LineLayer | MarkerSymbolLayer;
export type LayerType = AnyLayer['type'];
export type SymbolKind = Symbol['type'];

export const LAYER_LABEL: Record<LayerType, string> = {
  simpleFill: 'Dolgu',
  hatchFill: 'Tarama',
  gradientFill: 'Degrade',
  patternFill: 'Desen',
  imageFill: 'Görüntü dolgusu',
  centroidMarker: 'İç noktada işaret',
  simpleLine: 'Çizgi',
  markerLine: 'Çizgi boyunca işaret',
  shape: 'Şekil',
  svg: 'SVG çizimi',
  text: 'Yazı',
  raster: 'Görüntü',
  geometryGenerator: 'Geometri üretici',
};

export const LAYER_TYPES: Record<SymbolKind, LayerType[]> = {
  fill: ['simpleFill', 'hatchFill', 'patternFill', 'imageFill', 'simpleLine', 'markerLine', 'centroidMarker', 'geometryGenerator'],
  line: ['simpleLine', 'markerLine', 'geometryGenerator'],
  marker: ['shape', 'text', 'svg', 'raster', 'geometryGenerator'],
};

/** Layers that hold a marker symbol of their own (edited as child rows). */
export const hasMarker = (l: AnyLayer | undefined): l is Extract<AnyLayer, { marker: unknown }> =>
  !!l && (l.type === 'markerLine' || l.type === 'patternFill' || l.type === 'centroidMarker');

/** Layers whose own symbol's layers are child rows: those holding a marker, and a geometry generator (docs/adr/0226 §1). */
export const hasChildren = (l: AnyLayer | undefined): l is Extract<AnyLayer, { marker: unknown }> | GeometryGenerator => hasMarker(l) || l?.type === 'geometryGenerator';

/** The symbol whose layers are a parent's child rows. */
const childSymbol = (l: Extract<AnyLayer, { marker: unknown }> | GeometryGenerator): Symbol => (l.type === 'geometryGenerator' ? l.symbol : l.marker);

/** Layers a generator's symbol does not take: one level deep, no layer holding a symbol of its own (docs/adr/0226 §1). */
const NOT_IN_GENERATOR: ReadonlySet<LayerType> = new Set<LayerType>(['markerLine', 'patternFill', 'centroidMarker', 'geometryGenerator']);

/** The types a parent's child rows may be: a marker's markers, a generator's symbol's own types without nesting. */
export function childTypes(l: Extract<AnyLayer, { marker: unknown }> | GeometryGenerator): LayerType[] {
  if (l.type !== 'geometryGenerator') return LAYER_TYPES.marker.filter((t) => t !== 'geometryGenerator');
  return LAYER_TYPES[l.symbol.type].filter((t) => !NOT_IN_GENERATOR.has(t));
}

const dot = (id: string): MarkerLayer => ({ id, type: 'shape', shape: 'circle', size: 1, fill: 'ink' });

export function newLayer(type: LayerType, id: string, context: SymbolKind): AnyLayer {
  switch (type) {
    case 'simpleFill':
      return { id, type, color: '#C9D6E3' };
    case 'hatchFill':
      return { id, type, angle: 45, spacing: 2, width: 0.2, color: 'ink' };
    // A hatch object's gradient (docs/adr/0186 §3); the designer does not offer it.
    case 'gradientFill':
      return { id, type, color: 'ink', color2: '#FFFFFF', shape: 'linear', angle: 0 };
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
    case 'geometryGenerator':
      return newGenerator(id, context);
  }
}

/**
 * A new geometry generator for a symbol of `context` (docs/adr/0226 §1): a point's circle filled, a line's two ends
 * marked, an area's bounding box outlined.
 */
function newGenerator(id: string, context: SymbolKind): GeometryGenerator {
  if (context === 'marker')
    return {
      id,
      type: 'geometryGenerator',
      steps: [{ kind: 'circle', radius: 3 }],
      symbol: { type: 'fill', layers: [{ id: '0', type: 'simpleFill', color: '#4E79A740' }, { id: '1', type: 'simpleLine', color: '#4E79A7', width: 0.25 }] },
    };
  if (context === 'line') return { id, type: 'geometryGenerator', steps: [{ kind: 'vertices', which: 'ends' }], symbol: { type: 'marker', layers: [{ id: '0', type: 'shape', shape: 'circle', size: 1.5, fill: 'ink' }] } };
  return { id, type: 'geometryGenerator', steps: [{ kind: 'envelope' }], symbol: { type: 'line', layers: [{ id: '0', type: 'simpleLine', color: 'ink', width: 0.25, dash: [2, 1] }] } };
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
    case 'gradientFill':
      return `${l.shape === 'cylinder' ? 'silindir' : l.shape === 'spherical' ? 'küre' : 'doğrusal'} · ${l.angle}°`;
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
    case 'geometryGenerator':
      return `${l.steps.length ? l.steps.map((st) => STEP_LABEL[st.kind] ?? st.kind).join(' → ') : 'adımsız'} · ${CLASS_WORD[l.symbol.type]}`;
  }
}

/** A generator's symbol's class as the summary says it. */
export const CLASS_WORD: Record<SymbolKind, string> = { fill: 'alan', line: 'çizgi', marker: 'işaret' };

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
  return hasChildren(top) ? (childSymbol(top).layers[p[1]] as AnyLayer | undefined) : undefined;
}

/** A parent with its child symbol's layers replaced. */
function withChildren(parent: Extract<AnyLayer, { marker: unknown }> | GeometryGenerator, layers: readonly AnyLayer[]): AnyLayer {
  if (parent.type === 'geometryGenerator') return { ...parent, symbol: { ...parent.symbol, layers } as Symbol };
  return { ...parent, marker: { ...parent.marker, layers: layers as MarkerLayer[] } } as AnyLayer;
}

/** The symbol with the layer at `p` replaced (the marker or generator symbol of its parent for child rows). */
export function putLayer(s: Symbol, p: LayerPath, l: AnyLayer): Symbol {
  if (p.length === 1) return { ...s, layers: layersOf(s).map((x, i) => (i === p[0] ? l : x)) } as Symbol;
  const parent = layersOf(s)[p[0]];
  if (!hasChildren(parent)) return s;
  const layers = (childSymbol(parent).layers as readonly AnyLayer[]).map((x, j) => (j === p[1] ? l : x));
  return putLayer(s, [p[0]], withChildren(parent, layers));
}

/** The list a layer lives in: its siblings and a symbol with them replaced. */
function siblings(s: Symbol, p: LayerPath): { list: AnyLayer[]; index: number; set: (list: AnyLayer[]) => Symbol } {
  if (p.length === 1) return { list: [...layersOf(s)], index: p[0], set: (list) => ({ ...s, layers: list }) as Symbol };
  const parent = layersOf(s)[p[0]];
  const list = hasChildren(parent) ? [...(childSymbol(parent).layers as readonly AnyLayer[])] : [];
  return { list, index: p[1], set: (l) => (hasChildren(parent) ? putLayer(s, [p[0]], withChildren(parent, l)) : s) };
}

/** A layer id not yet in `list`: its length, or the next free number after it. */
export function uid(list: readonly AnyLayer[]): string {
  const used = new Set(list.map((l) => l.id));
  let i = list.length;
  while (used.has(String(i))) i++;
  return String(i);
}

/** The layer “Katman ekle” may also add into: the chosen child's parent, or the chosen layer that holds a symbol of its own. */
export function addParent(s: Symbol, selected: LayerPath): number | null {
  if (selected.length === 2) return selected[0];
  return hasChildren(layerAt(s, selected)) ? selected[0] : null;
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
  const owner = layersOf(s)[parent];
  const layer = newLayer(type, uid(list), hasChildren(owner) ? childSymbol(owner).type : 'marker');
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

// ── Geometri üretici's steps (docs/adr/0226 §1) ───────────────────────

export const STEP_LABEL: Record<GeneratorStepKind, string> = {
  centroid: 'Ağırlık merkezi',
  pointOnSurface: 'İç nokta',
  vertices: 'Köşeler',
  boundary: 'Sınır',
  offset: 'Paralel',
  extend: 'Uzat',
  buffer: 'Tampon',
  envelope: 'Sınırlayıcı kutu',
  hull: 'Dışbükey örtü',
  circle: 'Daire',
  wedge: 'Daire dilimi',
  polygon: 'Düzgün çokgen',
  translate: 'Öteleme',
  rotate: 'Döndür',
  scale: 'Ölçekle',
};

/** The steps in the menu's order: points, lines, areas, then moves. */
export const STEP_KINDS: GeneratorStepKind[] = ['centroid', 'pointOnSurface', 'vertices', 'boundary', 'offset', 'extend', 'buffer', 'envelope', 'hull', 'circle', 'wedge', 'polygon', 'translate', 'rotate', 'scale'];

export const VERTEX_CHOICES: { value: VertexChoice; label: string }[] = [
  { value: 'all', label: 'Hepsi' },
  { value: 'inner', label: 'Uçlar dışında' },
  { value: 'first', label: 'İlk' },
  { value: 'last', label: 'Son' },
  { value: 'ends', label: 'İki uç' },
];

export const BUFFER_SIDES: { value: 'both' | 'left' | 'right'; label: string }[] = [
  { value: 'both', label: 'İki yan' },
  { value: 'left', label: 'Sol yan' },
  { value: 'right', label: 'Sağ yan' },
];

/** A new step of a kind, with numbers that show at once. */
export function newStep(kind: GeneratorStepKind): GeneratorStep {
  switch (kind) {
    case 'vertices':
      return { kind, which: 'all' };
    case 'offset':
      return { kind, distance: 1 };
    case 'extend':
      return { kind, start: 0, end: 2 };
    case 'buffer':
      return { kind, distance: 2 };
    case 'circle':
      return { kind, radius: 2 };
    case 'wedge':
      return { kind, direction: 0, width: 60, radius: 5 };
    case 'polygon':
      return { kind, sides: 6, radius: 2 };
    case 'translate':
      return { kind, dx: 1, dy: 0 };
    case 'rotate':
      return { kind, angle: 45 };
    case 'scale':
      return { kind, x: 1.5 };
    default:
      return { kind } as GeneratorStep;
  }
}

/** A geometry's class between steps. */
export type GenClass = 'point' | 'line' | 'area';

/** The class after a step from `c`; null when the step takes no such geometry (its result is empty). */
function classAfter(kind: GeneratorStepKind, c: GenClass): GenClass | null {
  switch (kind) {
    case 'centroid':
    case 'pointOnSurface':
      return 'point';
    case 'vertices':
      return c === 'point' ? null : 'point';
    case 'boundary':
      return c === 'area' ? 'line' : null;
    case 'offset':
      return c === 'point' ? null : 'line';
    case 'extend':
      return c === 'line' ? 'line' : null;
    case 'buffer':
    case 'envelope':
    case 'hull':
    case 'circle':
    case 'wedge':
    case 'polygon':
      return 'area';
    default:
      return c;
  }
}

/** The class a geometry of `input` is after `steps`; null when a step leaves nothing (docs/adr/0226 §1, class fit). */
export function stepOutput(steps: readonly GeneratorStep[], input: GenClass): GenClass | null {
  let c: GenClass | null = input;
  for (const st of steps) {
    if (!c) return null;
    c = classAfter(st.kind, c);
  }
  return c;
}

/** The class a symbol of `kind` draws on: a fill symbol's areas, a line symbol's lines, a marker symbol's points. */
export const CLASS_OF: Record<SymbolKind, GenClass> = { fill: 'area', line: 'line', marker: 'point' };

/** A step's line in its list: its name and numbers in the layer's unit. */
export function stepSummary(st: GeneratorStep, u: string): string {
  const name = STEP_LABEL[st.kind] ?? st.kind;
  switch (st.kind) {
    case 'vertices':
      return `${name} · ${VERTEX_CHOICES.find((c) => c.value === (st.which ?? 'all'))?.label ?? st.which}`;
    case 'offset':
      return `${name} · ${num(st.distance)} ${u}`;
    case 'extend':
      return `${name} · ${num(st.start ?? 0)} / ${num(st.end ?? 0)} ${u}`;
    case 'buffer':
      return `${name} · ${num(st.distance)} ${u}${st.side && st.side !== 'both' ? `, ${BUFFER_SIDES.find((b) => b.value === st.side)?.label.toLocaleLowerCase('tr')}` : ''}`;
    case 'circle':
      return `${name} · ${num(st.radius)} ${u}`;
    case 'wedge':
      return `${name} · ${num(st.width)}° · ${num(st.radius)} ${u}`;
    case 'polygon':
      return `${name} · ${num(st.sides)} köşe · ${num(st.radius)} ${u}`;
    case 'translate':
      return `${name} · ${num(st.dx ?? 0)}, ${num(st.dy ?? 0)} ${u}`;
    case 'rotate':
      return `${name} · ${num(st.angle)}°`;
    case 'scale':
      return `${name} · ${num(st.x ?? st.y ?? 1)} × ${num(st.y ?? st.x ?? 1)}`;
    default:
      return name;
  }
}

/** The generator with a new step of `kind` at the end. */
export function addStep(l: GeometryGenerator, kind: GeneratorStepKind): GeometryGenerator {
  return { ...l, steps: [...l.steps, newStep(kind)] };
}

/** The generator with step `i` swapped with its neighbour; null at that end. */
export function moveStep(l: GeometryGenerator, i: number, delta: number): GeometryGenerator | null {
  const to = i + delta;
  if (i < 0 || i >= l.steps.length || to < 0 || to >= l.steps.length) return null;
  const steps = [...l.steps];
  [steps[i], steps[to]] = [steps[to], steps[i]];
  return { ...l, steps };
}

/** The generator without step `i`. */
export function removeStep(l: GeometryGenerator, i: number): GeometryGenerator {
  return { ...l, steps: l.steps.filter((_, k) => k !== i) };
}

/** The generator with step `i` replaced. */
export function putStep(l: GeometryGenerator, i: number, step: GeneratorStep): GeometryGenerator {
  return { ...l, steps: l.steps.map((x, k) => (k === i ? step : x)) };
}

/** The generator drawing with a symbol of another class: that class's starting symbol (`defaultFor`). */
export function setGeneratorKind(l: GeometryGenerator, kind: SymbolKind): GeometryGenerator {
  return l.symbol.type === kind ? l : { ...l, symbol: defaultFor(kind) };
}

// ── Draw effects (docs/adr/0226 §2) ────────────────────────────────────

export const EFFECT_LABEL: Record<EffectType, string> = {
  dropShadow: 'Gölge',
  outerGlow: 'Dış parıltı',
  blur: 'Bulanıklaştır',
  innerShadow: 'İç gölge',
  innerGlow: 'İç parıltı',
};

/** A new effect of a type, as the style core reads a missing value (`effects` in model.rs). */
export function newEffect(type: EffectType): Effect {
  switch (type) {
    case 'dropShadow':
      return { type, distance: 1, angle: 135, blur: 1.5, color: '#000000', opacity: 0.5 };
    case 'innerShadow':
      return { type, distance: 0.5, angle: 135, blur: 1, color: '#000000', opacity: 0.5 };
    case 'outerGlow':
      return { type, spread: 0.5, blur: 1.5, color: '#FFFFFF', opacity: 0.5 };
    case 'innerGlow':
      return { type, spread: 0.5, blur: 1, color: '#FFFFFF', opacity: 0.5 };
    case 'blur':
      return { type, blur: 1 };
  }
}

/** A layer's effect of a type, switched on or not. */
export const effectOf = (l: AnyLayer, type: EffectType): Effect | undefined => l.effects?.find((e) => e.type === type);

/** The effects in their drawn order (docs/adr/0226 §2), none as no list. */
function withEffects(l: AnyLayer, effects: readonly Effect[]): AnyLayer {
  const sorted = [...effects].sort((a, b) => EFFECT_TYPES.indexOf(a.type) - EFFECT_TYPES.indexOf(b.type));
  const { effects: _old, ...rest } = l as AnyLayer & { effects?: readonly Effect[] };
  return (sorted.length ? { ...rest, effects: sorted } : rest) as AnyLayer;
}

/** A layer with an effect switched on (added with its defaults, or its `enabled` taken off) or off (taken away). */
export function setEffect(l: AnyLayer, type: EffectType, on: boolean): AnyLayer {
  const rest = (l.effects ?? []).filter((e) => e.type !== type);
  if (!on) return withEffects(l, rest);
  const had = effectOf(l, type);
  if (had) {
    const { enabled: _e, ...kept } = had;
    return withEffects(l, [...rest, kept as Effect]);
  }
  return withEffects(l, [...rest, newEffect(type)]);
}

/** A layer with its effect of the same type replaced. */
export function putEffect(l: AnyLayer, effect: Effect): AnyLayer {
  return withEffects(l, [...(l.effects ?? []).filter((e) => e.type !== effect.type), effect]);
}

/** Whether a layer at `p` takes effects: a symbol's own layers (a generator's or a marker's child rows draw within it). */
export const takesEffects = (p: LayerPath): boolean => p.length === 1;

/** The effects switched on, as a list row says them: “gölge, iç parıltı”. */
export function effectsSummary(l: AnyLayer): string {
  return EFFECT_TYPES.filter((t) => {
    const e = effectOf(l, t);
    return !!e && e.enabled !== false;
  })
    .map((t) => EFFECT_LABEL[t].toLocaleLowerCase('tr'))
    .join(', ');
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
  /** A geometry generator's symbol (docs/adr/0226 §1): “Katman ekle”'s header and the form's note on its layers. */
  intoGenerator: (parent: string) => `“${parent}” sembolüne`,
  inGenerator: (parent: string) => `${parent} sembolünde`,
  drawsWith: 'Çizen sembol',
  steps: 'Adımlar',
  addStep: 'Adım ekle',
  noSteps: 'Adım yok: nesnenin kendi geometrisi çizilir.',
  nothingLeft: 'Bu adımlardan sonra bir şey kalmaz; adımları değiştirin.',
  /** Draw effects (docs/adr/0226 §2). */
  effects: 'Efektler',
  effectsInside: 'Efektler sembolün kendi katmanlarına konur; bu katman üstündekinin içinde çizilir.',
  notSaved: (first: string, more: number) => `Kaydedilemedi: ${first}${more > 0 ? ` (ve ${more} sorun daha)` : ''}`,
  saved: (name: string) => `“${name}” kaydedildi.`,
  cannotEdit: 'Bu sembol düzenlenemez: sistem sembollerinin kopyası düzenlenir.',
} as const;
