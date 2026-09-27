import { describe, expect, it } from 'vitest';
import type { MembershipView } from '../../contracts/generated/MembershipView';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import { ApiFailure } from './api';
import { parseTags } from './catalog';
import {
  DESCRIPTION_MAX,
  FORM_TEXTS,
  NAME_MAX,
  convertForm,
  convertReason,
  convertedLine,
  copyName,
  countText,
  creatablePlaces,
  duplicateSavable,
  failureReason,
  metadataPatch,
  metadataSavable,
  renameSavable,
} from './formsPlan';

/**
 * The catalog's project forms (fixtures/cloud/v1/forms.json, format in
 * fixtures/cloud/README.md): their words, the tags a field reads, what the
 * metadata form sends, when each button is on, the workspaces offered, the
 * conversion's words, the counts, lines and reasons. The file's answers are
 * worked out apart from this code (scripts/fixtures/forms_cases.py); the
 * desktop's forms check themselves against the same file.
 */

const files = import.meta.glob<string>('../../../../../fixtures/cloud/v1/forms.json', { query: '?raw', import: 'default', eager: true });
type Meta = { name: string; projectType: ProjectType; description: string; tags: string[] };
type Failure = { kind: 'api' | 'error' | 'other'; status?: number; error?: string; message?: string; fallback?: string; path?: string };
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  texts: unknown;
  limits: { name: number; description: number };
  tags: { text: string; tags: string[] }[];
  copyNames: { name: string; copy: string }[];
  metadata: { shown: Meta; cases: { title: string; now: Meta; patch: unknown; savable: boolean }[] };
  rename: { typed: string; current: string; savable: boolean }[];
  duplicate: { places: number; name: string; savable: boolean }[];
  places: { memberships: MembershipView[]; cases: { first: string; places: unknown }[] };
  convert: { title: string; project: { name: string; storage: ProjectStorage }; openDirty: boolean; expect: unknown }[];
  counts: { objects: string; text: string }[];
  convertedLines: { name: string; objects: string; to: ProjectStorage; text: string }[];
  failures: { title: string; error: Failure; text: string }[];
  convertFailures: { title: string; error: Failure; text: string }[];
};

/** FORM_TEXTS as the file writes them: a text made from values as `{ sample: [...], text }`. */
function fileTexts(o: object, samples: Record<string, unknown[]>, path = ''): unknown {
  return Object.fromEntries(
    Object.entries(o).map(([k, v]) => {
      const at = path ? `${path}.${k}` : k;
      if (typeof v === 'function') {
        const sample = samples[at];
        if (!sample) throw new Error(`Metin örneği yok: ${at}`);
        return [k, { sample, text: (v as (...a: unknown[]) => string)(...sample) }];
      }
      return [k, v && typeof v === 'object' && !Array.isArray(v) ? fileTexts(v as object, samples, at) : v];
    }),
  );
}

/** The samples the file gives, by their place in the texts. */
function samplesOf(o: unknown, path = '', out: Record<string, unknown[]> = {}): Record<string, unknown[]> {
  if (o && typeof o === 'object' && !Array.isArray(o)) {
    const r = o as Record<string, unknown>;
    if (Array.isArray(r.sample) && typeof r.text === 'string') out[path] = r.sample;
    else for (const [k, v] of Object.entries(r)) samplesOf(v, path ? `${path}.${k}` : k, out);
  }
  return out;
}

function failure(e: Failure): unknown {
  if (e.kind === 'api') return new ApiFailure(e.status ?? 0, { error: e.error, message: e.message, path: e.path }, e.fallback ?? e.message ?? '');
  if (e.kind === 'error') return Object.assign(new Error(e.message), e.path ? { path: e.path } : {});
  return 'bozuk';
}

const plain = (v: unknown): unknown => JSON.parse(JSON.stringify(v));

describe('project forms (fixtures/cloud/v1/forms.json)', () => {
  it('is a v1 forms file with the forms’ words and limits', () => {
    expect([F.format, F.version]).toEqual(['kentos.forms', 1]);
    expect(fileTexts(FORM_TEXTS, samplesOf(F.texts))).toEqual(F.texts);
    expect({ name: NAME_MAX, description: DESCRIPTION_MAX }).toEqual(F.limits);
  });

  it('reads tags, and offers a copy its name', () => {
    for (const c of F.tags) expect(parseTags(c.text), JSON.stringify(c.text)).toEqual(c.tags);
    for (const c of F.copyNames) expect(copyName(c.name)).toBe(c.copy);
  });

  it('sends only what the metadata form changed, and saves only then', () => {
    for (const c of F.metadata.cases) {
      const patch = metadataPatch(F.metadata.shown, c.now);
      expect(plain(patch), c.title).toEqual(c.patch);
      expect(metadataSavable(c.now.name, patch), c.title).toBe(c.savable);
    }
  });

  it('turns Yeniden adlandır and Kopyasını oluştur on only when they can go', () => {
    for (const c of F.rename) expect(renameSavable(c.typed, c.current), JSON.stringify(c)).toBe(c.savable);
    for (const c of F.duplicate) expect(duplicateSavable(c.places, c.name), JSON.stringify(c)).toBe(c.savable);
  });

  it('offers the workspaces the account may open projects in, the source’s first', () => {
    for (const c of F.places.cases) expect(creatablePlaces(F.places.memberships, c.first), c.first).toEqual(c.places);
  });

  it('says what a conversion to the other storage mode does', () => {
    for (const c of F.convert) expect(convertForm(c.project, c.openDirty), c.title).toEqual(c.expect);
  });

  it('writes counts, lines and reasons', () => {
    for (const c of F.counts) expect(countText(c.objects), JSON.stringify(c.objects)).toBe(c.text);
    for (const c of F.convertedLines) expect(convertedLine(c.name, c.objects, c.to)).toBe(c.text);
    for (const c of F.failures) expect(failureReason(failure(c.error)), c.title).toBe(c.text);
    for (const c of F.convertFailures) expect(convertReason(failure(c.error)), c.title).toBe(c.text);
  });
});
