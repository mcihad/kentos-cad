import { Signal, watchAll } from '../core/signal';
import type { CrsDefinition } from '../contracts/generated/CrsDefinition';
import type { DatumTransform } from '../contracts/generated/DatumTransform';
import type { DimensionStyleDef } from '../contracts/generated/DimensionStyleDef';
import type { DrawingFont } from '../contracts/generated/DrawingFont';
import type { DrawingUnit } from '../contracts/generated/DrawingUnit';
import type { LayerState } from '../contracts/generated/LayerState';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import type { ProjectVariable } from '../contracts/generated/ProjectVariable';
import type { ServiceConnection } from '../contracts/generated/ServiceConnection';
import type { SurveySettings } from '../contracts/generated/SurveySettings';
import type { TextStyleDef } from '../contracts/generated/TextStyleDef';
import type { TopologySettings } from '../contracts/generated/TopologySettings';
import type { Workspace } from '../contracts/generated/Workspace';
import { crsBySrid, DEFAULT_SRID, type CrsDef } from '../geo/crs';
import { sanitizedDimensionStyles, sanitizedTextStyles } from './annotationStyles';
import { sanitizedNetworks } from './networkRules';
import { sanitizedVariables } from './projectVariables';
import { sameJson } from './sameJson';
import { sameTopology, sanitizeTopology } from './topologyRules';
import { ANNOTATION_KINDS, annotationMm, paperHeight, sanitizedAnnotationHeights, type AnnotationHeights, type AnnotationKind } from './annotationScale';

/** Whether two projects' annotation heights are the same (none and none too). */
const sameHeights = (a: AnnotationHeights | null, b: AnnotationHeights | null): boolean => ANNOTATION_KINDS.every((k) => a?.[k] === b?.[k]);

export type AreaUnit = 'm2' | 'donum' | 'ha';
export type AngleUnit = 'grad' | 'deg';
export type { CrsDefinition, DatumTransform, DimensionStyleDef, DrawingFont, DrawingUnit, SurveySettings, TextStyleDef, Workspace };

/** How many of a drawing unit make a metre (docs/adr/0165 §2). */
export const UNIT_PER_METRE: Record<DrawingUnit, number> = { mm: 1000, cm: 100, m: 1 };
/** Every drawing unit a file may name. */
export const DRAWING_UNIT_IDS: readonly DrawingUnit[] = ['mm', 'cm', 'm'];

/** Every drawing typeface a file may name (contract `DrawingFont`; the list with names is app/appearance.ts). */
export const DRAWING_FONT_IDS: readonly DrawingFont[] = ['barlow', 'arimo', 'overpass', 'quicksand', 'architects-daughter', 'courier-prime', 'plex-mono'];

/**
 * Every project type a file may name (contract `Workspace`, docs/adr/0165).
 * What each shows, and which can be chosen yet, is app/workspaces.ts; the
 * model only keeps the value. Files written before types, and those that
 * named the former Hibrit mode, have their type not asked yet (null).
 */
export const WORKSPACE_IDS: readonly Workspace[] = ['cad', 'gis', 'plan3d', 'disaster'];

/** How the former Hibrit mode was written: a file naming it has its type not asked yet. */
export const LEGACY_HYBRID = 'hybrid';

/** A project's type as written, or null when it is not asked yet: absent, the former Hibrit mode (a server's metadata may still say it). */
const typeOf = (w: unknown): Workspace | null => ((WORKSPACE_IDS as readonly unknown[]).includes(w) ? (w as Workspace) : null);

