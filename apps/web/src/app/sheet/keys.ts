import { chordFromEvent, isTextInput, normalizeChord } from '../../core/keymap';
import type { Disposable } from '../../core/disposable';
import { listen } from '../../core/disposable';
import type { AppContext } from '../context';
import { toolKeys } from './toolCommands';
import type { SheetService } from './service';

/**
 * The sheet mode's keys (docs/sheet/design.md §11), registered in the app's
 * keymap so that tooltips, menus and the shortcut list show them: V Seç,
 * H El (Space held: a moment's hand), the mode's tools by the profile's
 * keys (M Harita or Görünüm penceresi, T Metin, L Lejant); Ctrl+G
 * grupla, Ctrl+Shift+G grubu çöz, Ctrl+D çoğalt, Ctrl+] / Ctrl+[ sıra (by
 * the key's place: Ü and Ğ on a Turkish Q keyboard), Ctrl+0 sayfayı sığdır,
 * Ctrl+1 gerçek boyut, Delete siler, F2 ad verir, Ctrl+Z / Ctrl+Y the sheet's
 * own undo, arrows move the choice (1 mm, Shift 10, Alt 0.1), Esc lets the
 * choice go. They hold only while a sheet is in front.
 *
 * While a sheet is in front a key that the drawing would take (a drawing
 * tool's letter, its undo, its aids) does not reach the drawing behind the
 * paper, where it would change what cannot be seen: it is held and the
 * message cell says why. A sheet key whose command cannot run now says why
 * too, instead of falling through to the drawing's key of the same letter.
 */

/** The sheet's chords and their commands (the arrows and brackets are read by key below). */
const SHEET_KEYS: readonly (readonly [string, string, unknown?])[] = [
  ['V', 'sheet.tool.select'],
  ['H', 'sheet.tool.hand'],
  ['Ctrl+G', 'sheet.group'],
  ['Ctrl+Shift+G', 'sheet.ungroup'],
  ['Ctrl+D', 'sheet.duplicateItems'],
  ['Ctrl+0', 'sheet.zoomPage'],
  ['Home', 'sheet.zoomPage'],
  ['Ctrl+1', 'sheet.zoomReal'],
  ['+', 'sheet.zoomIn'],
  ['-', 'sheet.zoomOut'],
  ['Delete', 'sheet.deleteItems'],
  ['F2', 'sheet.renameItem'],
  ['Ctrl+Z', 'sheet.undo'],
  ['Ctrl+Y', 'sheet.redo'],
  ['Ctrl+Shift+Z', 'sheet.redo'],
  ['Ctrl+A', 'sheet.selectAll'],
  ['Ctrl+P', 'sheet.print'],
  ['Esc', 'sheet.escape'],
];

/** Ctrl+] and Ctrl+[ by the key's place (KeyboardEvent.code), whatever the layout prints on it. */
const BRACKETS: Readonly<Record<string, readonly [string, string]>> = {
  BracketRight: ['sheet.order.forward', 'sheet.order.front'],
  BracketLeft: ['sheet.order.backward', 'sheet.order.back'],
};

/** Commands that may run from a key while a sheet is in front: the app's own, not the drawing's. */
export function keyAllowedInSheet(command: string): boolean {
  if (/^(sheet|file|cloud|help|workspace)\./.test(command)) return true;
  return ['tools.options', 'view.ribbonCollapse', 'view.commandSearch', 'view.keyTips', 'view.fullscreen', 'view.theme.toggle'].includes(command);
}

