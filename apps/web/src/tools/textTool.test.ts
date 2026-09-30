import { describe, expect, it } from 'vitest';
import type { TextEntity } from '../model/entities';
import { TextTool } from './annotateTools';
import { at, toolHarness } from './toolHarness';

/**
 * Yazı's options (docs/adr/0145 §6): Hiza from its menu or its name typed together, Genişlik, Zemin and Artır, kept
 * for the next texts and written with them. The desktop's are crates/native/interaction/tests/text.rs. Expected
 * values are worked out by hand. The tool's options are its static fields, so this file runs them in order from the
 * defaults.
 */
describe('Yazı: Hiza, Genişlik, Zemin and Artır', () => {
  const h = toolHarness();
  const tool = h.use(new TextTool(h.ctx));
  tool.activate();
  const prompt = () => tool.prompt.value;
  const newest = () => [...h.doc.all()].at(-1) as TextEntity;
  /** Clicks at (x, y) and types `text` in the field that opens (Enter); the field's request. */
  const write = (x: number, y: number, text: string) => {
    tool.pointerDown(at(x, y));
    const req = h.state.textInputs.at(-1)!;
    req.commit(text);
    return req;
  };

  it('the prompt names every option with its value', () => {
    expect(prompt()).toBe(
      'Yazı: yazının başlangıcına tıklayın [Yükseklik (Y): 2.5 mm / Açı (A): 0° / Hiza (H): sol taban / Genişlik (G): 1 / Zemin (Z): kapalı / Artır (R): kapalı]',
    );
  });

  it('Hiza: its name typed together, with or without the Turkish marks; a word that names none is said', () => {
    expect(tool.input('H')).toBe(true);
    expect(prompt()).toBe('Yazı: hizayı seçin ya da adını bitişik yazın: sağüst, orta, soltaban … [Hiza (H): sol taban]');
    expect(tool.input('sag-ust')).toBe(true);
    expect(prompt()).toContain('Hiza (H): sağ üst /');
    tool.input('H');
    expect(tool.input('yukarı')).toBe(true);
    expect(h.said().at(-1)).toContain('“yukarı” bir hiza adı değil');
    expect(prompt()).toContain('hizayı seçin');
    expect(tool.input('ORTA')).toBe(true);
    expect(prompt()).toContain('Hiza (H): orta /');
  });

  it("Hiza's menu: the twelve points row by row, the chosen one checked; a choice is taken as typed", () => {
    const choices = tool.optionChoices('H')!;
    expect(choices.map((c) => c.label)).toEqual(['Sol üst', 'Orta üst', 'Sağ üst', 'Sol orta', 'Orta', 'Sağ orta', 'Sol alt', 'Orta alt', 'Sağ alt', 'Sol taban', 'Orta taban', 'Sağ taban']);
    expect(choices.filter((c) => c.checked).map((c) => c.label)).toEqual(['Orta']);
    expect(choices[0].icon).toBe('textAlignTopLeft');
    expect(choices[9].icon).toBe('textAlignBaselineLeft');
    expect(tool.optionChoices('Y')).toBeNull();
    expect(tool.chooseOption('H', 'sağ taban')).toBe(true);
    expect(prompt()).toContain('Hiza (H): sağ taban /');
    expect(tool.chooseOption('Y', '3')).toBe(false);
  });

  it('Genişlik: over 0 and at most 100, else said and asked again; Zemin and Artır turn on and off', () => {
    tool.input('G');
    expect(prompt()).toBe("Yazı: genişlik çarpanını yazın (1: harflerin kendi eni; 0'dan büyük, en çok 100)");
    expect(tool.input('0')).toBe(true);
    expect(h.said().at(-1)).toBe("Genişlik çarpanı 0'dan büyük, en çok 100 olmalı; 0 verildi. Harflerin kendi eni için 1 yazın.");
    expect(prompt()).toContain('genişlik çarpanını yazın');
    expect(tool.input('0.8')).toBe(true);
    tool.input('Z');
    tool.input('R');
    expect(prompt()).toContain('Hiza (H): sağ taban / Genişlik (G): 0.8 / Zemin (Z): açık / Artır (R): açık]');
  });

  it('the field stands as the text will, and the text is written with its alignment, width factor and mask', () => {
    const req = write(10, 20, 'Ada 101');
    expect([req.align, req.widthFactor, req.initial]).toEqual(['baselineRight', 0.8, undefined]);
    const t = newest();
    expect([t.kind, t.text, t.p, t.align, t.widthFactor, t.mask]).toEqual(['text', 'Ada 101', { x: 10, y: 20 }, 'baselineRight', 0.8, true]);
    expect(h.said().at(-1)).toBe('Yazı eklendi: “Ada 101”');
  });

  it("Artır: the next field opens with the last text's number one more; a text with no number comes back as it is", () => {
    expect(write(10, 16, 'Ada 102').initial).toBe('Ada 102');
    expect(newest().text).toBe('Ada 102');
    expect(write(10, 12, 'Yol').initial).toBe('Ada 103');
    expect(write(10, 8, 'Yol').initial).toBe('Yol');
    // Off: the field opens empty.
    tool.input('R');
    expect(write(10, 4, 'Son').initial).toBeUndefined();
  });

  it('back to the defaults: none of the extras is written', () => {
    tool.input('H');
    tool.input('soltaban');
    tool.input('G');
    tool.input('1');
    tool.input('Z');
    write(0, 0, 'Düz');
    const t = newest();
    expect([t.text, 'align' in t, 'widthFactor' in t, 'mask' in t]).toEqual(['Düz', false, false, false]);
  });
});
