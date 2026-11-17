// A professional's new application (docs/adr/9000 §5): the municipality (those taking applications first), the kind it
// takes with the documents it usually needs and its legal deadline, the matter and the place; opened as a draft whose
// page takes the package.
import { portal, professional } from '../../api/portal';
import type { KindOption, MunicipalityCard } from '../../api/types';
import { go } from '../../app/router';
import { kinds, me } from '../../app/state';
import { h, replaceChildren } from '../../lib/dom';
import { form, formError, grid, orNull, textareaField, textField, values } from '../../ui/forms';
import { icon } from '../../ui/icons';
import { button, notice, pageHeader, panel } from '../../ui/kit';
import { toast } from '../../ui/toast';
import type { PageFn } from '../page';
import { registrationNotice } from './lists';

function kindCard(k: KindOption, checkedFirst: boolean): HTMLElement {
  const meta = [k.documents.length ? `${k.documents.length} belge` : '', k.deadline ? 'yasal süreli' : ''].filter(Boolean).join(', ');
  return h(
    'label',
    { class: 'choice' },
    h('input', { type: 'radio', name: 'kind', value: k.code, checked: checkedFirst }),
    h('span', { class: 'choice-title' }, k.label),
    h('span', { class: 'choice-text' }, k.description),
    meta ? h('span', { class: 'choice-meta' }, meta) : null,
  );
}

/** The chosen kind's documents (on what basis) and its legal deadline. */
function kindInfo(k: KindOption | undefined): HTMLElement {
  if (!k) return h('div');
  return h(
    'div',
    { class: 'kind-info' },
    h('strong', null, k.label),
    k.documents.length
      ? h('div', null, h('div', { class: 'field-hint', style: 'margin-bottom: 6px' }, k.documentsBasis ?? ''), h('div', { class: 'kind-info-docs' }, ...k.documents.map((d) => h('span', { class: 'chip' }, d.label))))
      : h('div', { class: 'field-hint' }, 'Bu tür için belge listesi tanımlı değil; belediye gerekeni bildirir.'),
    k.deadline ? h('div', { class: 'with-icon' }, icon('clock', 16), h('span', null, k.deadline)) : null,
  );
}

function municipalityCard(m: MunicipalityCard, checkedFirst: boolean): HTMLElement {
  return h(
    'label',
    { class: 'choice', title: m.acceptsApplications ? null : 'Bu belediye şu an portaldan başvuru almıyor.' },
    h('input', { type: 'radio', name: 'tenantId', value: m.tenantId, checked: checkedFirst, disabled: !m.acceptsApplications }),
    h('span', { class: 'choice-title' }, m.name),
    h('span', { class: 'choice-text' }, [m.province, m.district].filter(Boolean).join(', ')),
    h('span', { class: 'choice-meta' }, m.acceptsApplications ? (m.applicationKinds.length ? `${m.applicationKinds.length} türde başvuru alıyor` : 'Her türde başvuru alıyor') : 'Şu an başvuru almıyor'),
  );
}