/** Serialized form, stored inside the project file. */
export interface ProjectSettingsData {
  srid: number;
  lengthDecimals: number;
  areaDecimals: number;
  areaUnit: AreaUnit;
  angleUnit: AngleUnit;
  /** Plot scale denominator (1:1000 → 1000). */
  plotScale: number;
  /** The project's type (CAD or CBS: its scene, axes and ribbon); absent while it is not asked (docs/adr/0165 §1). */
  workspace?: Workspace;
  /** Typeface of the drawing's own text (text objects, dimension values, labels). */
  drawingFont: DrawingFont;
  /** A local project's unit (docs/adr/0165 §2): lengths are typed and read in it; absent: metres. */
  drawingUnit?: DrawingUnit;
  /** The project's second coordinate system (docs/adr/0167 §1): its values are shown beside the project's; absent: none. */
  secondSrid?: number;
  /** The project's own coordinate system when it is a definition (docs/adr/0168 §1); `srid` is then 0. */
  customCrs?: CrsDefinition;
  /** The second system when it is a definition (instead of `secondSrid`). */
  secondCustomCrs?: CrsDefinition;
  /** The project's datum choices (docs/adr/0168 §3); absent: EPSG's ways. */
  datumTransforms?: DatumTransform[];
  /** The project's survey constants and tolerances (docs/adr/0169 §3); absent: k = 0.13 and no tolerance. */
  survey?: SurveySettings;
  /** The project's named layer states (docs/adr/0177 §4), in the menu's order; absent: none. */
  layerStates?: LayerState[];
  /** The project's text styles (docs/adr/0183 §2), in the list's order; absent: none (Standart only). */
  textStyles?: TextStyleDef[];
  /** The project's dimension styles (docs/adr/0183 §3), in the list's order; absent: none (Standart only). */
  dimensionStyles?: DimensionStyleDef[];
  /** The project's topology rules, tolerance and exceptions (docs/adr/0202 §1); absent: none. */
  topology?: TopologySettings;
  /** The project's annotation heights on paper (docs/adr/0205 §1); absent: every kind's default. */
  annotation?: AnnotationHeights;
  /** The project's connections to map services, without their secrets (docs/adr/0208 §2); absent: none. */
  connections?: ServiceConnection[];
  /** The project's networks (docs/adr/0209 §2), in the list's order; absent: none. */
  networks?: NetworkDef[];
  /** The project's own `@` variables (docs/adr/0214 §2.3), in the settings window's order; absent: none. */
  variables?: ProjectVariable[];
}

/**
 * A change to some settings: absent fields are kept; a null `secondSrid`, `customCrs`, `secondCustomCrs`, `survey`,
 * `topology` or `annotation` removes it, an empty `datumTransforms` the datum choices.
 */
export type ProjectSettingsPatch = Partial<Omit<ProjectSettingsData, 'secondSrid' | 'customCrs' | 'secondCustomCrs' | 'survey' | 'topology' | 'annotation'>> & {
  secondSrid?: number | null;
  customCrs?: CrsDefinition | null;
  secondCustomCrs?: CrsDefinition | null;
  survey?: SurveySettings | null;
  topology?: TopologySettings | null;
  annotation?: AnnotationHeights | null;
};

/** The refraction coefficient of trigonometric heights when a project names none (docs/adr/0169 §3). */
export const REFRACTION = 0.13;

/** Whether `k` is one a project may name: finite, within [−1, 1] (the contract's `SurveySettings::refraction_holds`). */
export const refractionHolds = (k: number): boolean => Number.isFinite(k) && k >= -1 && k <= 1;

/** Whether `t` is a tolerance: finite and above zero. */
export const toleranceHolds = (t: number): boolean => Number.isFinite(t) && t > 0;

/** The lowest and the highest mean ellipsoidal height a project may name, m (the contract's `GROUND_HEIGHTS`, docs/adr/0171 §2). */
export const GROUND_HEIGHTS = [-500, 9000] as const;

/** Whether `h` is a mean ellipsoidal height a project may name (the contract's `SurveySettings::ground_height_holds`). */
export const groundHeightHolds = (h: number): boolean => Number.isFinite(h) && h >= GROUND_HEIGHTS[0] && h <= GROUND_HEIGHTS[1];

/** A network's a priori standard deviations (docs/adr/0203 §1): radians, metres, parts per million, m/√km. */
export interface SurveySigmas {
  direction: number;
  distance: number;
  ppm: number;
  centering: number;
  zenith: number;
  levelling: number;
}

/**
 * The a priori standard deviations a project names none of (the contract's `SIGMA_DEFAULTS`, docs/adr/0203 §1): a 3″
 * total station's (10 cc, 2 mm + 2 ppm, 1 mm centering) and engineering levelling's (2 mm/√km).
 */
export const SIGMA_DEFAULTS: Readonly<SurveySigmas> = {
  direction: Math.PI / 200_000,
  distance: 0.002,
  ppm: 2,
  centering: 0.001,
  zenith: Math.PI / 200_000,
  levelling: 0.002,
};

