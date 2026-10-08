import { describe, expect, it } from 'vitest';
import type { DimensionStyleDef, TextStyleDef } from './annotationStyles';
import { annotationHeightsProblem, followAnnotationScale, sanitizedAnnotationHeights, scaleChangeIsNone, type AnnotationHeights, type ScaleChange } from './annotationScale';
import type { Entity } from './entities';

/**
 * The web's annotation heights and following rule (docs/adr/0205 §1, §3) hold to the shared cases the independent
 * reference writes (scripts/fixtures/annotation_scale_cases.py, fixtures/text/v1/scale.json), exactly, as the
 * contract's do (crates/shared/contracts/tests/all/annotation_scale.rs).
 */

interface Case {
  name: string;
  op: 'problem' | 'sanitize' | 'isNone' | 'follow';
  heights?: AnnotationHeights;
  change?: ScaleChange;
  textStyles?: TextStyleDef[];
  dimensionStyles?: DimensionStyleDef[];
  entity?: Entity;
  expect: unknown;
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const doc = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/text/v1/scale.json', import.meta.url), 'utf8')) as { cases: Case[] };

function run(c: Case): unknown {
  switch (c.op) {
    case 'problem':
      return annotationHeightsProblem(c.heights!);
    case 'sanitize':
      return sanitizedAnnotationHeights(c.heights!) ?? null;
    case 'isNone':
      return scaleChangeIsNone(c.change!);
    case 'follow':
      return followAnnotationScale(c.entity!, c.change!, c.textStyles!, c.dimensionStyles!);
  }
}

describe('Yazı yükseklikleri: ortak durumlar', () => {
  it('has the reference’s cases', () => expect(doc.cases.length).toBeGreaterThanOrEqual(40));
  for (const c of doc.cases) it(c.name, () => expect(run(c)).toEqual(c.expect));
});