export const proNewApplication: PageFn = async ({ query }) => {
  const crumbs = [{ label: 'Meslek mensubu', href: '#/mesleki' }, { label: 'Başvurularım', href: '#/mesleki/basvurular' }, { label: 'Yeni başvuru' }];
  if (me.value?.professional?.status !== 'verified') {
    return { crumbs, body: h('div', { class: 'page' }, pageHeader({ title: 'Yeni başvuru' }), registrationNotice() ?? notice('warning', 'Başvuru için meslek mensubu kaydınızın doğrulanması gerekir.')) };
  }
  const municipalities = await portal.municipalities();
  const preset = query.get('belediye');
  const open = municipalities.filter((m) => m.acceptsApplications);
  const first = open.find((m) => m.tenantId === preset) ?? open[0];
  const kindGrid = h('div', { class: 'choice-grid', dataset: { field: 'kind' } });
  const info = h('div');
  const drawInfo = (code: string | undefined) => replaceChildren(info, kindInfo(kinds('application').find((k) => k.code === code)));
  const drawKinds = (tenantId: string | undefined) => {
    const m = municipalities.find((x) => x.tenantId === tenantId);
    const taken = kinds('application').filter((k) => !m || m.applicationKinds.length === 0 || m.applicationKinds.includes(k.code));
    replaceChildren(kindGrid, ...taken.map((k, i) => kindCard(k, i === 0)));
    drawInfo(taken[0]?.code);
  };
  drawKinds(first?.tenantId);
  const placeFields = h(
    'div',
    { class: 'form-grid three' },
    textField({ label: 'İl', name: 'province', value: first?.province }),
    textField({ label: 'İlçe', name: 'district', value: first?.district }),
    textField({ label: 'Mahalle', name: 'neighbourhood' }),
    textField({ label: 'Ada', name: 'block' }),
    textField({ label: 'Parsel', name: 'parcel' }),
    textField({ label: 'Adres', name: 'address' }),
  );
  const f = form(
    [
      h('div', { class: 'form-section' }, h('h3', null, 'Belediye'), h('div', { class: 'choice-grid', dataset: { field: 'tenantId' } }, ...municipalities.map((m) => municipalityCard(m, m.tenantId === first?.tenantId)))),
      h('div', { class: 'form-section' }, h('h3', null, 'Başvuru türü'), kindGrid, info),
      h(
        'div',
        { class: 'form-section' },
        h('h3', null, 'Başvuru'),
        grid(
          h('div', { class: 'span-2' }, textField({ label: 'Başlık', name: 'title', required: true, placeholder: 'Örnek: 412 ada 7 parsel konut yapı ruhsatı' })),
          h('div', { class: 'span-2' }, textareaField({ label: 'Açıklama', name: 'description', rows: 4, placeholder: 'Yapının niteliği, kat adedi, belediyenin bilmesi gerekenler…' })),
          textField({ label: 'Yapı sahibi', name: 'ownerName', hint: 'Yalnız ad soyad; kimlik numarası yazmayın.' }),
          h('div'),
        ),
      ),
      h('div', { class: 'form-section' }, h('h3', null, 'Yer'), placeFields),
      notice('info', 'Başvuru önce taslak olarak açılır.', 'Sonraki sayfada projeyi ve belgeleri ekler, belge listesine bakar ve gönderirsiniz; belediye o zaman görür.'),
      formError(),
      h('div', { class: 'form-actions' }, button('Vazgeç', { kind: 'ghost', href: '#/mesleki/basvurular' }), button('Taslağı oluştur', { kind: 'primary', type: 'submit' })),
    ],
    async (el) => {
      const v = values(el);
      const made = await professional.apply({
        tenantId: v.tenantId ?? '',
        kind: v.kind ?? '',
        title: v.title,
        description: v.description,
        ownerName: orNull(v.ownerName),
        site: { province: orNull(v.province), district: orNull(v.district), neighbourhood: orNull(v.neighbourhood), block: orNull(v.block), parcel: orNull(v.parcel), address: orNull(v.address) },
      });
      toast('Başvuru taslağı açıldı. Şimdi projeyi ve belgeleri ekleyin.');
      go(`#/mesleki/dosya/${made.id}`);
    },
  );
  f.addEventListener('change', (e) => {
    const t = e.target as HTMLInputElement;
    if (t.name === 'kind') drawInfo(t.value);
    if (t.name !== 'tenantId') return;
    drawKinds(t.value);
    const m = municipalities.find((x) => x.tenantId === t.value);
    const set = (name: string, value: string | null) => {
      const input = f.querySelector<HTMLInputElement>(`input[name=${name}]`);
      if (input) input.value = value ?? '';
    };
    set('province', m?.province ?? null);
    set('district', m?.district ?? null);
  });
  const body = h(
    'div',
    { class: 'page' },
    pageHeader({ title: 'Yeni başvuru', lead: 'Belediyeyi ve başvurunun türünü seçin; başvuru taslak olarak açılır, paketi sonra eklersiniz.', back: { label: 'Başvurularım', href: '#/mesleki/basvurular' } }),
    open.length ? panel(null, f) : notice('warning', 'Şu an portaldan başvuru alan bir belediye yok.'),
  );
  return { crumbs, body };
};
