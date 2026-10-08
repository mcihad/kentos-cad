import type { AppContext } from '../../app/context';
import { DisposableStore } from '../../core/disposable';
import { Signal, watchAll } from '../../core/signal';
import { Panel } from '../dock/Panel';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { hideTooltip } from '../widgets/tooltip';
import type { SheetHost } from './host';
import { itemTab } from './inspector/itemTab';
import { pageTab } from './inspector/pageTab';
import type { SectionCtx } from './inspector/parts';
import { preflightTab } from './inspector/preflightTab';

/**
 * Denetçi (docs/sheet/design.md §11): the chosen items' properties on the
 * Öğe tab (inspector/itemTab.ts), the sheet's on the Sayfa tab
 * (inspector/pageTab.ts), and what the engine's preflight finds on the Ön
 * denetim tab (inspector/preflightTab.ts), its count on the tab. One item
 * shows its values; several show what they share and “—” where they
 * differ, and a value given goes to all. Drawn again when the choice, the
 * book or the mode changes; the focused field keeps the focus; a section's
 * folding lasts while the window is open.
 */

export type InspectorTab = 'item' | 'page' | 'preflight';

export class SheetInspector extends Panel {
  readonly tab = new Signal<InspectorTab>('item');
  private readonly ctx: AppContext;
  private readonly host: SheetHost;
  private readonly content = new DisposableStore();
  private readonly folded = new Set<string>();
  private readonly tabs: HTMLElement;
  private readonly pfCount: HTMLElement;
  private scheduled = false;

  constructor(ctx: AppContext, host: SheetHost) {
    super({ title: 'Denetçi', className: 'sheet-insp' });
    this.ctx = ctx;
    this.host = host;
    this.pfCount = h('span', { class: 'sheet-insp__count' });
    const tabButton = (id: InspectorTab, label: string, extra?: Child) => {
      const b = h('button', { class: 'tab', type: 'button', role: 'tab', 'aria-selected': 'false', dataset: { tab: id } }, label, extra ?? null);
      b.addEventListener('click', () => this.tab.set(id));
      return b;
    };
    this.tabs = h('div', { class: 'panel__tabs sheet-insp__tabs', role: 'tablist', 'aria-label': 'Denetçi' }, tabButton('item', 'Öğe'), tabButton('page', 'Sayfa'), tabButton('preflight', 'Ön denetim', this.pfCount));
    this.setTabs(this.tabs);
    // The panel shows its tabs in place of its title, so its fold button keeps the title as its name.
    this.el.querySelector('.panel__toggle')?.setAttribute('aria-label', 'Denetçi');
    // One stop for the tabs: the arrows, Home and End move among them (the selected one takes Tab).
    this.tabs.addEventListener('keydown', (e) => {
      const tabs = [...this.tabs.querySelectorAll<HTMLElement>('[role="tab"]')];
      const at = tabs.indexOf(document.activeElement as HTMLElement);
      const to = e.key === 'ArrowRight' ? at + 1 : e.key === 'ArrowLeft' ? at - 1 : e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1 : null;
      if (at < 0 || to === null) return;
      e.preventDefault();
      const next = tabs[(to + tabs.length) % tabs.length];
      this.tab.set(next.dataset.tab as InspectorTab);
      next.focus();
    });
    const { state } = host;
    this.d.add(watchAll([state.selection, state.book, state.open, state.engine, host.profile, host.findings, this.tab, ctx.doc.settings.workspace], () => this.schedule()));
    this.d.add(() => this.content.dispose());
    this.render();
  }

  private schedule(): void {
    if (this.scheduled) return;
    this.scheduled = true;
    queueMicrotask(() => {
      this.scheduled = false;
      this.render();
    });
  }

