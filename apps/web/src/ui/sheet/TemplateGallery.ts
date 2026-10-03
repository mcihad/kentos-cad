import type { AppContext } from '../../app/context';
import { effectiveWorkspace } from '../../app/workspaces';
import type { PaperChoice } from '../../contracts/generated/sheet/PaperChoice';
import type { RankedTemplate } from '../../contracts/generated/sheet/RankedTemplate';
import { DisposableStore } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { errorText } from '../../product/sheet/engine';
import { readPaperColors, type PaperColors } from '../../render/sheet/paperPainter';
import { arrangeTemplates, categoryLabel, facetsOf, hiddenByMode, metasOf, SECTIONS, sectionOf, type ArrangedCard, type GallerySection, type TemplateCard } from '../../product/sheet/templates';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { confirmDialog } from '../widgets/confirm';
import { toggleSwitch } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { Dropdown } from '../widgets/Dropdown';
import { hideTooltip, tooltip } from '../widgets/tooltip';
import { badgesOf, detailsOf, paperPick } from './galleryDetails';
import { onGalleryKey } from './galleryKeys';
import { actionsOf, emptyText, paperText, sectionNote, type TemplateAction } from './galleryPlan';
import type { SheetHost } from './host';
import { openPublishDialog } from './PublishTemplateDialog';
import { openShareTemplateDialog } from './ShareTemplateDialog';
import { thumbCanvas } from './thumb';

/**
 * Pafta şablonları (docs/sheet/design.md §11, §11a, §12, §13): the template
 * library in the style manager's family (DESIGN.md §7.14). On the left the
 * sections, Sistem · Benim · Kurumum · Benimle paylaşılanlar, each saying why
 * when it has nothing to give now; in the middle the cards, each the
 * template's own plan drawn small as Kullan would make it (on this
 * project's drawing), with its badges, searched and filtered by paper and
 * kind; on the right the chosen template (galleryDetails.ts). The order is
 * the engine's for the project (`rankTemplates`: its type's templates
 * first, then its mode's, then the common ones); “Bütün türlerin
 * şablonları” shows the others too, badged with their type. Kullan makes a
 * sheet on the paper picked at the bottom; it asks first when the template
 * needs what the project lacks (the engine's preflight of it).
 */

export interface GalleryOptions {
  readonly section?: GallerySection;
  /** A card to choose when the window opens. */
  readonly select?: string;
}

export function openTemplateGallery(ctx: AppContext, host: SheetHost, opts: GalleryOptions = {}): void {
  new TemplateGallery(ctx, host, opts);
}

/** Galleries opened, for their elements' ids. */
let galleries = 0;

class TemplateGallery {
  private readonly ctx: AppContext;
  private readonly host: SheetHost;
  private readonly d = new DisposableStore();
  private readonly cardsD = new DisposableStore();
  private readonly detailsD = new DisposableStore();
  private readonly dialog: Dialog;
  private readonly nav: HTMLElement;
  private readonly grid: HTMLElement;
  private readonly head: HTMLElement;
  private readonly details: HTMLElement;
  private readonly status: HTMLElement;
  private readonly filters: HTMLElement;
  private readonly use: HTMLButtonElement;
  private readonly paperPick: HTMLElement;
  private section: GallerySection;
  private query = '';
  private paper: PaperChoice['paper'] | null = null;
  private kind: string | null = null;
  private allModes = false;
  private selected: string | null;
  private usePaper: PaperChoice | null = null;
  private listed: ArrangedCard[] = [];
  private ranked: { key: string; list: RankedTemplate[] } | null = null;
  private colors: PaperColors | null = null;
  private repaints: (() => void)[] = [];

