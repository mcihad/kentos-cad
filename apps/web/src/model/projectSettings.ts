import { Signal, watchAll } from '../core/signal';
import type { DrawingFont } from '../contracts/generated/DrawingFont';
import type { DrawingUnit } from '../contracts/generated/DrawingUnit';
import type { Workspace } from '../contracts/generated/Workspace';
import { crsBySrid, DEFAULT_SRID, type CrsDef } from '../geo/crs';

export type AreaUnit = 'm2' | 'donum' | 'ha';
export type AngleUnit = 'grad' | 'deg';
export type { DrawingFont, DrawingUnit, Workspace };

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
}

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
    watchAll([this.crs, this.lengthDecimals, this.areaDecimals, this.areaUnit, this.angleUnit, this.plotScale, this.workspace, this.drawingFont, this.drawingUnit], () =>
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
      // A type not asked yet is not written (docs/adr/0165 §1).
      ...(this.workspace.value ? { workspace: this.workspace.value } : {}),
      drawingFont: this.drawingFont.value,
      // Metres are not written: a file names a unit only when it has another (KCAD schema 11).
      ...(this.drawingUnit.value !== 'm' ? { drawingUnit: this.drawingUnit.value } : {}),
    };
  }

  /** The unit lengths are typed and read in: a local project's own, metres for any other (docs/adr/0165 §2). */
  get unit(): DrawingUnit {
    return this.crs.value.kind === 'local' ? this.drawingUnit.value : 'm';
  }

  /** Takes a whole snapshot's settings: one without a type has its type not asked yet. */
  replace(data: ProjectSettingsData): void {
    this.assign(data);
    this.workspace.set(typeOf(data.workspace));
    this.drawingUnit.set(data.drawingUnit ?? 'm');
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
    // A partial patch without a type keeps the project's (`replace` takes a whole snapshot's).
    if (data.workspace !== undefined) this.workspace.set(typeOf(data.workspace));
    if (data.drawingFont !== undefined) this.drawingFont.set(data.drawingFont);
    if (data.drawingUnit !== undefined) this.drawingUnit.set(data.drawingUnit);
  }
}
