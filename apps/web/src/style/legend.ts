import type { Entity } from '../model/entities';
import type { LayerStyle } from '../model/layers';
import type { Rule, Symbol, SymbolRef, SymbolSet } from '../model/style';
import { classesPresent } from './classify';
import { symbolsOfLayerStyle } from './fromLayer';
import type { GeometryClass } from './geometry';

/**
 * The legend of a drawing (plan açıklamaları): for each layer, what its
 * symbols mean. A layer without a renderer shows its own look; single,
 * categorized, graduated and rule-based renderers show one row per class
 * (only for the geometry the layer has); objects with their own symbol add
 * that symbol with its library name. Pure: the legend window draws it.
 */

export interface LegendEntry {
  label: string;
  symbol: Symbol | null;
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
    const entries: LegendEntry[] = [];
    const r = layer.style.renderer;
    if (!r) entries.push(...setEntries(symbolsOfLayerStyle(layer.style, layer.style.color), layer.name));
    else if (r.type === 'single') entries.push(...setEntries(r.symbols, layer.name));
    else if (r.type === 'categorized') {
      for (const k of r.categories) if (k.enabled !== false) entries.push(...setEntries(k.symbols, k.label || k.value));
      if (r.other) entries.push(...setEntries(r.other, 'Diğer değerler'));
    } else if (r.type === 'graduated') for (const k of r.classes) entries.push(...setEntries(k.symbols, k.label));
    else {
      const walk = (rules: readonly Rule[], prefix: string) => {
        for (const rule of rules) {
          if (rule.enabled === false) continue;
          const label = prefix ? `${prefix} › ${rule.label}` : rule.label;
          entries.push(...setEntries(rule.symbols, label));
          if (rule.children?.length) walk(rule.children, label);
        }
      };
      walk(r.rules, '');
    }
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
