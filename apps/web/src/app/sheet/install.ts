import '../../ui/sheet/sheet.css';
import type { Disposable } from '../../core/disposable';
import { DisposableStore } from '../../core/disposable';
import type { Signal } from '../../core/signal';
import type { RibbonExtension } from '../../ui/ribbon/Ribbon';
import { registerSheetIcons } from '../../ui/sheet/icons';
import { SheetTabs } from '../../ui/sheet/SheetTabs';
import type { SheetWorkspace } from '../../ui/sheet/SheetWorkspace';
import type { FrontHistory } from '../commands';
import type { AppContext } from '../context';
import { registerSheetCommands, type SheetWindow } from './commands';
import { sheetToolCommands } from './toolCommands';
import { bindSheetKeys } from './keys';
import { sheetRibbonTab } from './ribbonTab';
import { SheetService } from './service';

/**
 * Connects the sheet layouts to the app (docs/sheet/integration.md §3): one
 * call from the composition root once the shell exists. It makes the sheet
 * service, adds the sheet icons, the commands and their keys, the Model |
 * Pafta tabs under the drawing area, the contextual Pafta tab of the ribbon,
 * and, the first time a sheet comes forward, the workspace over the drawing
 * area and the sheet's cells in the status bar (loaded then, CLAUDE.md §20).
 * The engine (3.3 MB) is fetched then too, or when the gallery first opens,
 * never when the app starts. While a sheet is in front the shell carries
 * `data-sheet-mode` (the right dock and the drawing's status cells give way,
 * sheet.css), Geri al and Yinele go to the sheet's own history
 * (`frontHistory`), Yazdır ve pafta prints the sheet (`frontPrint`), and a
 * drawing command run from the ribbon or the command line brings Model back
 * first.
 */

/** What the shell gives the sheets (ui/shell/AppShell.ts). */
export interface SheetShell {
  /** The shell's root. */
  readonly el: HTMLElement;
  /** The drawing area: the workspace lies over it while a sheet is in front. */
  readonly viewportHost: HTMLElement;
  /** The row under the drawing area the Model | Pafta tabs go in. */
  readonly sheetTabs: HTMLElement;
  readonly status: { readonly el: HTMLElement };
  /** Adds a contextual tab to the ribbon. */
  extendRibbon(ext: RibbonExtension): Disposable;
}

const services = new WeakMap<AppContext, SheetService>();
const workspaces = new WeakMap<AppContext, SheetWorkspace>();

/** The sheet service of an app (tests and the screenshot script, scripts/e2e/sheet-shots.mjs). */
export const sheetsOf = (ctx: AppContext): SheetService | undefined => services.get(ctx);

/** The sheet workspace once loaded (the screenshot script finds items on the paper with it). */
export const workspaceOf = (ctx: AppContext): SheetWorkspace | undefined => workspaces.get(ctx);

/** Drawing commands that bring Model forward when run while a sheet is (from the ribbon, a menu, the command line). */
export function opensModel(id: string): boolean {
  if (id === 'tool.cancel' || id === 'tool.confirm' || id === 'edit.undo' || id === 'edit.redo') return false;
  return /^(tool|edit|draft)\./.test(id) || ['view.zoomExtents', 'view.zoomIn', 'view.zoomOut', 'view.zoomSelection', 'view.previous', 'view.next', 'view.rightPanel'].includes(id);
}

