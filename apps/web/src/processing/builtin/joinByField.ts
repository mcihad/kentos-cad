import type { Entity } from '../../model/entities';
import { joinPlan } from '../../model/ops/statistics';
import { fieldNames, fileTable } from '../parameters';
import { defineTool, type FileValue, type Shown } from '../types';
import { withAttr } from './attributeWrites';

/**
 * Anahtarla birleştir (docs/adr/0200 §6; QGIS "Join attributes by field value", ArcGIS Join Field, Netcad Veri
 * Aktar): fields of a source (another layer's objects, or a CSV, TXT or Excel file's rows) are copied into the target
 * objects whose key matches. Keys match trimmed; two numbers as numbers (`007` is `7`), otherwise as texts exactly
 * (the core's `ops::statistics::join_plan`). A key the source has more than once takes its first row; unmatched
 * targets and unused rows are counted and said. One undo step.
 */

/** Two names in their code points' order: a layer source's fields as both platforms list them. */
function codePointOrder(a: string, b: string): number {
  const x = [...a];
  const y = [...b];
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const d = x[i].codePointAt(0)! - y[i].codePointAt(0)!;
    if (d) return d < 0 ? -1 : 1;
  }
  return x.length - y.length;
}

/** A source as rows: its column names, then a row per object or line (cells as written; a missing one is empty). */
function sourceTable(objects: readonly Entity[] | null, file: FileValue | null): { header: string[]; rows: (readonly string[])[] } {
  if (!objects) return fileTable(file) ?? { header: [], rows: [] };
  const header = [...new Set(objects.flatMap((e) => Object.keys(e.attrs)))].sort(codePointOrder);
  return { header, rows: objects.map((e) => header.map((h) => (Object.hasOwn(e.attrs, h) ? e.attrs[h] : ''))) };
}

/** Why a column cannot be read: the names there are. */
const missing = (name: string, header: readonly string[]): string => {
  const names = header.filter(Boolean);
  return names.length ? `Kaynakta “${name}” alanı yok; alanları: ${names.join(', ')}.` : `Kaynakta “${name}” alanı yok; kaynağın hiç alanı yok.`;
};

