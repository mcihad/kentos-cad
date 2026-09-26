import { describe, expect, it } from 'vitest';
import { ACCENTS, DRAWING_FONTS, UI_FONTS } from '../appearance';
import { WORKSPACES } from '../workspaces';
import { createPreferences, PREF_KEYS, PREFERENCE_DEFAULTS, type PreferencesData } from '../state';
import { settingDescriptor } from '../../core/settings/schema';
import { LEGACY_PREFS } from './legacy';
import { SETTINGS_BACKUP, SETTINGS_STORAGE, SettingsStore, type KeyValueStorage } from './store';

/** localStorage as tests see it: strings by key, nothing shared with a real profile. */
class MemoryStorage implements KeyValueStorage {
  readonly items = new Map<string, string>();
  writes = 0;
  refuseWrites = false;
  constructor(init: Record<string, string> = {}) {
    for (const [k, v] of Object.entries(init)) this.items.set(k, v);
  }
  getItem(key: string): string | null {
    return this.items.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    if (this.refuseWrites) throw new DOMException('QuotaExceededError');
    this.writes++;
    this.items.set(key, value);
  }
  removeItem(key: string): void {
    this.items.delete(key);
  }
}

const AT = new Date('2026-09-25T20:00:00Z');

/**
 * `kentos.prefs.v1` as the app wrote it before typed settings: every
 * `PreferencesData` field of that version (persistedSignals saved them all
 * together), in its field order, after a user changed many of them.
 */
const OLD_PREFS = JSON.stringify({
  defaultSrid: 5254,
  defaultWorkspace: 'cad',
  defaultDrawingFont: 'arimo',
  snapAperture: 14,
  pickAperture: 6,
  snapEndpoint: true,
  snapMidpoint: false,
  snapCenter: true,
  snapNode: true,
  snapIntersection: true,
  snapPerpendicular: false,
  snapNearest: true,
  snapTangent: true,
  polarIncrement: 30,
  crosshair: 'full',
  uiScale: 'large',
  accent: 'teal',
  uiFont: 'inter',
  rendererPreference: 'webgpu',
  renderQuality: 'balanced',
  cursorInput: false,
  hoverInfo: false,
  symbolSize: 'screen',
  lineWeights: false,
  startScreen: false,
  shell: 'ribbon',
});

/** What each old field must be as a typed preference after the migration. */
const MIGRATED: PreferencesData = {
  defaultSrid: 5254,
  defaultWorkspace: 'cad',
  defaultDrawingFont: 'arimo',
  snapAperture: 14,
  pickAperture: 6,
  snapEndpoint: true,
  snapMidpoint: false,
  snapCenter: true,
  snapNode: true,
  snapIntersection: true,
  snapPerpendicular: false,
  snapNearest: true,
  snapTangent: true,
  polarIncrement: 30,
  crosshair: 'full',
  uiScale: 'large',
  accent: 'teal',
  uiFont: 'inter',
  rendererPreference: 'webgpu',
  msaa: 1,
  hiDpi: true,
  cursorInput: false,
  hoverInfo: false,
  symbolSize: 'screen',
  lineWeights: false,
  startScreen: false,
  shell: 'ribbon',
};

const values = (store: SettingsStore): PreferencesData => Object.fromEntries(Object.entries(PREF_KEYS).map(([f, k]) => [f, store.requested(k)])) as unknown as PreferencesData;