export function installSheets(ctx: AppContext, shell: SheetShell, frontHistory?: Signal<FrontHistory | null>, frontPrint?: Signal<(() => void) | null>): Disposable {
  const d = new DisposableStore();
  registerSheetIcons();
  const sheets = new SheetService(ctx);
  services.set(ctx, sheets);
  d.add(() => sheets.dispose());
  const { state } = sheets;

  let workspace: SheetWorkspace | null = null;
  let loading: Promise<SheetWorkspace> | null = null;
  const loadWorkspace = (): Promise<SheetWorkspace> =>
    (loading ??= Promise.all([import('../../ui/sheet/SheetWorkspace'), import('../../ui/sheet/statusCells')]).then(
      ([w, s]) => {
        const ws = new w.SheetWorkspace(ctx, sheets);
        d.add(() => ws.dispose());
        shell.viewportHost.append(ws.el);
        shell.status.el.prepend(s.sheetStatusCells(ctx, sheets, ws.stage, d));
        workspaces.set(ctx, ws);
        return (workspace = ws);
      },
      (e: Error) => {
        loading = null;
        throw e;
      },
    ));
  const failed = (what: string) => (e: Error) => ctx.log.error(`${what} yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`);

  const tabs = new SheetTabs(ctx, sheets);
  shell.sheetTabs.append(tabs.el);
  d.add(() => tabs.dispose());

  d.add(
    registerSheetCommands(ctx, sheets, {
      gallery: () => void import('../../ui/sheet/TemplateGallery').then((m) => m.openTemplateGallery(ctx, sheets), failed('Pafta şablonları')),
      stage: () => workspace?.stage ?? null,
      window: (name: SheetWindow, args?: unknown) => {
        if (name === 'preflight') return workspace?.showPreflight();
        void import('./windows').then((m) => m.openSheetWindow(ctx, sheets, name, args), failed('Pafta penceresi'));
      },
      ask: (title, label, value) => import('../../ui/sheet/widgets/askText').then((m) => m.askText({ title, label, value })),
    }),
  );
  // The mode's tools are commands of their own, registered again when the profile changes.
  let toolCommands: Disposable | null = null;
  d.add(
    sheets.tools.subscribe((tools) => {
      toolCommands?.();
      toolCommands = sheetToolCommands(ctx, sheets, tools);
    }, true),
  );
  d.add(() => toolCommands?.());
  d.add(bindSheetKeys(ctx, sheets, (held) => workspace?.stage.setSpace(held)));
  d.add(
    shell.extendRibbon({
      tabs: () => [sheetRibbonTab(sheets.profile.value)],
      version: sheets.profile,
      shown: sheets.inSheet,
      tip: { title: 'Pafta (bağlamsal)', description: 'Bir pafta öndeyken görünür: paftanın araçları, öğe ekleme, düzen ve çıktı.' },
    }),
  );

  // Geri al and Yinele (the quick access bar, their keys) take the sheet's history while a sheet is in front.
  if (frontHistory) {
    const offer = () => {
      const h = sheets.history.value;
      frontHistory.set(state.open.value !== null && h ? { undo: () => sheets.undo(), redo: () => sheets.redo(), canUndo: () => h.canUndo.value, canRedo: () => h.canRedo.value } : null);
    };
    let steps: Disposable[] = [];
    d.add(
      sheets.history.subscribe((h) => {
        steps.forEach((s) => s());
        steps = h ? [h.canUndo.subscribe(offer), h.canRedo.subscribe(offer)] : [];
        offer();
      }, true),
    );
    d.add(state.open.subscribe(offer));
    d.add(() => {
      steps.forEach((s) => s());
      frontHistory.set(null);
    });
  }

  // Yazdır ve pafta (`file.print`, Ctrl+P, the app menu) prints the sheet while one is in front (W-17).
  if (frontPrint) {
    const offer = () => frontPrint.set(state.open.value !== null ? () => ctx.commands.execute('sheet.print') : null);
    d.add(state.open.subscribe(offer, true));
    d.add(() => frontPrint.set(null));
  }

  // A sheet comes forward: the drawing's running command ends, the workspace takes the drawing area.
  d.add(
    state.open.subscribe((id) => {
      if (id === null) {
        shell.el.removeAttribute('data-sheet-mode');
        workspace?.show(false);
        return;
      }
      if (ctx.tools.activeId.value !== 'select' || ctx.tools.nested) ctx.tools.activate('select');
      void loadWorkspace().then(
        (ws) => {
          if (state.open.value === null) return;
          shell.el.setAttribute('data-sheet-mode', '');
          ws.show(true);
          ws.stage.focus();
        },
        (e: Error) => {
          failed('Pafta görünüşü')(e);
          state.openSheet(null);
        },
      );
    }),
  );
  // A drawing command run while a sheet is in front shows the drawing it works on.
  d.add(ctx.commands.events.on('executed', ({ command }) => sheets.inSheet.value && opensModel(command.id) && state.openSheet(null)));
  d.add(sheets.watchProject());
  return () => d.dispose();
}
