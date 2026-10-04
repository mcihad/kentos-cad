import { describe, expect, it } from 'vitest';
import type { SurveySettings } from '../contracts/generated/SurveySettings';
import { readSurvey, SURVEY_FIELDS, SURVEY_TEXTS, surveyTexts, withReduction } from './surveyForm';
import type { AngleUnit } from './projectSettings';

/**
 * Proje ayarları › Ölçme's form (docs/adr/0169 §3) against the shared cases (fixtures/project/v1/survey-form.json,
 * written by scripts/fixtures/survey_form_cases.py from the rules alone): the settings as texts, the texts read back
 * with their problems. The desktop reads the same file (crates/native/project/src/survey_form.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  fields: string[];
  messages: typeof SURVEY_TEXTS;
  texts: { name: string; unit: AngleUnit; survey: SurveySettings | null; texts: string[] }[];
  reads: { name: string; unit: AngleUnit; texts: string[]; survey: SurveySettings | null; problems: Record<string, string> }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/project/v1/survey-form.json', import.meta.url), 'utf8')) as File;

describe('Proje ayarları › Ölçme (docs/adr/0169 §3)', () => {
  it('writes and reads the settings as the shared cases say', () => {
    expect(file.format).toBe('kentos.survey-form');
    expect(file.messages).toEqual(SURVEY_TEXTS);
    expect(file.fields).toEqual([...SURVEY_FIELDS]);
    expect(file.texts.length).toBeGreaterThanOrEqual(6);
    for (const c of file.texts) expect(surveyTexts(c.survey, c.unit), c.name).toEqual(c.texts);
    expect(file.reads.length).toBeGreaterThanOrEqual(9);
    for (const c of file.reads) expect(readSurvey(c.texts, c.unit), c.name).toEqual({ survey: c.survey, problems: c.problems });
  });

  it('keeps the reduction to the grid beside the texts, only with a height (docs/adr/0171 §4)', () => {
    expect(withReduction(null, true)).toBeNull();
    expect(withReduction({ groundHeight: 850 }, false)).toEqual({ groundHeight: 850 });
    expect(withReduction({ groundHeight: 850 }, true)).toEqual({ groundHeight: 850, reduceToGrid: true });
  });
});