describe('the one migration of kentos.prefs.v1', () => {
  it('moves every value the user had, graphics to the device, and leaves the old entry untouched', () => {
    const storage = new MemoryStorage({ [LEGACY_PREFS]: OLD_PREFS });
    const store = SettingsStore.open(storage, () => AT);
    expect(values(store)).toEqual(MIGRATED);
    expect(store.report.migrated).toEqual({ from: `localStorage ${LEGACY_PREFS}`, at: '2026-09-25T20:00:00.000Z', moved: 27, dropped: [] });
    expect(storage.getItem(LEGACY_PREFS)).toBe(OLD_PREFS);
    const saved = JSON.parse(storage.getItem(SETTINGS_STORAGE)!);
    expect(saved.device).toEqual({ 'graphics.backend': 'webgpu', 'graphics.hiDpi': true, 'graphics.msaa': 1 });
    expect(saved.user['drafting.snapAperture']).toBe(14);
    expect(saved.migrations).toHaveLength(1);
    // Every drawing quality maps onto the two settings it was.
    for (const [quality, msaa, hiDpi] of [
      ['high', 4, true],
      ['balanced', 1, true],
      ['fast', 1, false],
    ] as const) {
      const s = SettingsStore.open(new MemoryStorage({ [LEGACY_PREFS]: JSON.stringify({ renderQuality: quality }) }), () => AT);
      expect([s.requested('graphics.msaa'), s.requested('graphics.hiDpi')], quality).toEqual([msaa, hiDpi]);
    }
  });

  it('happens once: later the typed store is read, whatever the old entry says', () => {
    const storage = new MemoryStorage({ [LEGACY_PREFS]: OLD_PREFS });
    const first = SettingsStore.open(storage, () => AT);
    first.set('drafting.snapAperture', 20);
    first.flush();
    storage.setItem(LEGACY_PREFS, JSON.stringify({ snapAperture: 5 }));
    const again = SettingsStore.open(storage, () => AT);
    expect(again.report.migrated).toBeNull();
    expect(again.requested('drafting.snapAperture')).toBe(20);
    expect(again.migrations).toHaveLength(1);
  });

  it('leaves behind only what the rules refuse, names it, and keeps it in the old entry', () => {
    const old = JSON.stringify({ snapAperture: 40, polarIncrement: 22.5, accent: 'purple', renderQuality: 'ultra', cursorInput: 'yes', olderSetting: 1, uiScale: 'large' });
    const storage = new MemoryStorage({ [LEGACY_PREFS]: old });
    const store = SettingsStore.open(storage, () => AT);
    expect(store.report.migrated?.moved).toBe(1);
    expect(store.report.migrated?.dropped.map((d) => [d.key, d.code])).toEqual([
      ['snapAperture', 'out_of_range'],
      ['polarIncrement', 'not_allowed'],
      ['accent', 'not_allowed'],
      ['renderQuality', 'not_allowed'],
      ['cursorInput', 'wrong_type'],
      ['olderSetting', 'unknown_key'],
    ]);
    expect(store.requested('appearance.uiScale')).toBe('large');
    expect(store.requested('drafting.snapAperture')).toBe(11);
    expect(storage.getItem(LEGACY_PREFS)).toBe(old);
  });

  it('an old entry that is not JSON moves nothing and stays as it was', () => {
    const storage = new MemoryStorage({ [LEGACY_PREFS]: '{"snapAperture": 14,' });
    const store = SettingsStore.open(storage, () => AT);
    expect(store.report.migrated?.moved).toBe(0);
    expect(store.report.migrated?.dropped[0].code).toBe('not_json');
    expect(storage.getItem(LEGACY_PREFS)).toBe('{"snapAperture": 14,');
    expect(store.requested('drafting.snapAperture')).toBe(11);
  });

  it('a fresh profile starts from the defaults and records no migration', () => {
    const storage = new MemoryStorage();
    const store = SettingsStore.open(storage, () => AT);
    expect(values(store)).toEqual(PREFERENCE_DEFAULTS);
    expect(store.report).toEqual({ migrated: null, recovered: null, memoryOnly: false });
    expect(JSON.parse(storage.getItem(SETTINGS_STORAGE)!).migrations).toBeUndefined();
  });
});

