// The platform administrators' overview, record and settings (docs/adr/9000 §3): the platform's figures, professionals
// waiting for their check, what was done lately and whether professionals may sign up.
import { platform } from '../../api/portal';
import type { PlatformEventView } from '../../api/types';
import { professionLabel } from '../../app/state';
import { h } from '../../lib/dom';
import { initials, moment, since } from '../../lib/format';
import { checked, form, formError, switchField } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { avatar, button, empty, notice, pageHeader, panel, stat } from '../../ui/kit';
import { toast } from '../../ui/toast';
import type { PageFn } from '../page';
import { reviewProfessional } from './people';

function eventIcon(e: PlatformEventView) {
  return icon(e.targetKind === 'tenant' ? 'building' : e.targetKind === 'professional' ? 'badge' : e.targetKind === 'setting' ? 'settings' : e.targetKind === 'file' ? 'shield' : 'user', 16);
}

export function eventList(events: PlatformEventView[]): HTMLElement {
  if (!events.length) return h('div', { class: 'panel-body' }, empty('list', 'Kayıt yok', 'Kurum, hesap ve meslek mensubu işlemleri burada tarihiyle görünür.'));
  return h('div', null, ...events.map((e) => h('div', { class: 'event-line' }, h('span', { class: 'when num' }, moment(e.at)), h('span', { class: 'kind-icon' }, eventIcon(e)), h('span', null, e.text))));
}

export const adminDashboard: PageFn = async ({ reload }) => {
  const [s, pending, events] = await Promise.all([platform.summary(), platform.professionals('pending'), platform.events(8)]);
  const pendingList = pending.length
    ? h(
        'ul',
        { class: 'list-plain' },
        ...pending.slice(0, 6).map((p) =>
          h(
            'li',
            null,
            avatar(p.displayName, initials(p.displayName), 'sm'),
            h('div', { class: 'grow two-line' }, h('div', { class: 'two-line-main' }, p.displayName), h('div', { class: 'two-line-sub' }, `${professionLabel(p.profession)}, sicil no ${p.chamberNo}, ${since(p.createdAt)} kayıt oldu`)),
            button('İncele', { kind: 'secondary', onClick: () => reviewProfessional(p, reload) }),
          ),
        ),
      )
    : h('div', { class: 'panel-body' }, empty('check', 'Bekleyen yok', 'Doğrulama bekleyen meslek mensubu kalmadı.'));
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Sistem yönetimi', lead: 'Platformdaki kurumlar, hesaplar ve meslek mensupları; yapılan her işlemin kaydı.', actions: [button('Yeni kurum', { kind: 'primary', icon: 'plus', href: '#/sistem/kurumlar?yeni=1' })] }),
    h(
      'div',
      { class: 'stats' },
      stat({ label: 'Belediyeler', value: s.municipalities, icon: 'building', hint: `${s.organisations} kurum, ${s.suspendedOrganisations} askıda`, href: '#/sistem/kurumlar' }),
      stat({ label: 'Meslek mensupları', value: s.professionals, icon: 'badge', hint: 'Doğrulanmış kayıtlar', href: '#/sistem/meslek-mensuplari?durum=verified' }),
      stat({ label: 'Doğrulama bekleyen', value: s.pendingProfessionals, icon: 'hourglass', tone: s.pendingProfessionals ? 'warning' : undefined, href: '#/sistem/meslek-mensuplari' }),
      stat({ label: 'Açık dosyalar', value: s.openDossiers, icon: 'inbox', hint: `Bu ay ${s.dossiersThisMonth} dosya gönderildi` }),
      stat({ label: 'Hesaplar', value: s.accounts, icon: 'users', hint: `${s.disabledAccounts} kapalı, ${s.activeSessions} açık oturum`, href: '#/sistem/hesaplar' }),
      stat({ label: 'Koltuklar', value: `${s.seatsUsed} / ${s.seatLimit}`, icon: 'grid', hint: `Kullanılan / verilen; ${s.projects} bulut projesi` }),
    ),
    h(
      'div',
      { class: 'cols-even' },
      panel('Doğrulama bekleyen meslek mensupları', pendingList, { class: 'panel-flush', actions: [button('Tümü', { kind: 'ghost', href: '#/sistem/meslek-mensuplari' })] }),
      h(
        'div',
        { class: 'stack' },
        panel(
          'Meslek mensubu kaydı',
          s.signupOpen
            ? notice('success', 'Kayıt açık.', 'Mimarlar ve mühendisler kendi hesaplarını açabiliyor; doğrulanana kadar başvuru gönderemezler.')
            : notice('info', 'Kayıt kapalı.', 'Meslek mensubu hesaplarını yalnız yöneticiler açıyor.'),
          { actions: [button('Ayarlar', { kind: 'ghost', href: '#/sistem/ayarlar' })] },
        ),
        panel('Son işlemler', eventList(events), { class: 'panel-flush', actions: [button('Denetim kaydı', { kind: 'ghost', href: '#/sistem/kayit' })] }),
      ),
    ),
  );
  return { crumbs: [{ label: 'Sistem yönetimi' }], body };
};

export const adminEvents: PageFn = async () => {
  const events = await platform.events(300);
  const body = h('div', { class: 'page' }, pageHeader({ title: 'Denetim kaydı', lead: 'Kurumların, hesapların ve meslek mensubu kayıtlarının her değişikliği ve taramanın geri çevirdiği dosyalar: kim, ne zaman, ne yaptı. Kayıt silinmez.' }), panel(null, eventList(events), { class: 'panel-flush' }));
  return { crumbs: [{ label: 'Sistem yönetimi', href: '#/sistem' }, { label: 'Denetim kaydı' }], body };
};

export const adminSettings: PageFn = async ({ reload }) => {
  const s = await platform.settings();
  const f = form(
    [
      switchField({
        label: 'Meslek mensupları kendi hesaplarını açabilsin',
        name: 'signup',
        checked: s.signupOpen,
        hint: 'Açıkken giriş sayfasında “Meslek mensubu olarak kayıt olun” görünür. Açılan hesap doğrulanana kadar başvuru gönderemez, iş alamaz; e-postası da doğrulanmış sayılmaz.',
      }),
      formError(),
      h('div', { class: 'form-actions' }, button('Kaydet', { kind: 'primary', type: 'submit' })),
    ],
    async (el) => {
      await platform.setSettings(checked(el, 'signup'));
      toast('Platform ayarları kaydedildi.');
      reload();
    },
  );
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Platform ayarları', lead: 'Bütün kurumlar için geçerli ayarlar. Her değişiklik denetim kaydına geçer.' }),
    h('div', { class: 'cols-2' }, panel('Kayıt', f), panel('Giriş', h('div', { class: 'stack', style: 'gap: 8px' }, h('div', null, 'Şimdilik yerel hesaplarla giriş yapılıyor (giriş adı ve parola).'), h('div', { class: 'muted' }, 'Kurumun kendi kimlik sunucusuyla giriş (OpenID Connect) sunucuda hazır; portala sonraki adımda eklenecek.')))),
  );
  return { crumbs: [{ label: 'Sistem yönetimi', href: '#/sistem' }, { label: 'Platform ayarları' }], body };
};
