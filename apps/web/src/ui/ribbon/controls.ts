import type { AppContext } from '../../app/context';
import { commandItem, resolveMenu, type SubmenuSpec } from '../../app/menus';
import type { SplitEntry } from '../../app/ribbon';
import { listen, type DisposableStore } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { h } from '../dom';
import { icon } from '../icons';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip, type TooltipContent } from '../widgets/tooltip';

/**
 * Ribbon buttons. A button is made once and keeps its subscriptions; the
 * panel only changes its size class (large, small, icon) as the window
 * narrows, so shrinking never rebuilds or re-subscribes anything.
 */

/** What a button needs from the ribbon around it. */
export interface ControlHost {
  /** After a button ran something: a ribbon opened over the drawing closes. */
  afterRun(): void;
  /** Right button on a command (quick access). */
  commandMenu(id: string, at: { x: number; y: number }): void;
}

/** A control and how to bring its state up to date (for state that hangs on nested signals). */
export interface Control {
  readonly el: HTMLElement;
  readonly sync?: () => void;
}

/** Ribbon label: the short name when there is one, without the dialog ellipsis. */
export function ribbonLabel(title: string, short?: string): string {
  return (short ?? title).replace(/…$/, '');
}

/** Tooltip of a command: tools add their mouse steps; pending features say so. */
export function commandTip(ctx: AppContext, id: string): TooltipContent {
  const cmd = ctx.commands.get(id);
  const tool = id.startsWith('tool.') ? ctx.tools.get(id.slice(5)) : undefined;
  if (tool) {
    return {
      title: tool.label,
      shortcut: tool.id === 'select' ? 'Esc' : ctx.keymap.chordFor(id),
      description: tool.description,
      steps: tool.ready ? tool.steps : undefined,
      note: tool.ready ? undefined : 'Geliştirme aşamasında',
    };
  }
  return {
    title: cmd?.title ?? id,
    shortcut: ctx.keymap.chordFor(id),
    description: cmd?.description,
    note: cmd?.pending ? (cmd.pendingNote ?? 'Geliştirme aşamasında') : cmd ? undefined : 'Bu komut bu oturumda yok.',
  };
}

/**
 * A command button. Tools show "active" as the toolbox does (filled
 * amber), toggles as pressed (soft amber); disabled and pending follow the
 * command.
 */
export function commandControl(ctx: AppContext, id: string, d: DisposableStore, host: ControlHost, extraClass = ''): Control {
  const cmd = ctx.commands.get(id);
  const isTool = id.startsWith('tool.');
  const b = h(
    'button',
    { class: `rbtn ${extraClass}`, type: 'button', 'aria-label': cmd?.title ?? id, dataset: { command: id } },
    icon(cmd?.icon ?? 'more', 20),
    h('span', { class: 'rbtn__label' }, ribbonLabel(cmd?.title ?? id, cmd?.short)),
  );
  if (isTool) b.dataset.tool = '';
  if (cmd?.pending) b.dataset.pending = '';
  const sync = () => {
    b.disabled = !ctx.commands.isEnabled(id);
    const checked = cmd?.isChecked?.();
    if (checked === undefined) b.removeAttribute('aria-pressed');
    else b.setAttribute('aria-pressed', String(checked));
  };
  sync();
  if (cmd?.watch) d.add(watchAll(cmd.watch, sync));
  // A click does not take the keyboard from the drawing: Enter still repeats the last command.
  d.add(listen<PointerEvent>(b, 'pointerdown', (e) => e.button === 0 && e.preventDefault()));
  d.add(
    listen(b, 'click', () => {
      if (!ctx.commands.execute(id)) return;
      // Typed coordinates go to the drawing, as after a toolbox click.
      if (isTool) ctx.view.focus();
      host.afterRun();
    }),
  );
  d.add(
    listen<MouseEvent>(b, 'contextmenu', (e) => {
      e.preventDefault();
      host.commandMenu(id, { x: e.clientX, y: e.clientY });
    }),
  );
  d.add(tooltip(b, () => commandTip(ctx, id), 'bottom'));
  return { el: b, sync };
}

