import '../../styles/invite.css';
import type { AppContext } from '../../app/context';
import { InvitationAcceptance, type AcceptState } from '../../app/cloud/acceptance';
import { dropInvitation, pendingInvitation } from '../../app/cloud/invitationLink';
import { acceptedHow } from '../../app/cloud/invitations';
import { workspaceName } from '../../app/cloud/session';
import { ROLE_LABEL, failureText } from '../../app/cloud/sharing';
import type { InvitationAccepted } from '../../contracts/generated/InvitationAccepted';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';

/**
 * “Projeye davet” (docs/adr/0035, 0042): the window of an invitation's link.
 * Signed out, it asks for the invited address's account first (the token
 * waits in this tab through the sign-in, also one that leaves the page);
 * signed in, it accepts the invitation once and says what it gave — the
 * project, its workspace, the role, guest or member — and offers to open
 * it. A refusal is said in the server's own words, with another account's
 * sign-in where that could get past it. Closing the window forgets the
 * token; the link itself works until it is used, withdrawn or out of date.
 */
export function openInvitationDialog(ctx: AppContext): void {
  const token = pendingInvitation();
  if (!token) return;
  const cloud = ctx.cloud;
  const acceptance = new InvitationAcceptance({ auth: cloud.auth, api: cloud.api, token, drop: () => dropInvitation() });
  /** A sign-in window takes over: the token stays for the window that comes back after it. */
  let handover = false;
  let opening: AbortController | null = null;
  /** Subscriptions ended with the window. */
  const stops: (() => void)[] = [];

  const body = h('div', { class: 'invite-accept', 'aria-live': 'polite' });
  const bar = h('span');
  const progress = h('div', { class: 'cloud-progress', hidden: true }, bar);
  const status = h('p', { class: 'cloud-status', role: 'status' });
  const other = h('button', { class: 'btn btn--ghost', type: 'button', hidden: true }, 'Başka hesapla giriş yap');
  const close = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const signIn = h('button', { class: 'btn btn--primary', type: 'button', hidden: true }, 'Giriş yap');
  const retry = h('button', { class: 'btn btn--primary', type: 'button', hidden: true }, 'Yeniden dene');
  const open = h('button', { class: 'btn btn--primary', type: 'button', hidden: true }, 'Projeyi aç');

  const dialog = new Dialog({
    title: 'Projeye davet',
    width: 520,
    className: 'dialog--cloud dialog--invite',
    // Over whatever the start asked (a recovery copy's question stays).
    stack: true,
    content: [body, progress, status],
    footer: [other, h('div', { class: 'dialog__spacer' }), close, signIn, retry, open],
    onClose: () => {
      for (const stop of stops) stop();
      acceptance.dispose();
      opening?.abort();
      if (!handover) dropInvitation();
    },
  });

  const lead = (iconName: string, title: string, ...text: Child[]) =>
    h('div', { class: 'invite-accept__lead' }, icon(iconName, 20), h('div', null, h('p', { class: 'invite-accept__title' }, title), ...text));
  const note = (text: string) => h('p', { class: 'cloud-hint' }, text);
  const who = () => {
    const me = cloud.me.value?.user;
    return me ? note(`Giriş yapan: ${me.displayName}${me.email ? ` (${me.email})` : ' (hesapta e-posta yok)'}.`) : null;
  };
  const buttons = (shown: HTMLButtonElement[], closeText = 'Vazgeç') => {
    for (const b of [other, signIn, retry, open]) b.hidden = !shown.includes(b);
    close.textContent = closeText;
    (shown.find((b) => b !== other) ?? close).focus();
  };

  const facts = (a: InvitationAccepted) => {
    const row = (term: string, value: string) => [h('dt', null, term), h('dd', null, value)];
    return h(
      'dl',
      { class: 'invite-accept__facts' },
      row('Proje', a.projectName),
      row('Çalışma alanı', workspaceName(a.tenantKind, a.tenantName, !!cloud.membership(a.tenantId))),
      row('Rolünüz', ROLE_LABEL[a.role]),
      row('Erişim', acceptedHow(a)),
    );
  };

  const paint = (s: AcceptState) => {
    status.textContent = '';
    delete status.dataset.kind;
    switch (s.kind) {
      case 'waiting': {
        const away = ctx.server.state.value === 'offline';
        replaceChildren(
          body,
          lead('cloud', 'Davet bağlantısı açıldı', note(away ? 'Sunucuya ulaşılamıyor; bağlantı gelince davet kabul edilir.' : 'Sunucuya bağlanılıyor…')),
        );
        return buttons([]);
      }
      case 'signin':
        replaceChildren(
          body,
          lead(
            'signIn',
            'Bir projeye davet edildiniz',
            note('Daveti kabul etmek için, davetin gönderildiği e-posta adresinin hesabıyla giriş yapın. Hesabın e-postası doğrulanmış olmalıdır.'),
          ),
          note('Bağlantı yalnız bir kez kullanılır; siz kabul edene ya da bu pencereyi kapatana kadar bu sekmede bekler.'),
        );
        return buttons([signIn]);
      case 'accepting':
        replaceChildren(body, lead('share', 'Davet kabul ediliyor…'));
        return buttons([]);
      case 'accepted':
        replaceChildren(
          body,
          lead('success', s.result.role === 'owner' ? `“${s.result.projectName}” sizin projeniz` : `“${s.result.projectName}” projesine erişiminiz var`),
          facts(s.result),
          s.result.guest ? note('Misafir olarak kurumun proje listesini, üyelerini ve öteki projelerini görmezsiniz. Proje, “Bulut projeleri” penceresinde “Projelerim”de durur.') : null,
        );
        return buttons([open], 'Kapat');
      case 'refused':
        replaceChildren(
          body,
          h('div', { class: 'invite-accept__error', role: 'alert' }, icon('warning', 18), h('p', null, s.failure.message)),
          s.another ? who() : null,
          s.another ? note('Davetin gönderildiği adresin hesabıyla girerseniz davet yeniden denenir.') : null,
        );
        return buttons(s.another ? [other] : [], 'Kapat');
      case 'failed':
        replaceChildren(body, h('div', { class: 'invite-accept__error', role: 'alert' }, icon('warning', 18), h('p', null, `Davet kabul edilemedi: ${s.message} Bağlantınızı denetleyip yeniden deneyin.`)));
        return buttons([retry], 'Kapat');
    }
  };

  /** A sign-in window (it closes this one); this window comes back after it, signed in or not. */
  const signInFirst = async (signOut: boolean) => {
    handover = true;
    const again = () => openInvitationDialog(ctx);
    try {
      if (signOut) await cloud.signOut();
      const m = await import('./LoginDialog');
      m.openLoginDialog(ctx, again, again);
    } catch (e) {
      handover = false;
      status.dataset.kind = 'error';
      status.textContent = failureText(e, 'Giriş penceresi açılamadı');
    }
  };

  const openProject = async () => {
    const s = acceptance.state.value;
    if (s.kind !== 'accepted' || opening) return;
    const abort = (opening = new AbortController());
    open.disabled = true;
    progress.hidden = false;
    bar.style.width = '0%';
    status.textContent = `“${s.result.projectName}” açılıyor…`;
    try {
      const ok = await cloud.open(s.result.tenantId, s.result.projectId, (done, total) => {
        bar.style.width = `${total ? Math.round((done / total) * 100) : 100}%`;
        status.textContent = `${done.toLocaleString('tr-TR')} / ${total.toLocaleString('tr-TR')} nesne`;
      }, abort.signal);
      if (ok) dialog.close();
      else if (!abort.signal.aborted) {
        // Stopped in the opening window, or it could not be read (said in the log).
        progress.hidden = true;
        status.textContent = '';
      }
    } catch (e) {
      if (abort.signal.aborted) return;
      progress.hidden = true;
      status.dataset.kind = 'error';
      status.textContent = failureText(e, 'Proje açılamadı');
    } finally {
      opening = null;
      open.disabled = false;
    }
  };

  signIn.addEventListener('click', () => void signInFirst(false));
  other.addEventListener('click', () => void signInFirst(true));
  retry.addEventListener('click', () => acceptance.retry());
  open.addEventListener('click', () => void openProject());
  close.addEventListener('click', () => dialog.close());
  stops.push(
    acceptance.state.subscribe(paint),
    ctx.server.state.subscribe(() => acceptance.state.value.kind === 'waiting' && paint(acceptance.state.value)),
  );
  paint(acceptance.state.value);
  acceptance.start();
}
