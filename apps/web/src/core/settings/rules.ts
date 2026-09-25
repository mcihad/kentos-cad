/**
 * The settings rules, the web's twin of `crates/shared/contracts/src/settings/rules.rs`
 * (docs/adr/0023): validating a value, which layer may hold which setting,
 * resolving the layers into the requested and the effective value, reading
 * and writing the stored document, presets. Both run
 * `fixtures/settings/v1/cases.json` and must report the same codes; a change
 * is made in both and there. Checks run in the Rust order: the key, the
 * layer, the type, whole number, the range, the choices.
 */
import type { ResolveReason } from '../../contracts/generated/ResolveReason';
import type { ResolvedSetting } from '../../contracts/generated/ResolvedSetting';
import type { SettingConstraint } from '../../contracts/generated/SettingConstraint';
import type { SettingDescriptor } from '../../contracts/generated/SettingDescriptor';
import type { SettingDiagnostic } from '../../contracts/generated/SettingDiagnostic';
import type { SettingErrorCode } from '../../contracts/generated/SettingErrorCode';
import type { SettingScope } from '../../contracts/generated/SettingScope';
import type { SettingsFile } from '../../contracts/generated/SettingsFile';
import type { SettingsMigration } from '../../contracts/generated/SettingsMigration';
import type { SettingsPolicy } from '../../contracts/generated/SettingsPolicy';
import type { SettingsPreset } from '../../contracts/generated/SettingsPreset';
import type { SettingsSchema } from '../../contracts/generated/SettingsSchema';
import { settingDescriptor, type SettingValue } from './schema';

export const SETTINGS_FORMAT = 'kentos.settings';
export const SETTINGS_VERSION = 1;

export type Checked = { readonly ok: true; readonly value: SettingValue } | { readonly ok: false; readonly code: SettingErrorCode };

/** A layer of values by key, as stored: what it holds may be invalid until checked. */
export type SettingsLayer = Record<string, unknown>;

const fail = (code: SettingErrorCode): Checked => ({ ok: false, code });

/** Equal values: numbers by value, the rest exactly. */
export const sameValue = (a: unknown, b: unknown): boolean => a === b;

/** The value as the setting keeps it, or why it cannot be one. */
export function validateSetting(d: SettingDescriptor, value: unknown): Checked {
  let v: SettingValue;
  if (d.type === 'boolean') {
    if (typeof value !== 'boolean') return fail('wrong_type');
    v = value;
  } else if (d.type === 'enum') {
    if (typeof value !== 'string') return fail('wrong_type');
    v = value;
  } else {
    if (typeof value !== 'number' || !Number.isFinite(value)) return fail('wrong_type');
    if (d.type === 'integer' && !Number.isInteger(value)) return fail('not_integer');
    if ((d.min !== undefined && value < d.min) || (d.max !== undefined && value > d.max)) return fail('out_of_range');
    // −0 is kept as 0, as the Rust side writes it.
    v = value === 0 ? 0 : value;
  }
  if (d.choices?.length && !d.choices.some((c) => sameValue(c.value, v))) return fail('not_allowed');
  return { ok: true, value: v };
}

const RANK: Partial<Record<SettingScope, number>> = { user: 1, device: 2, session: 3 };

/**
 * Whether a layer may hold a setting of `scope`: its own layer or a more
 * specific one (user < device < session); project settings only in the project.
 */
export function layerHolds(layer: SettingScope, scope: SettingScope): boolean {
  if (scope === 'project' || layer === 'project') return scope === layer;
  const l = RANK[layer];
  const s = RANK[scope];
  return l !== undefined && s !== undefined && l >= s;
}

/** The value `layer` would keep for `key`, or why it may not (key, layer, value). */
export function checkWrite(schema: SettingsSchema, layer: SettingScope, key: string, value: unknown): Checked {
  const d = settingDescriptor(key, schema);
  if (!d) return fail('unknown_key');
  if (!layerHolds(layer, d.scope)) return fail('wrong_scope');
  return validateSetting(d, value);
}

export interface SettingsLayers {
  user?: SettingsLayer;
  device?: SettingsLayer;
  session?: SettingsLayer;
  project?: SettingsLayer;
}

export interface Resolution {
  readonly settings: ReadonlyMap<string, ResolvedSetting>;
  readonly diagnostics: readonly SettingDiagnostic[];
}

const NO_POLICY: SettingsPolicy = { source: '', rules: {} };
const own = (o: object, key: string): boolean => Object.prototype.hasOwnProperty.call(o, key);

/**
 * Resolves every setting (TODOS.md SET-02, SET-03): the most specific layer's
 * valid value is requested (a project setting only from the project), the
 * organisation's policy locks or bounds it, and the host's constraints bring
 * it within what the device can use. Invalid values, values in a layer that
 * cannot hold them and unknown keys are reported and skipped.
 */
