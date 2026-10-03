import { openExportDialog, type ExportFormat } from '../../ui/sheet/ExportDialog';
import { openPageSetup } from '../../ui/sheet/PageSetupDialog';
import { openSaveTemplate } from '../../ui/sheet/SaveTemplateDialog';
import { openVariables } from '../../ui/sheet/VariablesDialog';
import type { AppContext } from '../context';
import type { SheetWindow } from './commands';
import { exportKpafta, exportPng, exportSvg, importKpafta, KPAFTA } from './exporting';
import { exportPdf, mapWays, pdfName, pdfNotes, tmCrsOf } from './pdfExport';
import type { SheetService } from './service';
import { saveAsTemplate } from './templateActions';

/**
 * The sheet's windows, loaded the first time one opens (CLAUDE.md §20):
 * Sayfa ayarları, Değişkenler, Şablon olarak kaydet, Dışa aktar and the
 * `.kpafta` file chooser. The windows are the interface's
 * (ui/sheet/*Dialog.ts); what they do with the book, the device's store and
 * the files is given to them from here.
 */
export function openSheetWindow(ctx: AppContext, s: SheetService, name: SheetWindow, args?: unknown): void {
  const sheet = s.state.open.value;
  switch (name) {
    case 'pageSetup':
      if (sheet) openPageSetup(ctx, s, sheet);
      return;
    case 'variables':
      if (sheet) openVariables(ctx, s, sheet);
      return;
    case 'saveTemplate':
      if (sheet) openSaveTemplate(ctx, s, sheet, (words, replace) => saveAsTemplate(ctx, s, sheet, words, replace).then((t) => !!t));
      return;
    case 'export': {
      if (!sheet) return;
      const a = args as { format?: ExportFormat; print?: boolean } | undefined;
      openExportDialog(ctx, s, sheet, { format: a?.format ?? 'pdf', print: a?.print }, {
        svg: (dpi) => exportSvg(ctx, s, sheet, dpi),
        png: (dpi) => exportPng(ctx, s, sheet, dpi),
        kpafta: () => exportKpafta(ctx, s),
        pdf: (choice, action, tab) => exportPdf(ctx, s, choice, action, tab),
        pdfWays: (choice) => mapWays(ctx, s, choice),
        pdfNotes: (choice) => pdfNotes(s, choice),
        pdfName: (choice) => pdfName(ctx, s, choice),
        pdfGeoReady: () => {
          const crs = ctx.doc.crs.value;
          return !!crs && !!tmCrsOf(crs);
        },
      });
      return;
    }
    case 'importKpafta':
      void pickText(ctx).then((f) => f && importKpafta(ctx, s, f.text, f.name));
      return;
    case 'preflight':
      return;
  }
}

/** Asks for a `.kpafta` file and reads it as text; null when cancelled (or said in the log when it could not be read). */
async function pickText(ctx: AppContext): Promise<{ name: string; text: string } | null> {
  try {
    const handle = await ctx.files.picker.open(KPAFTA);
    if (handle === null) return null;
    if (handle) return { name: handle.name, text: await (await handle.getFile()).text() };
  } catch (e) {
    ctx.log.error(`Dosya açılamadı: ${(e as Error).message}. Tarayıcının dosya iznini denetleyin.`);
    return null;
  }
  // A browser without the file system access dialogs: its own file input.
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.kpafta,application/json';
    input.addEventListener('change', () => {
      const file = input.files?.[0];
      if (!file) return resolve(null);
      file.text().then(
        (text) => resolve({ name: file.name, text }),
        () => resolve(null),
      );
    });
    input.click();
  });
}
