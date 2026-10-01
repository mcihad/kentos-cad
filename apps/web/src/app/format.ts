import { Signal, type ReadonlySignal } from '../core/signal';
import type { AngleUnit, AreaUnit } from '../model/projectSettings';
import { fixed } from '../core/displayNumber';

/** The unit fields formatting depends on (ProjectSettings satisfies it). */
export interface UnitSettings {
  readonly lengthDecimals: ReadonlySignal<number>;
  readonly areaDecimals: ReadonlySignal<number>;
  readonly areaUnit: ReadonlySignal<AreaUnit>;
  readonly angleUnit: ReadonlySignal<AngleUnit>;
  readonly changed?: ReadonlySignal<number>;
}

const GRAD_PER_DEG = 400 / 360;

/**
 * The only place numbers become user-facing text. Every panel, tool tag and
 * log line formats through here so the project's unit/precision settings
 * apply everywhere at once.
 *
 * Decimal separator is always "." — it matches what the command line
 * accepts ("Y,X" uses the comma as the coordinate separator), so a copied
 * value can be typed back in unchanged.
 */
export class Formatter {
  /** Bumped when any unit/precision setting changes. */
  readonly changed: ReadonlySignal<number>;
  private readonly prefs: UnitSettings;

  constructor(units: UnitSettings) {
    this.prefs = units;
    this.changed = units.changed ?? new Signal(0);
  }

  /** Grid coordinate (Y or X) without unit. */
  coord(v: number): string {
    return fixed(v, this.prefs.lengthDecimals.value);
  }

  length(m: number, unit = true): string {
    const s = fixed(m, this.prefs.lengthDecimals.value);
    return unit ? `${s} m` : s;
  }

  area(m2: number, unit = true): string {
    const d = this.prefs.areaDecimals.value;
    switch (this.prefs.areaUnit.value) {
      case 'donum':
        return unit ? `${fixed(m2 / 1000, d)} dönüm` : fixed(m2 / 1000, d);
      case 'ha':
        return unit ? `${fixed(m2 / 10_000, d)} ha` : fixed(m2 / 10_000, d);
      default:
        return unit ? `${fixed(m2, d)} m²` : fixed(m2, d);
    }
  }

  /** A slope in percent, two decimals (docs/adr/0147 §2). */
  percent(v: number): string {
    return fixed(v, 2);
  }

  get areaUnitLabel(): string {
    return { m2: 'm²', donum: 'dönüm', ha: 'ha' }[this.prefs.areaUnit.value];
  }

  /** Bearing given in grads (surveying semt), shown in the chosen angle unit. */
  bearing(grad: number, unit = true): string {
    if (this.prefs.angleUnit.value === 'deg') {
      const s = fixed(grad / GRAD_PER_DEG, 4);
      return unit ? `${s}°` : s;
    }
    const s = fixed(grad, 4);
    return unit ? `${s} g` : s;
  }

  /** An angle given in radians (angular dimensions), in the project's angle unit. */
  angle(rad: number, unit = true): string {
    return this.bearing((rad * 200) / Math.PI, unit);
  }

  get angleUnitLabel(): string {
    return this.prefs.angleUnit.value === 'deg' ? '°' : 'g';
  }

  /** The angle unit in words, for prompts: “derece” or “grad”. */
  get angleUnitName(): string {
    return this.prefs.angleUnit.value === 'deg' ? 'derece' : 'grad';
  }

  /** An angle typed in the project's angle unit, in radians (the inverse of `angle`). */
  angleFromTyped(value: number): number {
    return this.prefs.angleUnit.value === 'deg' ? (value * Math.PI) / 180 : (value * Math.PI) / 200;
  }

  point(p: { x: number; y: number }): string {
    return `Y ${this.coord(p.x)}  X ${this.coord(p.y)}`;
  }
}
