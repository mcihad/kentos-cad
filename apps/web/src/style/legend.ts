import type { Entity } from '../model/entities';
import type { LayerStyle } from '../model/layers';
import type { LayerRenderer, Rule, Symbol, SymbolRef, SymbolSet } from '../model/style';
import { classesPresent } from './classify';
import { symbolsOfLayerStyle } from './fromLayer';
import type { GeometryClass } from './geometry';
import { EXPONENTS, rampAt, shareOf, sizeAt, stepOf, withColor, withSize } from './thematic';

/**
 * The legend of a drawing (plan açıklamaları): for each layer, what its
 * symbols mean. A layer without a renderer shows its own look; single,
 * categorized, graduated and rule-based renderers show one row per class
 * (only for the geometry the layer has); objects with their own symbol add
 * that symbol with its library name. The thematic renderers (docs/adr/0213
 * §2): Sürekli renk five samples of its ramp, Orantılı sembol three sizes,
 * İki değişkenli renk a row per cell of its grid, Nokta yoğunluğu what a dot
 * stands for and its values' colours, Grafik its values' colours, Isı
 * haritası three samples, Kümeleme its cluster and its single points' rows,
 * Yayma its single points' rows, Ters alan its fill. Pure: the legend
 * window draws it.
 */

export interface LegendEntry {
  label: string;
  symbol: Symbol | null;
  /** The rows of one Orantılı sembol share a scale (CSS px per paper mm): their sizes compare (docs/adr/0213 §2.2). */
  pxPerMm?: number;
}

export interface LegendGroup {
  layerId: string;
  layerName: string;
  entries: LegendEntry[];
}

export interface LegendLayer {
  id: string;
  name: string;
  style: LayerStyle;
}

export interface LegendSources {
  entities(layerId: string): readonly Entity[];
  symbol(ref: string): Symbol | undefined;
  itemName(id: string): string | undefined;
}

const ORDER: readonly GeometryClass[] = ['fill', 'line', 'marker'];
const CLASS_NAME: Record<GeometryClass, string> = { fill: 'alan', line: 'çizgi', marker: 'nokta' };

