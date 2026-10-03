import { describe, expect, it } from 'vitest';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import { bookView } from './adapter';
import { templateBook, testEngine } from './engineTesting';
import { ENGINE_TEXTS, HAND, SELECT, SheetState } from './state';
import type { BookView } from './view';

/**
 * The sheet mode's state (docs/sheet/design.md §11): Model and the sheets as
 * tabs, the choice on the sheet in front, a book replaced under it, the
 * tool in hand, and why an action cannot run (no sheet in front, the engine
 * failed to load). The books are the engine's, made from its templates.
 */

const e = testEngine();
const ifraz = templateBook('sys:ifraz-paftasi', 'i');
const aplikasyon = templateBook('sys:aplikasyon-krokisi', 'a');

/** A view of a book holding the given sheets (from the two template books). */
function view(...which: ('ifraz' | 'aplikasyon')[]): BookView {
  const parts = which.map((w) => (w === 'ifraz' ? ifraz : aplikasyon).book.book);
  const book: SheetBook = { schema: e.info.bookSchema, sheets: parts.flatMap((b) => b.sheets), masters: parts.flatMap((b) => b.masters), assets: parts.flatMap((b) => b.assets), variables: [] };
  return bookView(book, { papers: e.paperSizes(), template: () => null, note: () => null, open: null });
}

describe('sheet state', () => {
  const ready = () => {
    const s = new SheetState();
    s.setBook(view('ifraz', 'aplikasyon'));
    return s;
  };

  it('starts on Model with an empty book, the engine not fetched yet and nothing to refuse for it', () => {
    const s = new SheetState();
    expect(s.open.value).toBeNull();
    expect(s.sheet).toBeNull();
    expect(s.book.value.sheets).toEqual([]);
    expect(s.engine.value).toEqual({ state: 'idle' });
    expect(s.whyNoEngine()).toBeNull();
    expect(s.whyNot(false)).toBe(ENGINE_TEXTS.noSheet);
    expect(s.tool.value).toBe(SELECT);
  });

  it('brings a sheet forward, ignores an unknown one and lets the choice go when the sheet changes', () => {
    const s = ready();
    s.openSheet(ifraz.sheet);
    expect(s.sheet?.paper.name).toBe('A3');
    s.select(['i-1']);
    s.openSheet('yok');
    expect(s.open.value).toBe(ifraz.sheet);
    expect([...s.selection.value]).toEqual(['i-1']);
    s.openSheet(aplikasyon.sheet);
    expect(s.selection.value.size).toBe(0);
    s.openSheet(null);
    expect(s.sheet).toBeNull();
  });

  it('chooses anew, adds and flips; never an unknown id', () => {
    const s = ready();
    s.openSheet(ifraz.sheet);
    s.select(['i-1', 'nothing']);
    expect([...s.selection.value]).toEqual(['i-1']);
    s.select(['i-3'], 'add');
    expect([...s.selection.value].sort()).toEqual(['i-1', 'i-3']);
    s.select(['i-1', 'i-4'], 'toggle');
    expect([...s.selection.value].sort()).toEqual(['i-3', 'i-4']);
    expect(s.chosen.map((i) => i.id)).toEqual(['i-3', 'i-4']);
  });

  it('chooses all that is neither hidden nor locked (the template’s frame is locked)', () => {
    const s = ready();
    s.openSheet(ifraz.sheet);
    s.selectAll();
    const chosen = s.chosen;
    expect(chosen.length).toBe(s.sheet!.items.filter((i) => !i.locked && !i.hidden).length);
    expect(chosen.length).toBeGreaterThan(5);
    expect(chosen.some((i) => i.locked || i.hidden)).toBe(false);
    expect(chosen.some((i) => i.id === 'i-0')).toBe(false);
  });

  it('keeps the sheet in front across a new book, and brings the left neighbour when it is gone', () => {
    const s = ready();
    s.openSheet(aplikasyon.sheet);
    s.select(['a-1']);
    s.setBook(view('ifraz', 'aplikasyon'));
    expect(s.open.value).toBe(aplikasyon.sheet);
    expect([...s.selection.value]).toEqual(['a-1']);
    // The sheet in front is gone: the tab left of it comes forward.
    s.setBook(view('ifraz'));
    expect(s.open.value).toBe(ifraz.sheet);
    // The first sheet gone: Model.
    s.setBook(view('aplikasyon'));
    expect(s.open.value).toBeNull();
  });

  it('says why per engine state: only a failed fetch refuses', () => {
    const s = ready();
    s.openSheet(ifraz.sheet);
    s.engine.set({ state: 'loading' });
    expect(s.whyNot(true)).toBeNull();
    s.engine.set({ state: 'failed', reason: ENGINE_TEXTS.failed('ağ hatası') });
    expect(s.whyNot(true)).toContain('ağ hatası');
    s.engine.set({ state: 'ready' });
    expect(s.whyNot(true)).toBeNull();
    expect(s.whyNot(false)).toBeNull();
  });

  it('holds the paper tool in hand: Seç, El, or a tool of the profile that adds an item', () => {
    const s = ready();
    s.tool.set(HAND);
    expect(s.tool.value.kind).toBe('hand');
    s.tool.set({ kind: 'add', tool: 'map', label: 'Görünüm penceresi', item: 'map' });
    expect(s.tool.value).toMatchObject({ kind: 'add', item: 'map' });
  });
});