export function resolveSettings(
  schema: SettingsSchema,
  layers: SettingsLayers,
  policy: SettingsPolicy = NO_POLICY,
  constraints: Readonly<Record<string, SettingConstraint | undefined>> = {},
): Resolution {
  const diagnostics: SettingDiagnostic[] = [];
  const ordered: [SettingScope, SettingsLayer][] = [
    ['user', layers.user ?? {}],
    ['device', layers.device ?? {}],
    ['session', layers.session ?? {}],
    ['project', layers.project ?? {}],
  ];
  for (const [layer, values] of ordered)
    for (const [key, value] of Object.entries(values)) {
      const checked = checkWrite(schema, layer, key, value);
      if (!checked.ok) diagnostics.push({ layer, key, code: checked.code });
    }
  for (const key of Object.keys(policy.rules)) if (!settingDescriptor(key, schema)) diagnostics.push({ layer: 'organization', key, code: 'unknown_key' });
  const settings = new Map<string, ResolvedSetting>();
  for (const d of schema.settings) settings.set(d.key, resolveOne(d, layers, policy, constraints[d.key], diagnostics));
  return { settings, diagnostics };
}

function resolveOne(d: SettingDescriptor, layers: SettingsLayers, policy: SettingsPolicy, constraint: SettingConstraint | undefined, diagnostics: SettingDiagnostic[]): ResolvedSetting {
  let requested: SettingValue = d.default;
  let source: SettingScope = 'default';
  for (const [layer, values] of [
    ['project', layers.project],
    ['session', layers.session],
    ['device', layers.device],
    ['user', layers.user],
  ] as const) {
    if (!values || !layerHolds(layer, d.scope) || !own(values, d.key)) continue;
    const checked = validateSetting(d, values[d.key]);
    if (!checked.ok) continue;
    requested = checked.value;
    source = layer;
    break;
  }

  let effective: SettingValue = requested;
  let reason: ResolveReason | undefined;
  let detail: string | undefined;
  let locked = false;
  const rule = own(policy.rules, d.key) ? policy.rules[d.key] : undefined;
  if (rule) {
    const badRule = (code: SettingErrorCode) => diagnostics.push({ layer: 'organization', key: d.key, code });
    const before = effective;
    if (rule.value !== undefined) {
      const lock = validateSetting(d, rule.value);
      if (lock.ok) {
        locked = true;
        effective = lock.value;
      } else badRule(lock.code);
    } else {
      if (typeof effective === 'number' && (d.type === 'integer' || d.type === 'number')) {
        const n = effective;
        const raised = rule.min === undefined ? n : Math.max(n, rule.min);
        const bounded = rule.max === undefined ? raised : Math.min(raised, rule.max);
        if (bounded !== n) effective = within(d, bounded, bounded > n);
      }
      if (rule.allowed?.length) {
        const allowed = validValues(d, rule.allowed);
        if (!allowed.length) badRule('not_allowed');
        else if (!allowed.some((a) => sameValue(a, effective))) effective = nearestAllowed(allowed, effective);
      }
    }
    if (!sameValue(before, effective)) {
      reason = locked ? 'organization_locked' : 'organization_limit';
      detail = policy.source;
    }
  }
  // The device last: whatever was asked or allowed, it can only use what it has.
  if (constraint?.allowed.length) {
    const allowed = validValues(d, constraint.allowed);
    if (allowed.length && !allowed.some((a) => sameValue(a, effective))) {
      effective = nearestAllowed(allowed, effective);
      reason = constraint.reason;
      detail = constraint.detail;
    }
  }
  if (sameValue(effective, requested)) {
    reason = undefined;
    detail = undefined;
  }
  const out: ResolvedSetting = { key: d.key, requested, source, effective, locked };
  if (reason) out.reason = reason;
  if (detail !== undefined) out.detail = detail;
  return out;
}

function validValues(d: SettingDescriptor, values: readonly unknown[]): SettingValue[] {
  const out: SettingValue[] = [];
  for (const v of values) {
    const checked = validateSetting(d, v);
    if (checked.ok) out.push(checked.value);
  }
  return out;
}

/** A bounded number made valid: onto a choice (the nearest below), else whole toward the bound. */
function within(d: SettingDescriptor, n: number, raised: boolean): SettingValue {
  if (d.choices?.length)
    return nearestAllowed(
      d.choices.map((c) => c.value),
      n,
    );
  if (d.type === 'integer') return raised ? Math.ceil(n) : Math.floor(n);
  return n;
}

/** The allowed value nearest `value` from below (numbers: the largest not above it, else the smallest); otherwise the first. */
export function nearestAllowed(allowed: readonly SettingValue[], value: SettingValue): SettingValue {
  if (allowed.some((a) => sameValue(a, value))) return value;
  if (typeof value === 'number') {
    const numbers = allowed.filter((a): a is number => typeof a === 'number');
    if (numbers.length) {
      const below = numbers.filter((a) => a <= value);
      return below.length ? Math.max(...below) : Math.min(...numbers);
    }
  }
  return allowed[0] ?? value;
}

// ── The stored document ──────────────────────────────────────────────

