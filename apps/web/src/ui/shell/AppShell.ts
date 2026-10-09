import type { AppContext } from '../../app/context';
import { DOCK_WIDTH, dockWidthOn } from '../../app/layoutPlan';
import type { ProcessingTab } from '../../app/state';
import { listen, type Disposable } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { BottomPanel } from '../bottom/BottomPanel';
import { Component } from '../Component';
import { RightDock } from '../dock/RightDock';
import { h } from '../dom';
import { Ribbon, type RibbonExtension } from '../ribbon/Ribbon';
import { StatusBar } from '../statusbar/StatusBar';
import { CommandBar } from './CommandBar';
import { CursorInput } from './CursorInput';
import { HoverCard } from './HoverCard';
import { SelectionChip } from './SelectionChip';
import { InlineTextEditor } from './InlineTextEditor';
import { ParagraphEditor } from './ParagraphEditor';
import { bindViewportMenus } from './viewportMenus';
import { splitter } from '../widgets/Splitter';
import { TimeBar } from '../time/TimeBar';

/**
 * Workbench layout. Regions are slots; each is filled by an independent
 * component that only knows AppContext. The ribbon is the only chrome, as on
 * the desktop (docs/adr/0155).
 *
 *   ribbon
 *   [viewport]                     | [right dock]
 *   [time slider, when open]       |
 *   [sheet tabs: Model | Pafta …]  |
 *   [bottom: panel + command line] |
 *   status bar
 */
export class AppShell extends Component {
  readonly el: HTMLElement;
  readonly viewportHost: HTMLElement;
  /** The row under the drawing area the sheet layouts' Model | Pafta tabs go in (app/sheet/install.ts). */
  readonly sheetTabs: HTMLElement;
  readonly bottom: BottomPanel;
  readonly status: StatusBar;
  private readonly ctx: AppContext;
  private readonly dock: RightDock;
  private readonly parts: Component[] = [];
  private readonly ribbon: Ribbon;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const { ui } = ctx;
    const dock = (this.dock = this.own(new RightDock(ctx)));
    this.bottom = this.own(new BottomPanel(ctx));
    const status = (this.status = this.own(new StatusBar(ctx)));

    this.viewportHost = h('div', { class: 'viewport' });
    this.sheetTabs = h('div', { class: 'shell__sheet-tabs' });

    // A drag starts from the width shown (the kept one may be wider than this window allows).
    let startW = 0;
    const split = splitter({
      orientation: 'vertical',
      label: 'Sağ panel genişliği',
      onStart: () => (startW = dockWidthOn(ui.dockWidth.value, innerWidth)),
      onDrag: (dx) => ui.dockWidth.set(dockWidthOn(startW - dx, innerWidth)),
      onReset: () => ui.dockWidth.set(DOCK_WIDTH.reset),
    });
    this.d.add(split.dispose);
    const right = h('div', { class: 'shell__right' }, split.el, dock.el);

    this.ribbon = this.own(new Ribbon(ctx));
    // Zaman sürgüsü's bar, under the drawing while the slider is open (docs/adr/0210 §10).
    const timeBar = this.own(new TimeBar(ctx));
    this.el = h(
      'div',
      { class: 'shell' },
      h('div', { class: 'shell__chrome' }, this.ribbon.el),
      h('div', { class: 'shell__body' }, h('main', { class: 'shell__center' }, this.viewportHost, timeBar.el, this.sheetTabs, this.bottom.el), right),
      status.el,
    );

    this.own(new InlineTextEditor(ctx, this.viewportHost));
    this.own(new ParagraphEditor(ctx, this.viewportHost));
    this.own(new CommandBar(ctx, this.viewportHost));
    this.own(new HoverCard(ctx, this.viewportHost));
    this.own(new SelectionChip(ctx, this.viewportHost));
    this.bottom.commandLine.direct = this.own(new CursorInput(ctx, this.viewportHost));

    // The kept width, within what the window allows now (app/layoutPlan.ts); a narrower window does not change what is kept.
    const dockWidth = () => this.el.style.setProperty('--dock-w', `${dockWidthOn(ui.dockWidth.value, innerWidth)}px`);
    this.d.add(ui.dockWidth.subscribe(dockWidth, true));
    this.d.add(listen(window, 'resize', dockWidth));
    this.d.add(watchAll([ui.rightVisible], () => right.toggleAttribute('hidden', !ui.rightVisible.value)));
    right.toggleAttribute('hidden', !ui.rightVisible.value);

    // Right-button menus over the drawing (idle, command, snap, grips).
    this.d.add(bindViewportMenus(ctx));
  }

  /** Klavye ipuçları: the ribbon's letters. */
  keyTips(): void {
    this.ribbon.showKeyTips();
  }

  /** Komut ara: the ribbon's search field. */
  searchCommands(): void {
    this.ribbon.focusSearch();
  }

  /** Adds a contextual tab to the ribbon (the sheet layouts' Pafta tab). */
  extendRibbon(ext: RibbonExtension): Disposable {
    return this.ribbon.extend(ext);
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
