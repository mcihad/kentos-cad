// The account itself (docs/adr/9000 §12), the same for everyone on the portal: the password, the sessions open on
// browsers and devices, and the personal access tokens a script or an AI agent signs in with.
import { account, portal } from '../api/portal';
import type { ApiTokenCreated, ApiTokenView, SessionView, TokenScope } from '../api/types';
import { h } from '../lib/dom';
import { moment, since } from '../lib/format';
import { form, formError, selectField, textField, values } from '../ui/forms';
import { confirm, openDialog } from '../ui/dialog';
import { type IconName, icon } from '../ui/icons';
import { badge, button, empty, pageHeader, panel, table, twoLine } from '../ui/kit';
import { toast } from '../ui/toast';
import type { PageFn } from './page';

/** What a session is, as its owner knows it: the browser, or the program and its device. */
function sessionName(s: SessionView): { icon: IconName; name: string; sub: string } {
  const how = s.method === 'oidc' ? 'kurum kimliğiyle' : s.method === 'token' ? 'erişim anahtarıyla' : 'parolayla';
  if (s.kind === 'cookie') return { icon: 'globe', name: 'Tarayıcı', sub: `Web portalı, ${how}` };
  const kind: Record<string, [IconName, string]> = {
    mobile: ['phone', 'Mobil uygulama'],
    desktop: ['monitor', 'KentOS masaüstü'],
    agent: ['robot', 'Yapay zekâ ajanı'],
    mcp: ['robot', 'Yapay zekâ ajanı'],
    python: ['key', 'Betik'],
  };
  const [ic, name] = kind[s.client ?? ''] ?? (['key', 'Program'] as [IconName, string]);
  return { icon: ic, name: s.label ? `${s.label}` : name, sub: s.label ? `${name}, ${how}` : how };
}

function sessionsPanel(list: SessionView[], reload: () => void): HTMLElement {
  return panel(
    'Açık oturumlar',
    table(
      [
        {
          label: 'Nerede',
          cell: (s) => {
            const n = sessionName(s);
            return h('div', { class: 'with-icon' }, h('span', { class: 'kind-icon' }, icon(n.icon, 18)), twoLine(h('span', null, n.name, s.current ? h('span', { style: 'margin-left: 8px' }, badge('info', 'Bu oturum')) : null), n.sub));
          },
        },
        { label: 'Açıldı', cell: (s) => h('span', { class: 'num muted' }, moment(s.createdAt)), class: 'narrow' },
        { label: 'Son kullanım', cell: (s) => h('span', { class: 'num muted' }, since(s.lastSeenAt)), class: 'narrow' },
        { label: 'En geç bitiş', cell: (s) => h('span', { class: 'num muted' }, moment(s.expiresAt)), class: 'narrow' },
        {
          label: '',
          class: 'narrow',
          cell: (s) =>
            s.current
              ? null
              : button('Kapat', {
                  kind: 'ghost',
                  icon: 'logout',
                  onClick: async () => {
                    const ok = await confirm({
                      title: 'Oturum kapatılsın mı?',
                      text: `${sessionName(s).name} hemen çıkış yapmış sayılır; yeniden girmesi gerekir.`,
                      confirm: 'Oturumu kapat',
                      danger: true,
                    });
                    if (!ok) return;
                    await account.endSession(s.id);
                    toast('Oturum kapatıldı.');
                    reload();
                  },
                }),
        },
      ],
      list,
      { empty: empty('lock', 'Açık oturum yok', 'Giriş yaptığınız tarayıcılar ve cihazlar burada görünür.') },
    ),
    { note: 'Kaybolan bir telefonun ya da bilmediğiniz bir cihazın oturumunu buradan kapatın.' },
  );
}

const SCOPE: Record<TokenScope, { tone: 'info' | 'warning'; label: string }> = {
  read: { tone: 'info', label: 'Yalnız okur' },
  write: { tone: 'warning', label: 'Okur ve yazar' },
};

/** The new token, shown this once, with what to do with it. */
function reveal(made: ApiTokenCreated): void {
  const value = h('code', { class: 'token-value mono' }, made.token);
  const copy = button('Kopyala', {
    icon: 'copy',
    onClick: async () => {
      try {
        await navigator.clipboard.writeText(made.token);
        toast('Anahtar panoya kopyalandı.');
      } catch {
        const range = document.createRange();
        range.selectNodeContents(value);
        getSelection()?.removeAllRanges();
        getSelection()?.addRange(range);
      }
    },
  });
  const d = openDialog({
    title: 'Anahtarınız hazır',
    lead: 'Bu anahtar bir daha gösterilmeyecek. Şimdi kopyalayıp programın ya da ajanın ayarına koyun.',
    body: h(
      'div',
      { class: 'stack', style: 'gap: 12px' },
      h('div', { class: 'token-box' }, value, copy),
      h(
        'div',
        { class: 'field-hint' },
        'Her istekte ',
        h('code', { class: 'mono' }, 'Authorization: Bearer <anahtar>'),
        ' başlığıyla gönderilir. API’nin açıklaması: ',
        h('a', { href: '/v1/openapi.json', target: '_blank', rel: 'noopener' }, '/v1/openapi.json'),
        ', yapay zekâ ajanları için kısa yol: ',
        h('a', { href: '/v1/llms.txt', target: '_blank', rel: 'noopener' }, '/v1/llms.txt'),
        '.',
      ),
      h('pre', { class: 'code-sample mono' }, `curl -H "Authorization: Bearer ${made.view.prefix}…" ${location.origin}/v1/me`),
    ),
    footer: [button('Kopyaladım, kapat', { kind: 'primary', onClick: () => d.close() })],
  });
}