/** Whether `s` is a standard deviation (a direction's, a distance's constant part, a zenith angle's, levelling's): finite and above zero. */
export const sigmaHolds = (s: number): boolean => Number.isFinite(s) && s > 0;

/** Whether `s` is a part that may be nothing (the parts per million, the centering): finite and not below zero. */
export const sigmaPartHolds = (s: number): boolean => Number.isFinite(s) && s >= 0;

/** The a priori standard deviations: the project's, else the defaults (the contract's `SurveySettings::sigmas`). */
export function surveySigmas(s: SurveySettings | null | undefined): SurveySigmas {
  const d = SIGMA_DEFAULTS;
  return {
    direction: s?.sigmaDirection ?? d.direction,
    distance: s?.sigmaDistance ?? d.distance,
    ppm: s?.sigmaPpm ?? d.ppm,
    centering: s?.sigmaCentering ?? d.centering,
    zenith: s?.sigmaZenith ?? d.zenith,
    levelling: s?.sigmaLevelling ?? d.levelling,
  };
}

/** Whether the project names any a priori standard deviation (the contract's `SurveySettings::has_sigmas`). */
export const hasSigmas = (s: SurveySettings | null | undefined): boolean =>
  !!s && [s.sigmaDirection, s.sigmaDistance, s.sigmaPpm, s.sigmaCentering, s.sigmaZenith, s.sigmaLevelling].some((v) => v !== undefined);

/**
 * The survey settings as a project keeps them (the contract's `SurveySettings::sanitized`): k where it holds and is not
 * the default, the tolerances and the ground height that hold, the reduction to the grid when asked for with a height
 * (docs/adr/0171), the a priori standard deviations that hold (docs/adr/0203 §1); null when nothing is left.
 */
export function sanitizeSurvey(s: SurveySettings | null | undefined): SurveySettings | null {
  if (!s) return null;
  const k = s.refraction;
  const kept: SurveySettings = {
    ...(k !== undefined && refractionHolds(k) && k !== REFRACTION ? { refraction: k } : {}),
    ...(s.faceHz !== undefined && toleranceHolds(s.faceHz) ? { faceHz: s.faceHz } : {}),
    ...(s.index !== undefined && toleranceHolds(s.index) ? { index: s.index } : {}),
    ...(s.faceSlope !== undefined && toleranceHolds(s.faceSlope) ? { faceSlope: s.faceSlope } : {}),
    ...(s.twoWay !== undefined && toleranceHolds(s.twoWay) ? { twoWay: s.twoWay } : {}),
    ...(s.traverseAngle !== undefined && toleranceHolds(s.traverseAngle) ? { traverseAngle: s.traverseAngle } : {}),
    ...(s.traverseCoord !== undefined && toleranceHolds(s.traverseCoord) ? { traverseCoord: s.traverseCoord } : {}),
    ...(s.groundHeight !== undefined && groundHeightHolds(s.groundHeight) ? { groundHeight: s.groundHeight } : {}),
    ...(s.sigmaDirection !== undefined && sigmaHolds(s.sigmaDirection) ? { sigmaDirection: s.sigmaDirection } : {}),
    ...(s.sigmaDistance !== undefined && sigmaHolds(s.sigmaDistance) ? { sigmaDistance: s.sigmaDistance } : {}),
    ...(s.sigmaPpm !== undefined && sigmaPartHolds(s.sigmaPpm) ? { sigmaPpm: s.sigmaPpm } : {}),
    ...(s.sigmaCentering !== undefined && sigmaPartHolds(s.sigmaCentering) ? { sigmaCentering: s.sigmaCentering } : {}),
    ...(s.sigmaZenith !== undefined && sigmaHolds(s.sigmaZenith) ? { sigmaZenith: s.sigmaZenith } : {}),
    ...(s.sigmaLevelling !== undefined && sigmaHolds(s.sigmaLevelling) ? { sigmaLevelling: s.sigmaLevelling } : {}),
  };
  // Reduced to the grid only when asked for and with a height to do it with.
  if (s.reduceToGrid === true && kept.groundHeight !== undefined) kept.reduceToGrid = true;
  return Object.keys(kept).length ? kept : null;
}

