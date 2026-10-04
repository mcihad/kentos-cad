import { describe, expect, it } from 'vitest';
import { textBox, type TextEntity } from '../model/entities';
import { ReadableTool } from './annotateTools';
import { pt, toolHarness } from './toolHarness';

/**
 * Okunur yap (docs/adr/0145 §6) over a document and a log: the upside-down texts of the selection turn half round,
 * their boxes where they were; a text that reads, one on a locked layer and what is not a text are left; one step. The
 * desktop's are crates/native/interaction/tests/all/text.rs. Centred alignments turn by amounts that need no width: worked
 * out by hand.
 */
function scene() {
  const h = toolHarness();
  const text = (layerId: string, x: number, y: number, words: string, rotation: number, align?: TextEntity['align']) =>
    h.add({ kind: 'text', layerId, p: pt(x, y), text: words, height: 2, rotation, ...(align && { align }) }) as TextEntity;
  const ada = text('cizim', -10, 5, 'Ada 101', 180, 'middleCenter');
  const yol = text('cizim', 10, 5, 'Yol 12', 200, 'baselineCenter');
  const park = text('cizim', 0, -5, 'Park', 30);
  const locked = text('kilitli', 0, -12, 'Kilitli', 180, 'middleCenter');
  const get = (t: TextEntity) => h.doc.get(t.id) as TextEntity;
  /** The tool started over these objects selected; it acts at once. */
  const run = (...ids: number[]) => {
    h.ctx.selection.set(ids);
    h.use(new ReadableTool(h.ctx)).activate();
  };
  /** A box's corners as a set, to a micrometre: where the text is drawn, whichever corner comes first. */
  const box = (t: TextEntity) => textBox({ ...t, font: h.ctx.doc.settings.drawingFont.value }).map((q) => `${q.x.toFixed(6)},${q.y.toFixed(6)}`).sort();
  return { h, ada, yol, park, locked, get, run, box };
}

const near = (a: { x: number; y: number }, x: number, y: number) => expect(Math.hypot(a.x - x, a.y - y)).toBeLessThan(1e-9);

describe('Okunur yap', () => {
  it('turns what reads upside down about its box; what reads and what is locked stay; one step', () => {
    const { h, ada, yol, park, locked, get, run, box } = scene();
    const boxes = [box(ada), box(yol)];
    run(ada.id, yol.id, park.id, locked.id);
    // Ada: middle centre at 180°, its point 0.08 of its height up; Yol: baseline centre at 200°, 0.92 heights along its old up.
    near(get(ada).p, -10, 5.16);
    near(get(yol).p, 10 + 1.84 * Math.sin((20 * Math.PI) / 180), 5 - 1.84 * Math.cos((20 * Math.PI) / 180));
    expect([get(ada).rotation, get(yol).rotation, get(park).rotation, get(locked).rotation]).toEqual([0, 20, 30, 180]);
    expect([get(ada).align, get(yol).align]).toEqual(['middleCenter', 'baselineCenter']);
    expect([box(get(ada)), box(get(yol))]).toEqual(boxes);
    expect(h.said().slice(-2)).toEqual(['1 nesne kilitli katmanda olduğu için atlandı.', '2 yazı okunur yapıldı.']);
    expect(h.doc.undo()).toBe('Okunur yap');
    expect([get(ada).rotation, get(yol).rotation]).toEqual([180, 200]);
  });

  it('says so when no text reads upside down, and writes nothing', () => {
    const { h, park, run } = scene();
    const revision = h.doc.revision;
    run(park.id);
    expect(h.said().at(-1)).toBe('Ters okunan yazı yok: 1 yazının hepsi okunuyor.');
    expect(h.doc.revision).toBe(revision);
  });

  it('says so when the selection holds no text', () => {
    const { h, run } = scene();
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 0) });
    run(line.id);
    expect(h.said().at(-1)).toBe('Seçimde yazı yok. Okunur yap yazı nesnelerini çevirir; yazıları seçip yeniden deneyin.');
  });
});