export type FileRead = { readonly ok: true; readonly file: SettingsFile; readonly diagnostics: readonly SettingDiagnostic[] } | { readonly ok: false; readonly code: SettingErrorCode };

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const SCOPES: readonly string[] = ['default', 'organization', 'user', 'device', 'project', 'session'] satisfies SettingScope[];
const CODES: readonly string[] = [
  'unknown_key',
  'wrong_type',
  'not_integer',
  'out_of_range',
  'not_allowed',
  'wrong_scope',
  'sensitive',
  'not_json',
  'not_settings',
  'unsupported_version',
] satisfies SettingErrorCode[];

/**
 * Reads a stored or exported settings document (TODOS.md SET-04), as
 * `SettingsFile::from_json` does: a leading BOM is skipped; another format or
 * version is refused; within the layers an invalid, sensitive or misplaced
 * value is dropped and reported, the others are kept, and an unknown key is
 * kept untouched and reported.
 */
export function readSettingsFile(text: string, schema: SettingsSchema): FileRead {
  const body = text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  let root: unknown;
  try {
    root = JSON.parse(body);
  } catch {
    return { ok: false, code: 'not_json' };
  }
  if (!isObject(root) || root.format !== SETTINGS_FORMAT) return { ok: false, code: 'not_settings' };
  const version = root.version;
  if (typeof version !== 'number' || !Number.isInteger(version)) return { ok: false, code: 'not_settings' };
  if (version !== SETTINGS_VERSION) return { ok: false, code: 'unsupported_version' };
  const diagnostics: SettingDiagnostic[] = [];
  const layers: Partial<Record<'user' | 'device', SettingsLayer>> = {};
  for (const name of ['user', 'device'] as const) {
    const raw = root[name];
    if (raw === undefined || raw === null) layers[name] = {};
    else if (isObject(raw)) layers[name] = readLayer(schema, name, raw, diagnostics);
    else return { ok: false, code: 'not_settings' };
  }
  const migrations = Array.isArray(root.migrations) && root.migrations.every(isMigration) ? (root.migrations as SettingsMigration[]) : [];
  const file: SettingsFile = { format: SETTINGS_FORMAT, version: SETTINGS_VERSION, user: layers.user ?? {}, device: layers.device ?? {} };
  if (migrations.length) file.migrations = migrations;
  return { ok: true, file, diagnostics };
}

function readLayer(schema: SettingsSchema, layer: 'user' | 'device', values: Record<string, unknown>, diagnostics: SettingDiagnostic[]): SettingsLayer {
  const kept: SettingsLayer = {};
  for (const [key, value] of Object.entries(values)) {
    const d = settingDescriptor(key, schema);
    if (!d) {
      // From a newer version: kept untouched, used by none.
      diagnostics.push({ layer, key, code: 'unknown_key' });
      kept[key] = value;
      continue;
    }
    if (d.sensitive) {
      diagnostics.push({ layer, key, code: 'sensitive' });
      continue;
    }
    const checked = checkWrite(schema, layer, key, value);
    if (checked.ok) kept[key] = checked.value;
    else diagnostics.push({ layer, key, code: checked.code });
  }
  return kept;
}

/** A migration record as the Rust reader accepts one; one malformed record drops them all (not the values). */
function isMigration(m: unknown): boolean {
  const u32 = (v: unknown) => typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= 0xffffffff;
  return (
    isObject(m) &&
    typeof m.from === 'string' &&
    typeof m.at === 'string' &&
    u32(m.moved) &&
    Array.isArray(m.dropped) &&
    m.dropped.every((d) => isObject(d) && SCOPES.includes(String(d.layer)) && typeof d.layer === 'string' && typeof d.key === 'string' && typeof d.code === 'string' && CODES.includes(d.code))
  );
}

/** The document as text, sensitive values left out: the layout the Rust side writes (keys sorted, two-space indent). */
export function writeSettingsFile(file: SettingsFile, schema: SettingsSchema): string {
  const keep = (layer: SettingsLayer): SettingsLayer =>
    Object.fromEntries(
      Object.keys(layer)
        .filter((k) => !settingDescriptor(k, schema)?.sensitive)
        .sort()
        .map((k) => [k, layer[k]]),
    );
  const out: SettingsFile = { format: SETTINGS_FORMAT, version: SETTINGS_VERSION, user: keep(file.user), device: keep(file.device) };
  if (file.migrations?.length) out.migrations = file.migrations;
  return `${JSON.stringify(out, null, 2)}\n`;
}

// ── Presets ──────────────────────────────────────────────────────────

export function settingsPreset(schema: SettingsSchema, id: string): SettingsPreset | undefined {
  return schema.presets.find((p) => p.id === id);
}

/** The preset of `group` whose every value equals the one `valueOf` gives, if any (the window says “Özel” otherwise). */
export function matchingPreset(schema: SettingsSchema, group: string, valueOf: (key: string) => unknown): SettingsPreset | undefined {
  return schema.presets.filter((p) => p.group === group).find((p) => Object.entries(p.values).every(([key, value]) => sameValue(valueOf(key), value)));
}
