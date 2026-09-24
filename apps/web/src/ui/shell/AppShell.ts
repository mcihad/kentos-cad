import type { AppContext } from '../../app/context';
import type { ProcessingTab, ShellKind } from '../../app/state';
import { watchAll } from '../../core/signal';
import { BottomPanel } from '../bottom/BottomPanel';
import { Component } from '../Component';
import { RightDock } from '../dock/RightDock';
import { h } from '../dom';
import { MenuBar } from '../menu/MenuBar';
import type { Ribbon } from '../ribbon/Ribbon';
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
 *   menubar + toolbar  (classic)  |  ribbon  (prefs.shell = 'ribbon')
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
  /** The top of the workbench: menu bar and toolbar, or the ribbon. */
  private readonly chrome: HTMLElement;
  private chromeParts: Component[] = [];
  private ribbon: Ribbon | null = null;
  /** Bumped on every switch, so a ribbon that finishes loading after the user switched back is dropped. */
  private chromeGen = 0;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const { ui } = ctx;
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

    this.chrome = h('div', { class: 'shell__chrome' });
    this.el = h(
      'div',
      { class: 'shell' },
      this.chrome,
      h('div', { class: 'shell__body' }, left, h('main', { class: 'shell__center' }, this.viewportHost, this.bottom.el), right),
      status.el,
    );
    this.d.add(ctx.prefs.shell.subscribe((kind) => this.mountChrome(kind), true));

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

  /** Komut ara: the ribbon's search field, or the command line (which also suggests commands) in the classic shell. */
  searchCommands(): void {
    if (this.ribbon) this.ribbon.focusSearch();
    else this.bottom.commandLine.focus();
  }

  /**
   * Puts the chosen chrome on top, live. The ribbon is a chunk of its own
   * (CLAUDE.md §20): it loads when first chosen, a placeholder of its height
   * holds the place meanwhile, and a failed load falls back to the classic
   * shell and says so.
   */
  private mountChrome(kind: ShellKind): void {
    const gen = ++this.chromeGen;
    this.chromeParts.forEach((p) => p.dispose());
    this.chromeParts = [];
    this.ribbon = null;
    this.el.dataset.shell = kind;
    if (kind === 'classic') {
      const menubar = new MenuBar(this.ctx);
      const toolbar = new Toolbar(this.ctx);
      this.chromeParts = [menubar, toolbar];
      this.chrome.replaceChildren(menubar.el, toolbar.el);
      return;
    }
    this.chrome.replaceChildren(h('div', { class: 'ribbon-placeholder', 'aria-hidden': 'true' }));
    import('../ribbon/Ribbon').then(
      ({ Ribbon }) => {
        if (gen !== this.chromeGen) return;
        const ribbon = new Ribbon(this.ctx);
        this.ribbon = ribbon;
        this.chromeParts = [ribbon];
        this.chrome.replaceChildren(ribbon.el);
      },
      (e: Error) => {
        if (gen !== this.chromeGen) return;
        this.ctx.log.error(`Şerit yüklenemedi (${e.message}). Klasik arayüz açıldı; ağ bağlantısını denetleyip Görünüm → Şerit arayüzü ile yeniden deneyin.`);
        this.ctx.prefs.shell.set('classic');
      },
    );
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
    this.chromeGen++;
    this.chromeParts.forEach((p) => p.dispose());
    this.parts.forEach((p) => p.dispose());
    super.dispose();
  }
}
