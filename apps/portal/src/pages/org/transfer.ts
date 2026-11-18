// A municipality transfers work to a professional (docs/adr/9000 §5): it picks a verified professional from its
// directory, says what the work is, where and by when, and opens the transfer as a draft; on the draft's page it adds
// the project and the documents and sends it.
import { organization } from '../../api/portal';
import type { ProfessionalCard } from '../../api/types';
import { go } from '../../app/router';
import { kinds, orgBySlug, professionLabel } from '../../app/state';
import { h, replaceChildren } from '../../lib/dom';
import { form, formError, grid, orNull, textareaField, textField, values } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { button, notice, pageHeader, panel } from '../../ui/kit';
import { toast } from '../../ui/toast';
import type { PageFn } from '../page';

function professionalPicker(tenant: string, preset: string | null): HTMLElement {
  const list = h('div', { class: 'pick-list' });
  const search = h('input', { class: 'input', type: 'search', placeholder: 'Ad, sicil no ya da büro adıyla arayın…', 'aria-label': 'Meslek mensubu ara' });
  let timer = 0;
  const fill = async () => {
    const people: ProfessionalCard[] = await organization.directory(tenant, search.value.trim() || undefined);
    const usable = people.filter((p) => p.status === 'verified');
    if (!usable.length) {
      replaceChildren(list, h('div', { class: 'notif-empty' }, 'Doğrulanmış meslek mensubu bulunamadı.'));
      return;
    }
    replaceChildren(
      list,
      ...usable.map((p) =>
        h(
          'label',
          { class: 'pick' },
          h('input', { type: 'radio', name: 'professionalId', value: p.userId, checked: p.userId === preset }),
          h('span', { class: 'file-icon' }, icon('badge', 18)),
          h('span', { class: 'party-text' }, h('span', { class: 'party-name' }, p.displayName), h('span', { class: 'party-sub' }, `${professionLabel(p.profession)}, sicil no ${p.chamberNo}${p.officeName ? `, ${p.officeName}` : ''}${p.province ? `, ${p.province}` : ''}`)),
          p.dossiers ? h('span', { class: 'muted' }, `${p.dossiers} dosya`) : null,
        ),
      ),
    );
  };
  search.addEventListener('input', () => {
    clearTimeout(timer);
    timer = window.setTimeout(() => void fill(), 250);
  });
  void fill();
  return h('div', { class: 'field', dataset: { field: 'professionalId' } }, h('span', { class: 'field-label' }, 'Meslek mensubu', h('span', { class: 'field-req' }, ' *')), search, list, h('div', { class: 'field-error', role: 'alert' }));
}

export const orgNewTransfer: PageFn = async ({ params, query }) => {
  const org = orgBySlug(params.slug);
  if (!org) throw new Error('Kurum bulunamadı.');
  const base = `#/kurum/${org.tenantSlug}`;
  const manager = ['owner', 'admin', 'project_manager'].includes(org.role);
  const crumbs = [{ label: org.tenantName, href: base }, { label: 'Proje aktarımları', href: `${base}/aktarimlar` }, { label: 'Yeni aktarım' }];
  if (!manager) {
    return { crumbs, body: h('div', { class: 'page' }, notice('warning', 'Proje aktarımını kurumunuzun proje yöneticileri ve yöneticileri yapar.')) };
  }
  const choices = kinds('transfer').map((k, i) =>
    h('label', { class: 'choice' }, h('input', { type: 'radio', name: 'kind', value: k.code, checked: i === 0 }), h('span', { class: 'choice-title' }, k.label), h('span', { class: 'choice-text' }, k.description)),
  );
  const f = form(
    [
      h('div', { class: 'form-section' }, h('h3', null, 'Kime'), professionalPicker(org.tenantId, query.get('meslek-mensubu'))),
      h('div', { class: 'form-section' }, h('h3', null, 'İşin türü'), h('div', { class: 'choice-grid', dataset: { field: 'kind' } }, ...choices)),
      h(
        'div',
        { class: 'form-section' },
        h('h3', null, 'İş'),
        grid(
          h('div', { class: 'span-2' }, textField({ label: 'Başlık', name: 'title', required: true, placeholder: 'Örnek: 501 ada 1–4 parseller zemin aplikasyonu' })),
          h('div', { class: 'span-2' }, textareaField({ label: 'Açıklama', name: 'description', rows: 4, placeholder: 'İstenen iş, uyulacak esaslar, teslim biçimi…' })),
          textField({ label: 'İstenen teslim günü', name: 'dueOn', type: 'date' }),
          h('div'),
        ),
      ),
      h(
        'div',
        { class: 'form-section' },
        h('h3', null, 'Yer'),
        h('div', { class: 'form-grid three' }, textField({ label: 'İl', name: 'province', value: org.province }), textField({ label: 'İlçe', name: 'district', value: org.district }), textField({ label: 'Mahalle', name: 'neighbourhood' }), textField({ label: 'Ada', name: 'block' }), textField({ label: 'Parsel', name: 'parcel' }), textField({ label: 'Adres', name: 'address' })),
      ),
      notice('info', 'Aktarım önce taslak olarak açılır.', 'Sonraki sayfada aktaracağınız projeyi ve belgeleri ekleyip işi gönderirsiniz; meslek mensubu o zaman görür.'),
      formError(),
      h('div', { class: 'form-actions' }, button('Vazgeç', { kind: 'ghost', href: `${base}/aktarimlar` }), button('Taslağı oluştur', { kind: 'primary', type: 'submit' })),
    ],
    async (el) => {
      const v = values(el);
      const picked = el.querySelector<HTMLInputElement>('input[name=professionalId]:checked');
      const made = await organization.transfer(org.tenantId, {
        professionalId: picked?.value ?? '',
        kind: v.kind,
        title: v.title,
        description: v.description,
        site: { province: orNull(v.province), district: orNull(v.district), neighbourhood: orNull(v.neighbourhood), block: orNull(v.block), parcel: orNull(v.parcel), address: orNull(v.address) },
        dueOn: orNull(v.dueOn),
      });
      toast('Aktarım taslağı açıldı. Şimdi projeyi ve belgeleri ekleyin.');
      go(`${base}/dosya/${made.id}`);
    },
  );
  const body = h('div', { class: 'page' }, pageHeader({ title: 'Yeni proje aktarımı', lead: 'Bir meslek mensubuna proje ya da iş aktarın: o işi üstlenir, hazırlar ve teslim eder.', back: { label: 'Proje aktarımları', href: `${base}/aktarimlar` } }), panel(null, f));
  return { crumbs, body };
};
