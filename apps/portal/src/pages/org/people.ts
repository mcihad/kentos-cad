// A municipality's people and settings (docs/adr/9000 §3–4): the directory of professionals it works with, its
// members as its owners and admins manage them (roles, seats, closing a membership), and what it says on the portal.
import { organization } from '../../api/portal';
import type { MemberView, TenantRole } from '../../api/types';
import { go, withQuery } from '../../app/router';
import { catalog, chamberOf, kinds, orgBySlug, professionLabel, professionalStatus, roleLabel } from '../../app/state';
import { h } from '../../lib/dom';
import { initials } from '../../lib/format';
import { openDialog } from '../../ui/dialog';
import { checkbox, checkedValues, checked, form, formError, grid, orNull, selectField, switchField, textField, values } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { avatar, badge, button, empty, notice, pageHeader, panel, table, twoLine } from '../../ui/kit';
import { toast } from '../../ui/toast';
import type { PageFn } from '../page';
import { searchBox } from './lists';

export const orgDirectory: PageFn = async ({ params, query }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  const q = query.get('ara') ?? '';
  const profession = query.get('meslek') ?? '';
  const people = await organization.directory(org.tenantId, q || undefined, profession || undefined);
  const manager = ['owner', 'admin', 'project_manager'].includes(org.role);
  const professionSelect = h(
    'select',
    { class: 'input', 'aria-label': 'Meslek', style: 'width: 220px' },
    h('option', { value: '' }, 'Bütün meslekler'),
    ...(catalog.value?.professions ?? []).map((p) => h('option', { value: p.code, selected: p.code === profession }, p.label)),
  );
  professionSelect.addEventListener('change', () => go(withQuery({ meslek: professionSelect.value || null })));
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Meslek mensupları', lead: 'Doğrulanmış mimarlar, harita ve inşaat mühendisleri; kurumunuzla dosyası olanlar da burada. Bir işi buradan aktarabilirsiniz.' }),
    h('div', { class: 'toolbar' }, searchBox(q, 'Ad, sicil no ya da büro adı…'), professionSelect, h('span', { class: 'toolbar-fill' }), h('span', { class: 'muted' }, `${people.length} kişi`)),
    table(
      [
        { label: 'Ad', cell: (p) => h('div', { class: 'party' }, avatar(p.displayName, initials(p.displayName), 'sm'), twoLine(p.displayName, p.officeName ?? '')) },
        { label: 'Meslek', cell: (p) => twoLine(professionLabel(p.profession), chamberOf(p.profession)) },
        { label: 'Sicil no', cell: (p) => h('span', { class: 'num' }, p.chamberNo), class: 'narrow' },
        { label: 'İl', cell: (p) => p.province ?? '—', class: 'narrow' },
        { label: 'Dosyalarımız', cell: (p) => h('span', { class: 'num' }, String(p.dossiers)), class: 'narrow right' },
        { label: 'Kayıt', cell: (p) => badge(professionalStatus(p.status).tone, professionalStatus(p.status).label), class: 'narrow' },
        {
          label: '',
          cell: (p) => (manager && p.status === 'verified' ? button('İş aktar', { kind: 'ghost', icon: 'send', href: `${base}/aktarimlar/yeni?meslek-mensubu=${p.userId}` }) : null),
          class: 'narrow',
        },
      ],
      people,
      { empty: empty('badge', 'Meslek mensubu bulunamadı', 'Aramanızı değiştirin. Yalnız doğrulanmış ve dizinde görünmeyi seçmiş meslek mensupları listelenir.') },
    ),
  );
  return { crumbs: [{ label: org.tenantName, href: base }, { label: 'Meslek mensupları' }], body };
};

const ROLES: TenantRole[] = ['owner', 'admin', 'project_manager', 'editor', 'viewer'];

function roleOptions(mine: TenantRole) {
  return ROLES.filter((r) => mine === 'owner' || r !== 'owner').map((r) => ({ value: r, label: roleLabel(r) }));
}

