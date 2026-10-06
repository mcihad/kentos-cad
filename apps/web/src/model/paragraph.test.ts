import { describe, expect, it } from 'vitest';
import paragraph from '../../../../fixtures/text/v1/paragraph.json?raw';
import type { TextRun } from './entities';
import { letterAt, textLayout, textRunsRetext, textRunsToggle, unitAt, type ParagraphLayout, type ParagraphText, type RunToggle } from './paragraph';

/**
 * A multi-line text (docs/adr/0182 §2, §4) through WASM against the shared cases (fixtures/text/v1/paragraph.json,
 * written from the rules by scripts/fixtures/paragraph_cases.py); natively crates/shared/geometry-core/tests/all/text.rs.
 */

interface File {
  format: string;
  version: number;
  layout: { name: string; text: ParagraphText; want: ParagraphLayout }[];
  toggle: { name: string; runs: TextRun[]; len: number; start: number; end: number; toggle: RunToggle; want: TextRun[] }[];
  retext: { name: string; runs: TextRun[]; before: string; after: string; want: TextRun[] }[];
}

const file = JSON.parse(paragraph) as File;

describe('a multi-line text (fixtures/text/v1/paragraph.json)', () => {
  it('breaks and stands as the rules say, bit for bit', () => {
    expect([file.format, file.version]).toEqual(['kentos.text-cases', 1]);
    expect(file.layout.length).toBeGreaterThanOrEqual(15);
    for (const c of file.layout) expect(textLayout(c.text), c.name).toEqual(c.want);
  });

  it("follows the editor's toggles and edits with its runs", () => {
    for (const c of file.toggle) expect(textRunsToggle(c.runs, c.len, c.start, c.end, c.toggle), c.name).toEqual(c.want);
    for (const c of file.retext) expect(textRunsRetext(c.runs, c.before, c.after), c.name).toEqual(c.want);
  });

  it('counts letters as Unicode scalar values where the text field counts UTF-16 units', () => {
    const text = 'a🏠b\nç';
    expect([0, 1, 3, 4, 5, 6].map((u) => letterAt(text, u))).toEqual([0, 1, 2, 3, 4, 5]);
    expect([0, 1, 2, 3, 4, 5].map((l) => unitAt(text, l))).toEqual([0, 1, 3, 4, 5, 6]);
  });
});
