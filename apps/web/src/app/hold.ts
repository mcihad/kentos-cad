import type { AppContext } from './context';

/**
 * A hold on the drawing while a large import writes into it (docs/adr/0138):
 * the view moves (the wheel zooms, the middle button pans, the zoom commands
 * run) and nothing else does, because anything edited meanwhile would join
 * the import's undo step and go with it at Durdur. The desktop holds the
 * same way (apps/desktop/src/exchange/drawing_import.rs `while_importing`).
 *
 * - every command but the view's is refused, and every tool, with `why`;
 * - the running tool ends first (the selection tool stays, doing nothing);
 * - keys go nowhere but `keep`, and Esc anywhere calls `onEscape`; a press on the drawing
 *   only pans (middle button), and the menus over it do not open;
 * - the ribbon, the panels, the command line and the status bar are inert.
 *
 * Returns the release, which puts everything back as it was.
 */
export function holdDrawing(ctx: AppContext, opts: { why: string; keep: HTMLElement; onEscape(): void }): () => void {
  const undo: (() => void)[] = [];
  let said = -Infinity;
  // A refusal says why, not once per key of a held-down shortcut.
  const refuse = () => {
    const now = performance.now();
    if (now - said > 1500) ctx.log.warn(opts.why);
    said = now;
  };

  if (ctx.tools.activeId.value !== 'select' || ctx.tools.nested) ctx.tools.activate('select');
  ctx.tools.hold = refuse;
  undo.push(() => (ctx.tools.hold = null));
  ctx.commands.hold = (cmd) => {
    if (VIEW_COMMANDS.has(cmd.id)) return true;
    refuse();
    return false;
  };
  undo.push(() => (ctx.commands.hold = null));

  const keys = (e: KeyboardEvent) => {
    // Esc stops, wherever the focus is (the panel's Durdur has it from the start).
    const escape = e.key === 'Escape';
    if (!escape && opts.keep.contains(e.target as Node)) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    if (escape && e.type === 'keydown') opts.onEscape();
  };
  for (const type of ['keydown', 'keyup'] as const) {
    window.addEventListener(type, keys, true);
    undo.push(() => window.removeEventListener(type, keys, true));
  }

  const view = ctx.view.element;
  if (view) {
    // The middle button pans, the wheel zooms: those go through; a press that would pick, draw or open a menu does not.
    const press = (e: PointerEvent | MouseEvent) => {
      if (e.type === 'pointerdown' && (e as PointerEvent).button === 1) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.type === 'pointerdown') refuse();
    };
    for (const type of ['pointerdown', 'dblclick', 'contextmenu'] as const) {
      view.addEventListener(type, press, true);
      undo.push(() => view.removeEventListener(type, press, true));
    }
    // Everything round the drawing: the parts over it (the toolbox, the command bar), then each level's siblings up to the page.
    const inert: HTMLElement[] = [];
    const mark = (el: Element) => {
      if (el instanceof HTMLElement && !el.inert && !(el instanceof HTMLCanvasElement) && !el.contains(opts.keep)) {
        el.inert = true;
        inert.push(el);
      }
    };
    for (const child of view.children) mark(child);
    for (let el: HTMLElement = view; el.parentElement && el.parentElement !== document.body; el = el.parentElement) for (const sibling of el.parentElement.children) if (sibling !== el) mark(sibling);
    undo.push(() => inert.forEach((el) => (el.inert = false)));
  }

  return () => {
    for (const u of undo.splice(0).reverse()) u();
  };
}

/** Commands that only move the view: the ones a hold lets run. */
const VIEW_COMMANDS: ReadonlySet<string> = new Set(['view.zoomExtents', 'view.zoomIn', 'view.zoomOut', 'view.fullscreen']);
