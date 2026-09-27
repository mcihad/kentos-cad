import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { SHARE_TEXTS, peopleView, revokeQuestion, revokedText, roleChangedText, shareLog, shareTip, sharedText, type PersonView } from '../../app/cloud/sharePlan';
import { GRANT_ROLES, ROLE_HINT, ROLE_LABEL, endOfDay, failureText, revokeEnvelope, shareEnvelope } from '../../app/cloud/sharing';
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
 * What it says and offers is sharePlan.ts's; this file draws it.
 */

/** Local today as a date field's value (the earliest end a share may have). */
function today(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

export type ShareTab = 'people' | 'invites';

const T = SHARE_TEXTS.people;

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
    { class: 'field', 'aria-label': T.role, disabled: true },
    GRANT_ROLES.map((r) => h('option', { value: r, selected: r === 'editor', title: ROLE_HINT[r] }, ROLE_LABEL[r])),
  );
  const until = h('input', { class: 'field', type: 'date', min: today(), 'aria-label': T.untilLabel, disabled: true });
  const add = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, T.share);
  const roleHint = h('p', { class: 'cloud-hint share-rolehint' }, ROLE_HINT.editor);
  const count = h('span', { class: 'share-count' });
  const people = h('div', { class: 'share-list', role: 'list', 'aria-label': T.listLabel }, h('p', { class: 'cloud-empty' }, T.loading));
  const policy = h('p', { class: 'cloud-hint share-policy', hidden: true });
  const status = h('p', { class: 'cloud-status', role: 'status' });
  const close = h('button', { class: 'btn', type: 'button' }, SHARE_TEXTS.close);
  const peoplePanel = h(
    'div',
    { class: 'share-panel', role: 'tabpanel', 'aria-label': SHARE_TEXTS.tabs.people },
    h(
      'div',
      { class: 'share-add' },
      h('div', { class: 'cloud-field share-add__who' }, h('span', null, T.add), finder.el),
      h('label', { class: 'cloud-field' }, h('span', null, T.role), role),
      h('label', { class: 'cloud-field' }, h('span', null, T.until), until),
      add,
    ),
    roleHint,
    h('h3', { class: 'share-title' }, h('span', null, T.title), count),
    people,
    policy,
  );
  const tabButtons = (
    [
      ['people', SHARE_TEXTS.tabs.people, peoplePanel],
      ['invites', SHARE_TEXTS.tabs.invites, invites.el],
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
    title: SHARE_TEXTS.title,
    width: 680,
    className: 'dialog--cloud dialog--share',
    stack: opts.stack,
    content: [
      h('p', { class: 'share-head' }, h('b', null, `“${target.name}”`), h('span', null, ` · ${target.tenantName}`)),
      storage,
      h(
        'div',
        { class: 'cloud-tabs share-tabs', role: 'tablist', 'aria-label': SHARE_TEXTS.tabs.label },
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
    add.title = shareTip(mayShare, !!chosen);
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
      replaceChildren(people, h('p', { class: 'cloud-empty', role: 'alert' }, failureText(e, T.readFailed)));
    }
    invites.setAccess(mayShare, list);
    refreshAdd();
  };

  const share = async () => {
    const who = finder.chosen();
    if (!who || busy || !mayShare) return;
    const expiresAt = until.value ? endOfDay(until.value) : null;
    if (until.value && !expiresAt) return say(T.badDate, 'error');
    const grant = role.value as GrantRole;
    // Shared already: sharing again changes the role (and the end).
    const had = list?.people.some((p) => p.userId === who.userId && p.grant);
    busy = true;
    refreshAdd();
    say(T.adding(who.displayName));
    try {
      const r = await api.accessCommand(shareEnvelope(target.tenantId, target.projectId, who.userId, grant, expiresAt ?? undefined));
      const text = sharedText(who.displayName, grant, r.changed, !!had);
      if (r.changed) ctx.log.success(shareLog(target.name, text));
      finder.clear();
      until.value = '';
      await load();
      say(text);
    } catch (e) {
      say(failureText(e, T.shareFailed), 'error');
    } finally {
      busy = false;
      refreshAdd();
      // Ready for the next person.
      if (open && mayShare) find.focus();
    }
  };

  const change = async (row: PersonView, next: GrantRole, select: HTMLSelectElement) => {
    select.disabled = true;
    say(T.changing(row.name));
    try {
      await api.accessCommand(shareEnvelope(target.tenantId, target.projectId, row.userId, next, row.expiresAt));
      const text = roleChangedText(row.name, next);
      ctx.log.success(shareLog(target.name, text));
      await load();
      say(text);
    } catch (e) {
      select.value = row.grant ?? next;
      select.disabled = false;
      say(failureText(e, T.changeFailed), 'error');
    }
  };

  const revoke = async (row: PersonView) => {
    const sure = await askRemove(revokeQuestion(row.name, target.name, row.guest));
    if (!sure || !open) return;
    say(T.revoking(row.name));
    try {
      await api.accessCommand(revokeEnvelope(target.tenantId, target.projectId, row.userId));
      const text = revokedText(row.name);
      ctx.log.success(shareLog(target.name, text));
      await load();
      say(text);
    } catch (e) {
      say(failureText(e, T.revokeFailed), 'error');
    }
  };

  // ── The list ────────────────────────────────────────────────────────

  const personRow = (r: PersonView): HTMLElement => {
    let roleCell: HTMLElement;
    if (r.canChange && r.grant) {
      const select = h(
        'select',
        { class: 'field share-rolesel', 'aria-label': T.roleLabel(r.name) },
        GRANT_ROLES.map((g) => h('option', { value: g, selected: g === r.grant, title: ROLE_HINT[g] }, ROLE_LABEL[g])),
      );
      select.addEventListener('change', () => void change(r, select.value as GrantRole, select));
      roleCell = select;
    } else roleCell = h('span', { class: 'share-role', dataset: r.blocked ? { blocked: '' } : {}, title: r.roleTip }, r.blocked ? icon('warning', 14) : null, r.roleLabel);
    let action: HTMLElement;
    if (r.canRevoke) {
      const b = h('button', { class: 'btn btn--ghost btn--small share-remove', type: 'button', 'aria-label': T.removeLabel(r.name) }, T.remove);
      b.addEventListener('click', () => void revoke(r));
      action = b;
    } else action = h('span', { class: 'share-fixed', title: r.fixed, 'aria-label': r.fixed }, icon('lock', 14));
    return h(
      'div',
      { class: 'share-row', role: 'listitem', dataset: { user: r.userId } },
      h('span', { class: 'share-avatar', 'aria-hidden': 'true' }, r.initials),
      h(
        'div',
        { class: 'share-who' },
        h('span', { class: 'share-name' }, r.name, r.you ? h('span', { class: 'share-you' }, T.you) : null),
        h('span', { class: 'share-sub', title: r.sub }, r.sub),
      ),
      roleCell,
      action,
    );
  };

  const render = () => {
    if (!list) return;
    const view = peopleView(list, me, mayShare);
    replaceChildren(storage, icon('server', 16), h('span', null, h('b', null, view.storage.lead), view.storage.detail));
    storage.hidden = false;
    count.textContent = view.count;
    replaceChildren(people, view.rows.map(personRow));
    policy.hidden = false;
    policy.textContent = view.policy;
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