const choiceKey = (e: SplitEntry) => `${e.command}|${e.option ?? ''}`;

/** Runs a split entry: the tool, then its method's option as if typed. */
export function runEntry(ctx: AppContext, e: SplitEntry): boolean {
  if (!ctx.commands.execute(e.command)) return false;
  if (e.option && !ctx.tools.active.input?.(e.option)) ctx.log.warn(`“${e.title}: ${e.label}” şu an başlatılamadı.`);
  if (e.command.startsWith('tool.')) ctx.view.focus();
  return true;
}

/**
 * A split button: a tool family (Dikdörtgen, Döndürülmüş dikdörtgen,
 * Düzgün çokgen) or one tool's methods (Daire: 2 nokta, 3 nokta …). The
 * top part runs the entry chosen last; the arrow lists them all. Large, the
 * icon is the top part and the label with its arrow the bottom part.
 */
export function splitControl(ctx: AppContext, key: string, entries: readonly SplitEntry[], d: DisposableStore, host: ControlHost): Control {
  // The choice last made stays on top, across sessions (AutoCAD keeps the last method too).
  const splits = ctx.ui.ribbonSplits;
  const current = () => entries.find((e) => choiceKey(e) === splits.value[key]) ?? entries[0];
  const main = h('button', { class: 'rsplit__main', type: 'button', dataset: { tool: '' } });
  const arrow = h('button', { class: 'rsplit__arrow', type: 'button', 'aria-haspopup': 'menu', 'aria-expanded': 'false' });
  const el = h('div', { class: 'rbtn rsplit', role: 'group', dataset: { split: key, commands: [...new Set(entries.map((e) => e.command))].join(' ') } }, main, arrow);
  const tools = new Set(entries.map((e) => e.command));
  const render = () => {
    const e = current();
    const cmd = ctx.commands.get(e.command);
    const icon20 = icon(cmd?.icon ?? 'more', 20);
    const label = ribbonLabel(e.title);
    main.replaceChildren(icon20, h('span', { class: 'rbtn__label rsplit__label' }, label));
    arrow.replaceChildren(h('span', { class: 'rbtn__label rsplit__label' }, label), h('span', { class: 'rbtn__caret' }, icon('chevronDown', 12)));
    main.dataset.command = e.command;
    main.setAttribute('aria-label', e.option || e.label !== e.title ? `${e.title}: ${e.label}` : e.title);
    arrow.setAttribute('aria-label', `${e.title}: diğer seçenekler`);
    if (cmd?.pending) main.dataset.pending = '';
    else delete main.dataset.pending;
  };
  const sync = () => {
    const on = tools.has(`tool.${ctx.tools.activeId.value}`);
    main.setAttribute('aria-pressed', String(on));
    el.toggleAttribute('data-active', on);
  };
  render();
  sync();
  d.add(ctx.tools.activeId.subscribe(sync));
  d.add(listen<PointerEvent>(main, 'pointerdown', (e) => e.button === 0 && e.preventDefault()));
  d.add(listen(main, 'click', () => runEntry(ctx, current()) && host.afterRun()));
  d.add(
    listen<MouseEvent>(main, 'contextmenu', (e) => {
      e.preventDefault();
      host.commandMenu(current().command, { x: e.clientX, y: e.clientY });
    }),
  );
  d.add(
    tooltip(
      main,
      () => {
        const e = current();
        const tip = commandTip(ctx, e.command);
        // A method names itself after the tool (Daire: 3 nokta).
        return e.label !== e.title ? { ...tip, title: `${e.title}: ${e.label}`, description: e.description ?? tip.description, steps: e.option ? undefined : tip.steps } : tip;
      },
      'bottom',
    ),
  );
  const open = (keyboard: boolean) => {
    arrow.setAttribute('aria-expanded', 'true');
    // A tool's methods are listed under its name; a family lists its tools.
    const methods = entries.every((e) => e.command === entries[0].command);
    const items: MenuItem[] = entries.map((e) => ({
      ...commandItem(ctx, e.command),
      label: e.label,
      hint: e.description,
      run: () => {
        splits.set({ ...splits.value, [key]: choiceKey(e) });
        render();
        if (runEntry(ctx, e)) host.afterRun();
      },
    }));
    const m = PopupMenu.open(methods ? [{ kind: 'header', label: entries[0].title }, ...items] : items, el.getBoundingClientRect(), {
      minWidth: 300,
      owner: el,
      onClose: () => arrow.setAttribute('aria-expanded', 'false'),
    });
    if (keyboard) m.focusFirst();
  };
  d.add(
    listen<PointerEvent>(arrow, 'pointerdown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      if (arrow.getAttribute('aria-expanded') === 'true') PopupMenu.closeAll();
      else open(false);
    }),
  );
  d.add(
    listen<KeyboardEvent>(arrow, 'keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        open(true);
      }
    }),
  );
  d.add(tooltip(arrow, () => ({ title: `${current().title}: diğer seçenekler`, description: entries.map((e) => e.label).join(' · ') }), 'bottom'));
  return { el, sync };
}

