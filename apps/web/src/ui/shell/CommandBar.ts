import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { SNAP_LABEL, type SnapKind } from '../../viewport/picking';
import { canCalcPoint } from '../../tools/pointCalc';
import { calcMenuItems } from './calcMenu';
import { optionButtons, parsePrompt } from '../promptOptions';
import { PopupMenu } from '../widgets/PopupMenu';

/**
 * Strip at the top of the drawing while a command runs: tool, the step it
 * waits for, its options as buttons and a reminder of what the right mouse
 * button and Esc do. Mouse users read and act here instead of the command
 * line at the bottom.
 */
export class CommandBar extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly tool: HTMLElement;
  private readonly step: HTMLElement;
  private readonly opts: HTMLElement;
  private readonly snap: HTMLElement;
  private readonly calc: HTMLButtonElement;

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    this.tool = h('span', { class: 'cmdbar__tool' });
    this.step = h('span', { class: 'cmdbar__step' });
    this.opts = h('span', { class: 'cmdbar__opts' });
    this.snap = h('span', { class: 'cmdbar__snap', hidden: true });
    // Netcad's coordinate calculator: offered whenever the command waits for a point.
    this.calc = h('button', { class: 'cmdbar__opt cmdbar__calc', type: 'button', title: 'Ölçüyle nokta hesapla (yan nokta, kesişim, açı-mesafe…)' }, icon('calc', 14), h('span', null, 'Nokta hesabı'));
    this.calc.addEventListener('pointerdown', (e) => e.preventDefault());
    this.calc.addEventListener('click', () => {
      const r = this.calc.getBoundingClientRect();
      PopupMenu.open(
        calcMenuItems(ctx),
        { x: r.left, y: r.bottom + 4 },
        { minWidth: 320 },
      );
    });
    this.el = h(
      'div',
      { class: 'cmdbar', role: 'status', 'aria-live': 'polite', hidden: true },
      h('div', { class: 'cmdbar__main' }, this.tool, this.step, this.opts, this.calc, this.snap),
      h(
        'span',
        { class: 'cmdbar__mouse', 'aria-hidden': 'true' },
        h('kbd', { class: 'cmdbar__btn' }, 'Sağ tık'),
        'onayla',
        h('kbd', { class: 'cmdbar__btn' }, 'Basılı sağ tık'),
        'menü',
        h('kbd', { class: 'cmdbar__btn' }, 'Esc'),
        'çık',
      ),
    );
    host.append(this.el);
    // The manager publishes a new tool's prompt before its id; follow both.
    this.d.add(watchAll([ctx.tools.prompt, ctx.tools.activeId], () => this.render()));
    this.render();
    this.d.add(ctx.view.snapOverride.subscribe((k) => this.renderSnap(k), true));
  }

  /** Badge for a one-shot snap picked from the right-button menu; × drops it. */
  private renderSnap(kind: SnapKind | null): void {
    this.snap.hidden = !kind;
    if (!kind) return;
    const clear = h('button', { class: 'cmdbar__snap-clear', type: 'button', 'aria-label': 'Tek seferlik keneti kaldır' }, icon('close', 12));
    clear.addEventListener('pointerdown', (e) => e.preventDefault());
    clear.addEventListener('click', () => this.ctx.view.snapOverride.set(null));
    replaceChildren(this.snap, icon('snap', 14), `Sonraki tık: ${SNAP_LABEL[kind]}`, clear);
  }

  private render(): void {
    const p = parsePrompt(this.ctx.tools.prompt.value);
    // The idle select tool says just "Komut"; nothing to explain then.
    this.el.hidden = !p.tool;
    if (!p.tool) return;
    const d = this.ctx.tools.activeDescriptor;
    replaceChildren(this.tool, d ? icon(d.icon, 16) : null, h('b', null, p.tool));
    replaceChildren(this.step, p.step, ...p.notes.map((n) => h('span', { class: 'cmdbar__note' }, n)));
    replaceChildren(this.opts, ...optionButtons(this.ctx, p.options, 'cmdbar__opt'));
    this.calc.hidden = !canCalcPoint(this.ctx);
  }
}