  private render(): void {
    const focused = (document.activeElement as HTMLElement | null)?.dataset?.key;
    // A tooltip of a field about to go goes with it (its field leaves the page without a pointer leaving it).
    hideTooltip(this.body);
    this.content.dispose();
    for (const b of this.tabs.querySelectorAll<HTMLElement>('[role="tab"]')) {
      b.setAttribute('aria-selected', String(b.dataset.tab === this.tab.value));
      b.tabIndex = b.dataset.tab === this.tab.value ? 0 : -1;
    }
    const findings = this.host.findings.value;
    const errors = findings.filter((f) => f.severity === 'error').length;
    this.pfCount.textContent = findings.length ? String(findings.length) : '';
    this.tabs.querySelector('[data-tab="preflight"]')?.setAttribute('aria-label', findings.length ? `Ön denetim, ${findings.length} bulgu` : 'Ön denetim');
    this.pfCount.dataset.level = errors ? 'error' : findings.some((f) => f.severity === 'warning') ? 'warning' : 'info';
    const sheet = this.host.state.sheet;
    const items = this.host.state.chosen;
    this.setMeta(this.tab.value === 'item' && items.length > 1 ? `${items.length} öğe` : '');
    let body: Child = null;
    if (sheet) {
      const section = (id: string, title: string, b: Child[]) => this.section(id, title, b);
      if (this.tab.value === 'page') body = pageTab(this.ctx, this.host, sheet, this.content, section);
      else if (this.tab.value === 'preflight') body = preflightTab(this.ctx, this.host, sheet.id, findings, this.content);
      else if (!items.length) body = this.nothingChosen();
      else {
        const why = this.host.whyReadOnly() ?? (items.some((i) => i.locked) ? 'Öğe kilitli: Öğeler listesinde kilidini açın.' : null);
        const c: SectionCtx = { ctx: this.ctx, host: this.host, items, sheet: sheet.source, readOnly: why, d: this.content };
        body = itemTab(c, {
          section,
          choosePicture: () => this.choosePicture(),
          viewCentre: () => this.host.mapPlace().center,
          viewSize: () => this.host.mapPlace().view,
          plotScale: () => this.host.mapPlace().scale,
          layers: () => this.ctx.doc.layers.leaves().filter((l) => l.type === 'layer').map((l) => ({ id: l.id, name: l.name })),
        });
      }
    }
    replaceChildren(this.body, body);
    if (focused) this.body.querySelector<HTMLElement>(`[data-key="${CSS.escape(focused)}"]`)?.focus();
  }

  private nothingChosen(): HTMLElement {
    return h(
      'div',
      { class: 'empty' },
      h('p', { class: 'empty__title' }, 'Seçili öğe yok'),
      h('p', { class: 'empty__text' }, 'Paftada bir öğeye tıklayın ya da Öğeler listesinden seçin. Shift ile ekler, Ctrl ile çıkarırsınız; birden çok öğede ortak değerler birlikte düzenlenir.'),
    );
  }

  /** A picture file for the chosen picture frame: its bytes kept on this device, the frame given it (one step). */
  private choosePicture(): void {
    const input = h('input', { type: 'file', accept: 'image/png,image/jpeg,image/svg+xml,.svg' });
    input.addEventListener('change', () => {
      const file = input.files?.[0];
      const ids = this.host.state.chosen.filter((i) => i.kind === 'picture').map((i) => i.id);
      if (!file || !ids.length) return;
      void this.host.addPicture(file).then((meta) => {
        if (!meta) return;
        const has = this.host.book()?.book.assets.some((a) => a.sha256 === meta.sha256);
        this.host.apply([...(has ? [] : [{ op: 'addAssets' as const, assets: [meta] }]), ...ids.map((id) => ({ op: 'setItemProps' as const, id, patch: { kind: { asset: meta.sha256 } } }))], `Resim: ${meta.name}`);
      });
    });
    input.click();
  }

  /** A section that folds (its state kept while the window is open). */
  private section(id: string, title: string, body: Child[]): HTMLElement {
    const folded = this.folded.has(id);
    const head = h('button', { class: 'sheet-insp__head', type: 'button', 'aria-expanded': String(!folded) }, icon(folded ? 'chevronRight' : 'chevronDown', 14), h('span', null, title));
    head.addEventListener('click', () => {
      if (this.folded.has(id)) this.folded.delete(id);
      else this.folded.add(id);
      this.schedule();
    });
    return h('section', { class: 'sheet-insp__section', dataset: { section: id } }, head, folded ? null : h('div', { class: 'sheet-insp__body' }, body));
  }
}
