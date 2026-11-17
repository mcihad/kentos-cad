// Organisations as the platform administrators manage them (docs/adr/9000 §3): every municipality and office with its
// seats, members, projects and dossiers; opening one with its first owner; its particulars, seats and suspension.
import { platform } from '../../api/portal';
import type { OrganizationCategory, PlatformTenant } from '../../api/types';
import { go } from '../../app/router';
import { categoryLabel, roleLabel } from '../../app/state';
import { h } from '../../lib/dom';
import { day, initials, slugOf } from '../../lib/format';
import { openDialog } from '../../ui/dialog';
import { checked, form, formError, grid, orNull, selectField, switchField, textField, values } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { avatar, badge, button, empty, facts, pageHeader, panel, table, twoLine } from '../../ui/kit';
import { toast } from '../../ui/toast';
import type { PageFn } from '../page';

const CATEGORIES = [
  { value: 'municipality', label: 'Belediye' },
  { value: 'office', label: 'Büro' },
  { value: 'other', label: 'Diğer kurum' },
];

function tenantStatus(t: PlatformTenant) {
  return t.status === 'active' ? badge('success', 'Etkin') : badge('danger', 'Askıda');
}

function newTenant(): void {
  const f = form(
    [
      h(
        'div',
        { class: 'form-section' },
        h('h3', null, 'Kurum'),
        grid(
          textField({ label: 'Kurum adı', name: 'name', required: true, placeholder: 'Örnek Belediyesi' }),
          textField({ label: 'Kısa ad', name: 'slug', required: true, hint: 'Adreslerde görünür: küçük harf, rakam ve tire.' }),
          selectField({ label: 'Tür', name: 'category', value: 'municipality', options: CATEGORIES }),
          textField({ label: 'Koltuk', name: 'seats', type: 'number', value: '10', hint: 'Aynı anda çalışabilecek üye sayısı.' }),
          textField({ label: 'İl', name: 'province' }),
          textField({ label: 'İlçe', name: 'district' }),
        ),
      ),
      h(
        'div',
        { class: 'form-section' },
        h('h3', null, 'İlk sahip'),
        h('p', { class: 'field-hint', style: 'margin: 0' }, 'Kurumun sahibi üyeleri ekler ve kurumun ayarlarını yönetir. Yeni bir hesap açılır.'),
        grid(
          textField({ label: 'Ad soyad', name: 'owner.displayName', required: true }),
          textField({ label: 'E-posta', name: 'owner.email', type: 'email' }),
          textField({ label: 'Giriş adı', name: 'owner.login', required: true }),
          textField({ label: 'İlk parola', name: 'owner.password', type: 'password', required: true, hint: 'En az 10 karakter; kişiye ayrıca iletin.' }),
        ),
      ),
      formError(),
    ],
    async (el) => {
      const v = values(el);
      const made = await platform.createTenant({
        slug: v.slug,
        name: v.name,
        category: v.category,
        province: orNull(v.province),
        district: orNull(v.district),
        seats: Number(v.seats || 0),
        owner: { userId: null, login: v['owner.login'], password: v['owner.password'], displayName: v['owner.displayName'], email: orNull(v['owner.email']) },
      });
      dlg.close();
      toast('Kurum açıldı.');
      go(`#/sistem/kurumlar/${made.id}`);
    },
  );
  const name = f.querySelector<HTMLInputElement>('input[name=name]');
  const slug = f.querySelector<HTMLInputElement>('input[name=slug]');
  let touched = false;
  slug?.addEventListener('input', () => (touched = true));
  name?.addEventListener('input', () => {
    if (!touched && slug) slug.value = slugOf(name.value);
  });
  const dlg = openDialog({ title: 'Yeni kurum', wide: true, body: f, footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button('Kurumu aç', { kind: 'primary', onClick: () => f.requestSubmit() })] });
}