const sameSurvey = (a: SurveySettings | null, b: SurveySettings | null): boolean =>
  a === b ||
  (a !== null &&
    b !== null &&
    a.refraction === b.refraction &&
    a.faceHz === b.faceHz &&
    a.index === b.index &&
    a.faceSlope === b.faceSlope &&
    a.twoWay === b.twoWay &&
    a.traverseAngle === b.traverseAngle &&
    a.traverseCoord === b.traverseCoord &&
    a.groundHeight === b.groundHeight &&
    a.reduceToGrid === b.reduceToGrid &&
    a.sigmaDirection === b.sigmaDirection &&
    a.sigmaDistance === b.sigmaDistance &&
    a.sigmaPpm === b.sigmaPpm &&
    a.sigmaCentering === b.sigmaCentering &&
    a.sigmaZenith === b.sigmaZenith &&
    a.sigmaLevelling === b.sigmaLevelling);

/**
 * Whether `second` may be the second system of a project in `crs`: another system, and the project has one — the
 * registry's or its own definition (`custom`; docs/adr/0167 §1, 0168 §1).
 */
export const secondAllowed = (crs: CrsDef, second: number | null | undefined, custom = false): second is number =>
  typeof second === 'number' && second !== 0 && second !== crs.srid && (crs.kind !== 'local' || custom);

export const PROJECT_SETTINGS_DEFAULTS: ProjectSettingsData = {
  srid: DEFAULT_SRID,
  lengthDecimals: 3,
  areaDecimals: 2,
  areaUnit: 'm2',
  angleUnit: 'grad',
  plotScale: 1000,
  workspace: 'gis',
  drawingFont: 'barlow',
};

/**
 * Settings that belong to a project, not to the user: they travel with the
 * .kcad file, so everyone who opens the project sees the same CRS, units
 * and plot scale. User preferences (theme, snap apertures…) live in
 * app/state.ts instead.
 */
export class ProjectSettings {
  readonly crs: Signal<CrsDef>;
  readonly lengthDecimals: Signal<number>;
  readonly areaDecimals: Signal<number>;
  readonly areaUnit: Signal<AreaUnit>;
  readonly angleUnit: Signal<AngleUnit>;
  readonly plotScale: Signal<number>;
  readonly workspace: Signal<Workspace | null>;
  readonly drawingFont: Signal<DrawingFont>;
  /** A local project's drawing unit; metres when none is set (and for any project with a coordinate system). */
  readonly drawingUnit: Signal<DrawingUnit>;
  /** The second coordinate system's SRID, or null: never the project's own, never a local project's (docs/adr/0167 §1). */
  readonly secondSrid: Signal<number | null>;
  /** The project's own coordinate system when it is a definition, or null; only without an EPSG code (docs/adr/0168 §1). */
  readonly customCrs: Signal<CrsDefinition | null>;
  /** The second system's definition, or null; never with a second EPSG code, never without a system of the project's. */
  readonly secondCustomCrs: Signal<CrsDefinition | null>;
  /** The project's datum choices (docs/adr/0168 §3). */
  readonly datumTransforms: Signal<readonly DatumTransform[]>;
  /** The project's survey constants and tolerances, or null: k = 0.13 and no tolerance (docs/adr/0169 §3). */
  readonly survey: Signal<SurveySettings | null>;
  /** The project's named layer states, as a project keeps them (docs/adr/0177 §4; `sanitizeLayerStates`). */
  readonly layerStates: Signal<readonly LayerState[]>;
  /** The project's text styles, as a project keeps them (docs/adr/0183 §2; `sanitizedTextStyles`). */
  readonly textStyles: Signal<readonly TextStyleDef[]>;
  /** The project's dimension styles, as a project keeps them (docs/adr/0183 §3; `sanitizedDimensionStyles`). */
  readonly dimensionStyles: Signal<readonly DimensionStyleDef[]>;
  /** The project's topology rules, tolerance and exceptions, or null, as a project keeps them (docs/adr/0202 §1; `sanitizeTopology`). */
  readonly topology: Signal<TopologySettings | null>;
  /** The project's annotation heights, or null, as a project keeps them (docs/adr/0205 §1; `sanitizedAnnotationHeights`). */
  readonly annotation: Signal<AnnotationHeights | null>;
  /** The project's connections to map services, without their secrets (docs/adr/0208 §2). */
  readonly connections: Signal<readonly ServiceConnection[]>;
  /** The project's networks, as a project keeps them (docs/adr/0209 §2; `sanitizedNetworks`). */
  readonly networks: Signal<readonly NetworkDef[]>;
  /** The project's own `@` variables, as a project keeps them (docs/adr/0214 §2.3; `sanitizedVariables`). */
  readonly variables: Signal<readonly ProjectVariable[]>;
  /** Bumped on any change; the document marks itself dirty from this. */
  readonly changed = new Signal(0);

