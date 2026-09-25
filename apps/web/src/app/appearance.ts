/**
 * Look of the interface that each user picks for themselves (Uygulama
 * ayarları → Görünüm): the accent colour and the typeface. Both are CSS
 * only: an accent is a set of token values (styles/accents.css, amber is
 * tokens.css itself), a typeface one of the families bundled with the app
 * (styles/fonts.css, assets/fonts; never fetched from a CDN).
 */

import type { DrawingFont } from '../model/projectSettings';

export type AccentId = 'navy' | 'amber' | 'teal' | 'bordeaux';

export interface AccentSpec {
  readonly id: AccentId;
  readonly label: string;
  /** The swatch drawn in settings, per theme (the fill each theme uses). */
  readonly swatch: { readonly dark: string; readonly light: string };
}

export const ACCENTS: readonly AccentSpec[] = [
  { id: 'navy', label: 'Lacivert', swatch: { dark: '#4c7fe0', light: '#1f4a96' } },
  { id: 'amber', label: 'Amber', swatch: { dark: '#f2b632', light: '#f0b02a' } },
  { id: 'teal', label: 'Petrol yeşili', swatch: { dark: '#2aa99d', light: '#0e6f68' } },
  { id: 'bordeaux', label: 'Bordo', swatch: { dark: '#c24a63', light: '#8a1f37' } },
];

export type UiFontId = 'jakarta' | 'inter' | 'plex' | 'source' | 'noto' | 'roboto' | 'system';

export interface UiFontSpec {
  readonly id: UiFontId;
  readonly label: string;
  /** CSS font-family stack; the bundled family first, the system's after it while it loads. */
  readonly family: string;
  readonly note: string;
}

const FALLBACK = "'Segoe UI', system-ui, -apple-system, sans-serif";

export const UI_FONTS: readonly UiFontSpec[] = [
  { id: 'jakarta', label: 'Plus Jakarta Sans', family: `'Plus Jakarta Sans', ${FALLBACK}`, note: 'Geniş ve modern; varsayılan' },
  { id: 'inter', label: 'Inter', family: `'Inter', ${FALLBACK}`, note: 'Ekran için çizilmiş, sık ve net' },
  { id: 'plex', label: 'IBM Plex Sans', family: `'IBM Plex Sans', ${FALLBACK}`, note: 'Teknik ve kurumsal' },
  { id: 'source', label: 'Source Sans 3', family: `'Source Sans 3', ${FALLBACK}`, note: 'Dar; dar ekranda çok yazı sığar' },
  { id: 'noto', label: 'Noto Sans', family: `'Noto Sans', ${FALLBACK}`, note: 'Geniş dil desteği' },
  { id: 'roboto', label: 'Roboto', family: `'Roboto', ${FALLBACK}`, note: 'Tanıdık ve dengeli' },
  { id: 'system', label: 'Sistem yazı tipi', family: FALLBACK, note: 'İşletim sisteminin yazı tipi; indirme yok' },
];

/**
 * Typefaces of the drawing's own text (Proje ayarları → Çizim yazı tipi):
 * a project setting, since the letters are part of what everyone sees. The
 * classic technical faces sit beside Barlow for those who draw as on the
 * drafting table or in AutoCAD.
 */
export interface DrawingFontSpec {
  readonly id: DrawingFont;
  readonly label: string;
  readonly family: string;
  readonly note: string;
}

const DRAWING_FALLBACK = 'system-ui, sans-serif';

export const DRAWING_FONTS: readonly DrawingFontSpec[] = [
  { id: 'barlow', label: 'Barlow', family: `Barlow, ${DRAWING_FALLBACK}`, note: 'Dar ve okunaklı; varsayılan' },
  { id: 'arimo', label: 'Arimo', family: `Arimo, Arial, ${DRAWING_FALLBACK}`, note: 'Arial ölçülerinde; AutoCAD ve Netcad yazılarıyla aynı genişlik' },
  { id: 'overpass', label: 'Overpass', family: `Overpass, ${DRAWING_FALLBACK}`, note: 'Karayolu levhası; DIN ve ISO teknik yazısına yakın' },
  { id: 'quicksand', label: 'Quicksand', family: `Quicksand, ${DRAWING_FALLBACK}`, note: 'İnce, yuvarlak uçlu; plotter (SHX romans) yazısına benzer' },
  { id: 'architects-daughter', label: 'Architects Daughter', family: `'Architects Daughter', ${DRAWING_FALLBACK}`, note: 'Mimari el yazısı; çizim masası havası' },
  { id: 'courier-prime', label: 'Courier Prime', family: `'Courier Prime', 'Courier New', monospace`, note: 'Daktilo; eş aralıklı' },
  { id: 'plex-mono', label: 'IBM Plex Mono', family: `'IBM Plex Mono', ui-monospace, monospace`, note: 'Eş aralıklı ve teknik' },
];

export const drawingFontById = (id: DrawingFont): DrawingFontSpec => DRAWING_FONTS.find((f) => f.id === id) ?? DRAWING_FONTS[0];

/**
 * Sets the drawing's typeface (--font-drawing, read with the canvas palette)
 * and resolves once its faces are loaded: a canvas does not ask for a face
 * by itself, so the caller redraws after it.
 */
export async function applyDrawingFont(id: DrawingFont): Promise<void> {
  const font = drawingFontById(id);
  document.documentElement.style.setProperty('--font-drawing', font.family);
  if (!document.fonts) return;
  const family = font.family.split(',')[0];
  await Promise.all([document.fonts.load(`500 12px ${family}`, 'Ağİ'), document.fonts.load(`600 12px ${family}`, 'şğı'), document.fonts.load(`italic 400 12px ${family}`, 'Aa')]).catch(() => undefined);
}

export const accentById = (id: AccentId): AccentSpec => ACCENTS.find((a) => a.id === id) ?? ACCENTS[0];
export const uiFontById = (id: UiFontId): UiFontSpec => UI_FONTS.find((f) => f.id === id) ?? UI_FONTS[0];

/** Sets the accent tokens (the drawing's colours are read again by the caller: view.refreshPalette). */
export function applyAccent(id: AccentId): void {
  document.documentElement.dataset.accent = accentById(id).id;
}

/**
 * Sets the interface typeface. Resolves once its faces have loaded (or at
 * once for the system's), so measurements taken after it (the ribbon's
 * panel widths, the overlay's labels) use the new letters.
 */
export async function applyUiFont(id: UiFontId): Promise<void> {
  const font = uiFontById(id);
  document.documentElement.style.setProperty('--font-ui', font.family);
  if (font.id === 'system' || !document.fonts) return;
  const family = font.family.split(',')[0];
  // Turkish letters live in the Latin Extended subset: load it with the Latin one.
  await Promise.all([document.fonts.load(`400 13px ${family}`, 'Aa'), document.fonts.load(`600 13px ${family}`, 'ğşıİ')]).catch(() => undefined);
}
