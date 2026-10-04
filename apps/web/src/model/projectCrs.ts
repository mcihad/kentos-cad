import type { CrsDefinition } from '../contracts/generated/CrsDefinition';
import type { CustomDatum } from '../contracts/generated/CustomDatum';
import type { DatumTransform } from '../contracts/generated/DatumTransform';
import type { RegistryDatum } from '../contracts/generated/RegistryDatum';
import { CRS_REGISTRY, crsBySrid, crsCode, crsTitle, LOCAL_SRID } from '../geo/crs';
import { systemOf, type Datum, type DatumChoice, type System } from './geom/crsTransform';
import type { ProjectSettings } from './projectSettings';

/**
 * The project's coordinate systems as the transforms read them (docs/adr/0168 §1–§3; the desktop's
 * `kentos_project::systems`): its own system and its second, each one of the registry's or a definition of the
 * project's (`customCrs`, `secondCustomCrs`), named; its datum choices (`datumTransforms`). The shared cases are
 * fixtures/geodesy/v1/project.json (scripts/fixtures/project_crs_cases.py, from the ADR's rules).
 */

/** What of a project's settings names its systems: the snapshot's fields, or a `ProjectSettings`' values (`crsSettings`). */
export interface CrsSettings {
  readonly srid: number;
  readonly customCrs?: CrsDefinition | null;
  readonly secondSrid?: number | null;
  readonly secondCustomCrs?: CrsDefinition | null;
  readonly datumTransforms?: readonly DatumTransform[];
}

/** A coordinate system of the project's, named: one of the registry's, or a definition of the project's own. */
export interface NamedSystem {
  /** “TUREF / TM30”, “Şantiye”. */
  readonly name: string;
  /** As a sentence names it: “TUREF / TM30 (EPSG:5254)”, “Şantiye (özel sistem)”. */
  readonly title: string;
  /** As a value or a chip shows it: “EPSG:5254”, “Özel sistem”. */
  readonly code: string;
  /** As the transforms read it; null for a definition whose base is not a projected system of the registry. */
  readonly system: System | null;
}

/** A project's settings as `ownSystem`, `secondSystem` and `datumChoices` read them. */
export const crsSettings = (s: ProjectSettings): CrsSettings => ({
  srid: s.crs.value.srid,
  customCrs: s.customCrs.value,
  secondSrid: s.secondSrid.value,
  secondCustomCrs: s.secondCustomCrs.value,
  datumTransforms: s.datumTransforms.value,
});

const registryNamed = (srid: number): NamedSystem | null => {
  const c = crsBySrid(srid);
  return c ? { name: c.name, title: crsTitle(c), code: crsCode(c), system: systemOf(c) } : null;
};

/** A definition as a sentence names it: “Şantiye (özel sistem)”. */
export const definitionTitle = (d: CrsDefinition): string => `${d.name} (özel sistem)`;

/** A definition's code as a value or a chip shows it. */
export const DEFINITION_CODE = 'Özel sistem';

const definitionNamed = (d: CrsDefinition): NamedSystem => ({ name: d.name, title: definitionTitle(d), code: DEFINITION_CODE, system: definitionSystem(d) });

/** The project's own definition, when its system is one (docs/adr/0168 §1). */
const ownDefinition = (s: ProjectSettings): CrsDefinition | null => (s.crs.value.srid === LOCAL_SRID ? s.customCrs.value : null);

/** The project's system's name: “TUREF / TM30”, its definition's “Şantiye” (the desktop's `project_name`). */
export const projectCrsName = (s: ProjectSettings): string => ownDefinition(s)?.name ?? s.crs.value.name;

/** The project's system as a sentence names it: “TUREF / TM30 (EPSG:5254)”, “Şantiye (özel sistem)”, “Yerel (koordinat sistemi yok)”. */
export const projectCrsTitle = (s: ProjectSettings): string => {
  const d = ownDefinition(s);
  return d ? definitionTitle(d) : crsTitle(s.crs.value);
};

/** The project's system's code: “EPSG:5254”, “Özel sistem”, “SRID 0”. */
export const projectCrsCode = (s: ProjectSettings): string => (ownDefinition(s) ? DEFINITION_CODE : crsCode(s.crs.value));

/** Whether its values are a latitude and a longitude. */
export const isGeographic = (n: NamedSystem): boolean => n.system?.kind === 'geographic';

/** Whether the project has a system of its own the registry or its definition names. */
const hasOwn = (s: CrsSettings): boolean => (s.srid === LOCAL_SRID ? !!s.customCrs : !!crsBySrid(s.srid));

/** The project's own system: the registry's by its SRID, or its definition; null without one (SRID 0) and for an SRID the registry does not have. */
export function ownSystem(s: CrsSettings): NamedSystem | null {
  if (s.srid !== LOCAL_SRID) return registryNamed(s.srid);
  return s.customCrs ? definitionNamed(s.customCrs) : null;
}

