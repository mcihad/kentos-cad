import '../../styles/ribbon.css';
import type { AppContext } from '../../app/context';
import { commandItem, menuById, resolveMenu } from '../../app/menus';
import { panelCommands, quickAccessOf, ribbonTabs, startTab, type RibbonTab } from '../../app/ribbon';
import { filterOf } from '../../app/workspaces';
import { fullscreenButton } from '../shell/fullscreenButton';
import { DisposableStore, listen, type Disposable } from '../../core/disposable';
import type { ReadonlySignal } from '../../core/signal';
import { Component } from '../Component';
import { h, overlayRoot } from '../dom';
import { icon } from '../icons';
import { brandButton } from '../shell/brandButton';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { commandControl, type ControlHost } from './controls';
import { KeyTips } from './keytips';
import { LEVELS, PanelView, type Level, type PanelHost } from './panels';
import { commandMenu, quickAccessMenu, RIBBON_TEXTS, ribbonMenu, withQuickAccess, type RibbonRow } from './ribbonPlan';
import { RibbonSearch } from './search';
import { crsTitle } from '../../geo/crs';

/**
 * A contextual tab another part of the app owns (the sheet layouts' Pafta
 * tab, docs/sheet/integration.md §3): its tabs are built by their owner and
 * asked for again when `version` changes; they show while `shown` is true,
 * open when they appear and give way to the last regular tab when they go.
 */
export interface RibbonExtension {
  tabs(): readonly RibbonTab[];
  readonly version: ReadonlySignal<unknown>;
  readonly shown: ReadonlySignal<boolean>;
  /** The tabs' tooltip. */
  readonly tip: { readonly title: string; readonly description: string };
}

/** One tab's panels, built the first time the tab opens and kept until the ribbon closes. */
class TabView {
  readonly el: HTMLElement;
  readonly panels: PanelView[];
  readonly d = new DisposableStore();
  /** Panel widths per level, measured once (they do not depend on the window). */
  widths: number[][] | null = null;

  constructor(ctx: AppContext, tab: RibbonTab, host: PanelHost) {
    this.panels = tab.panels.map((p) => new PanelView(ctx, p, this.d, host));
    this.el = h('div', { class: 'ribbon__panels', role: 'tabpanel', id: `ribbon-tab-${tab.id}`, 'aria-label': tab.label }, ...this.panels.map((p) => p.el));
  }

  /** State that hangs on nested signals (cloud conflicts …) is refreshed whenever the tab comes into view. */
  sync(): void {
    for (const p of this.panels) for (const s of p.syncs) s();
  }

  dispose(): void {
    this.d.dispose();
    this.el.remove();
  }
}

/**
 * The ribbon (Şerit): tabs of panels, the web's only chrome as the desktop's
 * (docs/adr/0155). Its content is derived (app/ribbon.ts) from the menu model
 * and the tool catalog, so a new tool or command appears in it by itself.
 *
 * It adapts: panels shrink step by step to the window (large buttons, then
 * small, then icons, then one button per panel); a contextual “Seçim” tab
 * appears with a selection; tabs holding the running tool carry a dot; the
 * processing tab follows the registry and the model library; the ribbon
 * folds to its tab row (Ctrl+F1, double-click a tab) and then opens over
 * the drawing on demand.
 */
