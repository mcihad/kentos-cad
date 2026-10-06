/**
 * A dimension's value as it is written (docs/adr/0183 §3): its look's prefix, its kind's own prefix (“R ”, “Ø ”,
 * “Y=”, “X=”, “t=”, “%”), the number, its look's suffix. A length or a coordinate in its look's unit and decimals
 * (else the project's), without the unit; a slope's percentage with its look's decimals (else 2); an angle in the
 * project's angle unit (decimals and unit are not an angle's). The desktop's twin is
 * `kentos_interaction::Format::dimension`; both pass the `dimensionValue` cases of fixtures/text/v1/styles.json.
 */
import { fixed } from '../core/displayNumber';
import type { DimensionLook } from './annotationStyles';
import { UNIT_PER_METRE, type DrawingUnit } from './projectSettings';

/** What the layout measured: the kind's prefix, what the value is, the value (metres, radians or a percentage). */
export interface Measured {
  prefix: string;
  unit: string;
  value: number;
}

/** The project's side: the unit lengths are read in, their decimals, how an angle is written. */
export interface ValueFormat {
  unit: DrawingUnit;
  lengthDecimals: number;
  angle(rad: number): string;
}

export function dimensionValue(m: Measured, look: Pick<DimensionLook, 'decimals' | 'unit' | 'prefix' | 'suffix'>, f: ValueFormat): string {
  const number =
    m.unit === 'angle'
      ? f.angle(m.value)
      : m.unit === 'percent'
        ? fixed(m.value, look.decimals ?? 2)
        : fixed(m.value * UNIT_PER_METRE[look.unit ?? f.unit], look.decimals ?? f.lengthDecimals);
  return `${look.prefix ?? ''}${m.prefix}${number}${look.suffix ?? ''}`;
}
