import { summarizeValues } from '../../model/ops/statistics';
import { defineTool } from '../types';

/**
 * Özet istatistik (docs/adr/0200 §5; QGIS "Statistics by categories", ArcGIS Summary Statistics): a field's
 * figures over the objects, group by group when a grouping field is given: how many objects, how many values read
 * as numbers, the sum, mean, least, most and the sample standard deviation. The table is the core's
 * (`ops::statistics::summarize`); the dialog shows it after the run with Panoya kopyala and CSV olarak kaydet. Edits
 * nothing.
 */
export const summaryStatistics = defineTool({
  id: 'statistics.summary',
  label: 'Özet istatistik',
  category: 'analysis',
  icon: 'statsSummary',
  description: 'Bir alanın toplamını, ortalamasını, en azını, en çoğunu ve standart sapmasını, isterseniz başka bir alanın değerlerine göre gruplayarak tablo olarak verir.',
  help: [
    'Örnekler: yapıların taban alanlarının kat sayısına göre toplamı; parsellerin tapu alanlarının ortalaması ve standart sapması.',
    'Değerler sayı olarak okunur (ondalık nokta ya da virgül); okunamayanlar atlanır ve söylenir. Toplam, en az ve en çok kesindir; ortalama ve standart sapma (örneklem) değerlerin en çok basamağından iki fazla basamakla yazılır.',
    'Gruplar adlarının doğal sırasıyla, boş grup “(boş)” en sonda; gruplanınca son satır bütün nesnelerin toplamıdır. Çizim değişmez.',
  ].join('\n\n'),
  keywords: ['istatistik', 'özet', 'toplam', 'ortalama', 'standart sapma', 'grupla', 'statistics', 'summary', 'group by', 'categories'],
  aliases: ['OZETIST', 'ISTATISTIK'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', scopes: ['layer', 'selection', 'visible', 'all'], description: 'İstatistiği alınan nesneler.' },
    { name: 'field', label: 'Alan', type: 'field', of: 'input', description: 'Değerleri sayı olarak okunan alan.' },
    { name: 'group', label: 'Grupla', type: 'field', of: 'input', optional: true, description: 'Boş bırakılırsa bütün nesneler tek satırdır.' },
  ] as const,
  outputs: [
    { name: 'table', label: 'Özet tablosu', type: 'table' },
    { name: 'count', label: 'Nesne sayısı', type: 'number' },
  ],
  run: async (v, _ctx, feedback) => {
    const list = v.input.entities;
    const field = v.field;
    const group = v.group ?? '';
    feedback.progress(0, 'Değerler okunuyor');
    const value = (attrs: Readonly<Record<string, string>>, name: string) => (Object.hasOwn(attrs, name) ? attrs[name] : null);
    let table: ReturnType<typeof summarizeValues>;
    try {
      table = summarizeValues(
        list.map((e) => (group ? value(e.attrs, group) : null)),
        list.map((e) => value(e.attrs, field)),
        !!group,
      );
    } catch (e) {
      // Numbers too large or too long to add exactly: said as the core says it.
      return { refused: e instanceof Error ? e.message : String(e) };
    }
    const read = list.length - table.skipped - list.filter((e) => (value(e.attrs, field) ?? '').trim() === '').length;
    if (table.skipped) feedback.warn(`${table.skipped} değer sayı olarak okunamadığı için atlandı.`);
    const groups = group ? table.rows.length - 1 : 0;
    return {
      outputs: { table: { columns: table.columns, rows: table.rows }, count: list.length },
      summary: `${list.length} nesnede “${field}”: ${read} değer okundu${group ? `, ${groups} grup.` : '.'}`,
    };
  },
});
