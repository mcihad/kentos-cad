import { Signal, type ReadonlySignal } from '../core/signal';
import { UNIT_PER_METRE, type AngleUnit, type AreaUnit, type DrawingUnit, type Workspace } from '../model/projectSettings';
import { fixed } from '../core/displayNumber';
import type { DimensionLook } from '../model/annotationStyles';
import { dimensionValue, type Measured } from '../model/dimensionValue';

/** The unit fields formatting depends on (ProjectSettings satisfies it). */
export interface UnitSettings {
  readonly lengthDecimals: ReadonlySignal<number>;
  readonly areaDecimals: ReadonlySignal<number>;
  readonly areaUnit: ReadonlySignal<AreaUnit>;
  readonly angleUnit: ReadonlySignal<AngleUnit>;
  /** The unit lengths are typed and read in: a local project's drawing unit (docs/adr/0165 §2); none: metres. */
  readonly unit?: DrawingUnit;
  /** The project's type, whose axes and angles readings and typed points follow (docs/adr/0165 §4); none: CBS's. */
  readonly workspace?: ReadonlySignal<Workspace | null>;
  readonly changed?: ReadonlySignal<number>;
}

/**
 * A project's axes and the way its angles run (docs/adr/0165 §4): a CBS project's Y east and X north, its directions
 * bearings (semt) from north, clockwise; a CAD project's X east and Y north, its angles from east, counter-clockwise.
 * A project not asked its type is shown as CBS.
 */
export type Axes = 'cad' | 'gis';

/** How a typed polar angle runs, and its unit (the core's `Angles`). */
export interface PolarAngles {
  readonly fromNorth: boolean;
  readonly grads: boolean;
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

  /** The project's length decimals (a km is written with them, docs/adr/0188 §2). */
  get lengthDecimals(): number {
    return this.prefs.lengthDecimals.value;
  }

  /**
   * The unit lengths are typed and read in: metres, or a local project's
   * drawing unit (docs/adr/0165 §2). Geometry stays in metres; only what
   * the user reads and types is in the unit.
   */
  get unit(): DrawingUnit {
    return this.prefs.unit ?? 'm';
  }

  /** The project's axes and the way its angles run (docs/adr/0165 §4). */
  get axes(): Axes {
    return this.prefs.workspace?.value === 'cad' ? 'cad' : 'gis';
  }

  /** How a typed polar angle runs and its unit: the project's type and angle unit (docs/adr/0165 §4). */
  get angles(): PolarAngles {
    return { fromNorth: this.axes === 'gis', grads: this.prefs.angleUnit.value === 'grad' };
  }

  /** The east axis's name: Y in a CBS project, X in a CAD one (docs/adr/0165 §4). */
  get eastLabel(): 'X' | 'Y' {
    return this.axes === 'cad' ? 'X' : 'Y';
  }

  /** The north axis's name: X in a CBS project, Y in a CAD one. */
  get northLabel(): 'X' | 'Y' {
    return this.axes === 'cad' ? 'Y' : 'X';
  }

  /** A point as it is typed: `Y,X` in a CBS project, `X,Y` in a CAD one; east first in both. */
  get pairLabel(): string {
    return `${this.eastLabel},${this.northLabel}`;
  }

  /** A relative point as it is typed: `@dY,dX` or `@dX,dY`. */
  get relativeLabel(): string {
    return `@d${this.eastLabel},d${this.northLabel}`;
  }

  /** A polar point as it is typed: a CBS project's `@mesafe<semt`, a CAD project's `@mesafe<açı`. */
  get polarLabel(): string {
    return this.axes === 'cad' ? '@mesafe<açı' : '@mesafe<semt';
  }

  /** A direction's reading's name: a CBS project's `Semt`, a CAD project's `Açı` (docs/adr/0165 §4). */
  get directionName(): string {
    return this.axes === 'cad' ? 'Açı' : 'Semt';
  }

  /**
   * A direction given as its semt in grads, as the project's type reads it: the semt, or a CAD project's angle from
   * east, counter-clockwise (100 − semt grads), in the project's angle unit (the desktop's `Format::direction`).
   */
  direction(semtGrad: number, withUnit = true): string {
    let g = semtGrad;
    if (this.axes === 'cad') {
      g = 100 - semtGrad;
      if (g < 0) g += 400;
      if (g >= 400) g -= 400;
    }
    return this.bearing(g, withUnit);
  }

  /** The value field's hint: “mesafe · Y,X · @dY,dX · @mesafe<semt” in the project's axes. */
  get inputHint(): string {
    return `mesafe · ${this.pairLabel} · ${this.relativeLabel} · ${this.polarLabel}`;
  }

  /**
   * Words written in a CBS project's terms (`Y,X`, `@dY,dX`, `Y (sağa)`, `X (yukarı)`) in the project's axes: a
   * tool's prompt and steps, a column's heading (the desktop's `Format::axes_text`).
   */
  axesText(text: string): string {
    if (this.axes === 'gis') return text;
    return text.replaceAll('dY,dX', 'dX,dY').replaceAll('Y,X', 'X,Y').replaceAll('Y (sağa)', 'X (sağa)').replaceAll('X (yukarı)', 'Y (yukarı)');
  }

  /** How many of the unit make a metre. */
  private get perMetre(): number {
    return UNIT_PER_METRE[this.unit];
  }

  /**
   * The same settings in the surveyor's terms: the Hesap windows' (field measurements are metres whatever a local
   * project's unit, read as Y, X and semt whatever its type; docs/adr/0165 §2, §4).
   */
  metric(): Formatter {
    if (this.unit === 'm' && this.axes === 'gis') return this;
    const p = this.prefs;
    // The surveyor's terms too: Y, X and semt whatever the project's type (docs/adr/0165 §4).
    return new Formatter({ lengthDecimals: p.lengthDecimals, areaDecimals: p.areaDecimals, areaUnit: p.areaUnit, angleUnit: p.angleUnit, changed: this.changed, unit: 'm' });
  }

  /** A length or coordinate typed in the unit, in metres (what the geometry keeps). */
  toMetres(typed: number): number {
    return typed / this.perMetre;
  }

  /** An area typed in the project's area unit (m², dönüm, ha; a local project's mm² or cm²), in square metres. */
  areaToSquareMetres(typed: number): number {
    if (this.unit !== 'm') return typed / (this.perMetre * this.perMetre);
    switch (this.prefs.areaUnit.value) {
      case 'donum':
        return typed * 1000;
      case 'ha':
        return typed * 10_000;
      default:
        return typed;
    }
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

  /**
   * A dimension's measured value as drawn (docs/adr/0183 §3): its look's prefix and suffix around its kind's prefix and
   * the number; a length or coordinate in its look's unit and decimals, else the project's, without the unit.
   */
  dimension(m: Measured, look: Pick<DimensionLook, 'decimals' | 'unit' | 'prefix' | 'suffix'> = {}): string {
    return dimensionValue(m, look, { unit: this.unit, lengthDecimals: this.prefs.lengthDecimals.value, angle: (a) => this.angle(a) });
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
    return `${this.eastLabel} ${this.coord(p.x)}  ${this.northLabel} ${this.coord(p.y)}`;
  }
}
