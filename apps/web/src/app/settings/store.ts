/**
 * The web's typed settings service (TODOS.md SET-02..05, docs/adr/0023): the
 * user and device layers kept in this browser (`localStorage
 * kentos.settings.v1`, the `kentos.settings` v1 document the desktop keeps
 * in its file and both export), the session layer in memory, the
 * organisation's policy and the device's constraints as inputs, and the
 * resolution of them all by the shared rules (core/settings).
 *
 * Opening it reads the stored document once. When there is none yet, the
 * older preference store (`kentos.prefs.v1`) is migrated into it, once; the
 * older entry is left as it was, as the backup. A document that cannot be
 * read, or holds values the rules refuse, is copied whole to
 * `kentos.settings.v1.backup` before anything is written over it, so no
 * value the user had is lost.
 *
 * Browser storage may be missing (private mode) or full: the service then
 * works in memory and says so in its report.
 */
import type { ResolvedSetting } from '../../contracts/generated/ResolvedSetting';
import type { SettingConstraint } from '../../contracts/generated/SettingConstraint';
import type { SettingDiagnostic } from '../../contracts/generated/SettingDiagnostic';
import type { SettingErrorCode } from '../../contracts/generated/SettingErrorCode';
import type { SettingsFile } from '../../contracts/generated/SettingsFile';
import type { SettingsMigration } from '../../contracts/generated/SettingsMigration';
import type { SettingsPolicy } from '../../contracts/generated/SettingsPolicy';
import { Signal } from '../../core/signal';
import { checkWrite, readSettingsFile, resolveSettings, writeSettingsFile, SETTINGS_FORMAT, SETTINGS_VERSION, type Resolution, type SettingsLayer } from '../../core/settings/rules';
import { SETTINGS_SCHEMA, settingDescriptor, type SettingValue } from '../../core/settings/schema';
import { LEGACY_PREFS, migrateLegacyPrefs } from './legacy';

/** Where the typed settings live in this browser. */
export const SETTINGS_STORAGE = 'kentos.settings.v1';
/** The text a recovery replaced: kept whole, so nothing the user had is lost. */
export const SETTINGS_BACKUP = 'kentos.settings.v1.backup';

/** The part of `Storage` the service uses (tests pass a map). */
export interface KeyValueStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

/** The layers a caller may write. */
export type WritableLayer = 'user' | 'device' | 'session';

/** What opening the service found. */
export interface OpenReport {
  /** The migration made now, when the typed store did not exist yet. */
  readonly migrated: SettingsMigration | null;
  /** The stored document could not be used as it was: why (a code for the whole, else the values dropped), and where its text was kept. */
  readonly recovered: { readonly code: SettingErrorCode | null; readonly diagnostics: readonly SettingDiagnostic[]; readonly backup: string } | null;
  /** Browser storage refused to be read or written: the settings last until the page closes. */
  readonly memoryOnly: boolean;
}

/** An import that could not be taken, or what it left out. */
export type ImportResult = { readonly ok: true; readonly diagnostics: readonly SettingDiagnostic[] } | { readonly ok: false; readonly code: SettingErrorCode };

const empty = (): SettingsFile => ({ format: SETTINGS_FORMAT, version: SETTINGS_VERSION, user: {}, device: {} });

export class SettingsStore {
  readonly schema = SETTINGS_SCHEMA;
  /** Bumped whenever what resolves changes: a value, the policy, a constraint. */
  readonly revision = new Signal(0);
  readonly report: OpenReport;
  private file: SettingsFile;
  private session: SettingsLayer = {};
  private policy: SettingsPolicy = { source: '', rules: {} };
  private constraints: Record<string, SettingConstraint> = {};
  private resolution: Resolution;
  private readonly storage: KeyValueStorage | null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private memoryOnly: boolean;

  private constructor(storage: KeyValueStorage | null, file: SettingsFile, report: OpenReport) {
    this.storage = storage;
    this.file = file;
    this.report = report;
    this.memoryOnly = report.memoryOnly;
    this.resolution = this.resolve();
  }

  /** A service over nothing but memory (tests, and a browser without storage). */
  static memory(): SettingsStore {
    return new SettingsStore(null, empty(), { migrated: null, recovered: null, memoryOnly: true });
  }

