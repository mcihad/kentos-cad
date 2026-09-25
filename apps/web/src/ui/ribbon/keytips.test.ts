import { describe, expect, it } from 'vitest';
import { assignKeyTips, lettersOf } from './keytips';

describe('ribbon key tips', () => {
  it('folds Turkish letters as the command line does', () => {
    expect(lettersOf('Çizim')).toBe('CIZIM');
    expect(lettersOf('İşlemler')).toBe('ISLEMLER');
    expect(lettersOf('Görünüm: ölçü')).toBe('GORUNUMOLCU');
  });

  it('gives one letter where the first letter is free, two otherwise, never a prefix of another', () => {
    const labels = ['Giriş', 'Çizim', 'Değiştir', 'Alan', 'Harita', 'Açıklama', 'Görünüm', 'İşlemler', 'Dosya'];
    const tips = assignKeyTips(labels);
    expect(tips[1]).toBe('C');
    expect(tips[4]).toBe('H');
    expect(tips[7]).toBe('I');
    expect(new Set(tips).size).toBe(labels.length);
    for (const a of tips) {
      expect(a).toMatch(/^[A-Z0-9]{1,2}$/);
      for (const b of tips) if (a !== b) expect(b.startsWith(a)).toBe(false);
    }
    // Shared first letters get two: Giriş and Görünüm, Değiştir and Dosya, Alan and Açıklama.
    expect(tips[0]).toHaveLength(2);
    expect(tips[0][0]).toBe('G');
  });

  it('leaves the reserved tips (quick access digits) alone and survives many look-alike labels', () => {
    const labels = Array.from({ length: 60 }, (_, i) => `Komut ${i}`);
    const tips = assignKeyTips(labels, new Set(['1', '2']));
    expect(new Set(tips).size).toBe(60);
    expect(tips.every((t) => t.length === 2 && t !== '1' && t !== '2')).toBe(true);
  });
});
