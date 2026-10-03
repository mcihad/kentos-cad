import { searchesFor, SHARE_TEXTS } from '../../app/cloud/sharePlan';
import { failureText } from '../../app/cloud/sharing';
import type { SheetTemplateCandidate } from '../../contracts/generated/sheet/SheetTemplateCandidate';
import type { SheetTemplateCandidates } from '../../contracts/generated/sheet/SheetTemplateCandidates';
import type { TemplateGrantRole } from '../../contracts/generated/sheet/TemplateGrantRole';
import { h, replaceChildren } from '../dom';

/**
 * Şablonu paylaş's “Kişi ekle” field (ShareTemplateDialog.ts), as the
 * project's share window finds people (ui/cloud/shareFind.ts, ADR 0024): a
 * combobox over the people the template may be shared with, found on the
 * server by name or e-mail as they are typed (two letters at least; only
 * members of an organisation the owner shares). The list is used with the
 * arrows and Enter, or the mouse; Esc closes it before it closes the
 * window. Someone the template is shared with already says their role.
 */

export interface TemplateFinderOptions {
  open(): boolean;
  search(query: string, signal: AbortSignal): Promise<SheetTemplateCandidates>;
  /** The role a found person has now, or null. */
  has(userId: string): TemplateGrantRole | null;
  roleLabel(role: TemplateGrantRole): string;
  say(text: string, kind?: 'info' | 'error'): void;
  picked(c: SheetTemplateCandidate | null): void;
  /** Enter with nothing to pick: share with the one picked. */
  submit(): void;
}

export interface TemplateFinder {
  readonly el: HTMLElement;
  readonly input: HTMLInputElement;
  chosen(): SheetTemplateCandidate | null;
  clear(): void;
  dispose(): void;
}

const SEARCH_MS = 250;
let serial = 0;

export const FINDER_TEXTS = {
  nobody: 'Bu adla ya da e-postayla bulunan kimse yok. Yalnız ortak kurumlarınızın etkin üyeleriyle paylaşabilirsiniz.',
} as const;

export function createTemplateFinder(o: TemplateFinderOptions): TemplateFinder {
  const listId = `tshare-suggest-${++serial}`;
  let chosen: SheetTemplateCandidate | null = null;
  let found: SheetTemplateCandidate[] = [];
  let active = -1;
  let timer = 0;
  let searching: AbortController | null = null;

  const find = h('input', {
    class: 'field',
    type: 'text',
    placeholder: SHARE_TEXTS.find.placeholder,
    'aria-label': SHARE_TEXTS.find.label,
    role: 'combobox',
    'aria-autocomplete': 'list',
    'aria-expanded': 'false',
    'aria-controls': listId,
    autocomplete: 'off',
    spellcheck: 'false',
  });
  const suggest = h('ul', { class: 'share-suggest', id: listId, role: 'listbox', 'aria-label': SHARE_TEXTS.find.list, hidden: true });

  const hide = () => {
    suggest.hidden = true;
    found = [];
    active = -1;
    find.setAttribute('aria-expanded', 'false');
    find.removeAttribute('aria-activedescendant');
    delete find.dataset.escape;
  };
  const mark = () => {
    suggest.querySelectorAll('[role=option]').forEach((li, i) => li.setAttribute('aria-selected', String(i === active)));
    if (active >= 0) find.setAttribute('aria-activedescendant', `${listId}-${active}`);
    else find.removeAttribute('aria-activedescendant');
  };
  const pick = (c: SheetTemplateCandidate) => {
    chosen = c;
    find.value = c.displayName;
    hide();
    o.picked(c);
    o.say('');
  };
  const show = (people: SheetTemplateCandidate[]) => {
    found = people;
    active = people.length ? 0 : -1;
    replaceChildren(
      suggest,
      people.length
        ? people.map((c, i) => {
            const now = o.has(c.userId);
            const li = h(
              'li',
              { class: 'share-suggest__item', role: 'option', id: `${listId}-${i}`, 'aria-selected': String(i === active) },
              h('span', { class: 'share-suggest__name' }, c.displayName),
              c.email ? h('span', { class: 'share-suggest__mail' }, c.email) : null,
              now ? h('span', { class: 'share-suggest__has' }, SHARE_TEXTS.find.has(o.roleLabel(now))) : null,
            );
            li.addEventListener('pointerdown', (e) => e.preventDefault());
            li.addEventListener('click', () => pick(c));
            return li;
          })
        : [h('li', { class: 'share-suggest__none' }, FINDER_TEXTS.nobody)],
    );
    suggest.hidden = false;
    find.setAttribute('aria-expanded', 'true');
    // While the list is open Esc closes it, not the window (Dialog honours data-escape="local").
    find.dataset.escape = 'local';
    mark();
  };
  const search = () => {
    clearTimeout(timer);
    searching?.abort();
    const query = find.value.trim();
    if (chosen && query !== chosen.displayName) {
      chosen = null;
      o.picked(null);
    }
    if (chosen || !searchesFor(query)) return hide();
    timer = setTimeout(() => {
      const abort = (searching = new AbortController());
      o.search(query, abort.signal).then(
        (r) => {
          if (!abort.signal.aborted && o.open() && find.value.trim() === query) show(r.candidates);
        },
        (e: unknown) => {
          if (abort.signal.aborted || !o.open()) return;
          hide();
          o.say(failureText(e, SHARE_TEXTS.find.failed), 'error');
        },
      );
    }, SEARCH_MS) as unknown as number;
  };

  find.addEventListener('input', search);
  find.addEventListener('keydown', (e) => {
    const listOpen = !suggest.hidden && found.length > 0;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (!listOpen) return;
      e.preventDefault();
      active = (active + (e.key === 'ArrowDown' ? 1 : found.length - 1)) % found.length;
      mark();
      document.getElementById(`${listId}-${active}`)?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (listOpen && active >= 0) pick(found[active]);
      else o.submit();
    } else if (e.key === 'Escape' && !suggest.hidden) {
      e.preventDefault();
      hide();
    }
  });
  find.addEventListener('blur', () => setTimeout(() => o.open() && document.activeElement !== find && hide(), 0));

  return {
    el: h('div', { class: 'share-find' }, find, suggest),
    input: find,
    chosen: () => chosen,
    clear() {
      chosen = null;
      find.value = '';
      hide();
    },
    dispose() {
      clearTimeout(timer);
      searching?.abort();
    },
  };
}
