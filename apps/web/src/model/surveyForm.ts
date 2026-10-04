import type { SurveySettings } from '../contracts/generated/SurveySettings';
import { fixed } from '../core/displayNumber';
import { number } from './definitionForm';
import { groundHeightHolds, REFRACTION, refractionHolds, sanitizeSurvey, toleranceHolds, type AngleUnit } from './projectSettings';

/**
 * Proje ayarları › Ölçme's form (docs/adr/0169 §3): the project's survey settings as eight texts (the refraction
 * coefficient k, the two faces' horizontal reading difference, the index error, the two faces' slope distance difference;
 * a traverse leg's two-way difference, a traverse's angular and linear misclosure; the mean ellipsoidal height of the
 * ground values, docs/adr/0171 §2) and the texts read back. The tolerances' angles are typed in cc in a gon project and in
 * arc seconds in a degree one and kept in radians; the lengths are typed in millimetres and kept in metres, the height in
 * metres. The desktop's twin is
 * `kentos_project::survey_form`; both pass fixtures/project/v1/survey-form.json (scripts/fixtures/survey_form_cases.py).
 */

/** What is said of a text that does not hold. */
export const SURVEY_TEXTS = {
  number: 'Sayı yazın.',
  refraction: '−1 ile 1 arasında bir sayı yazın; boş bırakılırsa 0.13.',
  tolerance: 'Sıfırdan büyük bir sayı yazın; denetlenmeyecekse boş bırakın.',
  height: '−500 ile 9000 m arasında bir yükseklik yazın; zemin değerleri gerekmiyorsa boş bırakın.',
} as const;

/** The form's fields, in order (the settings' keys). */
export const SURVEY_FIELDS = ['refraction', 'faceHz', 'index', 'faceSlope', 'twoWay', 'traverseAngle', 'traverseCoord', 'groundHeight'] as const;
export type SurveyField = (typeof SURVEY_FIELDS)[number];
export type SurveyTexts = [string, string, string, string, string, string, string, string];
/** The fields that are lengths (typed in millimetres). */
const LENGTHS: readonly SurveyField[] = ['faceSlope', 'twoWay', 'traverseCoord'];

/** The mark a tolerance's angle is typed with: cc (a ten-thousandth of a gon) in a gon project, ″ in a degree one. */
export const angleMark = (unit: AngleUnit): string => (unit === 'grad' ? 'cc' : '″');

/** A typed value as the settings keep it: cc × π / 2 000 000 and ″ × π / 648 000 rad, mm ÷ 1000 m. */
function stored(field: SurveyField, v: number, unit: AngleUnit): number {
  if (field === 'refraction' || field === 'groundHeight') return v;
  if (LENGTHS.includes(field)) return v / 1000;
  return unit === 'grad' ? (v * Math.PI) / 2_000_000 : (v * Math.PI) / 648_000;
}

/** A kept value in the unit it is typed in. */
function typed(field: SurveyField, v: number, unit: AngleUnit): number {
  if (field === 'refraction' || field === 'groundHeight') return v;
  if (LENGTHS.includes(field)) return v * 1000;
  return unit === 'grad' ? (v * 2_000_000) / Math.PI : (v * 648_000) / Math.PI;
}

/** The display rule's four decimals without trailing zeros or a bare point. */
function trimmed(v: number): string {
  let s = fixed(v, 4);
  if (s.includes('.')) s = s.replace(/0+$/, '').replace(/\.$/, '');
  return s === '' || s === '-0' ? '0' : s;
}

/** The texts the form shows for the settings; an absent value (k's default too) is an empty text. */
export function surveyTexts(survey: SurveySettings | null | undefined, unit: AngleUnit): SurveyTexts {
  return SURVEY_FIELDS.map((f) => {
    const v = survey?.[f];
    return v === undefined ? '' : trimmed(typed(f, v, unit));
  }) as SurveyTexts;
}

/** The texts read: the settings of the values that hold (null: the defaults), and what is said of each text that does not. */
export function readSurvey(texts: readonly string[], unit: AngleUnit): { survey: SurveySettings | null; problems: Partial<Record<SurveyField, string>> } {
  const survey: SurveySettings = {};
  const problems: Partial<Record<SurveyField, string>> = {};
  SURVEY_FIELDS.forEach((f, i) => {
    const t = texts[i] ?? '';
    if (!t.trim()) return;
    const v = number(t);
    if (v === null || !Number.isFinite(v)) {
      problems[f] = SURVEY_TEXTS.number;
      return;
    }
    if (f === 'refraction') {
      if (!refractionHolds(v)) problems[f] = SURVEY_TEXTS.refraction;
      else if (v !== REFRACTION) survey.refraction = v;
      return;
    }
    if (f === 'groundHeight') {
      if (groundHeightHolds(v)) survey.groundHeight = v;
      else problems[f] = SURVEY_TEXTS.height;
      return;
    }
    const kept = stored(f, v, unit);
    if (!(v > 0 && toleranceHolds(kept))) {
      problems[f] = SURVEY_TEXTS.tolerance;
      return;
    }
    survey[f] = kept;
  });
  return { survey: Object.keys(survey).length ? survey : null, problems };
}

/**
 * The settings read from the texts with the reduction to the grid the form keeps beside them (docs/adr/0171 §4): kept
 * only with a height, as a project keeps them (the desktop's `survey_form::with_reduction`).
 */
export function withReduction(survey: SurveySettings | null, reduce: boolean): SurveySettings | null {
  return sanitizeSurvey({ ...(survey ?? {}), ...(reduce ? { reduceToGrid: true } : {}) });
}
