import type { AppContext } from '../../app/context';
import type { BuiltinPanel, RibbonPanel, RibbonSize } from '../../app/ribbon';
import { listen, type DisposableStore } from '../../core/disposable';
import { ENTITY_KIND_LABEL, type EntityKind } from '../../model/entities';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { colorField, layerField, lineTypeField, scaleField, weightField } from '../toolbar/fields';
import { tooltip } from '../widgets/tooltip';
import { commandControl, menuControl, overflowMenu, splitControl, type Control, type ControlHost } from './controls';

/**
 * How much of a panel shows. The ribbon lowers levels, rightmost panels
 * first and all by one step before any by two, until the tab fits:
 *   0  as designed (large buttons, labels)
 *   1  every button small with its label, three to a column
 *   2  icons only, three to a column (fields narrower)
 *   3  one button naming the panel; it opens the panel below it
 */
export type Level = 0 | 1 | 2 | 3;
export const LEVELS: readonly Level[] = [0, 1, 2, 3];

/** What a panel asks of the ribbon besides running commands. */
export interface PanelHost extends ControlHost {
  /** The collapsed panel's button was pressed: show the panel under it. */
  openCollapsed(panel: PanelView): void;
  /** A panel launcher that opens another tab. */
  showTab(id: string): void;
}

interface Slot {
  readonly control: Control;
  readonly size: RibbonSize;
}

/** One panel of a tab: its controls, title row, launcher and the button it folds into. */
export class PanelView {
  readonly el: HTMLElement;
  readonly body: HTMLElement;
  readonly model: RibbonPanel;
  readonly syncs: (() => void)[] = [];
  readonly collapsedButton: HTMLButtonElement;
  level: Level = 0;
  private readonly arrange: (level: Level) => void;

  constructor(ctx: AppContext, model: RibbonPanel, d: DisposableStore, host: PanelHost) {
    this.model = model;
    this.body = h('div', { class: 'rpanel__body' });
    const launcher = model.launcher;
    const launch = launcher ? h('button', { class: 'rpanel__launch', type: 'button', 'aria-label': launcher.title }, icon('launcher', 12)) : null;
    if (launch && launcher) {
      d.add(
        listen(launch, 'click', () => {
          if ('tab' in launcher) host.showTab(launcher.tab);
          else {
            ctx.commands.execute(launcher.command, launcher.args);
            host.afterRun();
          }
        }),
      );
      d.add(tooltip(launch, () => ({ title: launcher.title })));
    }
    this.collapsedButton = h(
      'button',
      { class: 'rbtn rbtn--large rpanel__collapsed', type: 'button', 'aria-haspopup': 'true', 'aria-expanded': 'false', 'aria-label': model.label },
      icon(model.icon, 20),
      h('span', { class: 'rbtn__label' }, model.label, h('span', { class: 'rbtn__caret' }, icon('chevronDown', 12))),
    );
    d.add(listen(this.collapsedButton, 'click', () => host.openCollapsed(this)));
    d.add(tooltip(this.collapsedButton, () => ({ title: model.label, description: 'Pencere dar olduğu için panel tek düğmeye katlandı; tıklayınca açılır.' })));

    const builtin = model.items.find((i) => i.kind === 'builtin');
    if (builtin?.kind === 'builtin') {
      this.arrange = builtinPanel(ctx, builtin.name, this.body, d, host, this.syncs);
    } else {
      const slots: Slot[] = [];
      for (const item of model.items) {
        if (item.kind === 'builtin') continue;
        const control =
          item.kind === 'command' ? commandControl(ctx, item.id, d, host) : item.kind === 'split' ? splitControl(ctx, item.key, item.entries, d, host) : menuControl(ctx, item.menu, d, host);
        if (control.sync) this.syncs.push(control.sync);
        slots.push({ control, size: item.size });
      }
      this.arrange = (level) => arrangeSlots(this.body, slots, level);
    }

    // Seldom used commands wait under a ▾ beside the title (AutoCAD's panel expander).
    const overflow = model.overflow;
    const more = overflow
      ? h(
          'button',
          { class: 'rpanel__more', type: 'button', 'aria-haspopup': 'menu', 'aria-expanded': 'false', 'aria-label': `${model.label}: diğer araçlar`, dataset: { commands: overflow.join(' ') } },
          h('span', { class: 'rpanel__label' }, model.label),
          icon('chevronDown', 10),
        )
      : null;
    if (more && overflow) {
      d.add(
        listen<PointerEvent>(more, 'pointerdown', (e) => {
          if (e.button !== 0) return;
          e.preventDefault();
          more.setAttribute('aria-expanded', 'true');
          overflowMenu(ctx, overflow, more, host, () => more.setAttribute('aria-expanded', 'false'));
        }),
      );
      d.add(tooltip(more, () => ({ title: `${model.label}: diğer araçlar`, description: overflow.map((id) => ctx.commands.get(id)?.title ?? id).join(' · ') })));
    }
    this.el = h(
      'div',
      { class: 'rpanel', role: 'group', 'aria-label': model.label, dataset: { panel: model.label } },
      this.body,
      this.collapsedButton,
      h('div', { class: 'rpanel__foot' }, more ?? h('span', { class: 'rpanel__label' }, model.label), launch),
    );
    this.setLevel(0);
  }

