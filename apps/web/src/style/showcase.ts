import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { LibraryCategory, LibraryItem, LibrarySymbol } from '../model/style';

/**
 * The symbol catalogue drawn into a document (the demo's showcase): every
 * library symbol on a sample geometry of its kind with its name under it,
 * grouped like the library tree. Blocks (one per top category, e.g. a plan
 * level) stand side by side from `at` to the right; inside a block, each
 * section is a layer with its title and rows of cells. Sizes are metres at
 * the document's plot scale, so at 1:1000 a cell is a legend box of
 * 44 × 30 mm and symbols show at their paper size.
 */

const CELL_W = 44;
const CELL_H = 30;
const COLS = 8;
const BLOCK_GAP = 60;
const SECTION_GAP = 14;
const LABEL_H = 1.7;
const TITLE_H = 7;
const SECTION_H = 3.6;
/** Characters per label line before wrapping. */
const WRAP = 30;

const slugOf = (s: string) =>
  s
    .toLocaleLowerCase('tr')
    .replace(/[çğıöşü]/g, (c) => ({ ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u' })[c] ?? c)
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');

function wrap(text: string, width = WRAP): string[] {
  const lines: string[] = [];
  let cur = '';
  for (const word of text.split(/\s+/)) {
    if (cur && (cur + ' ' + word).length > width) {
      lines.push(cur);
      cur = word;
    } else cur = cur ? `${cur} ${word}` : word;
  }
  if (cur) lines.push(cur);
  return lines.slice(0, 3);
}

/** Sample geometry of a symbol's kind inside a cell whose top-left is (x, y). */
function sample(item: LibrarySymbol, x: number, y: number, layerId: string): NewEntity {
  const attrs: Record<string, string> = { Ad: item.name, Kimlik: item.id };
  if (item.reference) attrs.Kaynak = item.reference;
  const base = { layerId, attrs, symbol: item.id };
  switch (item.symbol.type) {
    case 'fill':
      return {
        ...base,
        kind: 'polygon',
        pts: [
          { x: x + 2, y: y - 2 },
          { x: x + CELL_W - 2, y: y - 2 },
          { x: x + CELL_W - 2, y: y - 22 },
          { x: x + 2, y: y - 22 },
        ],
      };
    case 'line':
      // Straight like the legend, with one bend to show corners and direction.
      return {
        ...base,
        kind: 'polyline',
        pts: [
          { x: x + 2, y: y - 14 },
          { x: x + CELL_W - 12, y: y - 14 },
          { x: x + CELL_W - 2, y: y - 6 },
        ],
      };
    case 'marker':
      return { ...base, kind: 'point', p: { x: x + CELL_W / 2, y: y - 12 } };
  }
}

export interface ShowcaseLayout {
  /** Layer ids created, top group first. */
  readonly layers: readonly string[];
  readonly entities: number;
}

/**
 * Adds the catalogue to `doc` below-right of `at` (top-left corner): a
 * layer group with a sub-group per block and a layer per section. Returns
 * what was made; entities go in with `doc.load` (no undo step: it is part
 * of the sample, not an edit).
 */
export function buildShowcase(doc: CadDocument, items: readonly LibraryItem[], categories: readonly LibraryCategory[], at: Vec2): ShowcaseLayout {
  const symbols = items.filter((i): i is LibrarySymbol => i.kind === 'symbol');
  const order = new Map(categories.map((c) => [c.path.join('/'), c.order ?? 0]));
  // Blocks: the first two path levels when the root has children ("MPYY / Uygulama imar planı"), else the root.
  const blockKey = (i: LibrarySymbol) => (i.path.length > 2 ? i.path.slice(0, 2) : i.path.slice(0, 1)).join('/');
  const blocks = new Map<string, LibrarySymbol[]>();
  for (const s of symbols) {
    const k = blockKey(s);
    let list = blocks.get(k);
    if (!list) blocks.set(k, (list = []));
    list.push(s);
  }
  const rank = (path: string) => {
    // Order of each level along the path, so siblings keep the library order.
    const parts = path.split('/');
    return parts.map((_, n) => order.get(parts.slice(0, n + 1).join('/')) ?? 999);
  };
  const cmp = (a: number[], b: number[]) => {
    for (let n = 0; n < Math.max(a.length, b.length); n++) if ((a[n] ?? -1) !== (b[n] ?? -1)) return (a[n] ?? -1) - (b[n] ?? -1);
    return 0;
  };
  const blockKeys = [...blocks.keys()].sort((a, b) => cmp(rank(a), rank(b)));

  const L = doc.layers;
  const root = L.add({ id: 'vitrin', name: 'Gösterim kataloğu', type: 'group', expanded: false, children: [] });
  const made: string[] = [root.id];
  const out: NewEntity[] = [];
  const titleLayer = L.add({ id: 'vitrin.basliklar', name: 'Başlıklar', style: { color: 'fg' } }, root.id);
  made.push(titleLayer.id);
  let x0 = at.x;
  for (const key of blockKeys) {
    const list = blocks.get(key)!;
    const blockName = key.split('/').join(' · ');
    const group = L.add({ id: `vitrin.${slugOf(key)}`, name: key.split('/').pop()!, type: 'group', expanded: false, children: [] }, root.id);
    made.push(group.id);
    out.push({ kind: 'text', layerId: titleLayer.id, p: { x: x0, y: at.y }, text: blockName, height: TITLE_H, rotation: 0, attrs: {} });
    let y = at.y - TITLE_H - 8;
    // Sections in library order, items in legend order within.
    const sections = new Map<string, LibrarySymbol[]>();
    for (const s of list) {
      const k = s.path.join('/');
      let sec = sections.get(k);
      if (!sec) sections.set(k, (sec = []));
      sec.push(s);
    }
    const sectionKeys = [...sections.keys()].sort((a, b) => cmp(rank(a), rank(b)));
    for (const sk of sectionKeys) {
      const sec = sections.get(sk)!;
      const title = sk.split('/').slice(key.split('/').length).join(' · ') || key.split('/').pop()!;
      const layer = L.add({ id: `vitrin.${slugOf(sk)}`, name: title, style: { color: 'fg' } }, group.id);
      made.push(layer.id);
      out.push({ kind: 'text', layerId: titleLayer.id, p: { x: x0, y }, text: title, height: SECTION_H, rotation: 0, attrs: {} });
      y -= SECTION_H + 3;
      sec.forEach((item, n) => {
        const cx = x0 + (n % COLS) * CELL_W;
        const cy = y - Math.floor(n / COLS) * CELL_H;
        out.push(sample(item, cx, cy, layer.id));
        wrap(item.name).forEach((line, k) => out.push({ kind: 'text', layerId: layer.id, p: { x: cx + 2, y: cy - 25 - k * LABEL_H * 1.35 }, text: line, height: LABEL_H, rotation: 0, attrs: {} }));
      });
      y -= Math.ceil(sec.length / COLS) * CELL_H + SECTION_GAP;
    }
    x0 += COLS * CELL_W + BLOCK_GAP;
  }
  // Adding into a group opens it; the catalogue starts folded so the layer panel stays short.
  for (const id of made) if (L.get(id)?.type === 'group') L.setExpanded(id, false);
  doc.load(out);
  return { layers: made, entities: out.length };
}
