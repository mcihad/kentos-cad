import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import {
  GRANT_ROLES,
  ROLE_HINT,
  ROLE_LABEL,
  STORAGE_TEXT,
  endOfDay,
  failureText,
  personRows,
  revokeEnvelope,
  shareEnvelope,
  type PersonRow,
} from '../../app/cloud/sharing';
import type { GrantRole } from '../../contracts/generated/GrantRole';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ShareCandidate } from '../../contracts/generated/ShareCandidate';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { askRemove } from '../widgets/confirm';
import { Dialog } from '../widgets/Dialog';
import type { ProjectTarget } from './ProjectActions';

/**
 * “Projeyi paylaş” (docs/adr/0015, TODOS.md CLOUD-16, CLOUD-21): who may use
 * the project, with the role each works with now and where it comes from
 * (the owner, a grant and its end, the organisation's policy for its
 * admins), and where the project keeps its content. A person found by name
 * or e-mail, among those the account may share with, is added with a role
 * and an optional end; a grant's role is changed in its row, and taken away
 * after a question. The server decides each request: the controls are
 * enabled only once it has shown the list (`project.share`), and a refusal
 * is said as the server says it, with what to do. The account's own access
 * and the owner's are never changed here.
 */

/** Local today as a date field's value (the earliest end a share may have). */
function today(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

const initials = (name: string) =>
  name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]!.toLocaleUpperCase('tr'))
    .join('') || '?';

const SEARCH_MS = 250;
let serial = 0;

