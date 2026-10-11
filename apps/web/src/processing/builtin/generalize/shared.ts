import type { NewEntity } from '../../../model/entities';
import type { ParamUnit, TableOutput } from '../../types';

/**
 * What the Genelleştirme tools share (docs/adr/0221; the desktop's `builtin/generalize/mod.rs`): their scopes, an angle
 * and a measure parameter, a run's result with its count and report.
 */

/** The scopes the tools offer, a layer first. */
export const GENERALIZE_SCOPES = ['layer', 'selection', 'visible', 'all'] as const;

/** An angle parameter in degrees. */
export const degreesParam = <N extends string>(name: N, label: string, max: number, fallback: number, description: string) =>
  ({ name, label, type: 'number', unit: '°', min: 0, max, default: fallback, description }) as const;

/** A measure parameter: `unit` m or m², at least `min`. */
export const measureParam = <N extends string>(name: N, label: string, unit: ParamUnit, min: number, fallback: number, description: string) =>
  ({ name, label, type: 'number', unit, min, default: fallback, description }) as const;

/** A report table, when it has rows. */
export const table = (columns: readonly string[], rows: string[][]): TableOutput | null => (rows.length ? { columns: [...columns], rows } : null);

/** A run that wrote these objects, with its count and, when it has rows, its report. */
export function finish(add: NewEntity[], summary: string, count: number, report: TableOutput | null = null) {
  return { changes: { add }, outputs: report ? { report, count } : { count }, summary };
}
