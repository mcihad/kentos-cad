import { emailProblem } from '../../app/cloud/invitations';
import { ROLE_LABEL, failureText } from '../../app/cloud/sharing';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ShareCandidate } from '../../contracts/generated/ShareCandidate';
import type { AppContext } from '../../app/context';
import { h, replaceChildren } from '../dom';
import type { ProjectTarget } from './ProjectActions';

/**
 * The share dialog's “Kişi ekle” field (docs/adr/0024): a combobox over the
 * people the project can be shared with, found on the server by name or
 * e-mail as they are typed (only among those the account may share with).
 * The list is used with the arrows and Enter, or the mouse; Esc closes it
 * before it closes the dialog. Where nobody is found and a whole e-mail was
 * typed, the list offers to invite that address instead (docs/adr/0042).
 */

export interface PersonFinderOptions {
  /** Who can use the project now: a found person's role, and the workspace's kind. */
  list(): ProjectAccessList | null;
  open(): boolean;
  say(text: string, kind?: 'info' | 'error'): void;
  /** The person picked changed (null: the text no longer names one). */
  picked(c: ShareCandidate | null): void;
  /** Nobody found for this whole e-mail: invite it instead. */
  invite(email: string): void;
  /** Enter with nothing to pick: share with the one picked. */
  submit(): void;
}

export interface PersonFinder {
  /** The field and its list. */
  readonly el: HTMLElement;
  readonly input: HTMLInputElement;
  chosen(): ShareCandidate | null;
  /** Empties the field for the next person. */
  clear(): void;
  dispose(): void;
}

const SEARCH_MS = 250;
let serial = 0;

export function createPersonFinder(ctx: AppContext, target: ProjectTarget, o: PersonFinderOptions): PersonFinder {
  const suggestId = `share-suggest-${++serial}`;
  let chosen: ShareCandidate | null = null;
  let found: ShareCandidate[] = [];
  let active = -1;
  let timer = 0;
  let searching: AbortController | null = null;

  const find = h('input', {
    class: 'field',
    type: 'text',
    placeholder: 'Ad ya da e-posta yazın',
    'aria-label': 'Paylaşılacak kişi',
    role: 'combobox',
    'aria-autocomplete': 'list',
    'aria-expanded': 'false',
    'aria-controls': suggestId,
    autocomplete: 'off',
    spellcheck: 'false',
    disabled: true,
  });
  const suggest = h('ul', { class: 'share-suggest', id: suggestId, role: 'listbox', 'aria-label': 'Bulunan kişiler', hidden: true });

  const hide = () => {
    suggest.hidden = true;
    found = [];
    active = -1;
    find.setAttribute('aria-expanded', 'false');
    find.removeAttribute('aria-activedescendant');
    // Esc closes the dialog again.
    delete find.dataset.escape;
  };
  const markActive = () => {
    suggest.querySelectorAll('[role=option]').forEach((li, i) => li.setAttribute('aria-selected', String(i === active)));
    if (active >= 0) find.setAttribute('aria-activedescendant', `${suggestId}-${active}`);
    else find.removeAttribute('aria-activedescendant');
  };
  const pick = (c: ShareCandidate) => {
    chosen = c;
    find.value = c.displayName;
    hide();
    o.picked(c);
    o.say('');
  };
  /** Nobody found: what the workspace allows, and, for a whole e-mail, an invitation instead. */
  const nobody = (query: string): HTMLElement[] => {
    const personal = o.list()?.tenantKind === 'personal';
    const none = personal
      ? `“${query}” ile eşleşen kimse yok. Kurumlarınızın dışından biriyle “Davetler”den e-postayla paylaşabilirsiniz.`
      : `“${query}” ile eşleşen etkin bir kurum üyesi yok. Kurum dışından biri “Davetler”den e-postayla davet edilir ve misafir olur.`;
    const out = [h('li', { class: 'share-suggest__none' }, none)];
    if (!emailProblem(query)) {
      const li = h('li', { class: 'share-suggest__item share-suggest__invite', role: 'option', id: `${suggestId}-0`, 'aria-selected': 'true' }, h('span', { class: 'share-suggest__name' }, `“${query.trim()}” adresine e-postayla davet gönder…`));
      li.addEventListener('pointerdown', (e) => e.preventDefault());
      li.addEventListener('click', () => {
        hide();
        o.invite(query.trim());
      });
      out.push(li);
    }
    return out;
  };
  const show = (query: string, candidates: ShareCandidate[]) => {
    found = candidates;
    active = candidates.length ? 0 : -1;
    const current = new Map(o.list()?.people.map((p) => [p.userId, p]) ?? []);
    replaceChildren(
      suggest,
      candidates.length
        ? candidates.map((c, i) => {
            const has = current.get(c.userId);
            const li = h(
              'li',
              { class: 'share-suggest__item', role: 'option', id: `${suggestId}-${i}`, 'aria-selected': String(i === active) },
              h('span', { class: 'share-suggest__name' }, c.displayName),
              c.email ? h('span', { class: 'share-suggest__mail' }, c.email) : null,
              has?.role ? h('span', { class: 'share-suggest__has' }, `şu an ${ROLE_LABEL[has.role]}`) : null,
            );
            // The field keeps the focus: typing goes on while the mouse picks.
            li.addEventListener('pointerdown', (e) => e.preventDefault());
            li.addEventListener('click', () => pick(c));
            return li;
          })
        : nobody(query),
    );
    suggest.hidden = false;
    find.setAttribute('aria-expanded', 'true');
    // While the list is open Esc closes it, not the dialog (Dialog honours data-escape="local").
    find.dataset.escape = 'local';
    markActive();
  };
  const search = () => {
    clearTimeout(timer);
    searching?.abort();
    const query = find.value.trim();
    if (chosen && query !== chosen.displayName) {
      chosen = null;
      o.picked(null);
    }
    if (chosen || query.replace(/\s+/g, '').length < 2) return hide();
    timer = setTimeout(() => {
      const abort = (searching = new AbortController());
      ctx.cloud.api.candidates(target.tenantId, target.projectId, query, abort.signal).then(
        (r) => {
          if (!abort.signal.aborted && o.open() && find.value.trim() === query) show(query, r.candidates);
        },
        (e: unknown) => {
          if (abort.signal.aborted || !o.open()) return;
          hide();
          o.say(failureText(e, 'Kişi aranamadı'), 'error');
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
      markActive();
      document.getElementById(`${suggestId}-${active}`)?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (listOpen && active >= 0) pick(found[active]);
      else if (!suggest.hidden && !found.length && !emailProblem(find.value)) {
        hide();
        o.invite(find.value.trim());
      } else o.submit();
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
