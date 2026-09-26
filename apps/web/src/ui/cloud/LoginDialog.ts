import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { h } from '../dom';
import { Dialog } from '../widgets/Dialog';

/**
 * "Buluta giriş": a local account (login and password) and, when the server
 * has one, the organisation's OpenID sign-in (a full-page redirect that
 * comes back to this page). `then` runs after a successful sign-in, so a
 * command that needed one (open a cloud project) goes on by itself;
 * `cancelled` when the window closes without one (an invitation's window
 * comes back).
 */
export function openLoginDialog(ctx: AppContext, then?: () => void, cancelled?: () => void): void {
  const cloud = ctx.cloud;
  const config = cloud.config.value;
  const login = h('input', { class: 'field', name: 'login', autocomplete: 'username', spellcheck: 'false', 'aria-label': 'Giriş adı' });
  const password = h('input', { class: 'field', name: 'password', type: 'password', autocomplete: 'current-password', 'aria-label': 'Parola' });
  const error = h('p', { class: 'cloud-error', role: 'alert', hidden: true });
  const submit = h('button', { class: 'btn btn--primary', type: 'submit' }, 'Giriş yap');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const local = config?.local !== false;
  const form = h(
    'form',
    { class: 'cloud-form', novalidate: true },
    local ? h('label', { class: 'cloud-field' }, h('span', null, 'Giriş adı'), login) : null,
    local ? h('label', { class: 'cloud-field' }, h('span', null, 'Parola'), password) : null,
    error,
  );
  const oidc = config?.oidc
    ? h('button', { class: 'btn cloud-oidc', type: 'button' }, config.oidc.label)
    : null;
  let signedIn = false;
  const dialog = new Dialog({
    title: 'Buluta giriş',
    width: 400,
    className: 'dialog--cloud',
    content: [form, oidc ? h('div', { class: 'cloud-or' }, local ? h('span', null, 'ya da') : null, oidc) : null],
    footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, local ? submit : null],
    onClose: () => signedIn || cancelled?.(),
  });
  const busy = (on: boolean) => {
    submit.disabled = on;
    submit.textContent = on ? 'Giriş yapılıyor…' : 'Giriş yap';
  };
  const show = (text: string) => {
    error.textContent = text;
    error.hidden = !text;
  };
  const go = async () => {
    if (!login.value.trim() || !password.value) {
      show('Giriş adını ve parolayı yazın.');
      (login.value.trim() ? password : login).focus();
      return;
    }
    busy(true);
    show('');
    try {
      const me = await cloud.signIn(login.value.trim(), password.value);
      signedIn = true;
      dialog.close();
      ctx.log.success(`${me.user.displayName} olarak giriş yapıldı.`);
      then?.();
    } catch (e) {
      busy(false);
      show(e instanceof ApiFailure ? e.message : 'Giriş yapılamadı.');
      password.select();
    }
  };
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    void go();
  });
  // The button sits in the dialog's footer, outside the form, so Enter does not submit by itself.
  for (const field of [login, password])
    field.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter') return;
      e.preventDefault();
      if (field === login && !password.value) password.focus();
      else void go();
    });
  submit.addEventListener('click', (e) => {
    e.preventDefault();
    void go();
  });
  cancel.addEventListener('click', () => dialog.close());
  oidc?.addEventListener('click', () => {
    const back = `${location.pathname}${location.search}`;
    location.assign(`${config!.oidc!.startUrl}?returnTo=${encodeURIComponent(back)}`);
  });
  (local ? login : oidc)?.focus();
}