export function legendOf(layers: readonly LegendLayer[], src: LegendSources): LegendGroup[] {
  const out: LegendGroup[] = [];
  for (const layer of layers) {
    const entities = src.entities(layer.id);
    const present = classesPresent(entities);
    const classes = ORDER.filter((c) => present[c]);
    if (!classes.length) continue;
    const resolve = (ref: SymbolRef | undefined): Symbol | null => (!ref ? null : 'ref' in ref ? (src.symbol(ref.ref) ?? null) : ref);
    /** The symbols of a set for the layer's geometry, the first one standing for the row (areas first). */
    const setEntries = (set: SymbolSet | undefined, label: string): LegendEntry[] => {
      const found = classes.map((c) => ({ c, s: resolve(set?.[c]) })).filter((x) => x.s);
      if (!found.length) return [];
      if (found.length === 1) return [{ label, symbol: found[0].s }];
      return found.map((x) => ({ label: `${label} (${CLASS_NAME[x.c]})`, symbol: x.s }));
    };
    /** A set's rows with every symbol changed by `f` (a thematic renderer's colour or size). */
    const changed = (set: SymbolSet | undefined, label: string, f: (s: Symbol) => Symbol): LegendEntry[] =>
      setEntries(set, label).map((e) => ({ label: e.label, symbol: e.symbol && f(e.symbol) }));
    const entries: LegendEntry[] = [];
    const rows = (r: LayerRenderer | undefined) => {
      if (!r) return void entries.push(...setEntries(symbolsOfLayerStyle(layer.style, layer.style.color), layer.name));
      switch (r.type) {
        case 'single':
          return void entries.push(...setEntries(r.symbols, layer.name));
        case 'categorized':
          for (const k of r.categories) if (k.enabled !== false) entries.push(...setEntries(k.symbols, k.label || k.value));
          if (r.other) entries.push(...setEntries(r.other, 'Diğer değerler'));
          return;
        case 'graduated':
          for (const k of r.classes) entries.push(...setEntries(k.symbols, k.label));
          return;
        case 'rules': {
          const walk = (rules: readonly Rule[], prefix: string) => {
            for (const rule of rules) {
              if (rule.enabled === false) continue;
              const label = prefix ? `${prefix} › ${rule.label}` : rule.label;
              entries.push(...setEntries(rule.symbols, label));
              if (rule.children?.length) walk(rule.children, label);
            }
          };
          return walk(r.rules, '');
        }
        case 'unclassed':
          for (const t of [0, 0.25, 0.5, 0.75, 1]) {
            const c = rampAt(r.ramp, stepOf(t) / 255);
            entries.push(...changed(r.symbols, legendNumber(r.min + (r.max - r.min) * t), (s) => withColor(s, c.slice(0, 7))));
          }
          if (r.other) entries.push(...setEntries(r.other, 'Değeri olmayanlar'));
          return;
        case 'proportional': {
          const unit = r.unit ?? 'mm';
          const e = EXPONENTS[r.scaling ?? 'area'];
          // The largest as four fifths of a row's picture (24 px high); paper mm only (px draw as they are).
          const shared = unit === 'mm' && r.maxSize > 0 ? { pxPerMm: (0.8 * 24) / r.maxSize } : {};
          for (const v of [r.minValue, (r.minValue + r.maxValue) / 2, r.maxValue]) {
            const size = sizeAt(stepOf(shareOf(v, r.minValue, r.maxValue)) / 255, r.minSize, r.maxSize, e);
            // Areas and points by the marker symbol, lines by the line symbol.
            const set: SymbolSet = { ...(present.fill || present.marker ? { marker: r.symbols.marker } : {}), ...(present.line ? { line: r.symbols.line } : {}) };
            const shown = (['line', 'marker'] as const).filter((c) => set[c] && resolve(set[c]));
            for (const c of shown) {
              const s = resolve(set[c]);
              if (s) entries.push({ label: shown.length > 1 ? `${legendNumber(v)} (${CLASS_NAME[c]})` : legendNumber(v), symbol: withSize(s, size, unit), ...(c === 'marker' ? shared : {}) });
            }
          }
          if (r.other) entries.push(...setEntries(r.other, 'Değeri olmayanlar'));
          return;
        }
        case 'bivariate': {
          const n = r.breaksX.length + 1;
          for (let j = 0; j < n; j++)
            for (let i = 0; i < n; i++) {
              const c = r.colors[j * n + i];
              if (!c) continue;
              const label = `${r.exprX} ${rangeLabel(r.breaksX, i)}, ${r.exprY} ${rangeLabel(r.breaksY, j)}`;
              entries.push(...changed(r.symbols, label, (s) => withColor(s, c.slice(0, 7))));
            }
          if (r.other) entries.push(...setEntries(r.other, 'Değeri olmayanlar'));
          return;
        }
        case 'dotDensity':
          entries.push({ label: `1 nokta = ${legendNumber(r.dotValue)}`, symbol: null });
          for (const f of r.fields)
            entries.push({ label: f.label || f.expr, symbol: { type: 'marker', layers: [{ id: 'd', type: 'shape', shape: 'circle', size: r.dotSize ?? 1, unit: r.unit ?? 'mm', fill: f.color }] } });
          return;
        case 'chart':
          for (const f of r.fields) entries.push({ label: f.label || f.expr, symbol: { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: f.color }] } });
          return;
        case 'heatmap':
          for (const [label, t] of [['Az', 0.25], ['Orta', 0.5], ['Çok', 1]] as const)
            entries.push({ label, symbol: { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: rampAt(r.ramp, t) }] } });
          return;
        case 'cluster': {
          const symbol = resolve(r.symbol) ?? { type: 'marker', layers: [{ id: 'k', type: 'shape', shape: 'circle', size: 24, unit: 'px', fill: layer.style.color, stroke: '#FFFFFF', strokeWidth: 1.5 }] };
          entries.push({ label: 'Küme', symbol });
          return rows(r.renderer);
        }
        case 'displacement':
          return rows(r.renderer);
        case 'inverted':
          return void entries.push({ label: 'Dışı', symbol: resolve(r.symbols.fill) });
      }
    };
    rows(layer.style.renderer);
    // Objects drawn with their own symbol: each symbol once, under its library name.
    const own = new Set<string>();
    for (const e of entities) if (e.symbol && !own.has(e.symbol)) own.add(e.symbol);
    for (const id of own) {
      const s = src.symbol(id);
      if (s) entries.push({ label: src.itemName(id) ?? id, symbol: s });
    }
    if (entries.length) out.push({ layerId: layer.id, layerName: layer.name, entries });
  }
  return out;
}

