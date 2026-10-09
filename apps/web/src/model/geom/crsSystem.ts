import type { CrsDef } from '../../geo/crs';
import type { System } from './crsTransform';

/*
 * A registry entry as the core's transforms read it, without the geometry core: the services worker
 * (io/services/worker.ts) gives it to the services module, which carries its own copy of the transforms.
 */

/** The core's system for a registry entry (the desktop's `transform_system`); null for the local one. */
export function systemOf(crs: CrsDef): System | null {
  if (crs.projection === 'Pseudo-Mercator') return { kind: 'mercator' };
  if (crs.datum === 'LOCAL') return null;
  if (crs.kind === 'geographic') return { kind: 'geographic', datum: crs.datum };
  if (crs.kind !== 'projected' || crs.centralMeridian === undefined) return null;
  return {
    kind: 'tm',
    datum: crs.datum,
    centralMeridian: crs.centralMeridian,
    scaleFactor: crs.scaleFactor ?? 1,
    falseEasting: crs.falseEasting ?? 0,
    falseNorthing: crs.falseNorthing ?? 0,
  };
}