  /**
   * Opens the settings kept in `storage` (see the module comment for the
   * order: stored document, else the one migration, recovering what cannot
   * be read). `now` dates the migration record.
   */
  static open(storage: KeyValueStorage | null, now: () => Date = () => new Date()): SettingsStore {
    if (!storage) return SettingsStore.memory();
    let text: string | null;
    let legacy: string | null;
    try {
      text = storage.getItem(SETTINGS_STORAGE);
      legacy = storage.getItem(LEGACY_PREFS);
    } catch {
      return SettingsStore.memory();
    }
    let file = empty();
    let migrated: SettingsMigration | null = null;
    let recovered: OpenReport['recovered'] = null;
    let write = false;
    if (text !== null) {
      const read = readSettingsFile(text, SETTINGS_SCHEMA);
      if (read.ok) {
        file = read.file;
        const lost = read.diagnostics.filter((d) => d.code !== 'unknown_key');
        if (lost.length) {
          recovered = { code: null, diagnostics: lost, backup: SETTINGS_BACKUP };
          write = true;
        }
      } else {
        recovered = { code: read.code, diagnostics: [], backup: SETTINGS_BACKUP };
        write = true;
      }
    }
    if (recovered) {
      try {
        storage.setItem(SETTINGS_BACKUP, text ?? '');
      } catch {
        // Kept nowhere else: work in memory rather than write over the only copy.
        return new SettingsStore(null, file, { migrated: null, recovered, memoryOnly: true });
      }
    }
    // No usable document: the older store, once (a document unreadable as a whole starts over from it too).
    if (text === null || (recovered && recovered.code !== null)) {
      if (legacy !== null) {
        const m = migrateLegacyPrefs(legacy, now());
        file = { ...empty(), user: m.user, device: m.device, migrations: [m.record] };
        migrated = m.record;
      }
      write = true;
    }
    const store = new SettingsStore(storage, file, { migrated, recovered, memoryOnly: false });
    if (write) store.flush();
    return store;
  }

  // ── Reading ──────────────────────────────────────────────────────────

  resolved(key: string): ResolvedSetting {
    const r = this.resolution.settings.get(key);
    if (!r) throw new Error(`Ayar şemasında yok: ${key}`);
    return r;
  }

  /** The value in use. */
  effective(key: string): SettingValue {
    return this.resolved(key).effective;
  }

  /** The value asked for (what the settings window edits). */
  requested(key: string): SettingValue {
    return this.resolved(key).requested;
  }

  /**
   * What `key` would resolve to if the user chose `value` (as `choose` would
   * store it): the settings window shows the effective value and its reason
   * before Kaydet.
   */
  preview(key: string, value: unknown): ResolvedSetting {
    const target = this.home(key);
    const layers = { user: { ...this.file.user }, device: { ...this.file.device }, session: { ...this.session } };
    if (target) {
      if (target !== 'session') delete layers.session[key];
      layers[target][key] = value;
    }
    const r = resolveSettings(this.schema, layers, this.policy, this.constraints).settings.get(key);
    if (!r) throw new Error(`Ayar şemasında yok: ${key}`);
    return r;
  }

  /** Values the layers hold but resolution did not use. */
  get diagnostics(): readonly SettingDiagnostic[] {
    return this.resolution.diagnostics;
  }

  /** The migrations recorded in the stored document, oldest first. */
  get migrations(): readonly SettingsMigration[] {
    return this.file.migrations ?? [];
  }

  /** Whether settings are only kept until the page closes (storage missing or refused). */
  get inMemory(): boolean {
    return this.memoryOnly;
  }

  // ── Writing ──────────────────────────────────────────────────────────

  /**
   * Writes one value into `layer` (by default the setting's own: user or
   * device; session settings into the session). Returns why it was refused,
   * or null. Nothing is written when it is refused.
   */
  set(key: string, value: unknown, layer?: WritableLayer): SettingErrorCode | null {
    const target = layer ?? this.home(key);
    if (!target) return settingDescriptor(key) ? 'wrong_scope' : 'unknown_key';
    const checked = checkWrite(this.schema, target, key, value);
    if (!checked.ok) return checked.code;
    const values = this.layer(target);
    if (Object.prototype.hasOwnProperty.call(values, key) && values[key] === checked.value) return null;
    values[key] = checked.value;
    this.changed(target !== 'session');
    return null;
  }