export class Ribbon extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly bar: HTMLElement;
  private readonly tabList: HTMLElement;
  private readonly strip: HTMLElement;
  private readonly qat: HTMLElement;
  private readonly qatD = new DisposableStore();
  private readonly search: RibbonSearch;
  private readonly host: PanelHost;
  private readonly tabButtons = new Map<string, HTMLButtonElement>();
  private readonly views = new Map<string, TabView>();
  private tabs: RibbonTab[] = [];
  /** Commands of each tab, for the running-tool dot and the search's "where". */
  private tabCommands = new Map<string, Set<string>>();
  private current = '';
  private lastRegular = 'home';
  private peeking = false;
  private pop: { panel: PanelView; el: HTMLElement; d: DisposableStore } | null = null;
  private fitFrame = 0;
  /** Listeners of the tab buttons, rebuilt with the work mode. */
  private tabsD = new DisposableStore();
  /** Contextual tabs other parts of the app add (`extend`), and which of them owns each tab. */
  private readonly extensions: RibbonExtension[] = [];
  private extensionOf = new Map<string, RibbonExtension>();
  private readonly keyTips: KeyTips;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const { ui } = ctx;
    this.host = {
      afterRun: () => this.afterRun(),
      commandMenu: (id, at) => this.commandMenu(id, at),
      openCollapsed: (panel) => this.openCollapsed(panel),
      showTab: (id) => this.select(id, { focus: false }),
    };

    this.qat = h('div', { class: 'ribbon__qat', role: 'toolbar', 'aria-label': RIBBON_TEXTS.quickAccess });
    this.tabList = h('div', { class: 'ribbon__tabs', role: 'tablist', 'aria-label': 'Şerit sekmeleri' });
    this.search = new RibbonSearch(ctx, this.d, {
      where: (id) => this.where(id),
      reveal: (id) => this.reveal(id),
      afterRun: () => this.afterRun(),
    });
    const docName = h('span', { class: 'ribbon__doc-name' });
    const dirty = h('span', { class: 'ribbon__dirty', title: 'Kaydedilmemiş değişiklikler var' });
    const crs = h('button', { class: 'ribbon__crs', type: 'button' }, icon('crs', 14), h('span', { class: 'ribbon__crs-name' }));
    const help = h('button', { class: 'ribbon__icon', type: 'button', 'aria-label': 'Yardım', 'aria-haspopup': 'menu' }, icon('help', 16));
    const fold = h('button', { class: 'ribbon__icon', type: 'button', 'aria-label': RIBBON_TEXTS.fold }, icon('chevronUp', 16));
    this.bar = h(
      'div',
      { class: 'ribbon__bar' },
      brandButton(ctx, this.d, 'ribbon__brand'),
      this.qat,
      this.tabList,
      h('div', { class: 'ribbon__doc' }, dirty, docName),
      this.search.el,
      crs,
      fullscreenButton(ctx, this.d, 'ribbon__icon'),
      help,
      fold,
    );
    this.strip = h('div', { class: 'ribbon__strip' });
    this.el = h('header', { class: 'ribbon', 'aria-label': 'Şerit' }, this.bar, this.strip);

    this.derive();
    this.renderTabs();
    this.renderQat();
    this.d.add(ui.ribbonQuickAccess.subscribe(() => this.renderQat()));
    this.d.add(() => this.qatD.dispose());
    this.select(startTab(ui.ribbonTab.value, this.tabs), { focus: false });

    // Document name, unsaved dot and the project's coordinate system.
    this.d.add(ctx.doc.name.subscribe((n) => (docName.textContent = n), true));
    this.d.add(ctx.doc.dirty.subscribe((v) => dirty.toggleAttribute('hidden', !v), true));
    this.d.add(ctx.doc.crs.subscribe((c) => (crs.querySelector('.ribbon__crs-name')!.textContent = c.name), true));
    this.d.add(listen(crs, 'click', () => ctx.commands.execute('crs.set')));
    this.d.add(tooltip(crs, () => ({ title: 'Koordinat sistemi', description: `${crsTitle(ctx.doc.crs.value)}. Değiştirmek için tıklayın.` })));
    this.d.add(
      listen<PointerEvent>(help, 'pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        help.setAttribute('aria-expanded', 'true');
        PopupMenu.open(resolveMenu(ctx, menuById('help')?.items ?? []), help.getBoundingClientRect(), {
          owner: help,
          minWidth: 220,
          onClose: () => help.setAttribute('aria-expanded', 'false'),
        });
      }),
    );
    this.d.add(tooltip(help, () => ({ title: 'Yardım', description: 'Klavye kısayolları ve KentOS CAD hakkında.' })));
    this.d.add(listen(fold, 'click', () => ctx.commands.execute('view.ribbonCollapse')));
    this.d.add(tooltip(fold, () => ({ title: ui.ribbonCollapsed.value ? RIBBON_TEXTS.pin : RIBBON_TEXTS.fold, shortcut: 'Ctrl+F1', description: RIBBON_TEXTS.foldTip })));

    // Folded to the tab row, or open.
    this.d.add(
      ui.ribbonCollapsed.subscribe((c) => {
        this.el.toggleAttribute('data-collapsed', c);
        fold.replaceChildren(icon(c ? 'chevronDown' : 'chevronUp', 16));
        fold.setAttribute('aria-label', c ? RIBBON_TEXTS.pin : RIBBON_TEXTS.fold);
        this.closePeek();
        this.scheduleFit();
      }, true),
    );

    // Dynamic parts: the contextual selection tab, the running-tool dots, the processing tab.
    this.d.add(ctx.selection.ids.subscribe(() => this.updateContextual(), true));
    this.d.add(ctx.tools.activeId.subscribe(() => this.updateDots(), true));
    this.d.add(ctx.processing.registry.version.subscribe(() => this.rebuild('processing')));
    this.d.add(ctx.processing.models.subscribe(() => this.rebuild('processing')));
    // The work mode changes which tabs, panels and buttons exist: everything is built again.
    this.d.add(ctx.doc.settings.workspace.subscribe(() => this.rebuildAll()));

    // Width: panels shrink or grow with the window; widths are measured again when the type scale or fonts change.
    const ro = new ResizeObserver(() => this.scheduleFit());
    ro.observe(this.el);
    this.d.add(() => ro.disconnect());
    this.d.add(ctx.prefs.textSize.subscribe(() => requestAnimationFrame(() => this.remeasure())));
    const fonts = document.fonts;
    if (fonts) {
      const loaded = () => this.remeasure();
      fonts.addEventListener('loadingdone', loaded);
      this.d.add(() => fonts.removeEventListener('loadingdone', loaded));
    }

    // Whatever runs a command (a shortcut too) closes the ribbon opened over the drawing and a folded panel's pop-up.
    this.d.add(ctx.commands.events.on('executed', ({ command }) => command.id !== 'view.ribbonCollapse' && this.afterRun()));

    this.keyTips = new KeyTips({
      root: this.el,
      tabs: () => this.visibleTabs().map((el) => ({ id: el.dataset.tab!, label: el.querySelector('.ribbon__tab-label')?.textContent ?? '', el })),
      quickAccess: () => [...this.qat.querySelectorAll<HTMLElement>('button.rbtn:not(:disabled)')],
      openTab: (id) => (this.ctx.ui.ribbonCollapsed.value ? this.openPeek(id) : this.select(id, { focus: false })),
      controls: () => [...this.focusables(), ...(this.pop ? [...this.pop.el.querySelectorAll<HTMLElement>('button:not(:disabled)')] : [])],
    });
    this.d.add(() => this.keyTips.hide());
    // A right click the controls did not take (a tab, a menu button, a panel's title, empty space) offers the fold,
    // never the browser's own menu; a text field keeps its menu.
    this.d.add(
      listen<MouseEvent>(this.el, 'contextmenu', (e) => {
        if (e.defaultPrevented || (e.target as Element).closest('input, textarea, [contenteditable]')) return;
        e.preventDefault();
        PopupMenu.open(this.rows(ribbonMenu()), { x: e.clientX, y: e.clientY });
      }),
    );
    this.bindKeys();
    this.bindAltTap();
    this.bindPeek();
    this.d.add(() => {
      this.tabsD.dispose();
      cancelAnimationFrame(this.fitFrame);
      this.closePop();
      this.views.forEach((v) => v.dispose());
    });
  }

  /** Klavye ipuçları (F6, or Alt tapped alone): letters over the tabs, then over the open tab's controls. */
  showKeyTips(): void {
    if (this.keyTips.shown) this.keyTips.hide();
    else this.keyTips.show();
  }

  /** Alt+Q: the search field (the ribbon opens over the drawing first when folded). */
  focusSearch(): void {
    this.search.focus();
  }

  /** Adds a contextual tab another part of the app owns; the returned function takes it away. */
  extend(ext: RibbonExtension): Disposable {
    this.extensions.push(ext);
    const subs = [
      ext.version.subscribe(() => ext.tabs().forEach((t) => this.rebuild(t.id))),
      ext.shown.subscribe(() => this.updateExtension(ext, true)),
    ];
    this.rebuildAll();
    return () => {
      subs.forEach((s) => s());
      this.extensions.splice(this.extensions.indexOf(ext), 1);
      this.rebuildAll();
    };
  }

  // ── Model ─────────────────────────────────────────────────────────────

  private derive(): void {
    const { ctx } = this;
    const added = this.extensions.map((e) => ({ e, tabs: e.tabs() }));
    this.extensionOf = new Map(added.flatMap(({ e, tabs }) => tabs.map((t) => [t.id, e] as const)));
    this.tabs = [
      ...ribbonTabs({
        tools: ctx.tools.list(),
        processing: ctx.processing.registry.tree(),
        models: ctx.processing.models.value,
        iconOf: (id) => ctx.commands.get(id)?.icon,
        filter: filterOf(ctx),
      }),
      ...added.flatMap((a) => a.tabs),
    ];
    this.tabCommands = new Map(this.tabs.map((t) => [t.id, new Set(t.panels.flatMap(panelCommands))]));
  }

  /** "Tab › Panel" of a command: its own tab rather than Giriş's picks, never the contextual tab. */
  private where(id: string): string | undefined {
    const order = [...this.tabs.filter((t) => !t.contextual && t.id !== 'home'), ...this.tabs.filter((t) => t.id === 'home')];
    for (const t of order) {
      const p = t.panels.find((p) => panelCommands(p).includes(id));
      if (p) return `${t.label} › ${p.label}`;
    }
    return undefined;
  }

  /** The processing tab follows the registry and the model library. */
  private rebuild(tabId: string): void {
    this.derive();
    this.views.get(tabId)?.dispose();
    this.views.delete(tabId);
    if (this.current === tabId) this.select(tabId, { focus: false });
    this.updateDots();
  }

  /** A new work mode: tabs and their views are built again; the open tab stays when it still exists. */
  private rebuildAll(): void {
    const was = this.current;
    this.closePeek();
    this.closePop();
    this.derive();
    this.views.forEach((v) => v.dispose());
    this.views.clear();
    this.tabsD.dispose();
    this.tabsD = new DisposableStore();
    this.tabButtons.clear();
    this.tabList.replaceChildren();
    this.renderTabs();
    const keep = this.tabs.some((t) => t.id === this.lastRegular && !t.contextual) ? this.lastRegular : 'home';
    this.select(keep, { focus: false });
    this.updateContextual();
    for (const ext of this.extensions) this.updateExtension(ext, false);
    // An added tab that was open stays open (the work mode changed while a sheet was in front).
    if (was !== this.current && this.extensionOf.get(was)?.shown.value) this.select(was, { focus: false });
    this.remeasure();
  }

  // ── Tabs ──────────────────────────────────────────────────────────────

  private renderTabs(): void {
    for (const t of this.tabs) {
      const b = h(
        'button',
        { class: `ribbon__tab${t.contextual ? ' ribbon__tab--context' : ''}`, type: 'button', role: 'tab', 'aria-selected': 'false', tabindex: '-1', 'aria-controls': `ribbon-tab-${t.id}`, dataset: { tab: t.id } },
        h('span', { class: 'ribbon__tab-label' }, t.label),
        t.contextual === 'selection' ? h('span', { class: 'ribbon__count num' }) : null,
      );
      // An added tab waits for its owner to show it.
      if (this.extensionOf.has(t.id)) b.hidden = true;
      this.tabsD.add(
        listen<PointerEvent>(b, 'pointerdown', (e) => {
          if (e.button !== 0) return;
          // A click leaves the keyboard where it was (Enter still repeats the last command).
          e.preventDefault();
          // Folded: a click opens the tab over the drawing; a second click on it closes it.
          if (this.ctx.ui.ribbonCollapsed.value) {
            if (this.peeking && this.current === t.id) this.closePeek();
            else this.openPeek(t.id);
            return;
          }
          this.select(t.id, { focus: false });
        }),
      );
      this.tabsD.add(listen(b, 'dblclick', () => this.ctx.commands.execute('view.ribbonCollapse')));
      this.tabsD.add(
        tooltip(b, () =>
          this.extensionOf.has(t.id)
            ? this.extensionOf.get(t.id)!.tip
            : t.contextual
            ? { title: `${t.label} (bağlamsal)`, description: 'Seçili nesneler varken görünür: seçimin özeti ve seçime uygulanan komutlar.' }
            : b.hasAttribute('data-active-tool')
              ? { title: t.label, description: `Çalışan araç bu sekmede: ${this.ctx.tools.activeDescriptor?.label ?? ''}.` }
              : null,
        ),
      );
      this.tabButtons.set(t.id, b);
      this.tabList.append(b);
    }
  }

  private select(id: string, opts: { focus: boolean }): void {
    const tab = this.tabs.find((t) => t.id === id) ?? this.tabs.find((t) => t.id === 'home')!;
    this.closePop();
    this.current = tab.id;
    if (!tab.contextual) {
      this.lastRegular = tab.id;
      this.ctx.ui.ribbonTab.set(tab.id);
    }
    for (const [tid, b] of this.tabButtons) {
      const on = tid === tab.id;
      b.setAttribute('aria-selected', String(on));
      b.tabIndex = on ? 0 : -1;
    }
    let view = this.views.get(tab.id);
    if (!view) {
      view = new TabView(this.ctx, tab, this.host);
      this.views.set(tab.id, view);
    }
    if (this.strip.firstChild !== view.el) this.strip.replaceChildren(view.el);
    view.sync();
    this.updateDots();
    this.fit();
    if (opts.focus) this.tabButtons.get(tab.id)?.focus();
  }

  private visibleTabs(): HTMLButtonElement[] {
    return [...this.tabButtons.values()].filter((b) => !b.hidden);
  }

  /** The selection tab shows while something is selected; leaving it empty returns to the tab before it. */
  private updateContextual(): void {
    const n = this.ctx.selection.size;
    const b = this.tabButtons.get('selection');
    if (!b) return;
    b.hidden = n === 0;
    b.querySelector('.ribbon__count')!.textContent = n ? String(n) : '';
    if (!n && this.current === 'selection') this.select(this.lastRegular, { focus: false });
    else if (this.current === 'selection') this.views.get('selection')?.sync();
    this.scheduleFit();
  }

  /** An added tab shows while its owner says so; it opens when it appears (`open`) and gives way when it goes. */
  private updateExtension(ext: RibbonExtension, open: boolean): void {
    const shown = ext.shown.value;
    for (const t of this.tabs.filter((x) => this.extensionOf.get(x.id) === ext)) {
      const b = this.tabButtons.get(t.id);
      if (!b) continue;
      const was = !b.hidden;
      b.hidden = !shown;
      if (shown && open && !was) this.select(t.id, { focus: false });
      else if (!shown && this.current === t.id) this.select(this.lastRegular, { focus: false });
    }
    this.scheduleFit();
  }

  /** A dot on the tabs (other than the open one) that hold the running tool. */
  private updateDots(): void {
    const id = this.ctx.tools.activeId.value;
    const cmd = `tool.${id}`;
    for (const t of this.tabs) {
      const has = id !== 'select' && t.id !== this.current && !!this.tabCommands.get(t.id)?.has(cmd);
      this.tabButtons.get(t.id)?.toggleAttribute('data-active-tool', has);
    }
  }

  // ── Fitting ───────────────────────────────────────────────────────────

  private scheduleFit(): void {
    cancelAnimationFrame(this.fitFrame);
    this.fitFrame = requestAnimationFrame(() => this.fit());
  }

  private remeasure(): void {
    this.closePop();
    this.views.forEach((v) => (v.widths = null));
    this.fit();
  }

  /**
   * Chooses each panel's level for the available width: every panel steps
   * down once (rightmost first) before any steps down twice. The tab's kept
   * panels (Giriş: Çizim, Değiştir) keep their large buttons and labels
   * until every other panel shows icons only; only folding into one button
   * comes before they shrink further. A step that would not save width (a
   * lone large button made small) is skipped, and such a panel waits its
   * turn for the level it would reach. The room the last step leaves over
   * goes back, the kept panels first and then left to right: each panel
   * returns to its largest form that still fits, so more buttons keep their
   * names.
   */
  private fit(): void {
    this.fitBar();
    const view = this.views.get(this.current);
    if (!view || !this.el.isConnected || !this.stripShown()) return;
    const style = getComputedStyle(this.strip);
    const available = this.strip.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    if (available <= 0) return;
    const widths = (view.widths ??= view.panels.map((p) =>
      LEVELS.map((l) => {
        p.setLevel(l);
        return p.el.getBoundingClientRect().width;
      }),
    ));
    const levels = view.panels.map((): Level => 0);
    let total = widths.reduce((s, w) => s + w[0], 0);
    const next = (i: number): Level | null => {
      for (const l of LEVELS) if (l > levels[i] && widths[i][l] < widths[i][levels[i]] - 0.5) return l;
      return null;
    };
    const order = view.panels.map((_, i) => i).sort((a, b) => Number(!!view.panels[a].model.keep) - Number(!!view.panels[b].model.keep) || b - a);
    const rank = (i: number, l: Level) => l + (view.panels[i].model.keep && l < 3 ? 1.5 : 0);
    while (total > available) {
      let pick = -1;
      let to: Level | null = null;
      for (const i of order) {
        const l = next(i);
        if (l !== null && (to === null || rank(i, l) < rank(pick, to))) {
          pick = i;
          to = l;
        }
      }
      if (to === null) break;
      total -= widths[pick][levels[pick]] - widths[pick][to];
      levels[pick] = to;
    }
    if (total <= available) {
      const back = view.panels.map((_, i) => i).sort((a, b) => Number(!view.panels[a].model.keep) - Number(!view.panels[b].model.keep) || a - b);
      for (const i of back) {
        // While a kept panel is shrunk the others stay at icons.
        const whole = view.panels.every((p, k) => !p.model.keep || levels[k] === 0);
        const floor = view.panels[i].model.keep || whole ? 0 : 2;
        const here = widths[i][levels[i]];
        const l = LEVELS.find((l) => l >= floor && l < levels[i] && total - here + widths[i][l] <= available);
        if (l === undefined) continue;
        total += widths[i][l] - here;
        levels[i] = l;
      }
    }
    // A folded panel that unfolds takes its controls back from the pop-up first.
    if (this.pop && levels[view.panels.indexOf(this.pop.panel)] !== 3) this.closePop();
    view.panels.forEach((p, i) => p.setLevel(levels[i]));
    this.strip.toggleAttribute('data-overflow', total > available + 0.5);
  }

  /** The tab row gives way in steps: the product name and search label, then the coordinate system's name. */
  private fitBar(): void {
    for (let t = 0; t <= 3; t++) {
      this.el.dataset.tight = String(t);
      if (this.bar.scrollWidth <= this.bar.clientWidth + 1) return;
    }
  }

  private stripShown(): boolean {
    return !this.ctx.ui.ribbonCollapsed.value || this.peeking;
  }

  // ── Folded ribbon: opening over the drawing ──────────────────────────

  private openPeek(id: string): void {
    this.peeking = true;
    this.el.setAttribute('data-peek', '');
    this.select(id, { focus: false });
  }

  private closePeek(): void {
    if (!this.peeking) return;
    this.peeking = false;
    this.el.removeAttribute('data-peek');
    this.closePop();
  }

  private afterRun(): void {
    this.closePop();
    this.closePeek();
  }

  private bindPeek(): void {
    // A press outside the ribbon (and its pop-ups) closes the ribbon opened over the drawing.
    this.d.add(
      listen<PointerEvent>(
        window,
        'pointerdown',
        (e) => {
          const t = e.target as Element;
          if (this.el.contains(t) || t.closest?.('.menu, .ribbon-pop, .rsearch__list')) return;
          this.closePop();
          this.closePeek();
        },
        true,
      ),
    );
    // Esc closes the pop-up panel, then the ribbon opened over the drawing, before it reaches the running tool.
    this.d.add(
      listen<KeyboardEvent>(
        window,
        'keydown',
        (e) => {
          if (e.key !== 'Escape' || PopupMenu.isOpen) return;
          if (this.pop) {
            e.preventDefault();
            const b = this.pop.panel.collapsedButton;
            this.closePop();
            b.focus();
          } else if (this.peeking) {
            e.preventDefault();
            this.closePeek();
            this.tabButtons.get(this.current)?.focus();
          }
        },
        true,
      ),
    );
    this.d.add(listen(window, 'blur', () => this.closePop()));
  }

  // ── A folded panel opened under its button ────────────────────────────

  private openCollapsed(panel: PanelView): void {
    if (this.pop?.panel === panel) return this.closePop();
    this.closePop();
    const d = new DisposableStore();
    const el = h('div', { class: 'ribbon-pop', role: 'group', 'aria-label': panel.model.label }, panel.body);
    overlayRoot().append(el);
    panel.el.dataset.open = '';
    panel.collapsedButton.setAttribute('aria-expanded', 'true');
    const r = panel.collapsedButton.getBoundingClientRect();
    const w = el.getBoundingClientRect().width;
    el.style.transform = `translate(${Math.round(Math.max(8, Math.min(r.left, innerWidth - w - 8)))}px, ${Math.round(r.bottom + 4)}px)`;
    d.add(listen(window, 'resize', () => this.closePop()));
    this.pop = { panel, el, d };
  }

  private closePop(): void {
    const pop = this.pop;
    if (!pop) return;
    this.pop = null;
    pop.d.dispose();
    // The panel's controls go back into it (folded, they are hidden there).
    pop.panel.el.prepend(pop.panel.body);
    delete pop.panel.el.dataset.open;
    pop.panel.collapsedButton.setAttribute('aria-expanded', 'false');
    pop.el.remove();
  }

  // ── Quick access bar ──────────────────────────────────────────────────

  private quickAccess(): string[] {
    return quickAccessOf(this.ctx.ui.ribbonQuickAccess.value, (id) => !!this.ctx.commands.get(id));
  }

  private renderQat(): void {
    this.qatD.dispose();
    const { ctx } = this;
    const host: ControlHost = this.host;
    const buttons = this.quickAccess().map((id) => commandControl(ctx, id, this.qatD, host, 'rbtn--qat rbtn--small rbtn--icon').el);
    const more = h('button', { class: 'ribbon__qat-more', type: 'button', 'aria-label': RIBBON_TEXTS.customize, 'aria-haspopup': 'menu' }, icon('chevronDown', 12));
    this.qatD.add(
      listen<PointerEvent>(more, 'pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        more.setAttribute('aria-expanded', 'true');
        PopupMenu.open(this.rows(quickAccessMenu(this.quickAccess(), (id) => !!this.ctx.commands.get(id))), more.getBoundingClientRect(), {
          owner: more,
          minWidth: 250,
          onClose: () => more.setAttribute('aria-expanded', 'false'),
        });
      }),
    );
    this.qatD.add(tooltip(more, () => ({ title: RIBBON_TEXTS.customize, description: RIBBON_TEXTS.customizeTip })));
    this.qat.replaceChildren(...buttons, more);
    this.scheduleFit();
  }

  /** The plan's rows (ribbonPlan.ts) as menu items: a command's own row, or a quick access row that adds or takes off. */
  private rows(rows: readonly RibbonRow[]): MenuItem[] {
    return rows.map((r): MenuItem => {
      if (r.kind === 'header' || r.kind === 'separator') return r;
      if (r.kind === 'command') return commandItem(this.ctx, r.command);
      const set = r.set;
      return {
        label: r.label ?? this.ctx.commands.get(r.command)?.title ?? r.command,
        icon: r.icon,
        checked: r.checked,
        disabled: r.disabled,
        hint: r.hint,
        run: set === undefined ? undefined : () => this.ctx.ui.ribbonQuickAccess.set(withQuickAccess(this.ctx.ui.ribbonQuickAccess.value, r.command, set)),
      };
    });
  }

  /** Right button on a ribbon command: add it to (or take it off) the quick access bar (ribbonPlan.ts `commandMenu`). */
  private commandMenu(id: string, at: { x: number; y: number }): void {
    PopupMenu.open(this.rows(commandMenu(id, this.quickAccess())), at);
  }

  // ── Search: showing where a command lives ─────────────────────────────

  private reveal(id: string): void {
    const tab =
      this.tabs.find((t) => !t.contextual && t.id !== 'home' && this.tabCommands.get(t.id)?.has(id)) ?? this.tabs.find((t) => t.id === 'home' && this.tabCommands.get(t.id)?.has(id));
    if (!tab) return;
    if (this.ctx.ui.ribbonCollapsed.value) this.openPeek(tab.id);
    else this.select(tab.id, { focus: false });
    const view = this.views.get(tab.id)!;
    const panel = view.panels.find((p) => panelCommands(p.model).includes(id));
    if (panel?.level === 3) this.openCollapsed(panel);
    // A command inside a split button flashes the split; one under the ▾, the ▾.
    const split = panel?.model.items.find((i) => i.kind === 'split' && i.entries.some((e) => e.command === id));
    const selector =
      split?.kind === 'split' ? `[data-split="${CSS.escape(split.key)}"]` : panel?.model.overflow?.includes(id) ? '.rpanel__more' : `[data-command="${CSS.escape(id)}"]`;
    const b = (panel?.el ?? view.el).querySelector<HTMLElement>(selector);
    if (!b) return;
    b.dataset.flash = '';
    b.focus();
    window.setTimeout(() => delete b.dataset.flash, 1600);
  }

  // ── Keyboard ──────────────────────────────────────────────────────────

  private bindKeys(): void {
    // Tabs: ←/→ move (and open, unless folded), Home/End, ↓ or Enter goes into the panels.
    this.d.add(
      listen<KeyboardEvent>(this.tabList, 'keydown', (e) => {
        const tabs = this.visibleTabs();
        const at = tabs.indexOf(document.activeElement as HTMLButtonElement);
        if (at < 0) return;
        let to = -1;
        if (e.key === 'ArrowRight') to = (at + 1) % tabs.length;
        else if (e.key === 'ArrowLeft') to = (at - 1 + tabs.length) % tabs.length;
        else if (e.key === 'Home') to = 0;
        else if (e.key === 'End') to = tabs.length - 1;
        if (to >= 0) {
          e.preventDefault();
          const id = tabs[to].dataset.tab!;
          if (this.ctx.ui.ribbonCollapsed.value && !this.peeking) tabs[to].focus();
          else this.select(id, { focus: true });
          return;
        }
        if (e.key === 'ArrowDown' || ((e.key === 'Enter' || e.key === ' ') && this.ctx.ui.ribbonCollapsed.value)) {
          e.preventDefault();
          const id = tabs[at].dataset.tab!;
          if (this.ctx.ui.ribbonCollapsed.value && !this.peeking) this.openPeek(id);
          else if (id !== this.current) this.select(id, { focus: false });
          this.focusables()[0]?.focus();
        }
      }),
    );
    // Panels: arrows walk the controls in reading order; Esc goes back to the tab.
    this.d.add(
      listen<KeyboardEvent>(this.strip, 'keydown', (e) => {
        if (PopupMenu.isOpen) return;
        const list = this.focusables();
        const at = list.indexOf(document.activeElement as HTMLElement);
        if (at < 0) return;
        const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
        if (step) {
          e.preventDefault();
          list[(at + step + list.length) % list.length].focus();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          this.tabButtons.get(this.current)?.focus();
        }
      }),
    );
  }

  /**
   * Alt pressed and released with nothing between shows the key tips (as in Office). The browser keeps
   * Alt+letter for its own menus, so the letters come after the release; preventing the release's default
   * keeps Chromium from moving the focus to its own menu. Inside a dialog Alt stays the dialog's.
   */
  private bindAltTap(): void {
    let armed = false;
    this.d.add(
      listen<KeyboardEvent>(
        window,
        'keydown',
        (e) => {
          armed = e.key === 'Alt' && !e.ctrlKey && !e.shiftKey && !e.metaKey && !e.repeat;
        },
        true,
      ),
    );
    this.d.add(listen(window, 'pointerdown', () => (armed = false), true));
    this.d.add(
      listen<KeyboardEvent>(
        window,
        'keyup',
        (e) => {
          if (e.key !== 'Alt' || !armed) return;
          armed = false;
          if (document.querySelector('.dialog') || PopupMenu.isOpen) return;
          e.preventDefault();
          this.showKeyTips();
        },
        true,
      ),
    );
  }

  private focusables(): HTMLElement[] {
    return [...this.strip.querySelectorAll<HTMLElement>('button:not(:disabled)')].filter((b) => b.offsetParent !== null);
  }
}
