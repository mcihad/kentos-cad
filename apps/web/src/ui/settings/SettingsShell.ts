import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';

/**
 * Shared frame for settings dialogs. A dialog edits a plain draft object;
 * nothing touches the live model until Kaydet. Sections declare which draft
 * keys they own so "Bu bölümü varsayılana döndür" resets only those.
 */

export interface DraftApi<D extends object> {
  readonly draft: D;
  readonly initial: D;
  /** Update one field; `rerender: false` lets a section patch itself (keeps focus while typing). */
  set<K extends keyof D>(key: K, value: D[K], rerender?: boolean): void;
}

export interface SectionDef<D extends object> {
  id: string;
  label: string;
  icon: string;
  title: string;
  lead: string;
  keys: (keyof D)[];
  render(api: DraftApi<D>): Child;
}

export interface SettingsShellOptions<D extends object> {
  title: string;
  /** Where these settings are stored, shown under the section list. */
  scope: { icon: string; title: string; detail: string };
  sections: SectionDef<D>[];
  initial: D;
  defaults: D;
  section?: string;
  onSave(draft: D, initial: D): void;
}

export class SettingsShell<D extends object> implements DraftApi<D> {
  readonly initial: D;
  draft: D;
  private readonly opts: SettingsShellOptions<D>;
  private section: SectionDef<D>;
  private readonly nav = h('nav', { class: 'settings__nav', 'aria-label': 'Ayar bölümleri' });
  private readonly navList = h('div', { class: 'settings__navlist' });
  private readonly content = h('div', { class: 'settings__content' });
  private readonly saveBtn: HTMLButtonElement;
  private readonly resetBtn: HTMLButtonElement;
  private readonly dialog: Dialog;

  constructor(opts: SettingsShellOptions<D>) {
    this.opts = opts;
    this.initial = { ...opts.initial };
    this.draft = { ...opts.initial };
    this.section = opts.sections.find((s) => s.id === opts.section) ?? opts.sections[0];

    this.nav.append(
      this.navList,
      h(
        'div',
        { class: 'settings__scope' },
        icon(opts.scope.icon, 16),
        h('div', null, h('div', { class: 'settings__scope-title' }, opts.scope.title), h('div', { class: 'settings__scope-detail' }, opts.scope.detail)),
      ),
    );

    const reset = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Bu bölümü varsayılana döndür');
    this.resetBtn = reset;
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.saveBtn = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, 'Kaydet');
    reset.addEventListener('click', () => this.resetSection());
    cancel.addEventListener('click', () => this.dialog.close());
    this.saveBtn.addEventListener('click', () => {
      opts.onSave(this.draft, this.initial);
      this.dialog.close();
    });

    this.dialog = new Dialog({
      title: opts.title,
      width: 940,
      className: 'dialog--settings',
      content: [h('div', { class: 'settings' }, this.nav, this.content)],
      footer: [reset, h('div', { class: 'dialog__spacer' }), cancel, this.saveBtn],
    });
    this.renderNav();
    this.renderContent();
  }

  set<K extends keyof D>(key: K, value: D[K], rerender = true): void {
    this.draft = { ...this.draft, [key]: value };
    this.saveBtn.disabled = JSON.stringify(this.draft) === JSON.stringify(this.initial);
    if (rerender) this.renderContent();
  }

  private resetSection(): void {
    for (const k of this.section.keys) this.draft = { ...this.draft, [k]: this.opts.defaults[k] };
    this.saveBtn.disabled = JSON.stringify(this.draft) === JSON.stringify(this.initial);
    this.renderContent();
  }

  private renderNav(): void {
    replaceChildren(
      this.navList,
      this.opts.sections.map((s) => {
        const b = h(
          'button',
          { class: 'settings__navitem', type: 'button', 'aria-current': s === this.section ? 'page' : null },
          icon(s.icon, 18),
          h('span', null, s.label),
        );
        b.addEventListener('click', () => {
          this.section = s;
          this.renderNav();
          this.renderContent();
          this.content.scrollTop = 0;
        });
        return b;
      }),
    );
  }

  private renderContent(): void {
    const sec = this.section;
    // A section without values of its own (Ayar dosyası) has nothing to reset.
    this.resetBtn.disabled = sec.keys.length === 0;
    this.resetBtn.title = sec.keys.length === 0 ? 'Bu bölümde varsayılana dönecek ayar yok.' : '';
    const scrollTop = this.content.scrollTop;
    // Keep keyboard focus across re-renders: remember the nearest labelled control.
    const active = document.activeElement as HTMLElement | null;
    const focusKey = active && this.content.contains(active) ? active.closest('[aria-label]')?.getAttribute('aria-label') : null;
    replaceChildren(
      this.content,
      h('header', { class: 'settings__head' }, h('h3', { class: 'settings__title' }, sec.title), h('p', { class: 'settings__lead' }, sec.lead)),
      sec.render(this),
    );
    this.content.scrollTop = scrollTop;
    if (focusKey) {
      const target = this.content.querySelector<HTMLElement>(`[aria-label="${CSS.escape(focusKey)}"]`);
      (target?.getAttribute('role') === 'radiogroup' ? target.querySelector<HTMLElement>('[aria-checked="true"]') : target)?.focus();
    }
  }
}

/** Titled group of rows inside a section. */
export function group(title: string, ...children: Child[]): HTMLElement {
  return h('div', { class: 'sgroup' }, h('div', { class: 'sgroup__title' }, title), children);
}