  constructor(init: Partial<ProjectSettingsData> = {}) {
    const d = { ...PROJECT_SETTINGS_DEFAULTS, ...init };
    this.crs = new Signal(crsBySrid(d.srid) ?? crsBySrid(DEFAULT_SRID)!);
    this.lengthDecimals = new Signal(d.lengthDecimals);
    this.areaDecimals = new Signal(d.areaDecimals);
    this.areaUnit = new Signal(d.areaUnit);
    this.angleUnit = new Signal(d.angleUnit);
    this.plotScale = new Signal(d.plotScale);
    this.workspace = new Signal(typeOf(d.workspace));
    this.drawingFont = new Signal(d.drawingFont);
    this.drawingUnit = new Signal(d.drawingUnit ?? 'm');
    this.customCrs = new Signal(this.crs.value.kind === 'local' ? (d.customCrs ?? null) : null);
    const custom = this.customCrs.value !== null;
    this.secondSrid = new Signal(secondAllowed(this.crs.value, d.secondSrid, custom) ? d.secondSrid : null);
    this.secondCustomCrs = new Signal(this.secondSrid.value === null && (this.crs.value.kind !== 'local' || custom) ? (d.secondCustomCrs ?? null) : null);
    this.datumTransforms = new Signal<readonly DatumTransform[]>(d.datumTransforms ?? []);
    this.survey = new Signal(sanitizeSurvey(d.survey), sameSurvey);
    this.layerStates = new Signal<readonly LayerState[]>(sanitizeLayerStates(d.layerStates ?? []));
    this.textStyles = new Signal<readonly TextStyleDef[]>(sanitizedTextStyles(d.textStyles ?? []));
    this.dimensionStyles = new Signal<readonly DimensionStyleDef[]>(sanitizedDimensionStyles(d.dimensionStyles ?? []));
    this.topology = new Signal(sanitizeTopology(d.topology), sameTopology);
    this.annotation = new Signal(sanitizedAnnotationHeights(d.annotation) ?? null, sameHeights);
    this.connections = new Signal<readonly ServiceConnection[]>(structuredClone(d.connections ?? []), sameJson);
    this.networks = new Signal<readonly NetworkDef[]>(sanitizedNetworks(d.networks ?? []), sameJson);
    this.variables = new Signal<readonly ProjectVariable[]>(sanitizedVariables(d.variables ?? []), sameJson);
    watchAll(
      [
        this.crs,
        this.lengthDecimals,
        this.areaDecimals,
        this.areaUnit,
        this.angleUnit,
        this.plotScale,
        this.workspace,
        this.drawingFont,
        this.drawingUnit,
        this.secondSrid,
        this.customCrs,
        this.secondCustomCrs,
        this.datumTransforms,
        this.survey,
        this.layerStates,
        this.textStyles,
        this.dimensionStyles,
        this.topology,
        this.annotation,
        this.connections,
        this.networks,
        this.variables,
      ],
      () => this.changed.update((v) => v + 1),
    );
  }

