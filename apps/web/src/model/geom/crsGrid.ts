import { crsForgetGrid, crsLoadGrid } from '../../wasm/core';

/**
 * NTv2 grids in the core (crates/shared/geometry-core/src/crs/ntv2.rs, docs/adr/0168 §4): a file read whole and checked,
 * kept under its SHA-256 for the project's grid choices (`DatumChoice`'s `grid`), shifts applied as PROJ applies them.
 */

/** What a grid says: the datums it shifts from and to, their ellipsoids' axes (m), its subgrids, its extent (degrees: west, south, east, north). */
export interface GridInfo {
  readonly from: string;
  readonly to: string;
  readonly fromAxes: readonly [number, number];
  readonly toAxes: readonly [number, number];
  readonly subgrids: number;
  readonly extent: readonly [number, number, number, number];
}

/** Why a file is not read as a grid. */
export type GridError = 'tooLarge' | 'notNtv2' | 'notSeconds' | 'header' | 'extent' | 'count' | 'truncated' | 'values';

/** Reads `bytes` as an NTv2 grid and keeps it under `id`; what it says, or why not. */
export function loadGrid(id: string, bytes: Uint8Array): GridInfo | { readonly error: GridError } {
  return crsLoadGrid(id, bytes) as GridInfo | { readonly error: GridError };
}

/** Lets the grid kept under `id` go. */
export function forgetGrid(id: string): void {
  crsForgetGrid(id);
}
