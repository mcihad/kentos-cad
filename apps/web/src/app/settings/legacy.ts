/**
 * The one migration of the web's older preference store (TODOS.md SET-04,
 * docs/adr/0023): `localStorage kentos.prefs.v1`, one JSON object of the
 * `PreferencesData` fields as `persistedSignals` wrote them, into the typed
 * layers. Every known field is checked by the shared rules and goes to its
 * setting's own layer (graphics to the device's); a value the rules refuse
 * is left behind and named in the migration record. The older entry itself
 * is never changed or removed: it stays as the backup.
 */
import type { SettingDiagnostic } from '../../contracts/generated/SettingDiagnostic';
import type { SettingsMigration } from '../../contracts/generated/SettingsMigration';
import { checkWrite, type SettingsLayer } from '../../core/settings/rules';
import { SETTINGS_SCHEMA, settingDescriptor } from '../../core/settings/schema';

export const LEGACY_PREFS = 'kentos.prefs.v1';

/**
 * The older fields and the settings they became; the same names as today's
 * `PreferencesData` except the drawing quality, which is two settings now.
 */
export const LEGACY_FIELDS: Readonly<Record<string, string>> = {
  defaultSrid: 'newProjects.srid',
  defaultWorkspace: 'newProjects.workspace',
  defaultDrawingFont: 'newProjects.drawingFont',
  snapAperture: 'drafting.snapAperture',
  pickAperture: 'drafting.pickAperture',
  snapEndpoint: 'snap.endpoint',
  snapMidpoint: 'snap.midpoint',
  snapCenter: 'snap.center',
  snapNode: 'snap.node',
  snapIntersection: 'snap.intersection',
  snapPerpendicular: 'snap.perpendicular',
  snapNearest: 'snap.nearest',
  snapTangent: 'snap.tangent',
  polarIncrement: 'drafting.polarIncrement',
  crosshair: 'appearance.crosshair',
  uiScale: 'appearance.uiScale',
  accent: 'appearance.accent',
  uiFont: 'appearance.uiFont',
  rendererPreference: 'graphics.backend',
  cursorInput: 'drafting.cursorInput',
  hoverInfo: 'drafting.hoverInfo',
  symbolSize: 'graphics.symbolSize',
  lineWeights: 'graphics.lineWeights',
  startScreen: 'appearance.startScreen',
  shell: 'appearance.shell',
};

/** The older drawing quality: 4× anti-aliasing at full resolution, full resolution alone, or neither. */
const QUALITY: Readonly<Record<string, { msaa: number; hiDpi: boolean }>> = {
  high: { msaa: 4, hiDpi: true },
  balanced: { msaa: 1, hiDpi: true },
  fast: { msaa: 1, hiDpi: false },
};

export interface Migrated {
  user: SettingsLayer;
  device: SettingsLayer;
  record: SettingsMigration;
}

/**
 * The typed values of an older `kentos.prefs.v1` text, and the record of what
 * moved and what did not. Text that is not a JSON object moves nothing; the
 * record says so and the older entry keeps it.
 */
export function migrateLegacyPrefs(text: string, at: Date): Migrated {
  const user: SettingsLayer = {};
  const device: SettingsLayer = {};
  const dropped: SettingDiagnostic[] = [];
  let moved = 0;
  let old: unknown;
  try {
    old = JSON.parse(text);
  } catch {
    old = null;
  }
  const put = (field: string, key: string, value: unknown) => {
    const d = settingDescriptor(key)!;
    const layer = d.scope === 'device' ? 'device' : 'user';
    const checked = checkWrite(SETTINGS_SCHEMA, layer, key, value);
    if (checked.ok) {
      (layer === 'device' ? device : user)[key] = checked.value;
      moved++;
    } else dropped.push({ layer, key: field, code: checked.code });
  };
  if (typeof old === 'object' && old !== null && !Array.isArray(old)) {
    for (const [field, value] of Object.entries(old)) {
      if (field === 'renderQuality') {
        const q = typeof value === 'string' && Object.prototype.hasOwnProperty.call(QUALITY, value) ? QUALITY[value] : null;
        if (q) {
          put(field, 'graphics.msaa', q.msaa);
          put(field, 'graphics.hiDpi', q.hiDpi);
        } else dropped.push({ layer: 'device', key: field, code: typeof value === 'string' ? 'not_allowed' : 'wrong_type' });
        continue;
      }
      const key = Object.prototype.hasOwnProperty.call(LEGACY_FIELDS, field) ? LEGACY_FIELDS[field] : undefined;
      if (key) put(field, key, value);
      else dropped.push({ layer: 'user', key: field, code: 'unknown_key' });
    }
  } else dropped.push({ layer: 'user', key: LEGACY_PREFS, code: old === null && text.trim() !== 'null' ? 'not_json' : 'not_settings' });
  return { user, device, record: { from: `localStorage ${LEGACY_PREFS}`, at: at.toISOString(), moved, dropped } };
}