export function bindSheetKeys(ctx: AppContext, sheets: SheetService, setSpace: (held: boolean) => void): Disposable {
  const inSheet = () => sheets.inSheet.value;
  const subs: Disposable[] = [];
  const chords = new Map<string, string>();
  for (const [chord, command] of SHEET_KEYS) {
    subs.push(ctx.keymap.bind(chord, command, { when: inSheet }));
    chords.set(normalizeChord(chord), command);
  }
  // The mode's tools' keys (the profile's): bound again when the profile changes.
  let toolSubs: Disposable[] = [];
  const bindTools = () => {
    toolSubs.forEach((d) => d());
    toolSubs = [];
    for (const [k, c] of [...chords]) if (c.startsWith('sheet.add.')) chords.delete(k);
    for (const [chord, command] of toolKeys(sheets.tools.value)) {
      toolSubs.push(ctx.keymap.bind(chord, command, { when: inSheet }));
      chords.set(normalizeChord(chord), command);
    }
  };
  subs.push(sheets.tools.subscribe(bindTools, true));
  subs.push(() => toolSubs.forEach((d) => d()));
  // Shown in tooltips and the shortcut list; the layer below runs them by the key's place.
  subs.push(ctx.keymap.bind('Ctrl+]', 'sheet.order.forward', { when: () => false }));
  subs.push(ctx.keymap.bind('Ctrl+[', 'sheet.order.backward', { when: () => false }));
  subs.push(ctx.keymap.bind('Ctrl+Shift+]', 'sheet.order.front', { when: () => false }));
  subs.push(ctx.keymap.bind('Ctrl+Shift+[', 'sheet.order.back', { when: () => false }));

  let said = -Infinity;
  let last = '';
  /** Says why a key did nothing; the same words again only after a pause (not once per key of a held-down one). */
  const say = (text: string) => {
    const now = performance.now();
    if (text !== last || now - said > 1500) ctx.log.warn(text);
    said = now;
    last = text;
  };
  const run = (e: KeyboardEvent, command: string, args?: unknown) => {
    e.preventDefault();
    e.stopImmediatePropagation();
    if (ctx.commands.isEnabled(command)) {
      ctx.commands.execute(command, args);
      return;
    }
    const c = ctx.commands.get(command);
    say(`${c?.title ?? command}: ${c?.whyDisabled?.() ?? 'şu an kullanılamıyor.'}`);
  };
  /** A window or a menu has the keys; a text field keeps its own (Esc and the arrows among them). */
  const elsewhere = (e: KeyboardEvent) => !!document.querySelector('.dialog-backdrop') || !!document.querySelector('.menu') || isTextInput(e.target as Element | null) || isTextInput(document.activeElement);

  subs.push(
    listen<KeyboardEvent>(
      window,
      'keydown',
      (e) => {
        if (!inSheet() || e.defaultPrevented || e.isComposing || elsewhere(e)) return;
        const focus = document.activeElement as HTMLElement | null;
        const onControl = !!focus && focus !== document.body && focus.matches('button, [role="tab"], [role="treeitem"], [role="option"], [role="slider"], a[href], .tree');
        // Space: the hand while it is held, unless a button or a list row has the focus (Space presses it).
        if (e.code === 'Space' && !e.ctrlKey && !e.altKey) {
          if (onControl) return;
          e.preventDefault();
          e.stopImmediatePropagation();
          if (!e.repeat) setSpace(true);
          return;
        }
        // Arrows move the choice when the paper (or nothing in particular) has the keys; a held arrow repeats.
        if (e.key.startsWith('Arrow') && !e.ctrlKey && !onControl && sheets.state.selection.value.size) {
          const dx = e.key === 'ArrowLeft' ? -1 : e.key === 'ArrowRight' ? 1 : 0;
          const dy = e.key === 'ArrowUp' ? -1 : e.key === 'ArrowDown' ? 1 : 0;
          // The step is the engine's: 1 mm, Shift 10 mm, Alt 0.1 mm.
          run(e, 'sheet.nudge', { dx, dy, step: e.shiftKey ? 'shift' : e.altKey ? 'alt' : 'plain' });
          return;
        }
        const bracket = BRACKETS[e.code];
        if (bracket && (e.ctrlKey || e.metaKey) && !e.altKey) return run(e, bracket[e.shiftKey ? 1 : 0]);
        const chord = chordFromEvent(e);
        if (!chord) return;
        const own = chords.get(chord);
        if (own) {
          // Enter, Space and the like keep their meaning on the focused control; the sheet's keys are letters and chords.
          if (onControl && (chord === 'Delete' || chord === 'F2' || chord === 'Home')) return;
          if (e.repeat && chord !== '+' && chord !== '-') {
            e.preventDefault();
            e.stopImmediatePropagation();
            return;
          }
          return run(e, own);
        }
        // A key of the drawing: held, and said why. The app's own keys (save, search, settings) go on.
        const b = ctx.keymap.resolve(e);
        if (b && !keyAllowedInSheet(b.command)) {
          e.preventDefault();
          e.stopImmediatePropagation();
          // Enter finishes a drawing command: on the paper there is none to finish, nothing to say.
          if (chord === 'Enter' || chord === 'Shift+Enter') return;
          const c = ctx.commands.get(b.command);
          say(`Pafta önde: “${c?.title ?? b.command}” çizim alanının kısayolu. Modele dönmek için Model sekmesine tıklayın.`);
        }
      },
      true,
    ),
  );
  subs.push(
    listen<KeyboardEvent>(
      window,
      'keyup',
      (e) => {
        if (e.code === 'Space') setSpace(false);
      },
      true,
    ),
  );
  // The hand lets go when the window loses the keys with Space down.
  subs.push(listen(window, 'blur', () => setSpace(false)));
  return () => subs.forEach((d) => d());
}