  toJSON(): ProjectSettingsData {
    return {
      srid: this.crs.value.srid,
      lengthDecimals: this.lengthDecimals.value,
      areaDecimals: this.areaDecimals.value,
      areaUnit: this.areaUnit.value,
      angleUnit: this.angleUnit.value,
      plotScale: this.plotScale.value,
      // A type not asked yet is not written (docs/adr/0165 §1).
      ...(this.workspace.value ? { workspace: this.workspace.value } : {}),
      drawingFont: this.drawingFont.value,
      // Metres are not written: a file names a unit only when it has another (KCAD schema 11).
      ...(this.drawingUnit.value !== 'm' ? { drawingUnit: this.drawingUnit.value } : {}),
      // Written only when there is one (KCAD schema 12).
      ...(this.secondSrid.value !== null ? { secondSrid: this.secondSrid.value } : {}),
      // Written only when there are (KCAD schema 13).
      ...(this.customCrs.value ? { customCrs: this.customCrs.value } : {}),
      ...(this.secondCustomCrs.value ? { secondCustomCrs: this.secondCustomCrs.value } : {}),
      ...(this.datumTransforms.value.length ? { datumTransforms: [...this.datumTransforms.value] } : {}),
      // Written only when there are (KCAD schema 14).
      ...(this.survey.value ? { survey: { ...this.survey.value } } : {}),
      // Written only when there are (KCAD schema 19).
      ...(this.layerStates.value.length ? { layerStates: structuredClone([...this.layerStates.value]) } : {}),
      // Written only when there are (KCAD schema 21).
      ...(this.textStyles.value.length ? { textStyles: structuredClone([...this.textStyles.value]) } : {}),
      ...(this.dimensionStyles.value.length ? { dimensionStyles: structuredClone([...this.dimensionStyles.value]) } : {}),
      // Written only when there are (KCAD schema 27).
      ...(this.topology.value ? { topology: structuredClone(this.topology.value) } : {}),
      // Written only when there are (KCAD schema 30).
      ...(this.annotation.value ? { annotation: { ...this.annotation.value } } : {}),
      // Written only when there are (KCAD schema 32).
      ...(this.connections.value.length ? { connections: structuredClone([...this.connections.value]) } : {}),
      // Written only when there are (KCAD schema 33).
      ...(this.networks.value.length ? { networks: structuredClone([...this.networks.value]) } : {}),
      // Written only when there are (KCAD schema 37).
      ...(this.variables.value.length ? { variables: structuredClone([...this.variables.value]) } : {}),
    };
  }

  /** A kind of annotation's height on paper, mm: the project's, else its default (docs/adr/0205 §1). */
  annotationMm(kind: AnnotationKind): number {
    return annotationMm(this.annotation.value ?? undefined, kind);
  }

  /** A kind of annotation's height in the drawing at the plot scale, metres (docs/adr/0205 §1). */
  annotationHeight(kind: AnnotationKind): number {
    return paperHeight(this.annotationMm(kind), this.plotScale.value);
  }

  /** The refraction coefficient k of trigonometric heights: the project's, or 0.13 (docs/adr/0169 §3). */
  get refraction(): number {
    return this.survey.value?.refraction ?? REFRACTION;
  }

  /** The project's mean ellipsoidal height for the ground values, or null (docs/adr/0171 §2). */
  get groundHeight(): number | null {
    return this.survey.value?.groundHeight ?? null;
  }

  /** Whether the survey windows take lengths between the ground and the grid (docs/adr/0171 §4). */
  get reducesToGrid(): boolean {
    const s = this.survey.value;
    return s?.reduceToGrid === true && s.groundHeight !== undefined;
  }

  /** The second coordinate system, when the project has one the registry knows (docs/adr/0167 §1). */
  get second(): CrsDef | null {
    const srid = this.secondSrid.value;
    return srid === null ? null : (crsBySrid(srid) ?? null);
  }

  /** Whether the project has a coordinate system: the registry's, or its own definition (docs/adr/0168 §1). */
  get hasSystem(): boolean {
    return this.crs.value.kind !== 'local' || this.customCrs.value !== null;
  }

  /** The unit lengths are typed and read in: a project without a coordinate system has its own, any other metres (docs/adr/0165 §2). */
  get unit(): DrawingUnit {
    return this.hasSystem ? 'm' : this.drawingUnit.value;
  }

  /** Takes a whole snapshot's settings: one without a type has its type not asked yet. */
  replace(data: ProjectSettingsData): void {
    this.assign({
      ...data,
      secondSrid: data.secondSrid ?? null,
      customCrs: data.customCrs ?? null,
      secondCustomCrs: data.secondCustomCrs ?? null,
      datumTransforms: data.datumTransforms ?? [],
      survey: data.survey ?? null,
      layerStates: data.layerStates ?? [],
      textStyles: data.textStyles ?? [],
      dimensionStyles: data.dimensionStyles ?? [],
      topology: data.topology ?? null,
      annotation: data.annotation ?? null,
      connections: data.connections ?? [],
      networks: data.networks ?? [],
      variables: data.variables ?? [],
    });
    this.workspace.set(typeOf(data.workspace));
    this.drawingUnit.set(data.drawingUnit ?? 'm');
  }

