/**
 * The ready basemaps (docs/adr/0208 §1): fixtures/services/v1/presets.json, the file the desktop reads too
 * (`kentos_services::presets`). Each is a service layer and, for one that asks for proof, the connection it needs
 * (its secret is the user's, kept on the device).
 */
import catalog from '../../../../fixtures/services/v1/presets.json';
import type { ServiceConnection } from '../contracts/generated/ServiceConnection';
import type { ServiceLayer } from '../contracts/generated/ServiceLayer';

export interface PresetGroup {
  readonly id: string;
  readonly name: string;
}

export interface Preset {
  readonly id: string;
  readonly group: string;
  readonly name: string;
  readonly icon: string;
  readonly service: ServiceLayer;
  readonly connection?: ServiceConnection;
  readonly attributionUrl?: string;
  /** The most requests at once to its host (OpenStreetMap's 2). */
  readonly perHost?: number;
  readonly note: string;
}

export const PRESET_GROUPS: readonly PresetGroup[] = catalog.groups;
export const PRESETS: readonly Preset[] = catalog.presets as unknown as readonly Preset[];

export function preset(id: string): Preset | undefined {
  return PRESETS.find((p) => p.id === id);
}

/** A preset's command: `basemap.` and its id in camel case (`osm-standard` → `basemap.osmStandard`). */
export function basemapCommand(id: string): string {
  return `basemap.${id.replace(/-([a-z0-9])/g, (_, c: string) => c.toUpperCase())}`;
}

/** The layer a preset adds: its own service, `preset` naming it and its connection's id (`presets::layer_of`). */
export function layerOf(p: Preset): ServiceLayer {
  return { ...p.service, preset: p.id, ...(p.connection ? { connection: p.connection.id } : {}) };
}

/** The most requests at once a service's host takes: its preset's word, else 6 (`presets::per_host`). */
export function perHost(service: ServiceLayer): number {
  return (service.preset !== undefined ? preset(service.preset)?.perHost : undefined) ?? 6;
}

/** A service layer's icon in the layer tree: its preset's, else its kind's (the desktop's `service_icon`). */
export function serviceIcon(service: ServiceLayer): string {
  const p = service.preset !== undefined ? preset(service.preset) : undefined;
  if (p) return p.icon;
  switch (service.kind) {
    case 'vector':
      return 'basemapVector';
    case 'google':
      return 'basemapGoogleRoad';
    case 'xyz':
      return 'basemap';
    default:
      return 'serviceAdd';
  }
}
