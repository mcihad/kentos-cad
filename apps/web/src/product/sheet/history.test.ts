import { describe, expect, it } from 'vitest';
import { SheetEngineError } from './engine';
import { templateBook, testEngine } from './engineTesting';
import { HISTORY_LIMIT, SheetHistory } from './history';

/**
 * The sheet mode's undo (docs/sheet/design.md §4, §10): each edit one step
 * named by the engine (or by the caller), undo and redo through the
 * engine's own inverses, the book back to the same digest, a refused edit
 * leaving the book and both stacks as they were, a load clearing them.
 */

const e = testEngine();

describe('SheetHistory', () => {
  it('takes a step back and makes it again, the book the same each way', () => {
    const { book, sheet } = templateBook();
    const h = new SheetHistory(e, book);
    const start = e.bookDigest(book);
    expect(h.canUndo.value).toBe(false);
    const label = h.apply([{ op: 'moveItems', ids: ['t-1'], delta: [5000, -2000] }]);
    expect(label).toBe('Taşı: Harita');
    const moved = e.bookDigest(h.book.value);
    expect(moved).not.toBe(start);
    expect(h.canUndo.value).toBe(true);
    expect(h.undoLabel).toBe('Taşı: Harita');
    expect(h.undo()).toBe('Taşı: Harita');
    expect(e.bookDigest(h.book.value)).toBe(start);
    expect(h.canRedo.value).toBe(true);
    expect(h.redo()).toBe('Taşı: Harita');
    expect(e.bookDigest(h.book.value)).toBe(moved);
    expect(h.book.value.book.sheets.find((s) => s.id === sheet)!.items.find((i) => i.id === 't-1')!.frame.left).toBe(26_000 + 5000);
  });

  it('applies several operations as one named step', () => {
    const { book } = templateBook();
    const h = new SheetHistory(e, book);
    h.apply(
      [
        { op: 'moveItems', ids: ['t-3'], delta: [1000, 0] },
        { op: 'renameItem', id: 't-3', name: 'Başlık' },
      ],
      'Başlığı düzelt',
    );
    expect(h.undoLabel).toBe('Başlığı düzelt');
    h.undo();
    expect(e.bookDigest(h.book.value)).toBe(e.bookDigest(book));
    expect(h.canUndo.value).toBe(false);
  });

  it('leaves the book and the stacks as they were when the engine refuses', () => {
    const { book } = templateBook();
    const h = new SheetHistory(e, book);
    h.apply([{ op: 'moveItems', ids: ['t-1'], delta: [1000, 0] }]);
    const before = h.book.value;
    // The template's frame is locked: the engine says so with its code.
    let err: unknown;
    try {
      h.apply([{ op: 'moveItems', ids: ['t-0'], delta: [1000, 0] }]);
    } catch (x) {
      err = x;
    }
    expect(err).toBeInstanceOf(SheetEngineError);
    expect((err as SheetEngineError).code).toBe('item_locked');
    expect(h.book.value).toBe(before);
    expect(h.undoLabel).toBe('Taşı: Harita');
    expect(h.canRedo.value).toBe(false);
  });

  it('drops the redo of a step taken back once a new edit is made', () => {
    const { book } = templateBook();
    const h = new SheetHistory(e, book);
    h.apply([{ op: 'moveItems', ids: ['t-1'], delta: [1000, 0] }]);
    h.undo();
    h.apply([{ op: 'hide', ids: ['t-1'], value: true }]);
    expect(h.canRedo.value).toBe(false);
    expect(h.redo()).toBeNull();
  });

  it('forgets the steps of the last book when another is read', () => {
    const a = templateBook();
    const b = templateBook('sys:genel-a4-dikey', 'g');
    const h = new SheetHistory(e, a.book);
    h.apply([{ op: 'moveItems', ids: ['t-1'], delta: [1000, 0] }]);
    h.reset(b.book);
    expect(h.canUndo.value).toBe(false);
    expect(h.undo()).toBeNull();
    expect(h.book.value).toBe(b.book);
  });

  it('keeps at most HISTORY_LIMIT steps', () => {
    const { book } = templateBook();
    const h = new SheetHistory(e, book);
    for (let i = 0; i < HISTORY_LIMIT + 5; i++) h.apply([{ op: 'moveItems', ids: ['t-1'], delta: [i % 2 ? 100 : -100, 0] }]);
    let n = 0;
    while (h.undo()) n++;
    expect(n).toBe(HISTORY_LIMIT);
  });
});
