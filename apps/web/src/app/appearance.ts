/**
 * Look of the interface that each user picks for themselves (Uygulama
 * ayarları → Görünüm): the accent colour and the typeface. Both are CSS
 * only: an accent is a set of token values (styles/accents.css, amber is
 * tokens.css itself), a typeface one of the families bundled with the app
 * (styles/fonts.css, assets/fonts; never fetched from a CDN).
 */

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
