import type { CadDocument } from '../model/document';
import type { LibraryCategory, LibraryItem } from '../model/style';

/**
 * What the app opens with and cannot draw without: the system symbol
 * library (every MPYY page) and the demo drawing, whose showcase shows the
 * whole library below the sheet. They are one chunk of their own (CLAUDE.md
 * §20) that main.ts starts fetching beside the geometry core, so the entry
 * script stays small and both downloads run at once.
 */
export interface StartContent {
  readonly system: { readonly items: readonly LibraryItem[]; readonly categories: readonly LibraryCategory[] };
  sampleProject(srid: number): CadDocument;
}

export async function loadStartContent(): Promise<StartContent> {
  const [{ SYSTEM_LIBRARY }, { createSampleProject }, { buildShowcase }] = await Promise.all([import('../style/system'), import('../model/sampleProject'), import('../style/showcase')]);
  return {
    system: SYSTEM_LIBRARY,
    sampleProject: (srid) => {
      const doc = createSampleProject(srid);
      // The demo carries the whole system symbol library as a catalogue below the sheet.
      if (doc.homeView) {
        buildShowcase(doc, SYSTEM_LIBRARY.items, SYSTEM_LIBRARY.categories, { x: doc.homeView.minX, y: doc.homeView.minY - 80 });
        doc.dirty.set(false);
      }
      return doc;
    },
  };
}
