import { Signal, watchAll } from '../core/signal';
import type { Workspace } from '../contracts/generated/Workspace';
import { crsBySrid, DEFAULT_SRID, type CrsDef } from '../geo/crs';

export type AreaUnit = 'm2' | 'donum' | 'ha';
export type AngleUnit = 'grad' | 'deg';
export type { Workspace };

/**
 * Every work mode a file may name (contract `Workspace`). What each shows,
 * and which can be chosen yet, is app/workspaces.ts; the model only keeps
 * the value. Files written before modes existed read as hybrid.
 */
export const WORKSPACE_IDS: readonly Workspace[] = ['hybrid', 'cad', 'gis', 'plan3d', 'disaster'];

/** Serialized form, stored inside the project file. */
export interface ProjectSettingsData {
  srid: number;
  lengthDecimals: number;
  areaDecimals: number;
  areaUnit: AreaUnit;
  angleUnit: AngleUnit;
  /** Plot scale denominator (1:1000 → 1000). */
  plotScale: number;
  /** The work mode the project opens in (menus, ribbon and tools shown). */
  workspace: Workspace;
}

export const PROJECT_SETTINGS_DEFAULTS: ProjectSettingsData = {
  srid: DEFAULT_SRID,
  lengthDecimals: 3,
  areaDecimals: 2,
  areaUnit: 'm2',
  angleUnit: 'grad',
  plotScale: 1000,
  workspace: 'hybrid',
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
  readonly workspace: Signal<Workspace>;
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
    this.workspace = new Signal(d.workspace);
    watchAll([this.crs, this.lengthDecimals, this.areaDecimals, this.areaUnit, this.angleUnit, this.plotScale, this.workspace], () =>
      this.changed.update((v) => v + 1),
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
      workspace: this.workspace.value,
    };
  }

  /** Applies a (partial) snapshot. Unknown SRIDs are rejected, not guessed. */
  assign(data: Partial<ProjectSettingsData>): void {
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
    // A snapshot without a mode (older files, partial patches) is hybrid only when it replaces the whole settings.
    if (data.workspace !== undefined) this.workspace.set(data.workspace);
  }
}
