import { describe, expect, it } from 'vitest';
import { GROUND_HEIGHTS, ProjectSettings, REFRACTION, sanitizeLayerStates, sanitizeSurvey } from './projectSettings';

/**
 * The project's survey settings (docs/adr/0169 §3) as the settings model keeps them: the contract's
 * `SurveySettings::sanitized` (k within [−1, 1] and not the default, tolerances above zero, a ground height within
 * [−500, 9000] m and the reduction to the grid only with one, docs/adr/0171; nothing left: none), the
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

  it('keeps a ground height within its bounds, and the reduction to the grid only with one (docs/adr/0171)', () => {
    expect(GROUND_HEIGHTS).toEqual([-500, 9000]);
    expect(sanitizeSurvey({ groundHeight: 850 })).toEqual({ groundHeight: 850 });
    expect(sanitizeSurvey({ groundHeight: -500 })).toEqual({ groundHeight: -500 });
    expect(sanitizeSurvey({ groundHeight: 9000.5 })).toBeNull();
    expect(sanitizeSurvey({ groundHeight: Number.POSITIVE_INFINITY })).toBeNull();
    expect(sanitizeSurvey({ reduceToGrid: true })).toBeNull();
    expect(sanitizeSurvey({ reduceToGrid: false, groundHeight: 120 })).toEqual({ groundHeight: 120 });
    expect(sanitizeSurvey({ reduceToGrid: true, groundHeight: 120 })).toEqual({ groundHeight: 120, reduceToGrid: true });
    const s = new ProjectSettings();
    expect(s.groundHeight).toBeNull();
    expect(s.reducesToGrid).toBe(false);
    s.assign({ survey: { groundHeight: 850, reduceToGrid: true } });
    expect(s.groundHeight).toBe(850);
    expect(s.reducesToGrid).toBe(true);
    const changed = s.changed.value;
    s.assign({ survey: { groundHeight: 850 } });
    expect(s.reducesToGrid).toBe(false);
    expect(s.changed.value).toBe(changed + 1);
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

  it('keeps the layer states as a project keeps them, writes them only when there are, and drops them on a whole snapshot (docs/adr/0177 §4)', () => {
    // The contract's `layer_states_are_kept_as_a_project_keeps_them`: of the same id or name the first, none empty; of the same node the first.
    expect(
      sanitizeLayerStates([
        { id: 'a', name: 'Kadastro', nodes: [{ node: 'parsel', visible: true }, { node: 'parsel', visible: false }, { node: '', visible: true }] },
        { id: 'a', name: 'İkinci', nodes: [] },
        { id: 'b', name: ' Kadastro ', nodes: [] },
        { id: 'c', name: '  ', nodes: [] },
        { id: 'd', name: 'Baskı', nodes: [{ node: 'bina', visible: true, locked: true, style: { color: '#E5484D', lineType: 'dashed', lineWeight: 0.5 } }] },
      ]).map((s) => [s.name, s.nodes.length]),
    ).toEqual([
      ['Kadastro', 1],
      ['Baskı', 1],
    ]);
    const s = new ProjectSettings();
    expect('layerStates' in s.toJSON()).toBe(false);
    const changed = s.changed.value;
    s.assign({ layerStates: [{ id: 'a', name: 'Görünüm', nodes: [{ node: '0', visible: false }] }] });
    expect(s.changed.value).toBe(changed + 1);
    expect(s.toJSON().layerStates).toEqual([{ id: 'a', name: 'Görünüm', nodes: [{ node: '0', visible: false }] }]);
    const { layerStates: _dropped, ...whole } = s.toJSON();
    s.replace(whole);
    expect(s.layerStates.value).toEqual([]);
  });
});
