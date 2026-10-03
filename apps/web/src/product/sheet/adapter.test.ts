import { describe, expect, it } from 'vitest';
import { bookView, detailOf, mm, um } from './adapter';
import { templateBook, testEngine } from './engineTesting';

/**
 * The engine's book as the interface shows it (adapter.ts): millimetres and
 * degrees, the paper's name from the engine's table, each item's line, the
 * engine's note for another mode's item on the sheet in front, and the
 * template a sheet came from.
 */

const e = testEngine();

describe('bookView', () => {
  it('turns micrometres and millidegrees into millimetres and degrees, and names the paper', () => {
    const { book, sheet } = templateBook();
    const view = bookView(book.book, { papers: e.paperSizes(), template: () => null, note: () => null, open: null });
    const s = view.sheets.find((x) => x.id === sheet)!;
    expect(s.paper).toMatchObject({ paper: 'a3', name: 'A3', orientation: 'landscape', widthMm: 420, heightMm: 297 });
    expect(s.paper.margins).toEqual({ left: 20, top: 10, right: 10, bottom: 10 });
    const map = s.items.find((i) => i.kind === 'map')!;
    expect(map.frame).toEqual({ left: 26, top: 16, width: 273, height: 265 });
    expect(map.anchors).toEqual({ h: 'leftRight', v: 'topBottom', box: 'margins' });
    expect(map.detail).toBe('1/1000');
    expect(map.source.id).toBe(map.id);
    expect(view.source).toBe(book.book);
  });

  it('asks the engine’s note only for the sheet in front, and says where a sheet came from', () => {
    const { book, sheet } = templateBook();
    const asked: string[] = [];
    const src = (open: string | null) => ({
      papers: e.paperSizes(),
      template: (id: string) => (id === 'sys:ifraz-paftasi' ? { name: 'İfraz / tevhit paftası', revision: 2 } : null),
      note: (item: { id: string }) => {
        asked.push(item.id);
        return e.itemNote('cad', item as never);
      },
      open,
    });
    expect(bookView(book.book, src(null)).sheets[0].items.every((i) => !i.note)).toBe(true);
    expect(asked).toEqual([]);
    const s = bookView(book.book, src(sheet)).sheets[0];
    expect(s.items.find((i) => i.kind === 'table')!.note).toContain('CBS projelerinin aracıdır');
    expect(s.items.filter((i) => i.note).length).toBe(1);
    // The template's revision 2 is newer than the sheet's 1.
    expect(s.template).toEqual({ name: 'İfraz / tevhit paftası', newer: true });
  });

  it('names the layout the paper chose (layout variants), and the next one when the paper turns', () => {
    const { book, sheet } = templateBook();
    const src = { papers: e.paperSizes(), template: () => null, note: () => null, open: sheet };
    const s = bookView(book.book, src).sheets[0];
    const own = book.book.sheets[0];
    expect(own.variants.length).toBeGreaterThan(1);
    expect(s.layout).toBe(own.variants.find((v) => v.id === own.activeVariant)!.name);
    // Portrait A3: the engine picks the portrait layout by itself (setPage with relayout).
    const portrait = { ...own.page, orientation: 'portrait' as const, size: { width: own.page.size.height, height: own.page.size.width } };
    const turned = e.applyOps(book, [{ op: 'setPage', owner: { kind: 'sheet', id: sheet }, page: portrait, relayout: true }]);
    const t = bookView(turned.book, src).sheets[0];
    const after = turned.book.sheets[0];
    expect(after.activeVariant).not.toBe(own.activeVariant);
    expect(t.layout).toBe(after.variants.find((v) => v.id === after.activeVariant)!.name);
    expect(t.paper.orientation).toBe('portrait');
  });

  it('writes one line per item that tells it from another of its kind', () => {
    const { book } = templateBook();
    const items = book.book.sheets[0].items;
    const lines = Object.fromEntries(items.map((i) => [i.kind.type, detailOf(i, items)]));
    expect(lines.table).toBe('katman: parsel');
    expect(lines.border).toBe('Çift çizgi');
    expect(lines.titleBlock).toMatch(/hücre$/);
  });

  it('rounds millimetres to whole micrometres, halves away from zero', () => {
    expect(um(12.3456)).toBe(12_346);
    expect(um(-0.0005)).toBe(-1);
    expect(mm(1500)).toBe(1.5);
  });
});