  constructor(ctx: AppContext, host: SheetHost, opts: GalleryOptions) {
    this.ctx = ctx;
    this.host = host;
    this.section = opts.section ?? 'system';
    this.selected = opts.select ?? null;
    const search = h('input', { class: 'field smgr__search', type: 'search', placeholder: 'Şablon ara: ifraz, imar, rapor…', 'aria-label': 'Şablon ara', spellcheck: 'false' });
    search.addEventListener('input', () => {
      this.query = search.value;
      this.renderCards();
    });
    search.addEventListener('keydown', (e) => {
      if (e.key !== 'ArrowDown') return;
      e.preventDefault();
      this.grid.querySelector<HTMLElement>('.tcard')?.focus();
    });
    this.filters = h('div', { class: 'tgal__filters', style: 'display:contents' });
    const all = toggleSwitch({ label: 'Bütün türlerin şablonları', checked: false, onChange: () => this.toggleAllModes() });
    const allBox = h('label', { class: 'tgal__all' }, all, 'Bütün türlerin şablonları');
    this.d.add(tooltip(allBox, () => ({ title: 'Bütün türlerin şablonları', description: 'Başka proje türlerinin şablonlarını da gösterir; tür rozetiyle. Veri türden bağımsızdır: her şablon kullanılabilir.' })));
    this.nav = h('nav', { class: 'tgal__nav', 'aria-label': 'Şablon kaynakları' });
    // One stop for the sources: the arrows, Home and End move among them and show each.
    this.nav.addEventListener('keydown', (e) => {
      const tabs = [...this.nav.querySelectorAll<HTMLElement>('.tgal__src')];
      const at = tabs.indexOf(document.activeElement as HTMLElement);
      const to = e.key === 'ArrowDown' ? at + 1 : e.key === 'ArrowUp' ? at - 1 : e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1 : null;
      if (at < 0 || to === null) return;
      e.preventDefault();
      this.section = tabs[(to + tabs.length) % tabs.length].dataset.section as GallerySection;
      this.renderAll();
      this.nav.querySelector<HTMLElement>(`.tgal__src[data-section="${this.section}"]`)?.focus();
    });
    this.head = h('div', { class: 'tgal__listhead' });
    this.grid = h('div', { class: 'tgal__grid', role: 'listbox', 'aria-label': 'Şablonlar', id: `tgal-grid-${++galleries}` });
    this.grid.addEventListener('keydown', (e) => onGalleryKey(this.grid, e, { choose: (id) => this.choose(id), use: () => void this.run('use'), remove: () => void this.run('delete') }));
    this.details = h('aside', { class: 'tgal__details' });
    this.status = h('div', { class: 'tgal__status', role: 'status' });
    this.paperPick = h('div', { class: 'tgal__paperpick' });
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    close.addEventListener('click', () => this.dialog.close());
    this.use = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, icon('check', 16), 'Kullan');
    this.use.addEventListener('click', () => void this.run('use'));
    this.d.add(tooltip(this.use, () => ({ title: 'Kullan', description: 'Seçili şablondan yeni bir pafta yapar; seçilen kâğıda kısıtlarıyla yerleşir.', note: this.current() ? (actionsOf(this.current()!, host.galleryAbilities()).use.reason ?? undefined) : 'Önce bir şablon seçin.' }), 'top'));