/** A number on the legend: at most two decimals, as the classes' labels write them. */
export function legendNumber(x: number): string {
  return String(Math.round(x * 100) / 100);
}

/** A class of breaks as the legend says it: `< b₀`, `b₀ – b₁`, `≥ bₙ`. */
export function rangeLabel(breaks: readonly number[], i: number): string {
  if (i === 0) return `< ${legendNumber(breaks[0])}`;
  if (i >= breaks.length) return `≥ ${legendNumber(breaks[breaks.length - 1])}`;
  return `${legendNumber(breaks[i - 1])} – ${legendNumber(breaks[i])}`;
}

// ── The legend window and its picture ──────────────────────────────────
// Pinned for both platforms by fixtures/style/v1/legend.json.

/** What the legend window says. */
export const LEGEND_TEXTS = {
  title: 'Lejant',
  save: 'PNG olarak kaydet',
  close: 'Kapat',
  visibleOnly: 'Yalnızca görünen katmanlar',
  headings: 'Katman adlarını başlık yaz',
  rows: (n: number) => `${n} satır`,
  nothing: 'Lejanta girecek çizilmiş nesne yok.',
  noRows: 'Lejantta satır yok.',
  saved: 'Lejant PNG olarak kaydedildi (beyaz kâğıt, 2× çözünürlük).',
  file: 'lejant.png',
  heading: 'LEJANT',
} as const;

/** The layers a legend reads: top of the list first, only the visible ones when asked. */
export function legendLayers(leaves: readonly (LegendLayer & { visible: boolean })[], visibleOnly: boolean): LegendLayer[] {
  return leaves.filter((l) => !visibleOnly || l.visible).map(({ id, name, style }) => ({ id, name, style }));
}

/** The picture's palette over the screen's: white paper, black ink, whatever the theme. */
export const LEGEND_PAPER = { background: [1, 1, 1, 1] as const, ink: '#000000', paper: '#FFFFFF', fg: '#111111', fgDim: '#555555' };

const FONT = 'Arial, "Liberation Sans", sans-serif';

/** A line of text on the legend picture: `align` right means `x` is where it ends. */
export interface LegendText {
  text: string;
  font: string;
  color: string;
  x: number;
  y: number;
  align: 'left' | 'right';
}

export interface LegendRow {
  kind: 'heading' | 'entry';
  label: LegendText;
  /** Where an entry's symbol is drawn (on white, framed); none for a heading or a symbol that is not there. */
  picture: { x: number; y: number; w: number; h: number; frame: string; frameWidth: number } | null;
}

/** The legend picture, in logical pixels (drawn at `scale` for print): its size, heading, the drawing's name and rows. */
export interface LegendLayout {
  scale: number;
  width: number;
  height: number;
  background: string;
  heading: LegendText;
  name: LegendText;
  rows: LegendRow[];
}

/** Where everything of the legend picture goes: the groups shown, with or without layer headings. */
export function legendLayout(groups: readonly LegendGroup[], headings: boolean, drawingName: string): LegendLayout {
  const W = 520;
  const ROW = 30;
  const count = groups.reduce((n, g) => n + g.entries.length + (headings ? 1 : 0), 0);
  const rows: LegendRow[] = [];
  let y = 56;
  for (const grp of groups) {
    if (headings) {
      rows.push({ kind: 'heading', label: { text: grp.layerName, font: `700 12px ${FONT}`, color: '#000000', x: 20, y: y + 19, align: 'left' }, picture: null });
      y += ROW;
    }
    for (const e of grp.entries) {
      rows.push({
        kind: 'entry',
        label: { text: e.label, font: `400 12px ${FONT}`, color: '#000000', x: 90, y: y + 19, align: 'left' },
        picture: e.symbol ? { x: 20, y: y + 3, w: 56, h: 24, frame: '#BBBBBB', frameWidth: 0.5 } : null,
      });
      y += ROW;
    }
  }
  return {
    scale: 2,
    width: W,
    height: 56 + count * ROW + 16,
    background: '#FFFFFF',
    heading: { text: LEGEND_TEXTS.heading, font: `700 18px ${FONT}`, color: '#000000', x: 20, y: 34, align: 'left' },
    name: { text: drawingName, font: `400 11px ${FONT}`, color: '#555555', x: W - 20, y: 34, align: 'right' },
    rows,
  };
}
