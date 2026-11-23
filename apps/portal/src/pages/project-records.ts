// A database project's records and the caller's access, as the server gives them to this caller (docs/adr/9194): each
// layer that has fields as a table of the fields the caller's role reads. The fields it does not read are named under
// the table, so one left out is never taken for an empty one. A value never written (NULL) is said apart from an empty
// text. A Veri girişi whose form scope narrows a layer is told it reads only some of its records.
import { cloud } from '../api/portal';
import type { FeatureLayer, FeaturePage, FeatureRecord, LayerField, ProjectAccessView, ProjectRole, ProjectSummary } from '../api/types';
import { type Child, h } from '../lib/dom';
import { byLabel, byLayer, recordBase, valueText, withheldWords } from '../lib/records';
import { empty, panel, table } from '../ui/kit';

/** The records the page reads at most: the first page of the server's, in id order. */
const READ = 500;
/** The rows a layer's table shows at most. */
const SHOWN = 100;

/** A role's name and what it may do, in one line (the words of the desktop's sharing window). */
const ROLES: Record<ProjectRole, [string, string]> = {
  owner: ['Sahip', 'Projenin her işini yapar: paylaşır, formları tasarlar ve yayınlar, projeyi devreder ya da siler.'],
  manager: ['Yönetici', 'Düzenleyicinin yapabildikleri; ad, ayar ve katmanları değiştirir, formları tasarlar ve yayınlar, projeyi paylaşır.'],
  editor: ['Düzenleyici', 'Nesneleri çizer, değiştirir ve siler.'],
  entry: [
    'Veri girişi',
    'Kayıtların özniteliklerini yalnız yayımlanmış formla, formun gösterdiği alanlarda düzenler; çizimi değiştiremez, indiremez.',
  ],
  commenter: ['Yorumcu', 'Görüntüleyicinin yapabildikleri ve yorum.'],
  viewer: ['Görüntüleyici', 'Projeyi açar, görür, indirir ve geçmişine bakar; değiştiremez.'],
};

const VIA: Record<ProjectAccessView['via'], string> = {
  owner: 'projenin sahibi',
  grant: 'paylaşımla',
  policy: 'kurumun kuralıyla',
};

/** Whether the caller reads and writes the records through forms alone (a Veri girişi), as the server decides it. */
export function throughForms(a: ProjectAccessView): boolean {
  return a.permissions.includes('forms.use') && !a.permissions.includes('feature.write');
}

/** The caller's role in the project, where it comes from and what it may do. */
export function accessView(a: ProjectAccessView): HTMLElement {
  const [name, hint] = ROLES[a.role];
  return h(
    'div',
    { class: 'access' },
    h('div', null, h('strong', null, name), h('span', { class: 'muted' }, ` · ${VIA[a.via]}`)),
    h('div', { class: 'field-hint' }, hint),
  );
}

/** A value in its cell: NULL muted, an empty text empty (said to a screen reader). */
function cell(field: LayerField, raw: string | undefined): Child {
  const text = valueText(field, raw);
  if (text === null) return h('span', { class: 'null-value', title: 'Hiç yazılmamış değer' }, 'NULL');
  if (text === '') return h('span', { class: 'empty-value', title: 'Boş metin', 'aria-label': 'Boş metin' });
  return text;
}

function layerTable(layer: FeatureLayer, records: FeatureRecord[]): HTMLElement {
  const columns = [
    { label: 'Ad', cell: (r: FeatureRecord) => recordBase(r).label ?? h('span', { class: 'faint' }, '—'), class: 'narrow' },
    ...layer.fields.map((f) => ({ label: f.alias ?? f.name, cell: (r: FeatureRecord) => cell(f, recordBase(r).attrs[f.name]) })),
  ];
  const notes: string[] = [];
  if (records.length > SHOWN) notes.push(`${SHOWN} / ${records.length} kayıt gösteriliyor.`);
  if (layer.narrowed) notes.push('Formun kapsamı bu katmanın yalnız bazı kayıtlarını size açıyor; öbürleri burada yok.');
  if (layer.withheld?.length) notes.push(withheldWords(layer.withheld));
  return h(
    'div',
    { class: 'records-layer' },
    h('div', { class: 'records-head' }, h('h3', null, layer.name), h('span', { class: 'muted num' }, `${records.length} kayıt`)),
    table(columns, byLabel(records).slice(0, SHOWN), {
      empty: h('span', { class: 'muted' }, layer.narrowed ? 'Formun kapsamında kayıt yok.' : 'Katmanda kayıt yok.'),
    }),
    ...notes.map((n) => h('div', { class: 'records-note' }, n)),
  );
}

/** The records panel of a database project; none for a file project, whose objects are in its revisions. */
export async function recordsPanel(p: ProjectSummary): Promise<HTMLElement | null> {
  if (p.storage !== 'database') return null;
  let page: FeaturePage;
  try {
    page = await cloud.records(p.tenantId, p.id, READ);
  } catch {
    return panel('Kayıtlar', empty('list', 'Kayıtlar okunamadı', 'Sunucuya ulaşılamadı; sayfayı yenileyin.'));
  }
  const layers = page.layers ?? [];
  if (!layers.length) return null;
  const by = byLayer(page.features);
  const read = layers.reduce((n, l) => n + (by.get(l.id)?.length ?? 0), 0);
  return panel('Kayıtlar', h('div', { class: 'records' }, ...layers.map((l) => layerTable(l, by.get(l.id) ?? []))), {
    note: page.next ? `İlk ${READ} kayıt okundu` : `${read} kayıt`,
    class: 'panel-flush',
  });
}
