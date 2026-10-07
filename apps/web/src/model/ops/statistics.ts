import { op } from '../../wasm/core';

/**
 * The numbers of the queries (docs/adr/0200 §4–§6): the core's `ops::statistics`, one rule for both platforms
 * (`kentos.statistics/1`). Values are texts as the objects hold them; one is read as a number with a point or a comma
 * and no exponent, and one that is not is skipped and counted. Sum, least and most are exact; the mean is rounded half
 * to even at the given scale (the field's), else at the inputs' largest scale + 2. The independent reference is
 * scripts/fixtures/spatial_query_cases.py.
 */

/** Bilgi al's statistics. */
export type StatKind = 'count' | 'sum' | 'mean' | 'min' | 'max' | 'first';

/** One group's statistic: its text (none: nothing to write) and the values that could not be read as numbers. */
export interface StatValue {
  readonly value?: string | null;
  readonly skipped: number;
}

/** Each group's statistic; `scales` gives each group's scale for Ortalama (its target field's), null for the rule's own. */
export const statisticMany = op<(groups: readonly (readonly (string | null)[])[], stat: StatKind, scales: readonly (number | null)[]) => StatValue[]>('statisticMany');

/** Özet istatistik's table: its columns and rows (texts), and how many values could not be read. */
export interface SummaryTable {
  readonly columns: string[];
  readonly rows: string[][];
  readonly skipped: number;
}

/** The summary of objects' values, each with its group when `grouped`. */
export const summarizeValues = op<(groups: readonly (string | null)[], values: readonly (string | null)[], grouped: boolean) => SummaryTable>('summarizeValues');

/**
 * How the targets' keys meet the source rows' (Anahtarla birleştir): per target the source row it takes (none:
 * unmatched), the keys the source has more than once (their first row taken), the source rows no target took.
 */
export interface JoinPlan {
  readonly matches: (number | null)[];
  readonly repeated: number;
  readonly unused: number;
}

export const joinPlan = op<(targets: readonly (string | null)[], sources: readonly (string | null)[]) => JoinPlan>('joinPlan');
