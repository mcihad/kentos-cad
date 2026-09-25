/**
 * Drawing quality (Uygulama ayarları → Çizim motoru → Çizim kalitesi): what
 * each level asks of the backend. Anti-aliasing is 4× multisampling, fixed
 * when a backend's context is made; the pixel ratio is the screen's (sharp
 * on Retina and 4K) or 1 (a quarter of the pixels there).
 */
export type RenderQuality = 'high' | 'balanced' | 'fast';

export const RENDER_QUALITY: Record<RenderQuality, { readonly antialias: boolean; readonly hiDpi: boolean }> = {
  high: { antialias: true, hiDpi: true },
  balanced: { antialias: false, hiDpi: true },
  fast: { antialias: false, hiDpi: false },
};