export const adminTenants: PageFn = async ({ query }) => {
  const tenants = await platform.tenants();
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Kurumlar', lead: 'Belediyeler ve bürolar; koltukları, üyeleri, projeleri ve dosyalarıyla.', actions: [button('Yeni kurum', { kind: 'primary', icon: 'plus', onClick: () => newTenant() })] }),
    table(
      [
        { label: 'Kurum', cell: (t) => h('div', { class: 'with-icon' }, h('span', { class: 'kind-icon' }, icon('building', 16)), twoLine(t.name, t.slug)) },
        { label: 'Tür', cell: (t) => categoryLabel(t.category), class: 'narrow' },
        { label: 'Yer', cell: (t) => [t.province, t.district].filter(Boolean).join(', ') || '—' },
        { label: 'Durum', cell: tenantStatus, class: 'narrow' },
        { label: 'Koltuk', cell: (t) => h('span', { class: 'num' }, `${t.seatsUsed} / ${t.seatLimit}`), class: 'narrow right' },
        { label: 'Üye', cell: (t) => h('span', { class: 'num' }, String(t.members)), class: 'narrow right' },
        { label: 'Proje', cell: (t) => h('span', { class: 'num' }, String(t.projects)), class: 'narrow right' },
        { label: 'Dosya', cell: (t) => h('span', { class: 'num' }, String(t.dossiers)), class: 'narrow right' },
        { label: 'Başvuru', cell: (t) => (t.category === 'municipality' ? (t.acceptsApplications ? 'Alıyor' : 'Almıyor') : '—'), class: 'narrow' },
        { label: 'Açılış', cell: (t) => h('span', { class: 'muted num' }, day(t.createdAt)), class: 'narrow' },
      ],
      tenants,
      { href: (t) => `#/sistem/kurumlar/${t.id}`, empty: empty('building', 'Kurum yok', 'İlk belediyeyi Yeni kurum ile açın.') },
    ),
  );
  if (query.get('yeni') === '1') queueMicrotask(() => newTenant());
  return { crumbs: [{ label: 'Sistem yönetimi', href: '#/sistem' }, { label: 'Kurumlar' }], body };
};

export const adminTenant: PageFn = async ({ params, reload }) => {
  const { tenant: t, members } = await platform.tenant(params.id);
  const f = form(
    [
      grid(
        textField({ label: 'Kurum adı', name: 'name', value: t.name, required: true }),
        selectField({ label: 'Tür', name: 'category', value: t.category, options: CATEGORIES }),
        textField({ label: 'İl', name: 'province', value: t.province }),
        textField({ label: 'İlçe', name: 'district', value: t.district }),
        textField({ label: 'Koltuk', name: 'seats', type: 'number', value: String(t.seatLimit), hint: `${t.seatsUsed} koltuk kullanılıyor; bundan aza inmez.` }),
        h('div'),
      ),
      switchField({ label: 'Kurum askıda', name: 'suspended', checked: t.status === 'suspended', hint: 'Askıdaki kurumun üyeleri kurumun sayfalarını ve projelerini açamaz; veriler silinmez.' }),
      formError(),
      h('div', { class: 'form-actions' }, button('Kaydet', { kind: 'primary', type: 'submit' })),
    ],
    async (el) => {
      const v = values(el);
      await platform.updateTenant(t.id, {
        name: v.name,
        category: v.category as OrganizationCategory,
        province: orNull(v.province),
        district: orNull(v.district),
        seats: Number(v.seats || 0),
        status: checked(el, 'suspended') ? 'suspended' : 'active',
      });
      toast('Kurum kaydedildi.');
      reload();
    },
  );
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: t.name, lead: `${categoryLabel(t.category)}, kısa adı ${t.slug}. ${day(t.createdAt)} açıldı.`, back: { label: 'Kurumlar', href: '#/sistem/kurumlar' }, actions: [tenantStatus(t)] }),
    h(
      'div',
      { class: 'cols-2' },
      h(
        'div',
        { class: 'stack' },
        panel('Bilgiler', f),
        panel(
          'Üyeler',
          table(
            [
              { label: 'Ad', cell: (m) => h('div', { class: 'party' }, avatar(m.displayName, initials(m.displayName), 'sm'), twoLine(m.displayName, m.email ?? '')) },
              { label: 'Rol', cell: (m) => roleLabel(m.role) },
              { label: 'Koltuk', cell: (m) => (m.seat ? 'Var' : 'Yok'), class: 'narrow' },
              { label: 'Durum', cell: (m) => (m.status === 'active' ? badge('success', 'Etkin') : badge('neutral', 'Kapalı')), class: 'narrow' },
            ],
            members,
            { empty: empty('users', 'Üye yok', 'Kurumun sahibi üyeleri kurumun Üyeler sayfasından ekler.') },
          ),
          { class: 'panel-flush', note: 'Üyeleri kurumun sahibi ve yöneticileri yönetir.' },
        ),
      ),
      panel(
        'Kullanım',
        facts([
          ['Koltuk', h('span', { class: 'num' }, `${t.seatsUsed} / ${t.seatLimit}`)],
          ['Üye', h('span', { class: 'num' }, String(t.members))],
          ['Bulut projesi', h('span', { class: 'num' }, String(t.projects))],
          ['Dosya', h('span', { class: 'num' }, String(t.dossiers))],
          ['Başvuru alımı', t.category === 'municipality' ? (t.acceptsApplications ? 'Açık' : 'Kapalı') : 'Belediye değil'],
        ]),
      ),
    ),
  );
  return { crumbs: [{ label: 'Sistem yönetimi', href: '#/sistem' }, { label: 'Kurumlar', href: '#/sistem/kurumlar' }, { label: t.name }], body };
};
