import { describe, expect, it } from 'vitest';
import type { TextEntity } from '../../model/entities';
import { pt, toolHarness } from '../../tools/toolHarness';
import { findMatches, replaceMatches, type FindQuery } from './findReplace';

/**
 * Bul ve değiştir's matches and write (docs/adr/0145 §6) over a document, without a DOM: the core's rule decides
 * (fixtures/text/v1/pattern.json holds it); here the scope, the locked and emptied texts, the trimming and the one
 * step. The desktop's are crates/native/interaction/tests/all/find_replace.rs.
 */
function scene() {
  const h = toolHarness();
  const text = (layerId: string, words: string, y: number) => h.add({ kind: 'text', layerId, p: pt(0, y), text: words, height: 2, rotation: 0 }) as TextEntity;
  const t = { a: text('cizim', 'Ada 101', 0), b: text('cizim', 'Ada 102', 4), yol: text('cizim', 'Yol 12', 8), locked: text('kilitli', 'Ada 9', 12), bare: text('cizim', 'Ada', 16) };
  const q = (over: Partial<FindQuery>): FindQuery => ({ find: '', replace: '', wildcard: false, caseless: true, wholeWord: false, selectionOnly: false, ...over });
  return { h, t, q };
}

describe('Bul ve değiştir', () => {
  it('finds every text the rule changes, in the drawing, with its layer; a locked one listed and blocked', () => {
    const { h, t, q } = scene();
    const found = findMatches(h.ctx, q({ find: 'Ada', replace: 'Parsel' }));
    expect(found.map((m) => [m.id, m.layer, m.old, m.new, m.blocked])).toEqual([
      [t.a.id, 'Çizim', 'Ada 101', 'Parsel 101', null],
      [t.b.id, 'Çizim', 'Ada 102', 'Parsel 102', null],
      [t.locked.id, 'Kilitli', 'Ada 9', 'Parsel 9', 'locked'],
      [t.bare.id, 'Çizim', 'Ada', 'Parsel', null],
    ]);
  });

  it('wildcards match the whole text and carry what each * took', () => {
    const { h, t, q } = scene();
    const found = findMatches(h.ctx, q({ find: 'Ada *', replace: 'Parsel *', wildcard: true }));
    expect(found.map((m) => [m.id, m.new])).toEqual([
      [t.a.id, 'Parsel 101'],
      [t.b.id, 'Parsel 102'],
      [t.locked.id, 'Parsel 9'],
    ]);
  });

  it('what becomes empty is blocked; what is left is trimmed', () => {
    const { h, t, q } = scene();
    const found = findMatches(h.ctx, q({ find: 'Ada', replace: '' }));
    expect(found.find((m) => m.id === t.a.id)?.new).toBe('101');
    expect(found.find((m) => m.id === t.bare.id)).toMatchObject({ new: '', blocked: 'empty' });
  });

  it('case and the selection narrow it', () => {
    const { h, t, q } = scene();
    expect(findMatches(h.ctx, q({ find: 'ada', replace: 'X', caseless: false }))).toEqual([]);
    expect(findMatches(h.ctx, q({ find: 'ada', replace: 'X' })).length).toBe(4);
    h.ctx.selection.set([t.a.id, t.yol.id]);
    expect(findMatches(h.ctx, q({ find: 'Ada', replace: 'X', selectionOnly: true })).map((m) => m.id)).toEqual([t.a.id]);
  });

  it('writes what may be written in one step “Bul ve değiştir”; the rest stays', () => {
    const { h, t, q } = scene();
    const found = findMatches(h.ctx, q({ find: 'Ada', replace: 'Parsel' }));
    expect(replaceMatches(h.ctx, found)).toBe(3);
    const get = (e: TextEntity) => (h.doc.get(e.id) as TextEntity).text;
    expect([get(t.a), get(t.b), get(t.locked), get(t.bare), get(t.yol)]).toEqual(['Parsel 101', 'Parsel 102', 'Ada 9', 'Parsel', 'Yol 12']);
    expect(h.doc.undo()).toBe('Bul ve değiştir');
    expect(get(t.a)).toBe('Ada 101');
    expect(replaceMatches(h.ctx, [])).toBe(0);
  });
});