  /**
   * The user chose these values (the settings window's Kaydet, a menu, the
   * status bar): each goes to its setting's own layer and replaces a session
   * override of it (`?renderer=` holds only until the user picks a backend).
   * One change; the refused values are named and not taken.
   */
  choose(values: Readonly<Record<string, unknown>>): Record<string, SettingErrorCode> {
    const refused: Record<string, SettingErrorCode> = {};
    let stored = false;
    let any = false;
    for (const [key, value] of Object.entries(values)) {
      const target = this.home(key);
      const checked = target ? checkWrite(this.schema, target, key, value) : ({ ok: false, code: settingDescriptor(key) ? 'wrong_scope' : 'unknown_key' } as const);
      if (!checked.ok || !target) {
        refused[key] = checked.ok ? 'wrong_scope' : checked.code;
        continue;
      }
      if (target !== 'session' && Object.prototype.hasOwnProperty.call(this.session, key)) {
        delete this.session[key];
        any = true;
      }
      const layer = this.layer(target);
      if (Object.prototype.hasOwnProperty.call(layer, key) && layer[key] === checked.value) continue;
      layer[key] = checked.value;
      any = true;
      stored ||= target !== 'session';
    }
    if (any) this.changed(stored);
    return refused;
  }

  /** Forgets the stored values of `keys` (all when none are given): they go back to their defaults. */
  reset(keys?: readonly string[]): void {
    const drop = (layer: SettingsLayer) => {
      for (const key of keys ?? Object.keys(layer)) delete layer[key];
    };
    drop(this.file.user);
    drop(this.file.device);
    this.changed(true);
  }

  /** Drops a session override (e.g. `?renderer=` once the user chooses a backend). */
  clearSession(key: string): void {
    if (!Object.prototype.hasOwnProperty.call(this.session, key)) return;
    delete this.session[key];
    this.changed(false);
  }

  /**
   * The organisation's policy: locks and bounds over every layer. No server
   * sends one yet; this is the explicit input it will use (docs/adr/0023).
   */
  setPolicy(policy: SettingsPolicy): void {
    this.policy = policy;
    this.changed(false);
  }

  /** What the device can use for one setting (the GPU's sample counts, WebGPU's presence); null removes it. */
  setConstraint(key: string, constraint: SettingConstraint | null): void {
    const before = JSON.stringify(this.constraints[key] ?? null);
    if (constraint) this.constraints[key] = constraint;
    else delete this.constraints[key];
    if (JSON.stringify(constraint) !== before) this.changed(false);
  }

  // ── Export and import ────────────────────────────────────────────────

  /** The stored settings as a `kentos.settings` document (the file the desktop reads too). */
  exportText(): string {
    return writeSettingsFile(this.file, this.schema);
  }

  /**
   * Takes the user and device values of a `kentos.settings` document in place
   * of the stored ones. The document is checked first; a refused document
   * changes nothing. Values the rules refuse are left out and named.
   */
  importText(text: string): ImportResult {
    const read = readSettingsFile(text, this.schema);
    if (!read.ok) return read;
    this.file = { ...this.file, user: { ...read.file.user }, device: { ...read.file.device } };
    this.changed(true);
    return { ok: true, diagnostics: read.diagnostics };
  }

  /** Writes the stored document now (the page is closing, a test reads it). */
  flush(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    if (!this.storage || this.memoryOnly) return;
    try {
      this.storage.setItem(SETTINGS_STORAGE, writeSettingsFile(this.file, this.schema));
    } catch {
      this.memoryOnly = true;
    }
  }

  dispose(): void {
    this.flush();
  }

  // ── Inside ───────────────────────────────────────────────────────────

  private home(key: string): WritableLayer | null {
    const scope = settingDescriptor(key)?.scope;
    return scope === 'user' || scope === 'device' || scope === 'session' ? scope : null;
  }

  private layer(layer: WritableLayer): SettingsLayer {
    return layer === 'session' ? this.session : this.file[layer];
  }

  private resolve(): Resolution {
    return resolveSettings(this.schema, { user: this.file.user, device: this.file.device, session: this.session }, this.policy, this.constraints);
  }

  private changed(persist: boolean): void {
    this.resolution = this.resolve();
    if (persist && this.storage && !this.memoryOnly) {
      if (this.timer !== null) clearTimeout(this.timer);
      this.timer = setTimeout(() => this.flush(), 250);
    }
    this.revision.update((v) => v + 1);
  }
}