export const orgMembers: PageFn = async ({ params, reload }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  const [members, portal] = await Promise.all([organization.members(org.tenantId), organization.portal(org.tenantId)]);
  const add = () => {
    const f = form(
      [
        grid(
          textField({ label: 'Ad soyad', name: 'displayName', required: true }),
          textField({ label: 'E-posta', name: 'email', type: 'email' }),
          textField({ label: 'Giriş adı', name: 'login', required: true, hint: 'Küçük harf, rakam, . _ - @' }),
          textField({ label: 'İlk parola', name: 'password', type: 'password', required: true, hint: 'En az 10 karakter; kişiye ayrıca iletin.' }),
          selectField({ label: 'Rol', name: 'role', value: 'editor', options: roleOptions(org.role), hint: 'Düzenleyici dosyalara bakar; proje yöneticisi karar verir ve iş aktarır.' }),
        ),
        formError(),
      ],
      async (el) => {
        const v = values(el);
        await organization.addMember(org.tenantId, { login: v.login, password: v.password, displayName: v.displayName, email: orNull(v.email), role: v.role as TenantRole });
        dlg.close();
        toast('Üye eklendi.');
        reload();
      },
    );
    const dlg = openDialog({ title: 'Üye ekle', lead: 'Yeni bir hesap açılır ve kuruma koltuğuyla eklenir.', body: f, footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button('Ekle', { kind: 'primary', onClick: () => f.requestSubmit() })] });
  };
  const edit = (m: MemberView) => {
    const f = form(
      [
        selectField({ label: 'Rol', name: 'role', value: m.role, options: roleOptions(org.role) }),
        switchField({ label: 'Koltuk', name: 'seat', checked: m.seat, hint: 'Koltuğu olmayan üye kurumun sayfalarını açamaz.' }),
        switchField({ label: 'Üyelik etkin', name: 'active', checked: m.status === 'active', hint: 'Kapatılan üyelik kurumda hiçbir şey göremez.' }),
        formError(),
      ],
      async (el) => {
        await organization.updateMember(org.tenantId, m.userId, { role: values(el).role as TenantRole, status: checked(el, 'active') ? 'active' : 'disabled', seat: checked(el, 'seat') });
        dlg.close();
        toast('Üyelik kaydedildi.');
        reload();
      },
    );
    const dlg = openDialog({ title: m.displayName, lead: 'Rol, koltuk ve üyelik', body: f, footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button('Kaydet', { kind: 'primary', onClick: () => f.requestSubmit() })] });
  };
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({
      title: 'Üyeler',
      lead: `${portal.seatsUsed} / ${portal.seatLimit} koltuk kullanılıyor. Koltuk sayısını sistem yönetimi belirler.`,
      actions: [button('Üye ekle', { kind: 'primary', icon: 'plus', onClick: add, disabled: portal.seatsUsed >= portal.seatLimit })],
    }),
    portal.seatsUsed >= portal.seatLimit ? h('div', { style: 'margin-bottom: 12px' }, notice('warning', 'Koltuklar dolu.', 'Yeni üye için bir üyenin koltuğunu boşaltın ya da sistem yönetiminden koltuk isteyin.')) : null,
    table(
      [
        { label: 'Ad', cell: (m) => h('div', { class: 'party' }, avatar(m.displayName, initials(m.displayName), 'sm'), twoLine(m.displayName, m.email ?? '')) },
        { label: 'Rol', cell: (m) => roleLabel(m.role) },
        { label: 'Koltuk', cell: (m) => (m.seat ? h('span', { class: 'with-icon' }, icon('check', 16), 'Var') : h('span', { class: 'faint' }, 'Yok')), class: 'narrow' },
        { label: 'Durum', cell: (m) => (m.status === 'active' ? badge('success', 'Etkin') : badge('neutral', 'Kapalı')), class: 'narrow' },
        { label: '', cell: (m) => button('Düzenle', { kind: 'ghost', onClick: () => edit(m) }), class: 'narrow' },
      ],
      members,
    ),
  );
  return { crumbs: [{ label: org.tenantName, href: base }, { label: 'Üyeler' }], body };
};

export const orgSettings: PageFn = async ({ params, reload }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  const p = await organization.portal(org.tenantId);
  const kindBoxes = kinds('application').map((k) => checkbox(h('span', null, k.label, k.deadline ? h('span', { class: 'faint' }, ' (yasal süreli)') : null), 'kinds', p.applicationKinds.length === 0 || p.applicationKinds.includes(k.code), k.code));
  const f = form(
    [
      h(
        'div',
        { class: 'form-section' },
        h('h3', null, 'Başvuru alımı'),
        switchField({ label: 'Portaldan başvuru alınsın', name: 'accepts', checked: p.acceptsApplications, hint: 'Kapalıyken meslek mensupları belediyenizi listede görür ama başvuru gönderemez.' }),
        h('div', { class: 'field', dataset: { field: 'applicationKinds' } }, h('span', { class: 'field-label' }, 'Alınan başvuru türleri'), h('div', { class: 'checks' }, ...kindBoxes), h('div', { class: 'field-hint' }, 'Hepsi seçiliyse her tür alınır.'), h('div', { class: 'field-error' })),
      ),
      h(
        'div',
        { class: 'form-section' },
        h('h3', null, 'İletişim'),
        grid(
          textField({ label: 'E-posta', name: 'contactEmail', type: 'email', value: p.contactEmail }),
          textField({ label: 'Telefon', name: 'contactPhone', value: p.contactPhone }),
          h('div', { class: 'span-2' }, textField({ label: 'Adres', name: 'address', value: p.address })),
        ),
      ),
      formError(),
      h('div', { class: 'form-actions' }, button('Kaydet', { kind: 'primary', type: 'submit' })),
    ],
    async (el) => {
      const v = values(el);
      const all = kinds('application').map((k) => k.code);
      const picked = checkedValues(el, 'kinds');
      await organization.setPortal(org.tenantId, {
        acceptsApplications: checked(el, 'accepts'),
        applicationKinds: picked.length === all.length ? [] : picked,
        contactEmail: orNull(v.contactEmail),
        contactPhone: orNull(v.contactPhone),
        address: orNull(v.address),
      });
      toast('Kurum ayarları kaydedildi.');
      reload();
    },
  );
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Kurum ayarları', lead: `${p.name}, ${[p.province, p.district].filter(Boolean).join(' / ')}. Kurumun adını, türünü ve koltuklarını sistem yönetimi değiştirir.` }),
    h('div', { class: 'cols-2' }, panel(null, f), panel('Kurum', h('div', { class: 'stack', style: 'gap: 8px' }, h('div', { class: 'two-line-main' }, p.name), h('div', { class: 'muted' }, `Kısa ad: ${p.slug}`), h('div', { class: 'muted' }, `Koltuk: ${p.seatsUsed} / ${p.seatLimit}`), h('div', { class: 'muted' }, `Rolünüz: ${roleLabel(p.role)}`)))),
  );
  return { crumbs: [{ label: org.tenantName, href: base }, { label: 'Kurum ayarları' }], body };
};
