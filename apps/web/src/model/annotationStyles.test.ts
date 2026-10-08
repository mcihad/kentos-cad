import { describe, expect, it } from 'vitest';
import {
  applyDimensionStyle,
  applyTextStyle,
  dimensionStylesProblem,
  faceProblem,
  followDimensionStyle,
  followTextStyle,
  lookOfDimension,
  lookProblem,
  sanitizedDimensionStyles,
  sanitizedTextStyles,
  textStylesProblem,
  type DimensionLook,
  type DimensionStyleDef,
  type TextFace,
  type TextLook,
  type TextStyleDef,
} from './annotationStyles';
import { dimensionValue, type Measured } from './dimensionValue';
import type { DrawingUnit } from './projectSettings';

/**
 * The style rules (docs/adr/0183) against the shared cases (fixtures/text/v1/styles.json, written by
 * scripts/fixtures/annotation_style_cases.py from the ADR, not KentOS code). The contract reads the same file
 * (crates/shared/contracts/tests/all/annotation.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  name: string;
  op: string;
  plotScale?: number;
  /** Standart's height when a case names the project's (docs/adr/0205 §1). */
  standardMm?: number;
  styles?: (TextStyleDef | DimensionStyleDef)[];
  style?: string | null;
  face?: TextFace;
  look?: DimensionLook;
  text?: TextLook;
  dimension?: DimensionLook & { height: number };
  old?: TextStyleDef | DimensionStyleDef;
  new?: TextStyleDef | DimensionStyleDef;
  measured?: Measured;
  project?: { unit: DrawingUnit; lengthDecimals: number };
  expect: unknown;
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/text/v1/styles.json', import.meta.url), 'utf8')) as { format: string; cases: Case[] };

function run(c: Case): unknown {
  const scale = c.plotScale ?? 1000;
  switch (c.op) {
    case 'textStylesProblem':
      return textStylesProblem(c.styles as TextStyleDef[]);
    case 'dimensionStylesProblem':
      return dimensionStylesProblem(c.styles as DimensionStyleDef[]);
    case 'faceProblem':
      return faceProblem(c.face!);
    case 'lookProblem':
      return lookProblem(c.look!);
    case 'applyText': {
      const style = (c.styles as TextStyleDef[]).find((s) => s.id === c.style) ?? null;
      return applyTextStyle(style, c.text!, scale);
    }
    case 'followText':
      return followTextStyle(c.old as TextStyleDef, c.new as TextStyleDef, c.text!, scale);
    case 'applyDimension': {
      const style = (c.styles as DimensionStyleDef[]).find((s) => s.id === c.style) ?? null;
      // Standart's height is the project's (docs/adr/0205 §1): 2.5 mm unless a case names one.
      const { look, height } = applyDimensionStyle(style, scale, c.standardMm ?? 2.5);
      return { ...look, height };
    }
    case 'followDimension': {
      const d = c.dimension!;
      const { look, height } = followDimensionStyle(c.old as DimensionStyleDef, c.new as DimensionStyleDef, lookOfDimension(d), d.height, scale);
      return { ...look, height };
    }
    // A dimension's value as written (docs/adr/0183 §3); angles are the project's, none among the cases.
    case 'dimensionValue':
      return dimensionValue(c.measured!, c.look!, { ...c.project!, angle: () => 'açı' });
  }
  throw new Error(`bilinmeyen işlem ${c.op}`);
}

describe('Yazı ve ölçü stilleri (docs/adr/0183)', () => {
  it('holds to the shared cases', () => {
    expect(file.format).toBe('kentos.annotation-style-cases');
    expect(file.cases.length).toBeGreaterThanOrEqual(60);
    for (const c of file.cases) expect(run(c), c.name).toEqual(c.expect);
  });

  it('keeps of a project’s styles those the rules let it have, the first of a name', () => {
    const ok: TextStyleDef = { id: 'a', name: 'Ada no', font: 'arimo' };
    const same: TextStyleDef = { id: 'b', name: 'ADA NO', font: 'barlow' };
    const bad: TextStyleDef = { id: 'c', name: 'Yatık', font: 'barlow', oblique: 90 };
    expect(sanitizedTextStyles([ok, same, bad]).map((s) => s.id)).toEqual(['a']);
    const dim: DimensionStyleDef = { id: 'd', name: 'Mimari', height: 2.5 };
    const zero: DimensionStyleDef = { id: 'e', name: 'Sıfır', height: 0 };
    expect(sanitizedDimensionStyles([dim, zero, { ...dim, id: 'f' }]).map((s) => s.id)).toEqual(['d']);
  });
});
