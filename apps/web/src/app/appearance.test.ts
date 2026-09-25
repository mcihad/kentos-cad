import { describe, expect, it } from 'vitest';
import accents from '../styles/accents.css?raw';
import fonts from '../styles/fonts.css?raw';
import page from '../../index.html?raw';
import { DRAWING_FONT_IDS } from '../model/projectSettings';
import { ACCENTS, DRAWING_FONTS, UI_FONTS } from './appearance';
import { PREFERENCE_DEFAULTS } from './state';

/** The bundled files, as the app's build sees them. */
const files = new Set(Object.keys(import.meta.glob('../assets/fonts/*/*', { query: '?url', eager: true })).map((k) => k.replace('../', '')));
const TOKENS = ['--c-accent', '--c-accent-ink', '--c-accent-text', '--c-accent-soft', '--c-accent-line', '--c-tooltip-accent', '--canvas-accent'];

describe('appearance', () => {
  it('names every drawing typeface the contract knows, Barlow first', () => {
    expect(DRAWING_FONTS.map((f) => f.id)).toEqual([...DRAWING_FONT_IDS]);
    expect(DRAWING_FONTS[0].id).toBe('barlow');
    expect(PREFERENCE_DEFAULTS.defaultDrawingFont).toBe('barlow');
  });

  it('starts with Lacivert and Plus Jakarta Sans', () => {
    expect(PREFERENCE_DEFAULTS.accent).toBe('navy');
    expect(PREFERENCE_DEFAULTS.uiFont).toBe('jakarta');
    expect(ACCENTS[0].id).toBe('navy');
    expect(UI_FONTS[0].id).toBe('jakarta');
  });

  it('defines every accent but amber (tokens.css itself) for both themes, every token', () => {
    for (const a of ACCENTS.filter((x) => x.id !== 'amber')) {
      for (const selector of [`:root:not([data-theme='light'])[data-accent='${a.id}']`, `:root[data-theme='light'][data-accent='${a.id}']`]) {
        const at = accents.indexOf(`${selector} {`);
        expect(at, selector).toBeGreaterThanOrEqual(0);
        const block = accents.slice(at, accents.indexOf('}', at));
        for (const t of TOKENS) expect(block, `${selector} ${t}`).toContain(`${t}:`);
      }
    }
  });

  it('loads nothing from another host (no CDN, no Google Fonts)', () => {
    expect(page).not.toMatch(/https?:\/\//);
    expect(fonts).not.toMatch(/https?:\/\//);
    // The drawing's own text face comes with the app too.
    expect(fonts).toContain("font-family: 'Barlow';");
  });

  it('bundles every typeface but the system one: local files with Latin and Turkish letters, and their licence', () => {
    for (const f of [...UI_FONTS.filter((x) => x.id !== 'system'), ...DRAWING_FONTS]) {
      const family = f.family.split(',')[0].trim();
      const quoted = family.startsWith("'") ? family : `'${family}'`;
      const faces = fonts.split('@font-face').filter((b: string) => b.includes(`font-family: ${quoted};`));
      // One face per subset, or per subset and weight/style for the static fonts.
      expect(faces.length >= 2 && faces.length % 2 === 0, family).toBe(true);
      // Latin Extended carries ğ (U+011F), ş (U+015F) and İ (U+0130); ı (U+0131) is in the Latin subset.
      expect(faces.some((b: string) => b.includes('U+0100-02BA') || b.includes('U+0100-02AF')), family).toBe(true);
      for (const b of faces) {
        const url = /url\('\.\.\/(assets\/fonts\/[^']+)'\)/.exec(b)![1];
        expect(files.has(url), url).toBe(true);
        expect(files.has(url.replace(/[^/]+$/, 'OFL.txt')), url).toBe(true);
      }
    }
  });
});