/** The ▾ beside a panel's title: its seldom used commands. */
export function overflowMenu(ctx: AppContext, ids: readonly string[], anchor: HTMLElement, host: ControlHost, onClose: () => void): PopupMenu {
  return PopupMenu.open(afterEach(ids.map((id) => commandItem(ctx, id)), () => host.afterRun()), anchor.getBoundingClientRect(), { minWidth: 220, owner: anchor, onClose });
}

/** A drop-down button for a submenu of the menu model (Tema, İçe aktar …). */
export function menuControl(ctx: AppContext, spec: SubmenuSpec, d: DisposableStore, host: ControlHost): Control {
  const b = h(
    'button',
    { class: 'rbtn', type: 'button', 'aria-haspopup': 'menu', 'aria-expanded': 'false', 'aria-label': spec.label, dataset: { menu: spec.label } },
    icon(spec.icon ?? 'more', 20),
    h('span', { class: 'rbtn__label' }, spec.label, h('span', { class: 'rbtn__caret' }, icon('chevronDown', 12))),
  );
  const open = (keyboard: boolean) => {
    b.setAttribute('aria-expanded', 'true');
    const m = PopupMenu.open(afterEach(resolveMenu(ctx, spec.items), () => host.afterRun()), b.getBoundingClientRect(), {
      minWidth: 220,
      owner: b,
      onClose: () => b.setAttribute('aria-expanded', 'false'),
    });
    if (keyboard) m.focusFirst();
  };
  d.add(
    listen<PointerEvent>(b, 'pointerdown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      if (b.getAttribute('aria-expanded') === 'true') PopupMenu.closeAll();
      else open(false);
    }),
  );
  d.add(
    listen<KeyboardEvent>(b, 'keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        open(true);
      }
    }),
  );
  // The choice in force, for menus of radio items (Tema: Koyu).
  d.add(
    tooltip(b, () => {
      const current = resolveMenu(ctx, spec.items).find((i) => i.checked && i.radio);
      return { title: spec.label, description: current ? `Şu an: ${current.label}.` : undefined };
    }),
  );
  return { el: b };
}

/** Runs `after` once an item of the menu (or of its submenus) has run. */
function afterEach(items: MenuItem[], after: () => void): MenuItem[] {
  return items.map((i) => ({
    ...i,
    run: i.run &&
      (() => {
        i.run!();
        after();
      }),
    items: i.items && (() => afterEach(typeof i.items === 'function' ? i.items() : i.items!, after)),
  }));
}
