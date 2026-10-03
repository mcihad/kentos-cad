import type { Item } from '../../contracts/generated/sheet/Item';
import type { PaperSize } from '../../contracts/generated/sheet/PaperSize';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import type { BookView, ItemView, SheetView } from './view';

/**
 * The engine's book as the interface shows it (view.ts): lengths from
 * micrometres to millimetres, angles from thousandths of a degree to
 * degrees, the paper's name from the engine's table, a master page's items
 * under the sheet's own, and one short line per item for the tree and the
 * inspector. Whether an item is another work mode's (design §11a) is the
 * engine's to say (`itemNote`); it is asked for the sheet in front only,
 * where the inspector shows it.
 */

export interface AdapterSources {
  /** The engine's papers (A0 … B4), for their names. */
  readonly papers: readonly PaperSize[];
  /** A template's name and newest revision by id, when the gallery knows it (system and this device's). */
  template(id: string): { readonly name: string; readonly revision: number } | null;
  /** The engine's note for another mode's item; null for this mode's. */
  note(item: Item): string | null;
  /** The sheet whose items get their notes (the one in front). */
  readonly open: string | null;
}

export const UM = 1000;
export const MDEG = 1000;

/** Micrometres as millimetres. */
export const mm = (um: number): number => um / UM;
/** Millimetres as whole micrometres (the engine's unit), halves away from zero. */
export const um = (millimetres: number): number => Math.sign(millimetres) * Math.round(Math.abs(millimetres) * UM);

const SHAPE_NAME: Record<string, string> = { rect: 'Dikdörtgen', ellipse: 'Elips', triangle: 'Üçgen', polygon: 'Çokgen' };

/** One line under an item's name: what tells it apart from another of its kind. */
export function detailOf(item: Item, items: readonly Item[]): string | undefined {
  const k = item.kind;
  switch (k.type) {
    case 'map':
      if (k.view.type === 'atlas') return 'Atlas';
      return k.view.center ? `1/${k.view.scale}` : `1/${k.view.scale} · yeri seçilmedi`;
    case 'text': {
      const line = k.content.split('\n')[0].trim();
      return line ? (line.length > 40 ? `${line.slice(0, 39)}…` : line) : 'boş';
    }
    case 'legend':
      return k.title || undefined;
    case 'table':
      return k.source.type === 'fixed' ? `${k.source.rows.length} satır` : k.source.type === 'layer' ? `katman: ${k.source.layer || '—'}` : 'önceki tablonun devamı';
    case 'coordinateList':
      return k.source.type === 'selection' ? 'seçili nesneler' : `katman: ${k.source.layer || '—'}`;
    case 'titleBlock':
      return `${k.rows.reduce((n, r) => n + r.cells.length, 0)} hücre`;
    case 'picture':
      return k.asset ? undefined : 'resim seçilmedi';
    case 'shape':
      return SHAPE_NAME[k.shape.type];
    case 'border':
      return k.style === 'double' ? 'Çift çizgi' : 'Tek çizgi';
    case 'group':
      return `${items.filter((i) => i.group === item.id).length} öğe`;
    default:
      return undefined;
  }
}

function itemView(item: Item, items: readonly Item[], master: boolean, note: string | null): ItemView {
  const bindings: Record<string, string> = {};
  for (const b of item.bindings) bindings[b.property] = b.expression;
  return {
    id: item.id,
    name: item.name,
    kind: item.kind.type,
    frame: { left: mm(item.frame.left), top: mm(item.frame.top), width: mm(item.frame.width), height: mm(item.frame.height) },
    rotation: item.rotation / MDEG,
    locked: master || item.locked,
    hidden: item.hidden,
    printable: item.printable,
    opacity: item.opacity,
    anchors: { h: item.constraints.h, v: item.constraints.v, box: item.constraints.relativeTo },
    group: item.group ?? null,
    bindings,
    master: master || undefined,
    note: note ?? undefined,
    detail: detailOf(item, items),
    source: item,
  };
}

export function sheetView(book: SheetBook, sheet: Sheet, src: AdapterSources): SheetView {
  const master = sheet.master ? book.masters.find((m) => m.id === sheet.master) : undefined;
  const p = sheet.page;
  const named = src.papers.find((x) => x.id === p.paper)?.name;
  const noted = src.open === sheet.id;
  const origin = sheet.origin ? src.template(sheet.origin.templateId) : null;
  return {
    id: sheet.id,
    name: sheet.name,
    paper: {
      paper: p.paper,
      name: named ?? 'Özel',
      orientation: p.orientation,
      widthMm: mm(p.size.width),
      heightMm: mm(p.size.height),
      margins: { left: mm(p.margins.left), top: mm(p.margins.top), right: mm(p.margins.right), bottom: mm(p.margins.bottom) },
    },
    items: [...(master?.items ?? []).map((i) => itemView(i, master!.items, true, null)), ...sheet.items.map((i) => itemView(i, sheet.items, false, noted ? src.note(i) : null))],
    master: master?.name,
    template: sheet.origin ? { name: origin?.name ?? sheet.origin.templateId, newer: !!origin && origin.revision > sheet.origin.revision } : undefined,
    layout: sheet.activeVariant ? sheet.variants.find((v) => v.id === sheet.activeVariant)?.name : undefined,
    source: sheet,
  };
}

export function bookView(book: SheetBook, src: AdapterSources): BookView {
  return { sheets: book.sheets.map((s) => sheetView(book, s, src)), source: book };
}
