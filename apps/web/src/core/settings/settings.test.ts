import { describe, expect, it } from 'vitest';
import cases from '../../../../../fixtures/settings/v1/cases.json?raw';
import type { SettingConstraint } from '../../contracts/generated/SettingConstraint';
import type { SettingDiagnostic } from '../../contracts/generated/SettingDiagnostic';
import type { SettingErrorCode } from '../../contracts/generated/SettingErrorCode';
import type { SettingScope } from '../../contracts/generated/SettingScope';
import type { SettingsPolicy } from '../../contracts/generated/SettingsPolicy';
import { checkWrite, matchingPreset, readSettingsFile, resolveSettings, settingsPreset, validateSetting, writeSettingsFile, type SettingsLayers } from './rules';
import { SETTINGS_SCHEMA, settingDescriptor } from './schema';

/**
 * The shared settings cases (fixtures/settings/v1): the Rust contracts run the
 * same file (crates/shared/contracts/tests/settings.rs), so the web and the
 * desktop give the same error codes and the same resolution (docs/adr/0023).
 */

type Outcome = { value?: unknown; error?: SettingErrorCode };
interface Expected {
  requested: unknown;
  source: SettingScope;
  effective: unknown;
  reason?: string;
  locked?: boolean;
}
interface Cases {
  format: string;
  version: number;
  values: { name: string; key: string; value: unknown; expect: Outcome }[];
  writes: { name: string; layer: SettingScope; key: string; value: unknown; expect: Outcome }[];
  resolve: { name: string; layers: SettingsLayers; policy?: SettingsPolicy; constraints?: Record<string, SettingConstraint>; expect: Record<string, Expected>; diagnostics: SettingDiagnostic[] }[];
  files: {
    name: string;
    text?: string;
    document?: unknown;
    expect: { error?: SettingErrorCode; user?: Record<string, unknown>; device?: Record<string, unknown>; migrations?: number; diagnostics?: SettingDiagnostic[] };
  }[];
  presets: { matching: { name: string; group: string; values: Record<string, unknown>; expect: string | null }[]; fill: { name: string; preset: string; expect: Record<string, unknown> }[] };
}

const file = JSON.parse(cases) as Cases;
const sorted = (d: readonly SettingDiagnostic[]) => d.map((x) => JSON.stringify({ layer: x.layer, key: x.key, code: x.code })).sort();
const outcome = (r: { ok: true; value: unknown } | { ok: false; code: SettingErrorCode }): Outcome => (r.ok ? { value: r.value } : { error: r.code });

describe('shared settings cases (fixtures/settings/v1)', () => {
  it('is the v1 case file', () => {
    expect(file.format).toBe('kentos.settings-cases');
    expect(file.version).toBe(1);
    expect(file.values.length).toBeGreaterThan(30);
  });

  for (const c of file.values)
    it(`değer: ${c.name}`, () => {
      const d = settingDescriptor(c.key);
      expect(d ? outcome(validateSetting(d, c.value)) : { error: 'unknown_key' }).toEqual(c.expect);
    });

  for (const c of file.writes)
    it(`yazma: ${c.name}`, () => {
      expect(outcome(checkWrite(SETTINGS_SCHEMA, c.layer, c.key, c.value))).toEqual(c.expect);
    });

  for (const c of file.resolve)
    it(`çözüm: ${c.name}`, () => {
      const r = resolveSettings(SETTINGS_SCHEMA, c.layers, c.policy, c.constraints);
      for (const [key, e] of Object.entries(c.expect)) {
        const got = r.settings.get(key)!;
        expect({ requested: got.requested, source: got.source, effective: got.effective, reason: got.reason, locked: got.locked }, key).toEqual({
          requested: e.requested,
          source: e.source,
          effective: e.effective,
          reason: e.reason,
          locked: e.locked ?? false,
        });
        expect(got.detail !== undefined, `${key}: a reason comes with its detail`).toBe(got.reason !== undefined);
      }
      expect(sorted(r.diagnostics)).toEqual(sorted(c.diagnostics));
    });

  for (const c of file.files)
    it(`dosya: ${c.name}`, () => {
      const text = c.text ?? JSON.stringify(c.document);
      const read = readSettingsFile(text, SETTINGS_SCHEMA);
      if (c.expect.error) {
        expect(read.ok ? 'okundu' : read.code).toBe(c.expect.error);
        return;
      }
      if (!read.ok) throw new Error(`reddedildi: ${read.code}`);
      expect(read.file.user).toEqual(c.expect.user ?? {});
      expect(read.file.device).toEqual(c.expect.device ?? {});
      expect(read.file.migrations?.length ?? 0).toBe(c.expect.migrations ?? 0);
      expect(sorted(read.diagnostics)).toEqual(sorted(c.expect.diagnostics ?? []));
    });

  for (const c of file.presets.matching)
    it(`hazır ayar: ${c.name}`, () => {
      expect(matchingPreset(SETTINGS_SCHEMA, c.group, (k) => c.values[k])?.id ?? null).toBe(c.expect);
    });
  for (const c of file.presets.fill)
    it(`doldurma: ${c.name}`, () => {
      expect(settingsPreset(SETTINGS_SCHEMA, c.preset)?.values).toEqual(c.expect);
    });
});

describe('settings document', () => {
  it('writes what it reads, keys sorted, keeping an unknown key', () => {
    const doc = { format: 'kentos.settings' as const, version: 1 as const, user: { 'drafting.snapAperture': 14, 'zz.future': 'keep' }, device: { 'graphics.msaa': 8 } };
    const text = writeSettingsFile(doc, SETTINGS_SCHEMA);
    expect(text).toBe(
      '{\n  "format": "kentos.settings",\n  "version": 1,\n  "user": {\n    "drafting.snapAperture": 14,\n    "zz.future": "keep"\n  },\n  "device": {\n    "graphics.msaa": 8\n  }\n}\n',
    );
    const read = readSettingsFile(text, SETTINGS_SCHEMA);
    expect(read.ok && read.file).toEqual(doc);
  });

  it('never writes a sensitive value', () => {
    const schema = { ...SETTINGS_SCHEMA, settings: SETTINGS_SCHEMA.settings.map((d) => (d.key === 'drafting.cursorInput' ? { ...d, sensitive: true } : d)) };
    const text = writeSettingsFile({ format: 'kentos.settings', version: 1, user: { 'drafting.cursorInput': false, 'drafting.snapAperture': 9 }, device: {} }, schema);
    expect(text).not.toContain('cursorInput');
    const read = readSettingsFile('{"format":"kentos.settings","version":1,"user":{"drafting.cursorInput":false}}', schema);
    expect(read.ok && read.diagnostics.map((d) => d.code)).toEqual(['sensitive']);
  });
});