  /**
   * Applies a (partial) snapshot. Unknown SRIDs are rejected, not guessed. A
   * second system that is no longer another system than the project's own,
   * or is a local project's, goes (docs/adr/0167 §1).
   */
  assign(data: ProjectSettingsPatch): void {
    if (data.srid !== undefined) {
      const crs = crsBySrid(data.srid);
      if (!crs) throw new Error(`EPSG:${data.srid} tanımlı değil`);
      this.crs.set(crs);
    }
    if (data.lengthDecimals !== undefined) this.lengthDecimals.set(data.lengthDecimals);
    if (data.areaDecimals !== undefined) this.areaDecimals.set(data.areaDecimals);
    if (data.areaUnit !== undefined) this.areaUnit.set(data.areaUnit);
    if (data.angleUnit !== undefined) this.angleUnit.set(data.angleUnit);
    if (data.plotScale !== undefined) this.plotScale.set(data.plotScale);
    // A partial patch without a type keeps the project's (`replace` takes a whole snapshot's).
    if (data.workspace !== undefined) this.workspace.set(typeOf(data.workspace));
    if (data.drawingFont !== undefined) this.drawingFont.set(data.drawingFont);
    if (data.drawingUnit !== undefined) this.drawingUnit.set(data.drawingUnit);
    // A definition of its own only without an EPSG code; a second system only where it may be, an EPSG code before a
    // definition (docs/adr/0167 §1, 0168 §1).
    const custom = data.customCrs === undefined ? this.customCrs.value : data.customCrs;
    this.customCrs.set(this.crs.value.kind === 'local' ? custom : null);
    const second = data.secondSrid === undefined ? this.secondSrid.value : data.secondSrid;
    this.secondSrid.set(secondAllowed(this.crs.value, second, this.customCrs.value !== null) ? second : null);
    const secondCustom = data.secondCustomCrs === undefined ? this.secondCustomCrs.value : data.secondCustomCrs;
    this.secondCustomCrs.set(this.secondSrid.value === null && this.hasSystem ? secondCustom : null);
    if (data.datumTransforms !== undefined) this.datumTransforms.set(data.datumTransforms);
    if (data.survey !== undefined) this.survey.set(sanitizeSurvey(data.survey));
    if (data.layerStates !== undefined) this.layerStates.set(sanitizeLayerStates(data.layerStates));
    if (data.textStyles !== undefined) this.textStyles.set(sanitizedTextStyles(data.textStyles));
    if (data.dimensionStyles !== undefined) this.dimensionStyles.set(sanitizedDimensionStyles(data.dimensionStyles));
    if (data.topology !== undefined) this.topology.set(sanitizeTopology(data.topology));
    if (data.annotation !== undefined) this.annotation.set(sanitizedAnnotationHeights(data.annotation ?? undefined) ?? null);
    if (data.connections !== undefined) this.connections.set(structuredClone([...data.connections]));
    if (data.networks !== undefined) this.networks.set(sanitizedNetworks(data.networks));
    if (data.variables !== undefined) this.variables.set(sanitizedVariables(data.variables));
  }

  /** The project's text style `id`, or null (Standart, or one it no longer has). */
  textStyle(id: string | undefined): TextStyleDef | null {
    return id === undefined ? null : (this.textStyles.value.find((s) => s.id === id) ?? null);
  }

  /** The project's dimension style `id`, or null (Standart, or one it no longer has). */
  dimensionStyle(id: string | undefined): DimensionStyleDef | null {
    return id === undefined ? null : (this.dimensionStyles.value.find((s) => s.id === id) ?? null);
  }
}

/**
 * The layer states as a project keeps them (the contract's `sanitized_layer_states`, docs/adr/0177 §4): of those with
 * the same id or name (the spaces at its ends aside) the first, none with an empty one; in each, of the same node the
 * first, none with an empty id.
 */
export function sanitizeLayerStates(states: readonly LayerState[]): LayerState[] {
  const out: LayerState[] = [];
  for (const s of states) {
    const name = s.name.trim();
    if (!s.id || !name || out.some((t) => t.id === s.id || t.name.trim() === name)) continue;
    const nodes: LayerState['nodes'] = [];
    for (const n of s.nodes) if (n.node && !nodes.some((m) => m.node === n.node)) nodes.push(n);
    out.push({ ...s, nodes });
  }
  return out;
}
