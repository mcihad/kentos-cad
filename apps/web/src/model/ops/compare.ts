import { op } from '../../wasm/core';

/**
 * Veri karşılaştır (docs/adr/0179): the core's `ops::compare`, one rule for both platforms. Two sets of objects, old
 * and new, are paired by location (the same kind within the search distance, the smallest location difference first)
 * or by a key attribute, and compared: geometry within the tolerance, attributes field by field. The independent
 * reference is scripts/fixtures/compare_cases.py.
 */

/** An object of a set: its shape (the geometry fields with its kind, as `geometryOf` gives them) and its attributes. */
export interface CompareMember {
  readonly shape: Record<string, unknown>;
  readonly attrs: Readonly<Record<string, string>>;
}

/** What the comparison is made with: metres for the search distance and the tolerance. */
export interface CompareSettings {
  readonly match: 'location' | 'key';
  readonly key?: string | null;
  readonly search: number;
  readonly tolerance: number;
  readonly ignore?: readonly string[] | null;
}

export type CompareStatus = 'same' | 'geometry' | 'attributes' | 'both' | 'added' | 'removed' | 'key';

/**
 * A row: its finding, the old and the new object (indices), the location difference (m) of a pair whose kinds agree,
 * the changed fields. An absent index or difference is none.
 */
export interface CompareRow {
  readonly status: CompareStatus;
  readonly old?: number;
  readonly new?: number;
  readonly distance?: number;
  readonly fields: readonly string[];
}

/** The rows: the new set's order (its pair, added or a key problem), then the old set's unpaired objects in theirs. */
export const dataCompare = op<(old: readonly CompareMember[], next: readonly CompareMember[], settings: CompareSettings) => CompareRow[]>('dataCompare');
