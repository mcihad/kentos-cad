import { op } from '../../wasm/core';

/**
 * Veride ara's matching (docs/adr/0178 §2, §4): the core's `ops::data_search`, one for both platforms. An object
 * arrives as a record, the words it holds; the answer is its row, by the first field that answers. The independent
 * reference is scripts/fixtures/data_search_cases.py; `../dataSearch.ts` makes the records.
 */

/** An object as the search reads it: its kind and layer path as the rows show them, and the words it holds. */
export interface SearchRecord {
  kind: string;
  layer: string;
  /** The object's label: a point's name, a parcel's number. */
  label: string | null;
  /** A text's words, a leader's note, a dimension's own text. */
  text: string | null;
  /** The name of the block an insert places. */
  block: string | null;
  /** Attribute name and value, in any order (the core looks at them by name). */
  attrs: [string, string][];
}

/** The fields to look in: attributes by their values, all of them or the one named. */
export interface SearchFields {
  label: boolean;
  text: boolean;
  block: boolean;
  attrs: boolean;
  attrName: string | null;
}

/** A column the rows are sorted by (the natural order); none: the drawing's order. */
export type SearchSort = 'layer' | 'kind' | 'field' | 'value';

export interface SearchQuery {
  pattern: string;
  /** Büyük küçük harf eşleşsin: letters match as they are, not the Turkish way folded. */
  matchCase: boolean;
  /** Tam sözcük: no letter, digit or `_` next to the match (without a `*`). */
  wholeWord: boolean;
  fields: SearchFields;
  sort: SearchSort | null;
  descending: boolean;
  /** Rows at most; 0: all. */
  limit: number;
}

/** What a row shows of the record's first answering field. */
export type SearchField = 'label' | 'text' | 'block' | 'attr';

export interface SearchRow {
  /** The record's index. */
  record: number;
  field: SearchField;
  /** The attribute's name, for `attr`. */
  name?: string | null;
  /** The field's words, trimmed. */
  value: string;
  /** How many other fields of the record answer too. */
  more: number;
}

export interface SearchFound {
  rows: SearchRow[];
  /** How many records answer, the rows cut at the limit or not. */
  total: number;
}

/** The records that answer the query: their rows, sorted and cut at the limit, and how many answer in all. */
export const dataSearch = op<(records: readonly SearchRecord[], query: SearchQuery) => SearchFound>('dataSearch');
