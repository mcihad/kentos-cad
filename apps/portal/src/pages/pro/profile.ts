// A professional's own pages (docs/adr/9000 §2, §7): their registration as an administrator checked it, what they keep
// up to date themselves, their password, and their cloud projects (where a transferred project they opened lands).
import { portal } from '../../api/portal';
import { chamberOf, me, professionLabel, professionalStatus } from '../../app/state';
import { h } from '../../lib/dom';
import { initials, moment } from '../../lib/format';
import { checked, form, formError, grid, orNull, switchField, textField, values } from '../../ui/forms';
import { avatar, badge, button, empty, facts, pageHeader, panel } from '../../ui/kit';
import { toast } from '../../ui/toast';
import type { PageFn } from '../page';
import { registrationNotice } from './lists';
import { projectsTable } from '../projects';

export const proProfile: PageFn = async ({ reload }) => {
  const m = await portal.me();
  me.set(m);
  const p = m.professional;
  if (!p) throw new Error('Hesabınız bir meslek mensubunun değil.');
  const st = professionalStatus(p.status);
  const edit = form(
    [
      grid(
        textField({ label: 'Büro ya da şirket adı', name: 'officeName', value: p.officeName }),
        textField({ label: 'Telefon', name: 'phone', value: p.phone }),
        textField({ label: 'İl', name: 'province', value: p.province }),
        h('div'),
      ),
      switchField({ label: 'Belediyelerin meslek mensupları dizininde görüneyim', name: 'listed', checked: p.listed, hint: 'Kapalıyken belediyeler sizi yalnız aranızdaki dosyalardan görür; size iş aktarmak için bulamaz.' }),
      formError(),
      h('div', { class: 'form-actions' }, button('Kaydet', { kind: 'primary', type: 'submit' })),
    ],
    async (el) => {
      const v = values(el);
      me.set(await portal.editProfessional({ officeName: orNull(v.officeName), phone: orNull(v.phone), province: orNull(v.province), listed: checked(el, 'listed') }));
      toast('Profil kaydedildi.');
      reload();
    },
  );
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Profil', lead: 'Meslek ve oda bilgileriniz sistem yöneticisinin doğruladığı biçimde kalır; değiştirmek için yöneticiye başvurun.' }),
    registrationNotice() ? h('div', { style: 'margin-bottom: 14px' }, registrationNotice()) : null,
    h(
      'div',
      { class: 'cols-2' },
      h('div', { class: 'stack' }, panel('İletişim ve görünürlük', edit), panel('Parola ve oturumlar', h('p', { class: 'panel-text' }, 'Parolanızı, açık oturumlarınızı ve erişim anahtarlarınızı ', h('a', { href: '#/hesabim' }, 'Hesabım'), ' sayfasından yönetirsiniz.'))),
      panel(
        'Meslek mensubu kaydı',
        h(
          'div',
          { class: 'stack', style: 'gap: 14px' },
          h('div', { class: 'party' }, avatar(m.user.displayName, initials(m.user.displayName)), h('div', { class: 'party-text' }, h('span', { class: 'party-name' }, m.user.displayName), h('span', { class: 'party-sub' }, m.user.email ?? ''), badge(st.tone, st.label))),
          facts([
            ['Meslek', professionLabel(p.profession)],
            ['Oda', chamberOf(p.profession)],
            ['Sicil no', h('span', { class: 'num' }, p.chamberNo)],
            ['Giriş adı', m.login ?? '—'],
            ['Kayıt', moment(p.createdAt)],
            ['Doğrulama', p.reviewedAt ? moment(p.reviewedAt) : 'Bekliyor'],
          ]),
          p.statusNote ? h('div', { class: 'field-hint' }, `Yönetici notu: ${p.statusNote}`) : null,
        ),
      ),
    ),
  );
  return { crumbs: [{ label: 'Meslek mensubu', href: '#/mesleki' }, { label: 'Profil' }], body };
};

export const proProjects: PageFn = async () => {
  const page = await portal.projects('mine');
  const projects = page.projects;
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Projelerim', lead: 'Buluttaki projeleriniz. Belediyenin aktardığı bir projeyi dosya sayfasından “Proje olarak aç” ile buraya alırsınız; teslim ederken “Projeden ekle” ile pakete koyarsınız.' }),
    projectsTable(projects, empty('folder', 'Bulutta projeniz yok', 'KentOS masaüstünden ya da web uygulamasından buluta kaydettiğiniz projeler burada görünür.')),
  );
  return { crumbs: [{ label: 'Meslek mensubu', href: '#/mesleki' }, { label: 'Projelerim' }], body };
};
