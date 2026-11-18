// Professionals and accounts as the platform administrators manage them (docs/adr/9000 §2–3): checking a professional's
// chamber registration and verifying it (or not, with the reason the person reads), and accounts: opened, closed,
// given a new password.
import { platform } from '../../api/portal';
import type { PlatformUser, ProfessionalStatus, ProfessionalView } from '../../api/types';
import { withQuery } from '../../app/router';
import { chamberOf, me, professionLabel, professionalStatus } from '../../app/state';
import { h } from '../../lib/dom';
import { day, initials, moment } from '../../lib/format';
import { openDialog } from '../../ui/dialog';
import { form, formError, grid, orNull, textareaField, textField, values } from '../../ui/forms';
import { avatar, badge, button, empty, facts, notice, pageHeader, table, tabs, twoLine } from '../../ui/kit';
import { toast } from '../../ui/toast';
import { searchBox } from '../org/lists';
import type { PageFn } from '../page';

/** The check of one professional: their registration as given, and the decision with its reason. */
export function reviewProfessional(p: ProfessionalView, reload: () => void): void {
  const st = professionalStatus(p.status);
  let decision: ProfessionalStatus = 'verified';
  const f = form(
    [
      h('div', { class: 'party' }, avatar(p.displayName, initials(p.displayName)), h('div', { class: 'party-text' }, h('span', { class: 'party-name' }, p.displayName), h('span', { class: 'party-sub' }, p.email ?? ''), badge(st.tone, st.label))),
      facts([
        ['Meslek', professionLabel(p.profession)],
        ['Oda', chamberOf(p.profession)],
        ['Sicil no', h('span', { class: 'num' }, p.chamberNo)],
        ['Büro', p.officeName ?? '—'],
        ['Telefon', p.phone ?? '—'],
        ['İl', p.province ?? '—'],
        ['Kayıt', moment(p.createdAt)],
      ]),
      notice('info', 'Sicil numarasını odanın kayıtlarıyla karşılaştırın.', 'Doğrulanan kişi belediyelere başvurabilir, ona iş aktarılabilir; hesabının e-postası da doğrulanmış sayılır.'),
      textareaField({ label: 'Açıklama', name: 'note', rows: 3, value: p.statusNote, hint: 'Onaylamıyor ya da askıya alıyorsanız gerekçeyi yazın; kişi bildirimde okur.' }),
      formError(),
    ],
    async (el) => {
      await platform.review(p.userId, decision, values(el).note ?? '');
      dlg.close();
      toast(decision === 'verified' ? 'Kayıt doğrulandı.' : decision === 'rejected' ? 'Kayıt onaylanmadı.' : 'Kayıt askıya alındı.');
      reload();
    },
  );
  const act = (s: ProfessionalStatus) => () => {
    decision = s;
    f.requestSubmit();
  };
  const footer = [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() })];
  if (p.status === 'verified') footer.push(button('Askıya al', { kind: 'danger', onClick: act('suspended') }));
  else footer.push(button('Onaylama', { kind: 'danger', onClick: act('rejected') }));
  if (p.status !== 'verified') footer.push(button('Doğrula', { kind: 'primary', icon: 'check', onClick: act('verified') }));
  const dlg = openDialog({ title: 'Meslek mensubu kaydı', body: f, footer });
}

export const adminProfessionals: PageFn = async ({ query, reload }) => {
  const status = (query.get('durum') as ProfessionalStatus | null) ?? 'pending';
  const [list, all] = await Promise.all([platform.professionals(status), platform.professionals()]);
  const n = (s: ProfessionalStatus) => all.filter((p) => p.status === s).length;
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Meslek mensupları', lead: 'Kayıt olan mimarlar ve mühendisler. Oda sicilini denetleyip doğrulayın; doğrulanmayan kişi başvuru gönderemez.' }),
    tabs(
      [
        { key: 'pending', label: 'Doğrulama bekleyen', count: n('pending'), href: withQuery({ durum: null }) },
        { key: 'verified', label: 'Doğrulanmış', count: n('verified'), href: withQuery({ durum: 'verified' }) },
        { key: 'rejected', label: 'Onaylanmamış', count: n('rejected'), href: withQuery({ durum: 'rejected' }) },
        { key: 'suspended', label: 'Askıda', count: n('suspended'), href: withQuery({ durum: 'suspended' }) },
      ],
      status,
    ),
    table(
      [
        { label: 'Ad', cell: (p) => h('div', { class: 'party' }, avatar(p.displayName, initials(p.displayName), 'sm'), twoLine(p.displayName, p.email ?? '')) },
        { label: 'Meslek', cell: (p) => twoLine(professionLabel(p.profession), chamberOf(p.profession)) },
        { label: 'Sicil no', cell: (p) => h('span', { class: 'num' }, p.chamberNo), class: 'narrow' },
        { label: 'Büro', cell: (p) => p.officeName ?? '—' },
        { label: 'Kayıt', cell: (p) => h('span', { class: 'muted num' }, day(p.createdAt)), class: 'narrow' },
        { label: 'Durum', cell: (p) => badge(professionalStatus(p.status).tone, professionalStatus(p.status).label), class: 'narrow' },
        { label: '', cell: (p) => button(p.status === 'pending' ? 'İncele' : 'Aç', { kind: p.status === 'pending' ? 'secondary' : 'ghost', onClick: () => reviewProfessional(p, reload) }), class: 'narrow' },
      ],
      list,
      { empty: empty('badge', status === 'pending' ? 'Bekleyen kayıt yok' : 'Kayıt yok', status === 'pending' ? 'Yeni kayıtlar burada görünür; size bildirim de gelir.' : 'Bu durumda meslek mensubu yok.') },
    ),
  );
  return { crumbs: [{ label: 'Sistem yönetimi', href: '#/sistem' }, { label: 'Meslek mensupları' }], body };
};

