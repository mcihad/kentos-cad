import type { EngineStatus } from '../../product/sheet/state';

/**
 * What the Model | Pafta tabs say at their right end (SheetTabs.ts): nothing
 * while the sheets can be made and edited, and otherwise why not, in a few
 * words (the tooltips and the commands say the whole reason). The engine is
 * fetched when a sheet is first opened; until then there is nothing to say.
 */
export function tabsNote(sheets: number, engine: EngineStatus): string {
  if (engine.state === 'loading') return 'Pafta motoru yükleniyor…';
  if (engine.state === 'failed') return sheets ? 'Paftalar açılamıyor: pafta motoru yüklenemedi.' : 'Pafta motoru yüklenemedi.';
  return '';
}
