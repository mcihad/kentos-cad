import { op } from '../../wasm/core';
import type { System } from './crsTransform';

/**
 * Coordinate systems read from WKT and PROJ strings and written as them (crates/shared/geometry-core/src/crs/text.rs,
 * docs/adr/0168 §5): WKT 1 (OGC and ESRI), WKT 2 and PROJ strings in; WKT 1, PROJ strings and, for a local system, WKT 2
 * out. A datum is the registry's when its name, ellipsoid and seven parameters say so, else the text's own; only the
 * transverse Mercator, latitude and longitude, and a local system derived by an affine transform; metres and degrees;
 * Greenwich.
 */

/** Why a text gave no system: not a definition, something not read (the word it stopped at), a unit, a prime meridian. */
export interface TextRefusal {
  readonly kind: 'syntax' | 'unsupported' | 'unit' | 'meridian';
  readonly detail: string;
}

/** A system read from a text: its name, the system, the EPSG code it names, the registry's system it is (or whose grid it shares). */
export interface ReadSystem {
  readonly name: string;
  readonly system: System;
  readonly authority?: number;
  readonly registry?: { readonly srid: number; readonly exact: boolean };
}

/** A system from WKT or a PROJ string, or why not. */
export const crsReadText = op<(text: string) => ReadSystem | { readonly error: TextRefusal }>('crsReadText');

/** A system as WKT (WKT 1; a local system as WKT 2, its base named `baseName`); null for the Pseudo-Mercator. */
export const crsWriteWkt = op<(name: string, system: System, baseName?: string | null) => string | null>('crsWriteWkt');

/** A system as a PROJ string; null for a local system and the Pseudo-Mercator. */
export const crsWriteProj = op<(system: System) => string | null>('crsWriteProj');
