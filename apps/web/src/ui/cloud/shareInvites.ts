import '../../styles/invite.css';
import type { AppContext } from '../../app/context';
import {
  INVITATION_STATE_LABEL,
  INVITE_DAY_CHOICES,
  INVITE_DAYS,
  INVITE_ROLES,
  emailProblem,
  invitationLink,
  invitationRevokeEnvelope,
  invitationSub,
  inviteEnvelope,
  inviteExpiry,
  inviteQuestion,
  sortInvitations,
} from '../../app/cloud/invitations';
import {
  INVITE_LINES,
  SHARE_TEXTS,
  dayText,
  invitationInitial,
  invitationRevokeQuestion,
  invitedLink,
  inviteAsk,
  inviteRules,
  inviteTip,
  invitesCount,
} from '../../app/cloud/sharePlan';
import { ROLE_HINT, ROLE_LABEL, failureText } from '../../app/cloud/sharing';
import type { GrantRole } from '../../contracts/generated/GrantRole';
import type { InvitationChange } from '../../contracts/generated/InvitationChange';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectInvitation } from '../../contracts/generated/ProjectInvitation';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { confirmDialog } from '../widgets/confirm';
import type { ProjectTarget } from './ProjectActions';

/**
 * The share dialog's “Davetler” tab (docs/adr/0035, 0042; TODOS.md
 * CLOUD-16, CLOUD-17): an invitation by e-mail — the address, a role up to
 * editor, how long it waits (14 days unless chosen, at most 90) — and the
 * project's invitations: the waiting ones first, then those of the last 30
 * days, each with who invited it when, how it ended or when it stops
 * waiting, and “Geri al” for a waiting one. A new invitation's link is
 * shown here once, with a copy button: the server keeps only its hash, and
 * the link is kept nowhere else (not in the log, not in any storage); the
 * inviter sends it. Every request is the server's to decide (`project.share`).
 * Its words and small rules are sharePlan.ts's.
 */

export interface InvitePanelOptions {
  /** The dialog's status line. */
  say(text: string, kind?: 'info' | 'error'): void;
  /** The dialog is still open. */
  open(): boolean;
}

export interface InvitePanel {
  readonly el: HTMLElement;
  /** What the access list said: whether the account may share, and who can use the project already. */
  setAccess(mayShare: boolean, access: ProjectAccessList | null): void;
  /** Asks the server for the invitations. */
  load(): Promise<void>;
  /** An invitation to `email` begins (from the people tab, where no member was found). */
  prefill(email: string): void;
  focus(): void;
}

const T = SHARE_TEXTS.invites;

