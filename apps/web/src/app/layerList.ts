import { fixed } from '../core/displayNumber';
import { LINE_TYPE_LABEL, type LayerNode } from '../model/layers';
import { DRAW_COLORS } from '../ui/ribbon/fields';

/**
 * Katman listesi (docs/adr/0177 §6; the desktop's `layer_list.rs`, shared cases fixtures/layers/v1/list.json from
 * scripts/fixtures/layer_list_cases.py): every node of the tree in its order is a row: its path, name, kind, its own
 * visibility and lock, whether it is the active layer, a layer's colour (a drawing colour's name, else the value in
 * capitals), line type and weight in mm (two decimals, a decimal comma), and its objects (a group's on all its layers).
 * The CSV file is UTF-8 with a byte order mark, semicolons and CR LF, as Excel's Turkish settings open it; the
 * clipboard takes the rows between tabs.
 */

export const LAYER_LIST_HEADER = ['Yol', 'Ad', 'Tür', 'Görünür', 'Kilitli', 'Etkin', 'Renk', 'Çizgi tipi', 'Kalınlık (mm)', 'Nesne sayısı'] as const;

const yes = (b: boolean) => (b ? 'Evet' : 'Hayır');

/** The header and a row per node, in the tree's order; `count` the objects on a layer. */
export function layerListRows(tree: readonly LayerNode[], active: string, count: (layer: string) => number): string[][] {
  const out: string[][] = [[...LAYER_LIST_HEADER]];
  const objects = (n: LayerNode): number => (n.type === 'layer' ? count(n.id) : n.children.reduce((s, c) => s + objects(c), 0));
  const go = (nodes: readonly LayerNode[], above: readonly string[]) => {
    for (const n of nodes) {
      const path = [...above, n.name];
      const layer = n.type === 'layer';
      out.push([
        path.join(' / '),
        n.name,
        layer ? 'Katman' : 'Grup',
        yes(n.visible),
        yes(n.locked),
        yes(n.id === active),
        layer ? (DRAW_COLORS.find((c) => c.value === n.style.color)?.name ?? n.style.color.toUpperCase()) : '',
        layer ? LINE_TYPE_LABEL[n.style.lineType] : '',
        layer ? fixed(n.style.lineWeight, 2).replace('.', ',') : '',
        String(objects(n)),
      ]);
      go(n.children, path);
    }
  };
  go(tree, []);
  return out;
}

/** A field between `sep`s: quoted, its quotes doubled, when it holds the separator, a quote or a line break. */
function field(text: string, sep: string): string {
  return text.includes(sep) || /["\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

/** The CSV file's text: a byte order mark, fields between semicolons, lines ending in CR LF. */
export function layerListCsv(rows: readonly (readonly string[])[]): string {
  return '﻿' + rows.map((r) => r.map((f) => field(f, ';')).join(';') + '\r\n').join('');
}

/** The clipboard's text: fields between tabs, lines ending in LF. */
export function layerListTsv(rows: readonly (readonly string[])[]): string {
  return rows.map((r) => r.map((f) => field(f, '\t')).join('\t') + '\n').join('');
}
