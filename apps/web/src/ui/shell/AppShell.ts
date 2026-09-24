import type { AppContext } from '../../app/context';
import type { ProcessingTab } from '../../app/state';
import { watchAll } from '../../core/signal';
import { BottomPanel } from '../bottom/BottomPanel';
import { Component } from '../Component';
import { RightDock } from '../dock/RightDock';
import { h } from '../dom';
import { MenuBar } from '../menu/MenuBar';
import { StatusBar } from '../statusbar/StatusBar';
import { Toolbar } from '../toolbar/Toolbar';
import { Toolbox } from '../toolbox/Toolbox';
import { CommandBar } from './CommandBar';
import { CursorInput } from './CursorInput';
import { HoverCard } from './HoverCard';
import { InlineTextEditor } from './InlineTextEditor';
import { bindViewportMenus } from './viewportMenus';
import { splitter } from '../widgets/Splitter';

/**
 * Workbench layout. Regions are slots; each is filled by an independent
 * component that only knows AppContext.
 *
 *   menubar
 *   toolbar
 *   [dock-left] [viewport + floating toolbox] | [right dock]
 *               [bottom: panel + command line] |
 *   status bar
 */
export class AppShell extends Component {
  readonly el: HTMLElement;
  readonly viewportHost: HTMLElement;
  readonly bottom: BottomPanel;
  private readonly ctx: AppContext;
  private readonly dock: RightDock;
  private readonly parts: Component[] = [];

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const { ui } = ctx;
    const menubar = this.own(new MenuBar(ctx));
    const toolbar = this.own(new Toolbar(ctx));
    const dock = (this.dock = this.own(new RightDock(ctx)));
    this.bottom = this.own(new BottomPanel(ctx));
    const status = this.own(new StatusBar(ctx));

    this.viewportHost = h('div', { class: 'viewport' });
    const left = h('div', { class: 'shell__left', 'data-empty': '' });

    let startW = 0;
    const split = splitter({
      orientation: 'vertical',
      label: 'Sağ panel genişliği',
      onStart: () => (startW = ui.dockWidth.value),
      onDrag: (dx) => ui.dockWidth.set(Math.round(Math.min(Math.max(startW - dx, 240), Math.min(560, innerWidth * 0.5)))),
      onReset: () => ui.dockWidth.set(312),
    });
    this.d.add(split.dispose);
    const right = h('div', { class: 'shell__right' }, split.el, dock.el);

    this.el = h(
      'div',
      { class: 'shell' },
      menubar.el,
      toolbar.el,
      h('div', { class: 'shell__body' }, left, h('main', { class: 'shell__center' }, this.viewportHost, this.bottom.el), right),
      status.el,
    );

    this.own(new Toolbox(ctx, { float: this.viewportHost, dock: left }));
    this.own(new InlineTextEditor(ctx, this.viewportHost));
    this.own(new CommandBar(ctx, this.viewportHost));
    this.own(new HoverCard(ctx, this.viewportHost));
    this.bottom.commandLine.direct = this.own(new CursorInput(ctx, this.viewportHost));

    this.d.add(ui.dockWidth.subscribe((w) => this.el.style.setProperty('--dock-w', `${w}px`), true));
    this.d.add(watchAll([ui.rightVisible], () => right.toggleAttribute('hidden', !ui.rightVisible.value)));
    right.toggleAttribute('hidden', !ui.rightVisible.value);

    // Right-button menus over the drawing (idle, command, snap, grips).
    this.d.add(bindViewportMenus(ctx));
  }

  /** Brings the processing toolbox (or its history) forward in the right dock. */
  showProcessing(tab: ProcessingTab): void {
    const { ui } = this.ctx;
    ui.rightVisible.set(true);
    ui.dockTab.set('processing');
    ui.processingTab.set(tab);
    if (tab === 'tools') queueMicrotask(() => this.dock.processing.focusSearch());
  }

  private own<T extends Component>(c: T): T {
    this.parts.push(c);
    return c;
  }

  override dispose(): void {
    this.parts.forEach((p) => p.dispose());
    super.dispose();
  }
}