export function createInvitePanel(ctx: AppContext, target: ProjectTarget, o: InvitePanelOptions): InvitePanel {
  const api = ctx.cloud.api;
  let mayShare = false;
  let access: ProjectAccessList | null = null;
  let invitations: ProjectInvitation[] = [];
  let busy = false;

  const email = h('input', { class: 'field', type: 'email', placeholder: T.placeholder, 'aria-label': T.emailLabel, autocomplete: 'off', spellcheck: 'false', disabled: true });
  const role = h(
    'select',
    { class: 'field', 'aria-label': T.roleLabel, disabled: true },
    INVITE_ROLES.map((r) => h('option', { value: r, selected: r === 'viewer', title: ROLE_HINT[r] }, ROLE_LABEL[r])),
  );
  const wait = h(
    'select',
    { class: 'field', 'aria-label': T.waitLabel, disabled: true },
    INVITE_DAY_CHOICES.map((d) => h('option', { value: String(d), selected: d === INVITE_DAYS }, dayText(d))),
  );
  const send = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, T.send);
  const roleHint = h('p', { class: 'cloud-hint share-rolehint' }, ROLE_HINT.viewer);
  const link = h('div', { class: 'invite-link', hidden: true });
  const count = h('span', { class: 'share-count' });
  const list = h('div', { class: 'share-list invite-list', role: 'list', 'aria-label': T.listLabel }, h('p', { class: 'cloud-empty' }, T.loading));
  const rules = h('p', { class: 'cloud-hint share-policy' });
  const el = h(
    'div',
    { class: 'share-panel', role: 'tabpanel', 'aria-label': SHARE_TEXTS.tabs.invites },
    h(
      'div',
      { class: 'share-add share-add--invite' },
      h('label', { class: 'cloud-field' }, h('span', null, T.email), email),
      h('label', { class: 'cloud-field' }, h('span', null, T.role), role),
      h('label', { class: 'cloud-field' }, h('span', null, T.wait), wait),
      send,
    ),
    roleHint,
    link,
    h('h3', { class: 'share-title' }, h('span', null, T.title), count),
    list,
    rules,
  );

  const refresh = () => {
    for (const c of [email, role, wait]) c.disabled = !mayShare || busy;
    send.disabled = !mayShare || busy || !email.value.trim();
    send.title = inviteTip(mayShare, email.value);
  };

  // ── The one-time link ───────────────────────────────────────────────

  /** A new invitation's link, shown once: kept in this panel's field only. */
  const showLink = (change: InvitationChange, days: number) => {
    const said = invitedLink(change, days);
    if (!change.token) {
      // A retry's stored answer: the link was in an answer that never arrived.
      replaceChildren(link, h('p', { class: 'invite-link__head' }, icon('warning', 16), h('span', null, said.what)), h('p', { class: 'invite-link__warn' }, said.warn));
      link.hidden = false;
      return;
    }
    const url = h('input', { class: 'field invite-link__url', readonly: true, value: invitationLink(change.token), 'aria-label': T.linkLabel, spellcheck: 'false' });
    const copy = h('button', { class: 'btn', type: 'button' }, icon('copy', 14), T.copy);
    copy.addEventListener('click', () => {
      navigator.clipboard.writeText(url.value).then(
        () => {
          copy.replaceChildren(icon('check', 14), T.copied);
          o.say(T.copiedSay);
        },
        () => {
          url.select();
          o.say(T.copyFailed, 'error');
        },
      );
    });
    url.addEventListener('focus', () => url.select());
    replaceChildren(
      link,
      h('p', { class: 'invite-link__head' }, icon('success', 16), h('span', null, said.what)),
      h('div', { class: 'invite-link__row' }, url, copy),
      h('p', { class: 'invite-link__warn' }, icon('warning', 14), h('span', null, said.warn)),
    );
    link.hidden = false;
    copy.focus();
  };

  // ── Inviting and withdrawing ────────────────────────────────────────

  /** What to ask before sending, when something the address has would change; true to go on. */
  const mayGo = async (address: string): Promise<boolean> => {
    const q = inviteQuestion(address, invitations, access);
    if (!q) return true;
    const ask = inviteAsk(address, q);
    const answer = await confirmDialog({
      title: ask.title,
      message: ask.message,
      details: ask.details,
      answers: [
        { value: 'cancel', label: ask.cancel },
        { value: 'go', label: ask.go, kind: 'primary' },
      ],
      cancel: 'cancel',
    });
    return answer === 'go';
  };

  const invite = async () => {
    if (busy || !mayShare) return;
    const address = email.value.trim();
    const bad = emailProblem(address);
    if (bad) {
      o.say(bad, 'error');
      email.focus();
      return;
    }
    if (!(await mayGo(address)) || !o.open()) return;
    const days = Number(wait.value);
    const grant = role.value as GrantRole;
    busy = true;
    refresh();
    o.say(T.inviting(address));
    try {
      const change = await api.lifecycle<InvitationChange>(inviteEnvelope(target.tenantId, target.projectId, address, grant, inviteExpiry(days)));
      if (!o.open()) return;
      // The log says who was invited, never the link.
      ctx.log.success(INVITE_LINES.invitedLog(target.name, change.invitation.email, change.invitation.role));
      showLink(change, days);
      email.value = '';
      o.say(T.created);
      await load();
    } catch (e) {
      o.say(failureText(e, T.sendFailed), 'error');
      if ((e as { path?: string }).path === 'email') email.focus();
    } finally {
      busy = false;
      refresh();
    }
  };

  const revoke = async (i: ProjectInvitation) => {
    const ask = invitationRevokeQuestion(i.email);
    const sure = await confirmDialog({
      title: ask.title,
      message: ask.message,
      details: ask.details,
      answers: [
        { value: 'cancel', label: ask.cancel },
        { value: 'revoke', label: ask.action, kind: 'danger' },
      ],
      cancel: 'cancel',
    });
    if (sure !== 'revoke' || !o.open()) return;
    o.say(T.revoking(i.email));
    try {
      await api.lifecycle<InvitationChange>(invitationRevokeEnvelope(target.tenantId, target.projectId, i.id));
      ctx.log.success(INVITE_LINES.revokedLog(target.name, i.email));
      o.say(INVITE_LINES.revokedSay(i.email));
      await load();
    } catch (e) {
      o.say(failureText(e, T.revokeFailed), 'error');
      await load();
    }
  };

  // ── The list ────────────────────────────────────────────────────────

  const row = (i: ProjectInvitation): HTMLElement => {
    let action: HTMLElement;
    if (i.state === 'pending' && mayShare) {
      const b = h('button', { class: 'btn btn--ghost btn--small share-remove', type: 'button', 'aria-label': T.revokeLabel(i.email) }, T.revoke);
      b.addEventListener('click', () => void revoke(i));
      action = b;
    } else action = h('span', { class: 'invite-state', dataset: { state: i.state } }, INVITATION_STATE_LABEL[i.state]);
    return h(
      'div',
      { class: 'share-row invite-row', role: 'listitem', dataset: { invitation: i.id, state: i.state } },
      h('span', { class: 'share-avatar', 'aria-hidden': 'true' }, invitationInitial(i.email)),
      h('div', { class: 'share-who' }, h('span', { class: 'share-name' }, i.email), h('span', { class: 'share-sub', title: invitationSub(i) }, invitationSub(i))),
      h('span', { class: 'share-role' }, ROLE_LABEL[i.role], i.state === 'pending' ? h('span', { class: 'invite-state', dataset: { state: 'pending' } }, INVITATION_STATE_LABEL.pending) : null),
      action,
    );
  };

  const render = () => {
    count.textContent = invitesCount(invitations);
    replaceChildren(
      list,
      invitations.length
        ? sortInvitations(invitations).map(row)
        : h('p', { class: 'cloud-empty' }, T.empty),
    );
  };

  const load = async () => {
    if (!mayShare) {
      replaceChildren(list, h('p', { class: 'cloud-empty' }, T.noRight));
      return;
    }
    try {
      const r = await api.invitations(target.tenantId, target.projectId);
      if (!o.open()) return;
      invitations = r.invitations;
      render();
    } catch (e) {
      if (o.open()) replaceChildren(list, h('p', { class: 'cloud-empty', role: 'alert' }, failureText(e, T.readFailed)));
    }
  };

  email.addEventListener('input', refresh);
  email.addEventListener('keydown', (e) => {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    void invite();
  });
  role.addEventListener('change', () => (roleHint.textContent = ROLE_HINT[role.value as GrantRole]));
  send.addEventListener('click', () => void invite());

  return {
    el,
    setAccess(may, list) {
      mayShare = may;
      access = list;
      rules.textContent = inviteRules(list?.tenantKind === 'personal');
      refresh();
    },
    load,
    prefill(address) {
      email.value = address;
      refresh();
    },
    focus: () => (mayShare ? email : el).focus(),
  };
}
