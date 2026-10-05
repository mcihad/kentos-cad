import type { Vec2 } from '../../model/geometry';
import type { SearchFields, SearchQuery, SearchRow, SearchSort } from '../../model/ops/dataSearch';
import { parsePointInput } from '../../tools/coordinateInput';

/**
 * The rules of the bottom panel's Arama tab (docs/adr/0178) that do not touch the page: its words, the query the
 * panel's choices make, a row's cells, the count line, which way a header click sorts and what a typed coordinate is.
 * The desktop's are in `apps/desktop/src/search/`.
 */

/** Rows listed at most; the count line says how many answer in all. */
export const SEARCH_LIMIT = 5000;

/** The panel's height Veride ara opens it to when it is shorter (CSS px): the bar takes two lines and the rows want room. */
export const SEARCH_PANEL_HEIGHT = 280;

/** The columns: their header and the core's sort key (none: Sıra, the drawing's order). */
export const SEARCH_COLUMNS: readonly { label: string; sort: SearchSort | null }[] = [
  { label: 'Sıra', sort: null },
  { label: 'Katman', sort: 'layer' },
  { label: 'Tür', sort: 'kind' },
  { label: 'Alan', sort: 'field' },
  { label: 'Değer', sort: 'value' },
];

export const SEARCH_TEXTS = {
  search: 'Veride ara',
  placeholder: 'Ara: öznitelik, yazı, ad, blok adı — ya da Y,X',
  searchHint: 'Nesnelerin etiketinde, yazısında, blok adında ve özniteliklerinde arar. * herhangi bir dizidir: Ada *, *101. Bir koordinat yazarsanız (Y,X) oraya gider ve işaretler.',
  allLayers: 'Bütün katmanlar',
  fieldsLabel: 'Ara:',
  labelField: 'Ad / etiket',
  labelHint: 'Nokta adı, parsel numarası gibi nesnenin etiketinde arar',
  textField: 'Yazı',
  textHint: 'Yazı nesnesinin metninde, kılavuzun notunda ve ölçünün kendi yazısında arar',
  blockField: 'Blok adı',
  blockHint: 'Blok yerleştirmelerinin bloğunun adında arar',
  attrsField: 'Öznitelikler',
  attrsHint: 'Öznitelik değerlerinde arar (nokta kodu ve blok öznitelikleri dahil)',
  allAttrs: 'Bütün öznitelikler',
  attrPick: 'Öznitelik',
  matchCase: 'Büyük küçük harf eşleşsin',
  wholeWord: 'Tam sözcük',
  wholeWordHint: 'Eşleşmenin iki yanında harf, rakam ya da _ olmaz (* varken geçmez)',
  onlySelected: 'Yalnız seçimde',
  selectAll: 'Hepsini seç',
  selectAllHint: 'Bulunan nesnelerin hepsini seçer ve ekrana sığdırır',
  show: 'Göster',
  showHint: 'Seçili sonuçlara yakınlaşır',
  go: 'Git',
  goHint: 'Görünümü bu koordinata getirir ve çizimde işaretler (Enter)',
  coordinate: 'Koordinat',
  mark: 'İşaret',
  unmark: 'İşareti kaldır',
  unmarkHint: 'Koordinata git’in çizimdeki işaretini kaldırır',
  noQuery: 'Aranacak sözü yazın: öznitelik değeri, yazı, nokta adı ya da blok adı. * herhangi bir dizidir; Y,X yazarsanız koordinata gider.',
  noData: 'Çizimde aranabilir değer yok: ad, etiket, yazı, blok adı ya da öznitelik.',
  noFields: 'Aranacak alan seçilmedi: Ad / etiket, Yazı, Blok adı ya da Öznitelikler düğmelerinden birini açın.',
  noMatch: (q: string) => `“${q}” ile eşleşen nesne yok. Alan düğmelerini, katmanı ve seçenekleri denetleyin.`,
} as const;

/** What the panel's choices are, kept for the session: leaving the tab and coming back keeps them. */
export interface SearchState {
  text: string;
  fields: SearchFields;
  matchCase: boolean;
  wholeWord: boolean;
  onlySelected: boolean;
  /** One layer by id; none: every layer. */
  layer: string | null;
  sort: SearchSort | null;
  descending: boolean;
}

export const searchDefaults = (): SearchState => ({
  text: '',
  fields: { label: true, text: true, block: true, attrs: true, attrName: null },
  matchCase: false,
  wholeWord: false,
  onlySelected: false,
  layer: null,
  sort: null,
  descending: false,
});

/** The core's query for the panel's choices (docs/adr/0178 §2, §4). */
export function queryOf(s: SearchState, limit = SEARCH_LIMIT): SearchQuery {
  return { pattern: s.text, matchCase: s.matchCase, wholeWord: s.wholeWord, fields: s.fields, sort: s.sort, descending: s.descending, limit };
}

/** Whether any field is asked for. */
export const fieldsOn = (f: SearchFields): boolean => f.label || f.text || f.block || f.attrs;

/** A row's Alan: the field that answered, an attribute by its name, “+n” for the others that answer too. */
export function fieldCell(row: Pick<SearchRow, 'field' | 'name' | 'more'>): string {
  const name = row.field === 'label' ? SEARCH_TEXTS.labelField : row.field === 'text' ? SEARCH_TEXTS.textField : row.field === 'block' ? SEARCH_TEXTS.blockField : (row.name ?? SEARCH_TEXTS.attrsField);
  return row.more > 0 ? `${name} +${row.more}` : name;
}

/** The count line: how many objects answer, and how many of them are listed when the list is cut. */
export function countText(listed: number, total: number): string {
  return listed < total ? `${listed} / ${total} sonuç` : `${total} sonuç`;
}

/** A header click: ascending, then descending, then the drawing's order; Sıra is the drawing's order. */
export function nextSort(sort: SearchSort | null, descending: boolean, column: SearchSort | null): { sort: SearchSort | null; descending: boolean } {
  if (column === null || (sort === column && descending)) return { sort: null, descending: false };
  return { sort: column, descending: sort === column };
}

/**
 * The place typed text names (docs/adr/0178 §6), in metres: an absolute point, east then north, as the Nokta tool
 * reads one (`toMetres` turns a coordinate typed in the project's unit into them); null for anything else.
 */
export function placeOf(text: string, toMetres: (typed: number) => number): Vec2 | null {
  return parsePointInput(text, null, null, undefined, toMetres);
}
