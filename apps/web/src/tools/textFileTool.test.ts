import { describe, expect, it } from 'vitest';
import type { TextEntity } from '../model/entities';
import { PlaceTextFileTool } from './textFileTool';
import { at, toolHarness } from './toolHarness';

/**
 * Metin dosyası yerleştir (docs/adr/0145 §6): the file asked for as the tool starts, its lines written one under the
 * other 1.5 heights apart with Yazı's options, one step; a cancelled picker or a refused file leaving. The desktop's
 * are crates/native/interaction/tests/text_file.rs. Places worked out by hand.
 */
const flush = () => new Promise((r) => setTimeout(r, 0));

function started(picked: { name: string; bytes: Uint8Array } | null) {
  const h = toolHarness();
  Object.assign(h.ctx, { files: { pickForImport: async () => picked } });
  const tool = h.use(new PlaceTextFileTool(h.ctx));
  tool.activate();
  return { h, tool };
}

describe('Metin dosyası yerleştir', () => {
  it('writes the lines one under the other in one step, an empty line keeping its place', async () => {
    const { h, tool } = started({ name: 'satirlar.txt', bytes: new TextEncoder().encode('Ada 101\n\nAda 103\n') });
    expect(tool.prompt.value).toBe('Metin dosyası yerleştir: metin dosyasını seçin');
    await flush();
    expect(tool.prompt.value).toBe("Metin dosyası yerleştir: ilk satırın başlangıcına tıklayın [“satirlar.txt”: 2 yazı; Yazı'nın seçenekleriyle: 2.5 mm, 0°, sol taban]");
    tool.pointerDown(at(2, 4));
    // 2.5 mm at 1:1000 is 2.5 m; the third line is 2 × 1.5 × 2.5 = 7.5 m down.
    const texts = [...h.doc.all()].filter((e): e is TextEntity => e.kind === 'text').map((t) => [t.text, t.p.x, t.p.y]);
    expect(texts).toEqual([
      ['Ada 101', 2, 4],
      ['Ada 103', 2, -3.5],
    ]);
    expect(h.said().at(-1)).toBe('“satirlar.txt”: 2 satır yazı olarak yerleştirildi.');
    expect(h.state.exited).toBe(1);
    expect(h.doc.undo()).toBe('Metin dosyası yerleştir');
  });

  it('a cancelled picker leaves; a refused file is said, and the tool leaves', async () => {
    const cancelled = started(null);
    await flush();
    expect(cancelled.h.state.exited).toBe(1);
    const refused = started({ name: 'a.txt', bytes: new Uint8Array([0xfe, 0x41]) });
    await flush();
    expect(refused.h.state.exited).toBe(1);
    expect(refused.h.said().at(-1)).toBe('“a.txt” UTF-8 değil. Dosyayı UTF-8 olarak kaydedip yeniden deneyin.');
  });
});
