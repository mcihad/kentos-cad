import { describe, expect, it } from 'vitest';
import { ProjectSettings, REFRACTION, sanitizeSurvey } from './projectSettings';

/**
 * The project's survey settings (docs/adr/0169 §3) as the settings model keeps them: the contract's
 * `SurveySettings::sanitized` (k within [−1, 1] and not the default, tolerances above zero; nothing left: none), the
 * refraction the computations read, a patch that removes them, and a whole snapshot's settings without them.
 */
describe('ProjectSettings › survey (docs/adr/0169 §3)', () => {
  it('keeps only what holds, as the contract does', () => {
    expect(sanitizeSurvey(undefined)).toBeNull();
    expect(sanitizeSurvey({})).toBeNull();
    expect(sanitizeSurvey({ refraction: REFRACTION })).toBeNull();
    expect(sanitizeSurvey({ refraction: 1.5, faceHz: 0, index: -1, faceSlope: Number.NaN })).toBeNull();
    expect(sanitizeSurvey({ refraction: -1, faceSlope: 0.005 })).toEqual({ refraction: -1, faceSlope: 0.005 });
    expect(sanitizeSurvey({ index: 1e-5, refraction: 0.2 })).toEqual({ refraction: 0.2, index: 1e-5 });
  });

  it('reads k, writes the settings only when the project has them, and drops them on a patch or a whole snapshot', () => {
    const s = new ProjectSettings();
    expect(s.refraction).toBe(REFRACTION);
    expect('survey' in s.toJSON()).toBe(false);
    const changed = s.changed.value;
    s.assign({ survey: { refraction: 0.14, faceHz: 3e-5 } });
    expect(s.refraction).toBe(0.14);
    expect(s.toJSON().survey).toEqual({ refraction: 0.14, faceHz: 3e-5 });
    expect(s.changed.value).toBe(changed + 1);
    // The same values again are no change.
    s.assign({ survey: { faceHz: 3e-5, refraction: 0.14 } });
    expect(s.changed.value).toBe(changed + 1);
    s.assign({ survey: null });
    expect(s.refraction).toBe(REFRACTION);
    expect('survey' in s.toJSON()).toBe(false);
    s.assign({ survey: { faceSlope: 0.004 } });
    const { survey: _dropped, ...whole } = s.toJSON();
    s.replace(whole);
    expect(s.survey.value).toBeNull();
  });
});
