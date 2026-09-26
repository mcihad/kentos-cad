/**
 * The typed settings schema (docs/adr/0023), read from the file the Rust
 * contracts generate (`crates/shared/contracts/src/settings`,
 * `KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings`): every
 * setting's key, type, domain, default, scope, hosts and texts, the groups and
 * the presets. The desktop reads the same list natively.
 */
import raw from '../../contracts/generated/settingsSchema.json?raw';
import type { ResolveReason } from '../../contracts/generated/ResolveReason';
import type { SettingDescriptor } from '../../contracts/generated/SettingDescriptor';
import type { SettingErrorCode } from '../../contracts/generated/SettingErrorCode';
import type { SettingsSchema } from '../../contracts/generated/SettingsSchema';

/** A setting's value: every setting is a switch, a number or a choice (text). */
export type SettingValue = boolean | number | string;

function load(text: string): SettingsSchema {
  const schema = JSON.parse(text) as SettingsSchema;
  if (schema.format !== 'kentos.settings-schema' || schema.version !== 1) throw new Error(`Ayar şeması okunamadı: ${String(schema.format)} ${String(schema.version)}`);
  return schema;
}

export const SETTINGS_SCHEMA: SettingsSchema = load(raw);

const byKey = new Map(SETTINGS_SCHEMA.settings.map((d) => [d.key, d]));

export function settingDescriptor(key: string, schema: SettingsSchema = SETTINGS_SCHEMA): SettingDescriptor | undefined {
  return schema === SETTINGS_SCHEMA ? byKey.get(key) : schema.settings.find((d) => d.key === key);
}

const errors = new Map(SETTINGS_SCHEMA.errors.map((e) => [e.code, e.message]));
const reasons = new Map(SETTINGS_SCHEMA.reasons.map((r) => [r.reason, r.message]));

/** What an error code means and how to fix it, as the desktop says it too. */
export const settingErrorMessage = (code: SettingErrorCode): string => errors.get(code) ?? code;

/** Why the value in use differs from the one asked for. */
export const resolveReasonMessage = (reason: ResolveReason): string => reasons.get(reason) ?? reason;

/** The schema's default of a setting this app knows (a missing key is a programming error). */
export function settingDefault(key: string): SettingValue {
  const d = byKey.get(key);
  if (!d) throw new Error(`Ayar şemasında yok: ${key}`);
  return d.default;
}