describe('recovery', () => {
  it('a stored document that is not JSON is kept as the backup and the settings start over from the old entry', () => {
    const storage = new MemoryStorage({ [SETTINGS_STORAGE]: '{"format":"kentos.settings",', [LEGACY_PREFS]: OLD_PREFS });
    const store = SettingsStore.open(storage, () => AT);
    expect(store.report.recovered).toEqual({ code: 'not_json', diagnostics: [], backup: SETTINGS_BACKUP });
    expect(storage.getItem(SETTINGS_BACKUP)).toBe('{"format":"kentos.settings",');
    expect(values(store)).toEqual(MIGRATED);
  });

  it('invalid values fall back to their defaults, the rest stay, and the whole text is kept as the backup', () => {
    const text = JSON.stringify({ format: 'kentos.settings', version: 1, user: { 'drafting.snapAperture': 99, 'drafting.polarIncrement': 30, 'zz.future': 1 }, device: { 'graphics.msaa': 'x', 'graphics.hiDpi': false } });
    const storage = new MemoryStorage({ [SETTINGS_STORAGE]: text });
    const store = SettingsStore.open(storage, () => AT);
    expect(store.report.recovered?.diagnostics.map((d) => [d.layer, d.key, d.code])).toEqual([
      ['user', 'drafting.snapAperture', 'out_of_range'],
      ['device', 'graphics.msaa', 'wrong_type'],
    ]);
    expect(storage.getItem(SETTINGS_BACKUP)).toBe(text);
    expect([store.requested('drafting.snapAperture'), store.requested('drafting.polarIncrement'), store.requested('graphics.msaa'), store.requested('graphics.hiDpi')]).toEqual([11, 30, 4, false]);
    const rewritten = JSON.parse(storage.getItem(SETTINGS_STORAGE)!);
    expect(rewritten.user).toEqual({ 'drafting.polarIncrement': 30, 'zz.future': 1 });
  });

  it('a document of a newer version is kept as the backup, not read', () => {
    const text = JSON.stringify({ format: 'kentos.settings', version: 2, user: { 'drafting.snapAperture': 14 } });
    const storage = new MemoryStorage({ [SETTINGS_STORAGE]: text });
    const store = SettingsStore.open(storage, () => AT);
    expect(store.report.recovered?.code).toBe('unsupported_version');
    expect(storage.getItem(SETTINGS_BACKUP)).toBe(text);
    expect(store.requested('drafting.snapAperture')).toBe(11);
  });

  it('storage that refuses writes keeps the settings in memory and says so', () => {
    const storage = new MemoryStorage({ [LEGACY_PREFS]: OLD_PREFS });
    storage.refuseWrites = true;
    const store = SettingsStore.open(storage, () => AT);
    expect(values(store)).toEqual(MIGRATED);
    expect(store.inMemory).toBe(true);
    expect(store.set('drafting.snapAperture', 9)).toBeNull();
    expect(store.requested('drafting.snapAperture')).toBe(9);
  });

  it('never writes over a corrupt document it could not back up', () => {
    const storage = new MemoryStorage({ [SETTINGS_STORAGE]: 'garbage' });
    storage.refuseWrites = true;
    const store = SettingsStore.open(storage, () => AT);
    expect(store.inMemory).toBe(true);
    store.set('drafting.snapAperture', 9);
    store.flush();
    expect(storage.getItem(SETTINGS_STORAGE)).toBe('garbage');
  });
});