function tokensPanel(list: ApiTokenView[], reload: () => void): HTMLElement {
  const make = form(
    [
      h(
        'div',
        { class: 'form-grid three' },
        textField({ label: 'Adı', name: 'name', required: true, placeholder: 'Rapor ajanı, yedekleme betiği…', maxlength: 80 }),
        selectField({
          label: 'Yetkisi',
          name: 'scope',
          value: 'read',
          options: [
            { value: 'read', label: 'Yalnız okur' },
            { value: 'write', label: 'Okur ve yazar' },
          ],
        }),
        selectField({
          label: 'Geçerlilik',
          name: 'days',
          value: '30',
          options: [
            { value: '7', label: '7 gün' },
            { value: '30', label: '30 gün' },
            { value: '90', label: '90 gün' },
            { value: '365', label: '1 yıl' },
          ],
        }),
      ),
      formError(),
      h('div', { class: 'form-actions' }, button('Anahtar oluştur', { kind: 'primary', icon: 'key', type: 'submit' })),
    ],
    async (el) => {
      const v = values(el);
      const made = await account.createToken(v.name.trim(), v.scope as TokenScope, Number(v.days));
      el.reset();
      reload();
      reveal(made);
    },
  );
  return panel(
    'Erişim anahtarları',
    h(
      'div',
      { class: 'stack', style: 'gap: 0' },
      h(
        'p',
        { class: 'panel-text' },
        'Betiklerin ve yapay zekâ ajanlarının sizin adınıza API’ye bağlanması için. “Yalnız okur” anahtarı hiçbir şeyi değiştiremez; anahtar yeni anahtar üretemez, parolanızı değiştiremez. En çok 20 anahtar.',
      ),
      table(
        [
          { label: 'Anahtar', cell: (t) => twoLine(t.name, h('span', { class: 'mono' }, `${t.prefix}…`)) },
          { label: 'Yetkisi', cell: (t) => badge(SCOPE[t.scope].tone, SCOPE[t.scope].label), class: 'narrow' },
          { label: 'Son kullanım', cell: (t) => h('span', { class: 'num muted' }, t.lastUsedAt ? since(t.lastUsedAt) : 'Hiç'), class: 'narrow' },
          { label: 'Bitiş', cell: (t) => h('span', { class: 'num muted' }, moment(t.expiresAt)), class: 'narrow' },
          {
            label: '',
            class: 'narrow',
            cell: (t) =>
              button('Geri al', {
                kind: 'ghost',
                icon: 'trash',
                onClick: async () => {
                  const ok = await confirm({
                    title: '“' + t.name + '” geri alınsın mı?',
                    text: 'Bu anahtarı kullanan program ya da ajan hemen bağlanamaz olur.',
                    confirm: 'Anahtarı geri al',
                    danger: true,
                  });
                  if (!ok) return;
                  await account.revokeToken(t.id);
                  toast('Anahtar geri alındı.');
                  reload();
                },
              }),
          },
        ],
        list,
        { empty: empty('key', 'Anahtarınız yok', 'Bir betiğe ya da yapay zekâ ajanına API erişimi vermek için aşağıdan oluşturun.') },
      ),
      h('div', { class: 'panel-section' }, make),
    ),
  );
}

function passwordPanel(): HTMLElement {
  return panel(
    'Parola',
    form(
      [
        textField({ label: 'Şimdiki parola', name: 'current', type: 'password', autocomplete: 'current-password', required: true }),
        textField({ label: 'Yeni parola', name: 'new', type: 'password', autocomplete: 'new-password', required: true, hint: 'En az 10 karakter. Öbür tarayıcı ve cihazlardaki oturumlarınız kapanır; erişim anahtarlarınız kalır.' }),
        formError(),
        h('div', { class: 'form-actions' }, button('Parolayı değiştir', { type: 'submit' })),
      ],
      async (el) => {
        const v = values(el);
        await portal.changePassword(v.current, v.new);
        el.reset();
        toast('Parolanız değişti; öbür oturumlarınız kapandı.');
      },
    ),
  );
}

export const accountPage: PageFn = async ({ reload }) => {
  const [sessions, tokens] = await Promise.all([account.sessions(), account.tokens()]);
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Hesabım', lead: 'Parolanız, giriş yaptığınız tarayıcı ve cihazlar, programların ve yapay zekâ ajanlarının anahtarları.' }),
    h(
      'div',
      { class: 'stack' },
      sessionsPanel(sessions, reload),
      tokensPanel(tokens, reload),
      h('div', { class: 'cols-2' }, passwordPanel(), panel('API', h(
        'div',
        { class: 'stack', style: 'gap: 8px' },
        h('p', { class: 'panel-text' }, 'Masaüstü, mobil uygulama ve ajanlar aynı API’yi kullanır. Uçların tamamı, gövdeleri ve hataları makinenin okuyacağı biçimde:'),
        h('div', { class: 'with-icon' }, icon('file', 16), h('a', { href: '/v1/openapi.json', target: '_blank', rel: 'noopener' }, 'OpenAPI 3.1 açıklaması')),
        h('div', { class: 'with-icon' }, icon('robot', 16), h('a', { href: '/v1/llms.txt', target: '_blank', rel: 'noopener' }, 'Yapay zekâ ajanları için kısa yol (llms.txt)')),
      ))),
    ),
  );
  return { crumbs: [{ label: 'Hesabım' }], body };
};
