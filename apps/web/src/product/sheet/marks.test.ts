import { describe, expect, it } from 'vitest';
import type { RenderInputs } from '../../contracts/generated/sheet/RenderInputs';
import { templateBook, testEngine } from './engineTesting';
import { missingMark } from './marks';

/** The mark the web quotes for a value not given yet is the one the core writes on the paper. */
describe('the missing-value mark', () => {
  it('is the core’s', () => {
    const e = testEngine();
    const { book, sheet } = templateBook('sys:ifraz-paftasi');
    const inputs: RenderInputs = {
      mode: 'design',
      project: { name: 'Deneme', user: 'Ayşe', date: '2026-10-03', crsName: '' },
      capabilities: { georeferenced: false, attributeLayers: false },
      maps: [],
      legends: [],
      tables: [],
      coordinates: [],
    };
    // The ifraz template asks for its place: none given, the paper writes the mark.
    const texts = e.displayList(book, sheet, inputs).prims.flatMap((p) => (p.type === 'text' ? [p.text] : []));
    expect(texts.some((t) => t.includes(missingMark('il')))).toBe(true);
    expect(missingMark('ada')).toBe('‹ada?›');
  });
});