describe('writing, reset, export and import', () => {
  it('refuses a value the rules refuse and keeps the old one', () => {
    const store = SettingsStore.memory();
    expect(store.set('drafting.snapAperture', 3)).toBe('out_of_range');
    expect(store.set('graphics.msaa', 8, 'user')).toBe('wrong_scope');
    expect(store.set('project.srid', 2320)).toBe('wrong_scope');
    expect(store.set('nope.key', 1)).toBe('unknown_key');
    expect(store.requested('drafting.snapAperture')).toBe(11);
  });

  it('reset forgets the stored values; export and import carry them across', () => {
    const storage = new MemoryStorage();
    const store = SettingsStore.open(storage, () => AT);
    store.choose({ 'drafting.snapAperture': 17, 'graphics.msaa': 8, 'appearance.uiScale': 'xlarge' });
    const text = store.exportText();
    store.reset();
    expect([store.requested('drafting.snapAperture'), store.requested('graphics.msaa')]).toEqual([11, 4]);
    const other = SettingsStore.open(new MemoryStorage(), () => AT);
    expect(other.importText(text)).toEqual({ ok: true, diagnostics: [] });
    expect([other.requested('drafting.snapAperture'), other.requested('graphics.msaa'), other.requested('appearance.uiScale')]).toEqual([17, 8, 'xlarge']);
    // A file that is not a settings document changes nothing.
    expect(other.importText('{"format":"kentos.document","version":1}')).toEqual({ ok: false, code: 'not_settings' });
    expect(other.requested('drafting.snapAperture')).toBe(17);
    store.reset(['appearance.uiScale']);
    expect(JSON.parse(store.exportText()).user).toEqual({});
  });

  it('the drawing engine: requested stays, the device limits the effective value and says why', () => {
    const store = SettingsStore.memory();
    store.choose({ 'graphics.msaa': 8 });
    store.setConstraint('graphics.msaa', { allowed: [1, 4], reason: 'device_unsupported', detail: 'WebGL2 en çok 4× destekliyor.' });
    expect(store.resolved('graphics.msaa')).toMatchObject({ requested: 8, effective: 4, reason: 'device_unsupported', detail: 'WebGL2 en çok 4× destekliyor.', source: 'device' });
    store.setConstraint('graphics.msaa', null);
    expect(store.effective('graphics.msaa')).toBe(8);
  });

  it('?renderer= overrides the device for the session, until the user chooses', () => {
    const store = SettingsStore.memory();
    store.choose({ 'graphics.backend': 'webgl2' });
    expect(store.set('graphics.backend', 'webgpu', 'session')).toBeNull();
    expect(store.resolved('graphics.backend')).toMatchObject({ requested: 'webgpu', source: 'session' });
    store.choose({ 'graphics.backend': 'webgl2' });
    expect(store.resolved('graphics.backend')).toMatchObject({ requested: 'webgl2', source: 'device' });
    expect(store.set('graphics.backend', 'vulkan', 'session')).toBe('not_allowed');
  });

  it('the organisation locks a preference; the preference is kept', () => {
    const store = SettingsStore.memory();
    store.choose({ 'drafting.cursorInput': false });
    store.setPolicy({ source: 'Kurum politikası', rules: { 'drafting.cursorInput': { value: true } } });
    expect(store.resolved('drafting.cursorInput')).toMatchObject({ requested: false, effective: true, locked: true, reason: 'organization_locked' });
    store.setPolicy({ source: '', rules: {} });
    expect(store.effective('drafting.cursorInput')).toBe(false);
  });
});

describe('preferences over the settings service', () => {
  it('a signal set writes the preference; a refused value goes back to the one in use', () => {
    const store = SettingsStore.memory();
    const prefs = createPreferences(store);
    prefs.snapAperture.set(20);
    expect(store.requested('drafting.snapAperture')).toBe(20);
    prefs.snapAperture.set(500);
    expect(prefs.snapAperture.value).toBe(20);
    expect(store.requested('drafting.snapAperture')).toBe(20);
  });

  it('the signals hold the effective value and follow imports, resets and constraints', () => {
    const store = SettingsStore.memory();
    const prefs = createPreferences(store);
    prefs.msaa.set(8);
    store.setConstraint('graphics.msaa', { allowed: [1, 4], reason: 'device_unsupported', detail: '' });
    expect(prefs.msaa.value).toBe(4);
    expect(store.requested('graphics.msaa')).toBe(8);
    store.reset();
    expect(prefs.msaa.value).toBe(4);
    store.setConstraint('graphics.msaa', null);
    expect(prefs.msaa.value).toBe(4);
  });

  it('every preference is a setting of the schema, with the same kind of default', () => {
    for (const [field, key] of Object.entries(PREF_KEYS)) {
      const d = settingDescriptor(key);
      expect(d, `${field} → ${key}`).toBeDefined();
      expect(d!.scope === 'user' || d!.scope === 'device', key).toBe(true);
      expect(d!.hosts, key).toContain('web');
    }
    // The choices the schema offers are the app's own lists.
    const choices = (key: string) => settingDescriptor(key)!.choices!.map((c) => c.value);
    expect(choices('appearance.accent')).toEqual(ACCENTS.map((a) => a.id));
    expect(choices('appearance.uiFont')).toEqual(UI_FONTS.map((f) => f.id));
    expect(choices('newProjects.drawingFont')).toEqual(DRAWING_FONTS.map((f) => f.id));
    expect(choices('project.drawingFont')).toEqual(DRAWING_FONTS.map((f) => f.id));
    expect(choices('project.workspace')).toEqual(WORKSPACES.map((w) => w.id));
    expect(choices('newProjects.workspace')).toEqual(WORKSPACES.filter((w) => w.status === 'ready').map((w) => w.id));
  });
});
