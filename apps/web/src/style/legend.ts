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
