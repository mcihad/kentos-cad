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
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { askRemove } from '../widgets/confirm';
import { Dialog } from '../widgets/Dialog';
import type { ProjectTarget } from './ProjectActions';
import { createPersonFinder } from './shareFind';
import { createInvitePanel } from './shareInvites';

/**
 * “Projeyi paylaş” (docs/adr/0015, TODOS.md CLOUD-16, CLOUD-21): who may use
 * the project, with the role each works with now and where it comes from
 * (the owner, a grant and its end, the organisation's policy for its
 * admins), and where the project keeps its content. A person found by name
 * or e-mail, among those the account may share with, is added with a role
 * and an optional end; a grant's role is changed in its row, and taken away
 * after a question. Someone else is invited by e-mail on the “Davetler”
 * tab (docs/adr/0035, 0042: shareInvites.ts); a guest who accepted is
 * listed here, their role given by invitation. The server decides each
 * request: the controls are enabled only once it has shown the list
 * (`project.share`), and a refusal is said as the server says it, with what
 * to do. The account's own access and the owner's are never changed here.
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

export type ShareTab = 'people' | 'invites';

export function openShareDialog(ctx: AppContext, target: ProjectTarget, opts: { stack?: boolean; done?: () => void; tab?: ShareTab } = {}): void {
  const api = ctx.cloud.api;
  const me = ctx.cloud.me.value?.user.id ?? '';
  let list: ProjectAccessList | null = null;
  let mayShare = false;
  let busy = false;
  let open = true;

  const say = (text: string, kind: 'info' | 'error' = 'info') => {
    status.textContent = text;
    status.dataset.kind = kind;
  };
  const finder = createPersonFinder(ctx, target, {
    list: () => list,
    open: () => open,
    say: (text, kind) => say(text, kind),
    picked: () => refreshAdd(),
    invite: (email) => {
      showTab('invites');
      invites.prefill(email);
      invites.focus();
    },
    submit: () => void share(),
  });
  const find = finder.input;
  const invites = createInvitePanel(ctx, target, { say: (text, kind) => say(text, kind), open: () => open });

  const storage = h('p', { class: 'share-storage', hidden: true });
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
  const peoplePanel = h(
    'div',
    { class: 'share-panel', role: 'tabpanel', 'aria-label': 'Kişiler' },
    h(
      'div',
      { class: 'share-add' },
      h('div', { class: 'cloud-field share-add__who' }, h('span', null, 'Kişi ekle'), finder.el),
      h('label', { class: 'cloud-field' }, h('span', null, 'Rol'), role),
      h('label', { class: 'cloud-field' }, h('span', null, 'Bitiş (isteğe bağlı)'), until),
      add,
    ),
    roleHint,
    h('h3', { class: 'share-title' }, h('span', null, 'Erişimi olanlar'), count),
    people,
    policy,
  );
  const tabButtons = (
    [
      ['people', 'Kişiler', peoplePanel],
      ['invites', 'Davetler', invites.el],
    ] as const
  ).map(([id, text, panel]) => {
    const b = h('button', { class: 'tab', type: 'button', role: 'tab', 'aria-selected': 'false', dataset: { tab: id } }, text);
    b.addEventListener('click', () => {
      showTab(id);
      (id === 'people' ? find : invites).focus();
    });
    return { id, b, panel };
  });
  /** One tab shows: its form has the one amber button of the dialog. */
  const showTab = (id: ShareTab) => {
    for (const t of tabButtons) {
      t.b.setAttribute('aria-selected', String(t.id === id));
      t.panel.hidden = t.id !== id;
    }
  };

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
        { class: 'cloud-tabs share-tabs', role: 'tablist', 'aria-label': 'Paylaşım' },
        tabButtons.map((t) => t.b),
      ),
      peoplePanel,
      invites.el,
      status,
    ],
    footer: [h('div', { class: 'dialog__foot-spacer' }), close],
    onClose: () => {
      open = false;
      finder.dispose();
      opts.done?.();
    },
  });

  const refreshAdd = () => {
    for (const c of [find, role, until]) c.disabled = !mayShare || busy;
    const chosen = finder.chosen();
    add.disabled = !mayShare || busy || !chosen;
    add.title = !mayShare ? 'Bu projede paylaşım yetkiniz yok (project.share).' : !chosen ? 'Önce “Kişi ekle” alanında bir kişi arayıp listeden seçin.' : '';
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
    invites.setAccess(mayShare, list);
    refreshAdd();
  };

  const share = async () => {
    const who = finder.chosen();
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
      finder.clear();
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
        row.guest ? 'Kurum dışından olduğu için erişimini yeniden davetle geri verebilirsiniz.' : 'Yeniden paylaşarak erişimini geri verebilirsiniz.',
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
      const why = r.guest ? 'Misafirin rolü davetle verilir. Değiştirmek için erişimini kaldırıp yeni rolle yeniden davet edin.' : null;
      roleCell = h('span', { class: 'share-role', dataset: r.blocked ? { blocked: '' } : {}, title: why }, r.blocked ? icon('warning', 14) : null, r.roleLabel);
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
    const outside = 'Kurum dışından biri “Davetler”den e-postayla davet edilir ve misafir olur.';
    policy.textContent =
      list.tenantKind === 'personal'
        ? 'Kişisel alanınızdaki bu proje yalnız paylaştığınız ve davet ettiğiniz kişilere açıktır.'
        : list.adminsAccessAllProjects
          ? `Kurumun politikası açık: kurum sahibi ve yöneticileri paylaşılmamış kurum projelerine de yönetici olarak erişir. Paylaşım kurumun üyeleriyledir; ${outside}`
          : `Kurumun politikası kapalı: kurum yöneticileri de yalnız kendileriyle paylaşılan projelere erişir. Paylaşım kurumun üyeleriyledir; ${outside}`;
  };

  role.addEventListener('change', () => (roleHint.textContent = ROLE_HINT[role.value as GrantRole]));
  add.addEventListener('click', () => void share());
  close.addEventListener('click', () => dialog.close());
  showTab(opts.tab ?? 'people');
  refreshAdd();
  void load().then(() => {
    if (!open) return;
    if (mayShare) (opts.tab === 'invites' ? invites : find).focus();
    void invites.load();
  });
}
