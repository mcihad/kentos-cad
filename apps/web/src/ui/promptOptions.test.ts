import { describe, expect, it } from 'vitest';
import { optionForKey, parsePrompt } from './promptOptions';

describe('parsePrompt', () => {
  it('splits tool, step and options', () => {
    const p = parsePrompt('Çoklu çizgi: sonraki noktayı belirtin [Yay (Y) / Geri (G) / Bitir (Enter)]');
    expect(p.tool).toBe('Çoklu çizgi');
    expect(p.step).toBe('sonraki noktayı belirtin');
    expect(p.options.map((o) => o.key)).toEqual(['Y', 'G', 'Enter']);
    expect(p.options[0].label).toBe('Yay');
  });
  it('keeps option values and plain notes apart', () => {
    const p = parsePrompt('Ötele: ötelenecek nesneyi seçin [mesafe 1.000 m; Noktadan geç (N): kapalı]');
    expect(p.notes).toEqual(['mesafe 1.000 m']);
    expect(p.options).toEqual([{ label: 'Noktadan geç', key: 'N', value: 'kapalı' }]);
  });
  it('reads multi-letter keys and values with spaces', () => {
    const p = parsePrompt('Tarama: kapalı alanın içine tıklayın [Desen (D): Çizgili 45°]');
    expect(p.options[0]).toEqual({ label: 'Desen', key: 'D', value: 'Çizgili 45°' });
    const c = parsePrompt('Daire: merkez noktasını belirtin [2 nokta (2N) / 3 nokta (3N) / Teğet-teğet-yarıçap (TTY)]');
    expect(c.options.map((o) => o.key)).toEqual(['2N', '3N', 'TTY']);
  });
  it('leaves a prompt without tool name or options alone', () => {
    expect(parsePrompt('Komut')).toEqual({ tool: '', step: 'Komut', options: [], notes: [] });
  });
  it('keeps parentheses in the step text', () => {
    const p = parsePrompt('Tutamaç: yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)');
    expect(p.tool).toBe('Tutamaç');
    expect(p.step).toBe('yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)');
  });
});

describe('option letters', () => {
  it('matches the exact key first, then the letter without Turkish marks', () => {
    const { options } = parsePrompt('Ölçü: ilk nokta [Çap (Ç) / Daire (C2) / Sol (S)]');
    expect(optionForKey(options, 'c')?.label).toBe('Çap');
    expect(optionForKey(options, 'ç')?.label).toBe('Çap');
    expect(optionForKey(options, 's')?.label).toBe('Sol');
    expect(optionForKey(options, 'x')).toBeUndefined();
  });
});
