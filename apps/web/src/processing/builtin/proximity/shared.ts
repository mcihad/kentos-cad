import { fixed } from '../../../core/displayNumber';
import type { Entity, EntityKind } from '../../../model/entities';
import type { DefaultsContext } from '../../types';
import { AREA_KINDS, QUERY_KINDS } from '../selectByLocation';

/**
 * What the proximity tools share (docs/adr/0215): the objects they take, the measure's choice, how an object is named
 * in a table or on a line, and how distances, areas and bearings are written (the display rule, the project's
 * decimals and angle unit). The desktop's twin is `kentos-processing`'s `builtin/proximity/mod.rs`.
 */

/** What can be measured: what Konuma göre seç takes (docs/adr/0200 §1). */
export const PROXIMITY_KINDS: readonly EntityKind[] = QUERY_KINDS;
/** What can neighbour: the areas. */
export const NEIGHBOR_KINDS: readonly EntityKind[] = AREA_KINDS;
/** The scopes the tools offer, a layer first. */
export const PROXIMITY_SCOPES = ['layer', 'selection', 'visible', 'all'] as const;

/** The measure's choices (docs/adr/0215 §2.1). */
export const MEASURE_OPTIONS = [
  { value: 'edges', label: 'Kenardan kenara', hint: 'En kısa uzaklık; değen, kesişen ya da içinde kalan 0' },
  { value: 'centers', label: 'Merkezden merkeze', hint: 'Ağırlık merkezleri (alan) ya da yer noktaları arası' },
] as const;

/** The most rows a table of these tools has: past it the run is refused before it measures. */
export const MOST_ROWS = 1_000_000;

/** An object's name (docs/adr/0215 §2.3): its field's value, else its label, else its place in its list from 1. */
export function nameOf(e: Entity, field: string, place: number): string {
  const value = field && Object.hasOwn(e.attrs, field) ? e.attrs[field].trim() : '';
  if (value) return value;
  const label = (e.label ?? '').trim();
  return label || String(place + 1);
}

/** A length as the project writes it: its length decimals, the display rule (docs/adr/0149). */
export const lengthText = (units: DefaultsContext, d: number): string => fixed(d, units.lengthDecimals);

/** An area as the project writes it. */
export const areaText = (units: DefaultsContext, a: number): string => fixed(a, units.areaDecimals);

/** A bearing (radians, clockwise from north) in the project's angle unit with four decimals. */
export const bearingText = (units: DefaultsContext, t: number): string => fixed(units.angleUnit === 'deg' ? (t * 180) / Math.PI : (t * 200) / Math.PI, 4);

/** The bound a typed maximum gives: none for 0 or less. */
export const bound = (max: number): number => (max > 0 ? max : Infinity);
