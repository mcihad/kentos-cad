import type { AppContext } from './context';

/**
 * Default shortcuts. Letter keys follow the produced character, so they
 * match the key caps on Turkish Q and F layouts. Browser-reserved chords
 * (Ctrl+N, Ctrl+T, Ctrl+W, Ctrl+Shift+T) are avoided on purpose.
 */
export function registerDefaultKeybindings(ctx: AppContext): void {
  const { keymap, tools } = ctx;
  const global = { allowInInput: true };

  keymap.bind('Ctrl+Alt+N', 'file.new', global);
  keymap.bind('Ctrl+O', 'file.open', global);
  keymap.bind('Ctrl+S', 'file.save', global);
  keymap.bind('Ctrl+Shift+S', 'file.saveAs', global);
  keymap.bind('Ctrl+P', 'file.print', global);

  keymap.bind('Ctrl+Z', 'edit.undo');
  keymap.bind('Ctrl+Y', 'edit.redo');
  keymap.bind('Ctrl+Shift+Z', 'edit.redo');
  keymap.bind('Ctrl+X', 'edit.cut');
  keymap.bind('Ctrl+C', 'edit.copy');
  keymap.bind('Ctrl+V', 'edit.paste');
  keymap.bind('Ctrl+Shift+V', 'edit.pasteOriginal');
  keymap.bind('Ctrl+A', 'edit.selectAll');

  keymap.bind('Home', 'view.zoomExtents');
  keymap.bind('+', 'view.zoomIn');
  keymap.bind('-', 'view.zoomOut');
  keymap.bind('Ctrl+Shift+F', 'view.zoomSelection');
  keymap.bind('F2', 'view.bottomPanel', global);
  keymap.bind('F4', 'view.rightPanel', global);
  keymap.bind('F9', 'view.toolbox', global);

  keymap.bind('F3', 'draft.snap', global);
  keymap.bind('F7', 'draft.grid', global);
  keymap.bind('F8', 'draft.ortho', global);
  keymap.bind('F10', 'draft.polar', global);
  // F11 (AutoCAD's key) is the browser's full screen; Shift+F3 sits next to snaps.
  keymap.bind('Shift+F3', 'draft.tracking', global);

  keymap.bind('F1', 'help.shortcuts', global);
  keymap.bind('Ctrl+F1', 'view.ribbonCollapse', global);
  keymap.bind('Alt+Q', 'view.commandSearch', global);
  keymap.bind('Ctrl+,', 'tools.options', global);
  keymap.bind('Space', 'commandline.focus');
  keymap.bind('Esc', 'tool.cancel');
  keymap.bind('Enter', 'tool.confirm');

  for (const d of tools.list()) {
    // Esc is shared with "cancel"; the select tool only displays it.
    if (d.shortcut && d.id !== 'select') keymap.bind(d.shortcut, `tool.${d.id}`);
  }
}