/** The project's second system (docs/adr/0167 §1): the registry's or a definition; null without one, or without a system of the project's. */
export function secondSystem(s: CrsSettings): NamedSystem | null {
  if (!hasOwn(s)) return null;
  if (s.secondSrid !== undefined && s.secondSrid !== null && s.secondSrid !== 0 && s.secondSrid !== s.srid) return registryNamed(s.secondSrid);
  return s.secondCustomCrs ? definitionNamed(s.secondCustomCrs) : null;
}

/** The project's datum choices as the transforms take them (§3). */
export function datumChoices(s: CrsSettings): DatumChoice[] {
  return (s.datumTransforms ?? []).flatMap((t): DatumChoice[] => {
    const head = { from: t.from, to: t.to, name: t.name };
    if (t.helmert && !t.grid) return [{ ...head, helmert: t.helmert }];
    if (t.grid && !t.helmert) return [{ ...head, grid: t.grid.accuracy === undefined ? { id: t.grid.id } : { id: t.grid.id, accuracy: t.grid.accuracy } }];
    return [];
  });
}

/** A definition's datum: the registry's by name or the project's own; null unless exactly one is given. */
const datumOf = (d: { readonly datum?: RegistryDatum; readonly customDatum?: CustomDatum }): Datum | null =>
  d.datum !== undefined && d.customDatum === undefined ? d.datum : d.customDatum !== undefined && d.datum === undefined ? d.customDatum : null;

/** A definition as the transforms read it (§1); null when a local system's base is not a projected system of the registry. */
export function definitionSystem(d: CrsDefinition): System | null {
  const s = d.system;
  switch (s.kind) {
    case 'tm': {
      const datum = datumOf(s);
      if (datum === null) return null;
      return {
        kind: 'tm',
        datum,
        ...(s.latitudeOfOrigin !== undefined ? { latitudeOfOrigin: s.latitudeOfOrigin } : {}),
        centralMeridian: s.centralMeridian,
        scaleFactor: s.scaleFactor,
        falseEasting: s.falseEasting,
        falseNorthing: s.falseNorthing,
      };
    }
    case 'geographic': {
      const datum = datumOf(s);
      return datum === null ? null : { kind: 'geographic', datum };
    }
    case 'local': {
      const { srid, definition } = s.base;
      let base: System | null = null;
      if (srid !== undefined && definition === undefined) {
        const c = crsBySrid(srid);
        base = c && c.kind === 'projected' ? systemOf(c) : null;
      } else if (definition !== undefined && srid === undefined) base = definitionSystem(definition);
      return base ? { kind: 'local', base, plane: s.plane } : null;
    }
  }
}

/** A value written with its keys in order: two systems compared whatever order their fields came in. */
const canonical = (v: unknown): string =>
  JSON.stringify(v, (_k, x: unknown) =>
    x && typeof x === 'object' && !Array.isArray(x) ? Object.fromEntries(Object.entries(x as Record<string, unknown>).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) : x,
  );

/** A datum the core reads as a definition's two fields: the registry's by name, or the project's own. */
const datumFields = (d: Datum): { datum: RegistryDatum } | { customDatum: CustomDatum } =>
  typeof d === 'string'
    ? { datum: d }
    : { customDatum: { name: d.name, ellipsoid: { ...d.ellipsoid }, ...(d.toWgs84 ? { toWgs84: { ...d.toWgs84, translation: [...d.toWgs84.translation], rotation: [...d.toWgs84.rotation] } } : {}) } };

type Tm = Extract<System, { kind: 'tm' }>;

/** A transverse Mercator the core reads as a definition's. */
const tmFrom = (s: Tm) => ({
  kind: 'tm' as const,
  ...datumFields(s.datum),
  ...(s.latitudeOfOrigin !== undefined && s.latitudeOfOrigin !== 0 ? { latitudeOfOrigin: s.latitudeOfOrigin } : {}),
  centralMeridian: s.centralMeridian,
  scaleFactor: s.scaleFactor,
  falseEasting: s.falseEasting,
  falseNorthing: s.falseNorthing,
});

/**
 * A system the core reads (from WKT or PROJ, docs/adr/0168 §5) as the project's definition named `name`: the registry's
 * datums by name, any other as the project's own; a local system's base the registry's projected system with every
 * value the same, else a definition of its own named “<name> tabanı”. Null for what a definition cannot be (the
 * Pseudo-Mercator, a local system on another base). The desktop's `kentos_project::systems::definition_from`.
 */
export function definitionFrom(name: string, s: System): CrsDefinition | null {
  switch (s.kind) {
    case 'geographic':
      return { name, system: { kind: 'geographic', ...datumFields(s.datum) } };
    case 'tm':
      return { name, system: tmFrom(s) };
    case 'local': {
      if (s.base.kind !== 'tm') return null;
      const want = canonical(s.base);
      const entry = CRS_REGISTRY.find((c) => c.kind === 'projected' && canonical(systemOf(c)) === want);
      const base = entry ? { srid: entry.srid } : { definition: { name: `${name} tabanı`, system: tmFrom(s.base) } };
      return { name, system: { kind: 'local', base, plane: { ...s.plane } } };
    }
    default:
      return null;
  }
}