export function openShareDialog(ctx: AppContext, target: ProjectTarget, opts: { stack?: boolean; done?: () => void } = {}): void {
  const api = ctx.cloud.api;
  const me = ctx.cloud.me.value?.user.id ?? '';
  const suggestId = `share-suggest-${++serial}`;
  let list: ProjectAccessList | null = null;
  let mayShare = false;
  let chosen: ShareCandidate | null = null;
  let found: ShareCandidate[] = [];
  let active = -1;
  let busy = false;
  let open = true;
  let timer = 0;
  let searching: AbortController | null = null;

  const storage = h('p', { class: 'share-storage', hidden: true });
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
  const role = h(
    'select',
    { class: 'field', 'aria-label': 'Rol', disabled: true },
    GRANT_ROLES.map((r) => h('option', { value: r, selected: r === 'editor', title: ROLE_HINT[r] }, ROLE_LABEL[r])),
  );
  const until = h('input', { class: 'field', type: 'date', min: today(), 'aria-label': 'Bitiş tarihi (isteğe bağlı)', disabled: true });
  const add = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, 'Paylaş');
  const roleHint = h('p', { class: 'cloud-hint share-rolehint' }, ROLE_HINT.editor);
  const count = h('span', { class: 'share-count' });
  const people = h('div', { class: 'share-list', role: 'list', 'aria-label': 'Erişimi olanlar' }, h('p', { class: 'cloud-empty' }, 'Erişimi olanlar yükleniyor…'));
  const policy = h('p', { class: 'cloud-hint share-policy', hidden: true });
  const status = h('p', { class: 'cloud-status', role: 'status' });
  const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');

  const dialog = new Dialog({
    title: 'Projeyi paylaş',
    width: 680,
    className: 'dialog--cloud dialog--share',
    stack: opts.stack,
    content: [
      h('p', { class: 'share-head' }, h('b', null, `“${target.name}”`), h('span', null, ` · ${target.tenantName}`)),
      storage,
      h(
        'div',
        { class: 'share-add' },
        h('div', { class: 'cloud-field share-add__who' }, h('span', null, 'Kişi ekle'), h('div', { class: 'share-find' }, find, suggest)),
        h('label', { class: 'cloud-field' }, h('span', null, 'Rol'), role),
        h('label', { class: 'cloud-field' }, h('span', null, 'Bitiş (isteğe bağlı)'), until),
        add,
      ),
      roleHint,
      h('h3', { class: 'share-title' }, h('span', null, 'Erişimi olanlar'), count),
      people,
      policy,
      status,
    ],
    footer: [h('div', { class: 'dialog__foot-spacer' }), close],
    onClose: () => {
      open = false;
      clearTimeout(timer);
      searching?.abort();
      opts.done?.();
    },
  });

  const say = (text: string, kind: 'info' | 'error' = 'info') => {
    status.textContent = text;
    status.dataset.kind = kind;
  };
  const refreshAdd = () => {
    for (const c of [find, role, until]) c.disabled = !mayShare || busy;
    add.disabled = !mayShare || busy || !chosen;
    add.title = !mayShare ? 'Bu projede paylaşım yetkiniz yok (project.share).' : !chosen ? 'Önce “Kişi ekle” alanında bir kişi arayıp listeden seçin.' : '';
  };

  // ── Finding a person ────────────────────────────────────────────────

  const hideSuggest = () => {
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
    hideSuggest();
    refreshAdd();
    say('');
  };
  const showSuggest = (query: string, candidates: ShareCandidate[]) => {
    found = candidates;
    active = candidates.length ? 0 : -1;
    const current = new Map(list?.people.map((p) => [p.userId, p]) ?? []);
    const none =
      list?.tenantKind === 'personal'
        ? `“${query}” ile eşleşen kimse yok. Kişisel projeler şimdilik kurumlarınızdaki kişilerle paylaşılır; e-postayla davet sonraki sürümde.`
        : `“${query}” ile eşleşen etkin bir kurum üyesi yok. Kurum projeleri yalnız kurumun üyeleriyle paylaşılır.`;
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
        : h('li', { class: 'share-suggest__none' }, none),
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
      refreshAdd();
    }
    if (chosen || query.replace(/\s+/g, '').length < 2) return hideSuggest();
    timer = setTimeout(() => {
      const abort = (searching = new AbortController());
      api.candidates(target.tenantId, target.projectId, query, abort.signal).then(
        (r) => {
          if (!abort.signal.aborted && open && find.value.trim() === query) showSuggest(query, r.candidates);
        },
        (e: unknown) => {
          if (abort.signal.aborted || !open) return;
          hideSuggest();
          say(failureText(e, 'Kişi aranamadı'), 'error');
        },
      );
    }, SEARCH_MS) as unknown as number;
  };

  // ── Sharing, changing a role, taking it away ────────────────────────

  const load = async () => {
    try {
      const next = await api.access(target.tenantId, target.projectId);
      if (!open) return;
      list = next;
      mayShare = true;
      render();
    } catch (e) {
      if (!open) return;
      if (e instanceof ApiFailure && e.code === 'forbidden') mayShare = false;
      replaceChildren(people, h('p', { class: 'cloud-empty', role: 'alert' }, failureText(e, 'Erişim listesi okunamadı')));
    }
    refreshAdd();
  };

  const share = async () => {
    const who = chosen;
    if (!who || busy || !mayShare) return;
    const expiresAt = until.value ? endOfDay(until.value) : null;
    if (until.value && !expiresAt) return say('Bitiş tarihi okunamadı: takvimden bir gün seçin ya da alanı boş bırakın.', 'error');
    const grant = role.value as GrantRole;
    // Shared already: sharing again changes the role (and the end).
    const had = list?.people.some((p) => p.userId === who.userId && p.grant);
    busy = true;
    refreshAdd();
    say(`${who.displayName} ekleniyor…`);
    try {
      const r = await api.accessCommand(shareEnvelope(target.tenantId, target.projectId, who.userId, grant, expiresAt ?? undefined));
      const text = !r.changed
        ? `${who.displayName} zaten ${ROLE_LABEL[grant]} rolündeydi; değişen bir şey yok.`
        : had
          ? `${who.displayName} artık ${ROLE_LABEL[grant]}.`
          : `${who.displayName} projeye ${ROLE_LABEL[grant]} olarak eklendi.`;
      if (r.changed) ctx.log.success(`“${target.name}”: ${text}`);
      chosen = null;
      find.value = '';
      until.value = '';
      await load();
      say(text);
    } catch (e) {
      say(failureText(e, 'Paylaşılamadı'), 'error');
    } finally {
      busy = false;
      refreshAdd();
      // Ready for the next person.
      if (open && mayShare) find.focus();
    }
  };

  const change = async (row: PersonRow, next: GrantRole, select: HTMLSelectElement) => {
    select.disabled = true;
    say(`${row.name} için rol değiştiriliyor…`);
    try {
      await api.accessCommand(shareEnvelope(target.tenantId, target.projectId, row.userId, next, row.expiresAt));
      const text = `${row.name} artık ${ROLE_LABEL[next]}.`;
      ctx.log.success(`“${target.name}”: ${text}`);
      await load();
      say(text);
    } catch (e) {
      select.value = row.grant ?? next;
      select.disabled = false;
      say(failureText(e, 'Rol değiştirilemedi'), 'error');
    }
  };

  const revoke = async (row: PersonRow) => {
    const sure = await askRemove({
      title: 'Erişimi kaldır',
      message: `${row.name}, “${target.name}” projesine artık erişemesin mi?`,
      details: [
        'Projeyi şu anda açık tutuyorsa kaydı hemen durur; gönderilmemiş değişiklikleri kendi cihazında kalır.',
        'Daha önce indirdiği kopyalar ve ekranında gördükleri geri alınamaz.',
        'Yeniden paylaşarak erişimini geri verebilirsiniz.',
      ],
      action: 'Erişimi kaldır',
    });
    if (!sure || !open) return;
    say(`${row.name} için erişim kaldırılıyor…`);
    try {
      await api.accessCommand(revokeEnvelope(target.tenantId, target.projectId, row.userId));
      const text = `${row.name} artık projeye erişemiyor.`;
      ctx.log.success(`“${target.name}”: ${text}`);
      await load();
      say(text);
    } catch (e) {
      say(failureText(e, 'Erişim kaldırılamadı'), 'error');
    }
  };

  // ── The list ────────────────────────────────────────────────────────

  const personRow = (r: PersonRow): HTMLElement => {
    let roleCell: HTMLElement;
    if (r.canChange && r.grant) {
      const select = h(
        'select',
        { class: 'field share-rolesel', 'aria-label': `${r.name} için rol` },
        GRANT_ROLES.map((g) => h('option', { value: g, selected: g === r.grant, title: ROLE_HINT[g] }, ROLE_LABEL[g])),
      );
      select.addEventListener('change', () => void change(r, select.value as GrantRole, select));
      roleCell = select;
    } else {
      roleCell = h('span', { class: 'share-role', dataset: r.blocked ? { blocked: '' } : {} }, r.blocked ? icon('warning', 14) : null, r.roleLabel);
    }
    let action: HTMLElement;
    if (r.canRevoke) {
      const b = h('button', { class: 'btn btn--ghost btn--small share-remove', type: 'button', 'aria-label': `${r.name} erişimini kaldır` }, 'Kaldır');
      b.addEventListener('click', () => void revoke(r));
      action = b;
    } else {
      const why = r.you
        ? 'Kendi erişiminizi buradan değiştiremezsiniz; proje sahibine ya da başka bir yöneticiye başvurun.'
        : r.owner
          ? 'Proje sahibinin erişimi paylaşımla değişmez.'
          : !r.grant
            ? 'Kurum politikasından gelen erişim paylaşımla değişmez.'
            : 'Bu erişimi değiştirme yetkiniz yok.';
      action = h('span', { class: 'share-fixed', title: why, 'aria-label': why }, icon('lock', 14));
    }
    const sub = [r.email, r.source].filter(Boolean).join(' · ');
    return h(
      'div',
      { class: 'share-row', role: 'listitem', dataset: { user: r.userId } },
      h('span', { class: 'share-avatar', 'aria-hidden': 'true' }, initials(r.name)),
      h(
        'div',
        { class: 'share-who' },
        h('span', { class: 'share-name' }, r.name, r.you ? h('span', { class: 'share-you' }, ' (siz)') : null),
        h('span', { class: 'share-sub', title: sub }, sub),
      ),
      roleCell,
      action,
    );
  };

  const render = () => {
    if (!list) return;
    const text = STORAGE_TEXT[list.storage];
    replaceChildren(storage, icon('server', 16), h('span', null, h('b', null, `Saklama: ${text.title}. `), text.detail));
    storage.hidden = false;
    const rows = personRows(list, me, mayShare);
    count.textContent = `${rows.filter((r) => !r.blocked).length} kişi erişebiliyor`;
    replaceChildren(people, rows.map(personRow));
    policy.hidden = false;
    policy.textContent =
      list.tenantKind === 'personal'
        ? 'Kişisel alanınızdaki bu proje yalnız paylaştığınız kişilere açıktır.'
        : list.adminsAccessAllProjects
          ? 'Kurumun politikası açık: kurum sahibi ve yöneticileri paylaşılmamış kurum projelerine de yönetici olarak erişir. Proje yalnız kurumun üyeleriyle paylaşılır.'
          : 'Kurumun politikası kapalı: kurum yöneticileri de yalnız kendileriyle paylaşılan projelere erişir. Proje yalnız kurumun üyeleriyle paylaşılır.';
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
      else void share();
    } else if (e.key === 'Escape' && !suggest.hidden) {
      e.preventDefault();
      hideSuggest();
    }
  });
  find.addEventListener('blur', () => setTimeout(() => open && document.activeElement !== find && hideSuggest(), 0));
  role.addEventListener('change', () => (roleHint.textContent = ROLE_HINT[role.value as GrantRole]));
  add.addEventListener('click', () => void share());
  close.addEventListener('click', () => dialog.close());
  refreshAdd();
  void load().then(() => open && mayShare && find.focus());
}