export const joinByField = defineTool({
  id: 'attributes.joinByField',
  label: 'Anahtarla birleştir',
  category: 'attributes',
  icon: 'joinField',
  description: 'Ortak bir anahtar alanla (parsel numarası, kimlik) başka bir katmandan ya da CSV, TXT, Excel dosyasından öznitelik aktarır.',
  help: [
    'Örnek: tapu kayıtlarının malik ve hisse sütunlarını parsel numarasıyla parsellere aktarmak.',
    'Anahtarlar baştaki ve sondaki boşluklar atılarak karşılaştırılır; iki taraf da sayıysa sayı olarak (007 ile 7 aynı), değilse metin olarak tam. Kaynakta aynı anahtar birden çok kez varsa ilk satır alınır ve söylenir.',
    'Dosyanın ilk satırı sütun adlarıdır; Excel dosyasının ilk sayfası okunur. Aktarılacak alanlar boş bırakılırsa anahtardan başka bütün alanlar aktarılır; önek yeni alanların adının başına eklenir. Kaynakta boş olan değer hedefte alanı boşaltır.',
  ].join('\n\n'),
  keywords: ['birleştir', 'anahtar', 'aktar', 'veri aktar', 'csv', 'excel', 'tablo', 'ilişkilendir', 'join', 'join field', 'lookup'],
  aliases: ['ANAHTARBIRLESTIR', 'VERIAKTAR'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'target', label: 'Hedef nesneler', type: 'features', scopes: ['layer', 'selection', 'visible', 'all'], writes: true, description: 'Özniteliklerin aktarılacağı nesneler; kilitli katmandakiler alınmaz.' },
    { name: 'targetKey', label: 'Hedef anahtar', type: 'field', of: 'target', description: 'Hedef nesnelerin anahtar alanı.' },
    {
      name: 'sourceKind',
      label: 'Kaynak',
      type: 'enum',
      options: [
        { value: 'layer', label: 'Katman', hint: 'Başka nesnelerin öznitelikleri' },
        { value: 'file', label: 'Dosya', hint: 'CSV, TXT ya da Excel dosyası' },
      ],
      default: 'layer',
    },
    { name: 'source', label: 'Kaynak nesneler', type: 'features', scopes: ['layer', 'all', 'visible', 'selection'], visibleWhen: (v: Shown) => v.sourceKind !== 'file', description: 'Özniteliklerin alınacağı nesneler.' },
    { name: 'file', label: 'Dosya', type: 'file', accept: ['.csv', '.txt', '.xlsx'], visibleWhen: (v: Shown) => v.sourceKind === 'file', description: 'İlk satırı sütun adları olan CSV, TXT ya da Excel (.xlsx) dosyası.' },
    { name: 'sourceKey', label: 'Kaynak anahtar', type: 'field', of: ['source', 'file'], optional: true, description: 'Boş bırakılırsa hedef anahtarla aynı ad.' },
    { name: 'fields', label: 'Aktarılacak alanlar', type: 'field', of: ['source', 'file'], multiple: true, optional: true, description: 'Boş bırakılırsa anahtardan başka bütün alanlar.' },
    { name: 'prefix', label: 'Önek', type: 'string', allowEmpty: true, default: '', placeholder: 'Tapu ', description: 'Aktarılan alanların adının başına eklenir.' },
    {
      name: 'existing',
      label: 'Var olan değerler',
      type: 'enum',
      options: [
        { value: 'overwrite', label: 'Üzerine yaz', hint: 'Hedefteki değer kaynağınkiyle değişir' },
        { value: 'empty', label: 'Yalnız boşlara', hint: 'Hedefte dolu olan değer kalır' },
      ],
      default: 'overwrite',
    },
  ] as const,
  outputs: [
    { name: 'changed', label: 'Değişen nesneler', type: 'features' },
    { name: 'count', label: 'Eşleşen nesne sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const targets = v.target.entities;
    const fromFile = v.sourceKind === 'file';
    const { header, rows } = sourceTable(fromFile ? null : (v.source?.entities ?? []), fromFile ? v.file : null);
    const key = v.sourceKey || v.targetKey;
    const k = header.indexOf(key);
    if (k < 0) return { refused: missing(key, header) };
    const wanted = fieldNames({ multiple: true }, v.fields ?? '');
    for (const name of wanted) if (!header.includes(name)) return { refused: missing(name, header) };
    const take = wanted.length ? wanted.map((name) => header.indexOf(name)) : header.flatMap((h, i) => (i !== k && h && header.indexOf(h) === i ? [i] : []));
    feedback.progress(0, 'Anahtarlar eşleniyor');
    const value = (e: Entity, name: string) => (Object.hasOwn(e.attrs, name) ? e.attrs[name] : null);
    const plan = joinPlan(
      targets.map((t) => value(t, v.targetKey)),
      rows.map((r) => r[k] ?? null),
    );
    const update: { id: number; patch: Partial<Entity> }[] = [];
    let matched = 0;
    targets.forEach((t, n) => {
      const m = plan.matches[n];
      if (m === null || m === undefined) return;
      matched++;
      let current: Entity = t;
      for (const i of take) {
        const name = (v.prefix ?? '') + header[i];
        if (v.existing === 'empty' && (value(current, name) ?? '').trim() !== '') continue;
        const cell = rows[m][i] ?? '';
        const attrs = withAttr(ctx, current, name, cell === '' ? null : cell);
        if (attrs) current = { ...current, attrs };
      }
      if (current !== t) update.push({ id: t.id, patch: { attrs: current.attrs } });
    });
    if (plan.repeated) feedback.warn(`${plan.repeated} anahtar kaynakta birden çok kez var; ilk satırları alındı.`);
    if (targets.length - matched) feedback.info(`${targets.length - matched} hedef nesnenin kaynakta eşi yok.`);
    if (plan.unused) feedback.info(`${plan.unused} kaynak satırı hiçbir hedefle eşleşmedi.`);
    return {
      changes: { update },
      outputs: { changed: update.map((u) => u.id), count: matched },
      summary: `${matched} nesne eşleşti; ${update.length} nesnede ${take.length} alan aktarıldı.`,
    };
  },
});