  setLevel(level: Level): void {
    this.level = level;
    this.el.dataset.level = String(level);
    // A folded panel shown in its pop-up is laid out in full.
    this.arrange(level === 3 ? 0 : level);
  }
}

/** Large buttons stand alone; small ones stack three to a column, in order. */
function arrangeSlots(body: HTMLElement, slots: readonly Slot[], level: Level): void {
  body.textContent = '';
  let column: HTMLElement | null = null;
  for (const { control, size } of slots) {
    const large = level === 0 && size === 'large';
    control.el.classList.toggle('rbtn--large', large);
    control.el.classList.toggle('rbtn--small', !large);
    control.el.classList.toggle('rbtn--icon', level === 2);
    if (large) {
      body.append(control.el);
      column = null;
      continue;
    }
    if (!column || column.childElementCount === 3) {
      column = h('div', { class: 'rpanel__col' });
      body.append(column);
    }
    column.append(control.el);
  }
}

type Arrange = (level: Level) => void;

/** Panels with live fields: the current layer and properties, and what is selected. */
function builtinPanel(ctx: AppContext, name: BuiltinPanel, body: HTMLElement, d: DisposableStore, host: ControlHost, syncs: (() => void)[]): Arrange {
  const small = (ids: string[]) =>
    ids.map((id) => {
      const c = commandControl(ctx, id, d, host, 'rbtn--small');
      if (c.sync) syncs.push(c.sync);
      return c.el;
    });
  const iconOnly = (els: HTMLElement[], on: boolean) => els.forEach((e) => e.classList.toggle('rbtn--icon', on));
  const width = (el: HTMLElement, px: number) => (el.style.width = `calc(${px}px * var(--ui-scale))`);

  if (name === 'layers') {
    const field = layerField(ctx, d, { width: 204 });
    const buttons = small(['layer.new', 'layer.newGroup', 'layer.showAll', 'view.rightPanel']);
    body.append(
      h(
        'div',
        { class: 'rpanel__stack' },
        field,
        h('div', { class: 'rpanel__row' }, buttons[0], buttons[1]),
        h('div', { class: 'rpanel__row' }, buttons[2], buttons[3]),
      ),
    );
    return (level) => {
      width(field, level >= 2 ? 150 : 204);
      iconOnly(buttons, level >= 2);
    };
  }

  if (name === 'properties') {
    const fields = [colorField(ctx, d), lineTypeField(ctx, d), weightField(ctx, d)];
    const scale = scaleField(ctx, d);
    body.append(h('div', { class: 'rpanel__stack' }, ...fields), h('div', { class: 'rpanel__stack' }, scale));
    return (level) => {
      fields.forEach((f) => width(f, level >= 2 ? 132 : level === 1 ? 162 : 188));
      width(scale, level >= 2 ? 112 : 134);
    };
  }

  // The selection: how much, of which kinds, and the selection commands.
  const count = h('span', { class: 'rsel__count num' });
  const noun = h('span', { class: 'rsel__noun' });
  const kinds = h('div', { class: 'rsel__kinds' });
  const buttons = small(['view.zoomSelection', 'edit.deselect', 'edit.invertSelection']);
  body.append(h('div', { class: 'rsel__total' }, count, noun), kinds, h('div', { class: 'rpanel__col' }, ...buttons));
  const sync = () => {
    // Counting a large selection is left for when the tab is in view (the ribbon syncs it on show).
    if (!body.isConnected) return;
    const byKind = new Map<EntityKind, number>();
    for (const id of ctx.selection.ids.value) {
      const e = ctx.doc.get(id);
      if (e) byKind.set(e.kind, (byKind.get(e.kind) ?? 0) + 1);
    }
    count.textContent = String(ctx.selection.size);
    noun.textContent = 'nesne seçili';
    const rows = [...byKind].sort((a, b) => b[1] - a[1]);
    const shown = rows.length > 3 ? rows.slice(0, 2) : rows;
    replaceChildren(
      kinds,
      ...shown.map(([k, n]) => h('span', { class: 'rsel__kind' }, h('span', { class: 'num' }, String(n)), ` ${ENTITY_KIND_LABEL[k].toLocaleLowerCase('tr-TR')}`)),
      rows.length > 3 ? h('span', { class: 'rsel__kind rsel__kind--more' }, `${rows.length - 2} tür daha`) : null,
    );
  };
  d.add(ctx.selection.ids.subscribe(sync, true));
  syncs.push(sync);
  return (level) => {
    kinds.hidden = level >= 2;
    iconOnly(buttons, level >= 2);
  };
}