    this.dialog = new Dialog({
      title: 'Pafta şablonları',
      width: 1240,
      className: 'dialog--styles dialog--tgal tgal',
      content: [
        h('div', { class: 'tgal__bar' }, h('div', { class: 'smgr__searchbox' }, icon('search', 15), search), this.filters, allBox),
        h('div', { class: 'tgal__cols' }, this.nav, h('section', { class: 'tgal__list' }, this.head, this.grid), this.details),
      ],
      footer: [this.status, h('div', { class: 'dialog__foot-spacer' }), this.paperPick, close, this.use],
      onClose: () => {
        this.cardsD.dispose();
        this.detailsD.dispose();
        this.d.dispose();
      },
    });
    for (const p of host.providers) {
      this.d.add(watchAll([p.state, p.cards], () => this.renderAll()));
      void p.refresh();
    }
    this.d.add(host.painted.subscribe(() => this.repaints.forEach((r) => r())));
    // The library's line (synced, offline …) where the window has nothing else to say.
    this.d.add(host.cloudLine.subscribe(() => !this.status.dataset.own && this.say('')));
    this.renderAll();
    queueMicrotask(() => search.focus());
  }

  // ── Data ─────────────────────────────────────────────────────────

  private allCards(): TemplateCard[] {
    return this.host.providers.flatMap((p) => p.cards.value);
  }

  /** The engine's order of every card for the project (kept while the cards and the project stay). */
  private rankedCards(cards: readonly TemplateCard[]): RankedTemplate[] {
    const engine = this.host.engine();
    if (!engine) return [];
    const t = this.host.traits();
    const key = `${t.workspace}|${t.projectType}|${cards.map((c) => `${c.id}@${c.revision}`).join(',')}`;
    if (this.ranked?.key === key) return this.ranked.list;
    try {
      this.ranked = { key, list: engine.rankTemplates(metasOf(cards), t.workspace, t.projectType) };
    } catch (e) {
      this.say(`Şablonlar sıralanamadı: ${errorText(e)}`, 'warn');
      this.ranked = { key, list: cards.map((c) => ({ id: c.id, fit: 'common', matches: true })) };
    }
    return this.ranked.list;
  }

  private stateOf(section: GallerySection) {
    return this.host.providers.find((p) => p.section === section)?.state.value ?? { state: 'unavailable' as const, reason: 'Bu kaynak bu sürümde yok.' };
  }

  private current(): TemplateCard | undefined {
    return this.allCards().find((c) => c.id === this.selected);
  }

  private paperColors(): PaperColors {
    return (this.colors ??= readPaperColors(this.dialog.el.querySelector('.tgal') ?? document.documentElement));
  }

  private paperName = (p: PaperChoice['paper']): string => this.host.engine()?.paperSizes().find((x) => x.id === p)?.name ?? (p === 'custom' ? 'Özel' : p.toUpperCase());

  // ── Drawing ──────────────────────────────────────────────────────

  private renderAll(): void {
    this.renderNav();
    this.renderFilters();
    this.renderCards();
  }

  private renderNav(): void {
    const cards = this.allCards();
    const ranked = this.rankedCards(cards);
    const tabs = h('div', { class: 'tgal__tabs', role: 'tablist', 'aria-label': 'Şablon kaynakları', 'aria-orientation': 'vertical' });
    replaceChildren(
      tabs,
      ...SECTIONS.map((s) => {
        const state = this.stateOf(s.id);
        const shown = arrangeTemplates(cards, ranked, { section: s.id, text: '', paper: null, kind: null, allModes: this.allModes }).length;
        const noteText = sectionNote(state);
        const noteId = `${this.grid.id}-${s.id}`;
        // Its name is the source and its count; its note (why it is empty, when it comes) is its description.
        const b = h(
          'button',
          {
            class: 'tgal__src',
            type: 'button',
            role: 'tab',
            'aria-selected': String(s.id === this.section),
            'aria-label': state.state === 'ready' ? `${s.label}, ${shown} şablon` : s.label,
            'aria-describedby': noteText ? noteId : undefined,
            'aria-controls': this.grid.id,
            tabindex: s.id === this.section ? '0' : '-1',
            dataset: { section: s.id },
          },
          icon(s.icon, 15),
          h('span', null, s.label),
          h('span', { class: 'tgal__src-count' }, state.state === 'ready' ? String(shown) : ''),
          noteText ? h('span', { class: 'tgal__src-note', id: noteId }, noteText) : null,
        );
        b.addEventListener('click', () => {
          this.section = s.id;
          this.renderAll();
        });
        return b;
      }),
    );
    replaceChildren(this.nav, tabs, h('p', { class: 'tgal__navnote' }, this.orderNote()));
  }

  /** Says how the list is ordered for this project (design §11a). */
  private orderNote(): string {
    const mode = effectiveWorkspace(this.host.traits().workspace).label;
    return `Sıra: önce işin türüne, sonra proje türüne (${mode}) uyanlar, sonra ortak şablonlar.${this.allModes ? ' Başka türlerinkiler en sonda, tür rozetiyle.' : ''}`;
  }

  private renderFilters(): void {
    const facets = facetsOf(this.allCards().filter((c) => sectionOf(c.source) === this.section));
    const pick = <T extends string>(label: string, all: string, values: readonly T[], text: (v: T) => string, current: T | null, take: (v: T | null) => void) => {
      const dd = new Dropdown({
        label,
        ariaLabel: label,
        items: () => [{ label: all, radio: true, checked: current === null, run: () => take(null) }, { kind: 'separator' }, ...values.map((v) => ({ label: text(v), radio: true, checked: v === current, run: () => take(v) }))],
      });
      dd.set(h('span', { class: 'dropdown__text' }, current === null ? all : text(current)));
      return dd.el;
    };
    replaceChildren(
      this.filters,
      pick('Kâğıt', 'Hepsi', facets.papers, this.paperName, this.paper, (v) => {
        this.paper = v;
        this.renderAll();
      }),
      pick('Tür', 'Hepsi', facets.kinds, categoryLabel, this.kind, (v) => {
        this.kind = v;
        this.renderAll();
      }),
    );
  }

  private toggleAllModes(): void {
    this.allModes = !this.allModes;
    this.dialog.el.querySelector<HTMLElement>('.tgal__all .switch')?.setAttribute('aria-checked', String(this.allModes));
    this.renderAll();
  }

  private renderCards(): void {
    const cards = this.allCards();
    const ranked = this.rankedCards(cards);
    const state = this.stateOf(this.section);
    const searching = !!this.query.trim() || !!this.paper || !!this.kind;
    this.listed = arrangeTemplates(cards, ranked, { section: this.section, text: this.query, paper: this.paper, kind: this.kind, allModes: this.allModes });
    const label = SECTIONS.find((s) => s.id === this.section)!.label;
    const hidden = this.allModes ? 0 : hiddenByMode(cards, ranked, this.section);
    replaceChildren(
      this.head,
      h('span', null, state.state === 'ready' ? `${label}: ${this.listed.length} şablon` : label),
      hidden ? h('span', null, `· ${hidden} şablon başka türlerin (“Bütün türlerin şablonları” ile görünür)`) : null,
    );
    hideTooltip(this.grid);
    this.cardsD.dispose();
    this.repaints = [];
    // A template the cloud gave its own id (Buluta eşitle) stays chosen under it.
    if (this.selected && !cards.some((c) => c.id === this.selected)) this.selected = cards.find((c) => c.formerIds?.includes(this.selected!))?.id ?? null;
    if (!this.listed.length) {
      const e = emptyText(this.section, searching, state);
      replaceChildren(this.grid, h('div', { class: 'tgal__empty' }, icon(state.state === 'soon' ? 'history' : state.state === 'unavailable' ? 'info' : 'sheetTemplate', 28), h('b', null, e.title), e.text ? h('span', null, e.text) : null));
    } else if (this.section === 'org') replaceChildren(this.grid, ...this.orgGroups());
    else replaceChildren(this.grid, ...this.listed.map((a) => this.card(a)));
    this.rove();
    this.renderDetails();
  }

  /** “Kurumum”: the cards under their organisation's name, the organisations by name (each keeps the engine's order). */
  private orgGroups(): HTMLElement[] {
    const by = new Map<string, ArrangedCard[]>();
    for (const a of this.listed) {
      const name = a.card.organization?.name ?? '';
      by.set(name, [...(by.get(name) ?? []), a]);
    }
    return [...by]
      .sort(([a], [b]) => a.localeCompare(b, 'tr'))
      .map(([name, list]) => h('div', { class: 'tgal__group', role: 'group', 'aria-label': name }, h('div', { class: 'tgal__group-name', 'aria-hidden': 'true' }, name), ...list.map((a) => this.card(a))));
  }

  private card(a: ArrangedCard): HTMLElement {
    const c = a.card;
    const first = c.papers[0];
    const b = h(
      'button',
      { class: 'tcard', type: 'button', role: 'option', 'aria-selected': String(c.id === this.selected), 'aria-label': c.name, tabindex: '-1', dataset: { id: c.id } },
      h('span', { class: 'tcard__pic' }, this.picture(c, 150, 100)),
      h('span', { class: 'tcard__name' }, c.name),
      h('span', { class: 'tcard__meta' }, `${first ? paperText(first, this.paperName) : ''} · ${categoryLabel(c.category)}`),
      h('span', { class: 'tcard__badges' }, badgesOf(a)),
    );
    b.addEventListener('click', () => this.choose(c.id));
    b.addEventListener('dblclick', () => void this.run('use'));
    this.cardsD.add(tooltip(b, () => ({ title: c.name, description: c.description || undefined }), 'bottom'));
    return b;
  }

  /** The template's own plan, drawn small; a template the engine cannot make into a sheet says so in its place. */
  private picture(c: TemplateCard, width: number, height: number): HTMLElement | null {
    const made = this.host.templatePreview(c.template);
    const list = made ? this.host.plan(made.book, made.sheet) : null;
    if (!list) return h('span', { class: 'tcard__nopic' }, 'çizilemedi');
    const t = thumbCanvas(list, width, this.host.sources, this.paperColors(), `${c.name}: küçük resim`, height);
    this.repaints.push(t.paint);
    return t.canvas;
  }

  private choose(id: string): void {
    this.selected = id;
    this.usePaper = null;
    for (const el of this.grid.querySelectorAll<HTMLElement>('.tcard')) el.setAttribute('aria-selected', String(el.dataset.id === id));
    this.rove();
    this.renderDetails();
  }

  /** One stop for the cards: the chosen one takes Tab (else the first); the arrows move among them (galleryKeys.ts). */
  private rove(): void {
    const cards = [...this.grid.querySelectorAll<HTMLElement>('.tcard')];
    const at = Math.max(0, cards.findIndex((el) => el.dataset.id === this.selected));
    cards.forEach((el, i) => (el.tabIndex = i === at ? 0 : -1));
  }

  private renderDetails(): void {
    this.detailsD.dispose();
    const a = this.listed.find((x) => x.card.id === this.selected);
    const c = a?.card;
    const can = this.host.galleryAbilities();
    this.use.disabled = !c || actionsOf(c, can).use.reason !== null;
    replaceChildren(this.paperPick, ...(c ? paperPick(c, this.usePaper, this.paperName, (p) => ((this.usePaper = p), this.renderDetails())) : []));
    if (!c || !a) {
      this.say('');
      replaceChildren(
        this.details,
        h(
          'div',
          { class: 'smgr__none' },
          icon('sheetTemplate', 28),
          h('div', { class: 'smgr__none-title' }, 'Bir şablon seçin'),
          h('p', null, 'Sistem şablonları salt okunurdur: kullanılır, kopyalanır, kopyası düzenlenir. Kendi şablonlarınız önce bu cihazda saklanır; buluta eşitlenince paylaşılır.'),
        ),
      );
      return;
    }
    const sizes = this.host.engine()?.paperSizes() ?? [];
    replaceChildren(
      this.details,
      ...detailsOf(
        {
          arranged: a,
          can,
          picture: this.picture(c, 270, 170),
          needs: this.host.templateNeeds(c.template),
          paperName: this.paperName,
          paperSize: (p) => {
            const s = sizes.find((x) => x.id === p.paper);
            if (!s) return null;
            const [w, hh] = [s.width / 1000, s.height / 1000];
            return p.orientation === 'landscape' ? { width: Math.max(w, hh), height: Math.min(w, hh) } : { width: Math.min(w, hh), height: Math.max(w, hh) };
          },
          run: (k) => void this.run(k),
        },
        this.detailsD,
      ),
    );
    const reason = actionsOf(c, can).use.reason;
    this.say(reason ?? '', reason ? 'warn' : 'ok');
  }

  /** The window's own line; without one, what the cloud library says now. */
  private say(text: string, kind: 'ok' | 'warn' = 'ok'): void {
    const line = text || this.host.cloudLine.value.text;
    this.status.textContent = line;
    this.status.dataset.kind = text ? kind : 'cloud';
    if (text) this.status.dataset.own = '1';
    else delete this.status.dataset.own;
  }

  // ── Actions ──────────────────────────────────────────────────────

  private async run(action: TemplateAction): Promise<void> {
    const a = this.listed.find((x) => x.card.id === this.selected);
    if (!a) return;
    const c = a.card;
    const v = actionsOf(c, this.host.galleryAbilities())[action];
    if (action === 'share') {
      openShareTemplateDialog(this.ctx, this.host, c, { stack: true, reason: v.reason });
      return;
    }
    if (action === 'publish') {
      if (v.reason) return this.say(v.reason, 'warn');
      openPublishDialog(this.host, c, {
        // Kurumum, the organisation's template chosen once its list has it.
        done: (id) =>
          void Promise.all(this.host.providers.filter((p) => p.section === 'org').map((p) => p.refresh())).then(() => {
            this.section = 'org';
            this.selected = id;
            this.renderAll();
          }),
      });
      return;
    }
    if (!v.shown) return;
    if (v.reason) return this.say(v.reason, 'warn');
    const needs = action === 'use' ? this.host.templateNeeds(c.template) : [];
    if (needs.length) {
      const answer = await confirmDialog({
        title: 'Projede eksik olanlar var',
        message: `“${c.name}” şablonundan yapılacak paftada eksikler olacak:`,
        details: needs.map((f) => `${f.message} ${f.fix}`),
        answers: [
          { value: 'cancel', label: 'Vazgeç' },
          { value: 'use', label: 'Yine de kullan', kind: 'primary' },
        ],
        cancel: 'cancel',
      });
      if (answer !== 'use') return;
    }
    const done = await this.host.templateAction(c, action, action === 'use' || action === 'edit' ? this.usePaper : null);
    if (done && (action === 'use' || action === 'edit')) this.dialog.close();
  }
}
