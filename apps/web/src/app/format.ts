import { Signal, type ReadonlySignal } from '../core/signal';
import { UNIT_PER_METRE, type AngleUnit, type AreaUnit, type DrawingUnit } from '../model/projectSettings';
import { fixed } from '../core/displayNumber';

/** The unit fields formatting depends on (ProjectSettings satisfies it). */
export interface UnitSettings {
  readonly lengthDecimals: ReadonlySignal<number>;
  readonly areaDecimals: ReadonlySignal<number>;
  readonly areaUnit: ReadonlySignal<AreaUnit>;
  readonly angleUnit: ReadonlySignal<AngleUnit>;
  /** The unit lengths are typed and read in: a local project's drawing unit (docs/adr/0165 §2); none: metres. */
  readonly unit?: DrawingUnit;
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

  /**
   * The unit lengths are typed and read in: metres, or a local project's
   * drawing unit (docs/adr/0165 §2). Geometry stays in metres; only what
   * the user reads and types is in the unit.
   */
  get unit(): DrawingUnit {
    return this.prefs.unit ?? 'm';
  }

  /** How many of the unit make a metre. */
  private get perMetre(): number {
    return UNIT_PER_METRE[this.unit];
  }

  /**
   * The same settings in metres: the Hesap windows' (surveying, whose field
   * measurements are metres whatever a local project's unit; docs/adr/0165 §2).
   */
  metric(): Formatter {
    if (this.unit === 'm') return this;
    const p = this.prefs;
    return new Formatter({ lengthDecimals: p.lengthDecimals, areaDecimals: p.areaDecimals, areaUnit: p.areaUnit, angleUnit: p.angleUnit, changed: this.changed, unit: 'm' });
  }

  /** A length or coordinate typed in the unit, in metres (what the geometry keeps). */
  toMetres(typed: number): number {
    return typed / this.perMetre;
  }

  /** A length or coordinate in metres, in the unit (a field's starting value). */
  fromMetres(m: number): number {
    return m * this.perMetre;
  }

  /**
   * A length in metres in the unit with the digits it needs, not rounded to
   * the length decimals: a limit in a message, a kept value in a prompt
   * (`0.000001` m, `0.001` mm). Without the unit; the desktop's `Format::plain`.
   */
  plain(m: number): string {
    return String(Number(this.fromMetres(m).toPrecision(15)));
  }

  /** Grid coordinate (Y or X) without unit. */
  coord(v: number): string {
    return fixed(this.fromMetres(v), this.prefs.lengthDecimals.value);
  }

  length(m: number, unit = true): string {
    const s = fixed(this.fromMetres(m), this.prefs.lengthDecimals.value);
    return unit ? `${s} ${this.unit}` : s;
  }

  /** The length unit in words, for prompts: “metre”, or a local project's “milimetre” or “santimetre”. */
  get lengthUnitName(): string {
    return { mm: 'milimetre', cm: 'santimetre', m: 'metre' }[this.unit];
  }

  /** The length unit's mark: `m`, or a local project's `mm` or `cm`. */
  get lengthUnitLabel(): string {
    return this.unit;
  }

  area(m2: number, unit = true): string {
    const d = this.prefs.areaDecimals.value;
    // A local project in millimetres or centimetres reads its areas in the unit squared.
    if (this.unit !== 'm') {
      const s = fixed(m2 * this.perMetre * this.perMetre, d);
      return unit ? `${s} ${this.unit}²` : s;
    }
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
    if (this.unit !== 'm') return `${this.unit}²`;
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