function userKind(u: PlatformUser): HTMLElement {
  if (u.platformAdmin) return badge('info', 'Sistem yöneticisi');
  if (u.profession) return badge(u.professionalStatus ? professionalStatus(u.professionalStatus).tone : 'neutral', professionLabel(u.profession));
  return badge('neutral', u.organizations ? `${u.organizations} kurum üyesi` : 'Hesap');
}

function userActions(u: PlatformUser, reload: () => void): HTMLElement | null {
  if (u.id === me.value?.user.id) return h('span', { class: 'faint' }, 'Sizin hesabınız');
  const password = () => {
    const f = form([textField({ label: 'Yeni parola', name: 'password', type: 'password', required: true, hint: 'En az 10 karakter. Hesabın açık oturumları kapanır.' }), formError()], async (el) => {
      await platform.editUser(u.id, { password: values(el).password });
      dlg.close();
      toast('Yeni parola verildi.');
    });
    const dlg = openDialog({ title: `${u.displayName}: yeni parola`, body: f, footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button('Parolayı ver', { kind: 'primary', onClick: () => f.requestSubmit() })] });
  };
  const toggle = async () => {
    try {
      await platform.editUser(u.id, { status: u.status === 'active' ? 'disabled' : 'active' });
      toast(u.status === 'active' ? 'Hesap kapatıldı.' : 'Hesap açıldı.');
      reload();
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), 'danger');
    }
  };
  return h(
    'div',
    { class: 'file-actions' },
    u.login ? button('Parola', { kind: 'ghost', icon: 'key', onClick: password }) : null,
    button(u.status === 'active' ? 'Kapat' : 'Aç', { kind: u.status === 'active' ? 'ghost' : 'secondary', icon: u.status === 'active' ? 'lock' : 'check', onClick: () => void toggle() }),
  );
}

export const adminUsers: PageFn = async ({ query, reload }) => {
  const q = query.get('ara') ?? '';
  const users = await platform.users(q || undefined);
  const create = () => {
    const f = form(
      [
        grid(
          textField({ label: 'Ad soyad', name: 'displayName', required: true }),
          textField({ label: 'E-posta', name: 'email', type: 'email' }),
          textField({ label: 'Giriş adı', name: 'login', required: true }),
          textField({ label: 'İlk parola', name: 'password', type: 'password', required: true }),
        ),
        h('p', { class: 'field-hint', style: 'margin: 0' }, 'Hesap bir kuruma üye olmadan açılır; kurumun sahibi ya da yöneticisi onu üye yapar.'),
        formError(),
      ],
      async (el) => {
        const v = values(el);
        await platform.createUser({ login: v.login, password: v.password, displayName: v.displayName, email: orNull(v.email) });
        dlg.close();
        toast('Hesap açıldı.');
        reload();
      },
    );
    const dlg = openDialog({ title: 'Yeni hesap', body: f, footer: [button('Vazgeç', { kind: 'ghost', onClick: () => dlg.close() }), button('Hesabı aç', { kind: 'primary', onClick: () => f.requestSubmit() })] });
  };
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Hesaplar', lead: 'Platformdaki bütün hesaplar. Kapatılan hesap oturum açamaz, açık oturumları da kapanır.', actions: [button('Yeni hesap', { kind: 'primary', icon: 'plus', onClick: create })] }),
    h('div', { class: 'toolbar' }, searchBox(q, 'Ad, e-posta ya da giriş adı…'), h('span', { class: 'toolbar-fill' }), h('span', { class: 'muted' }, `${users.length} hesap`)),
    table(
      [
        { label: 'Ad', cell: (u) => h('div', { class: 'party' }, avatar(u.displayName, initials(u.displayName), 'sm'), twoLine(u.displayName, u.email ?? '')) },
        { label: 'Giriş adı', cell: (u) => u.login ?? h('span', { class: 'faint' }, 'Kurum girişi') },
        { label: 'Tür', cell: userKind },
        { label: 'Durum', cell: (u) => (u.status === 'active' ? badge('success', 'Etkin') : badge('neutral', 'Kapalı')), class: 'narrow' },
        { label: 'Açılış', cell: (u) => h('span', { class: 'muted num' }, day(u.createdAt)), class: 'narrow' },
        { label: '', cell: (u) => userActions(u, reload), class: 'narrow' },
      ],
      users,
      { empty: empty('users', 'Hesap bulunamadı', 'Aramanızı değiştirin.') },
    ),
  );
  return { crumbs: [{ label: 'Sistem yönetimi', href: '#/sistem' }, { label: 'Hesaplar' }], body };
};
