import { describe, expect, it } from 'vitest';
import { modedPalette, parseHex, resolveColor, viewAlpha, viewHex, viewRgba, type CanvasPalette } from './color';

/**
 * Görünüm kipleri's colour rule (docs/adr/0195 §1), the same numbers as the desktop's
 * (crates/native/style/src/color.rs `the_modes_keep_alpha_and_name_their_colours`).
 */
describe('Görünüm kipleri’s colours', () => {
  const paper = { ink: '#000000' };
  it('keep alpha and name their colours', () => {
    const c = parseHex('#3E63DD33');
    expect(viewRgba(c, 'color', paper)).toEqual(c);
    expect(viewRgba(c, 'mono', paper)).toEqual([0, 0, 0, c[3]]);
    const y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    expect(viewRgba(c, 'gray', paper)).toEqual([y, y, y, c[3]]);
    // 0.2126·62 + 0.7152·99 + 0.0722·221 = 99.94 → 100 (0x64).
    expect(viewHex('#3E63DD', 'gray', paper)).toBe('#646464');
    expect(viewHex('#3E63DD33', 'mono', paper)).toBe('#00000033');
    expect(viewHex('kırmızı', 'gray', paper)).toBe('kırmızı');
    expect(viewAlpha(0.2, true)).toBe(1);
    expect(viewAlpha(0, true)).toBe(0);
    expect(viewAlpha(0.2, false)).toBe(0.2);
  });

  it('keeps the halo and the paper of a moded palette, and names every colour in the mode', () => {
    const pal = { label: '#C7D0DA', labelHalo: '#151B22', fg: '#E4EAF0', fgDim: '#A9B4C0', ink: '#FFFFFF', paper: '#151B22' } as CanvasPalette;
    expect(modedPalette(pal, 'color')).toBe(pal);
    const mono = modedPalette(pal, 'mono');
    expect([mono.label, mono.fg, mono.fgDim, mono.labelHalo, mono.paper]).toEqual(['#FFFFFF', '#FFFFFF', '#FFFFFF', '#151B22', '#151B22']);
    expect(resolveColor('#E5484D', mono)).toBe('#FFFFFF');
    expect(resolveColor('fg', mono)).toBe('#FFFFFF');
  });
});
