import { geoDissolve } from '../../../model/ops/geoprocess';
import { statisticMany } from '../../../model/ops/statistics';
import { fieldNames } from '../../parameters';
import { defineTool } from '../../types';
import { GEO_KINDS, GEO_SCOPES, inputNotes, newObject, outputStyle } from './shared';

/**
 * Gruplayarak birleştir (docs/adr/0201 §4; ArcGIS Dissolve, QGIS Birleştir): the objects grouped by a field's value
 * (all one group without one), each group's areas joined in the core's overlay, its paths and points gathered; one
 * object a group, or each joined part on its own. A group's object carries the group's value, how many objects it
 * joined and the sums of the fields asked for (the statistics' exact rule, `kentos.statistics/1`).
 */
export const geometryDissolve = defineTool({
  id: 'geometry.dissolve',
  label: 'Gruplayarak birleştir',
  category: 'geometry',
  icon: 'geoDissolve',
  description: 'Nesneleri bir alanın değerine göre gruplayıp her grubu tek nesnede birleştirir: alanların ortak sınırları kalkar, istenen alanların değerleri toplanır.',
  help: [
    'Örnekler: parselleri ada numarasına göre birleştirip adaları elde etmek; aynı kullanım türündeki alanları tek alan yapmak.',
    'Grupla boşsa bütün nesneler tek gruptur. Alanlar birleşir, ortak sınırları kalkar; çizgiler ve noktalar grubun çok parçalı nesnesi olur. Çok parçalı kapalıysa her bağlı parça ayrı nesne olarak, büyükten küçüğe yazılır.',
    'Sonucun öznitelikleri: grup alanı ve değeri, “Nesne sayısı” ve Toplanacak alanların toplamları. Toplamlar kesindir; sayı olarak okunamayan değerler atlanır ve söylenir.',
  ].join('\n\n'),
  keywords: ['birleştir', 'grupla', 'dissolve', 'erit', 'ada', 'tevhit', 'topla', 'merge'],
  aliases: ['GRUPBIRLESTIR', 'DISSOLVE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GEO_SCOPES, description: 'Birleştirilecek alanlar, çizgiler ve noktalar.' },
    { name: 'group', label: 'Grupla', type: 'field', of: 'input', optional: true, description: 'Değeri aynı olan nesneler bir grup olur; boşsa hepsi tek grup.' },
    { name: 'sums', label: 'Toplanacak alanlar', type: 'field', of: 'input', multiple: true, optional: true, description: 'Her grubun bu alanlardaki değerleri toplanır.' },
    { name: 'multi', label: 'Çok parçalı', type: 'boolean', default: true, description: 'Açıkken grup başına tek nesne; kapalıyken her bağlı parça ayrı nesne.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Birleştirilen' }, newLayerStyle: outputStyle('#6E56CF'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'dissolved', label: 'Birleştirilen nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const input = v.input.entities;
    const group = (v.group ?? '').trim();
    const sums = fieldNames({ multiple: true }, v.sums ?? '');
    inputNotes(feedback, input);
    // Groups by their first object's place; a missing value is the empty one.
    const keys: string[] = [];
    const groups = input.map((e) => {
      const key = group && Object.hasOwn(e.attrs, group) ? e.attrs[group] : '';
      let k = keys.indexOf(key);
      if (k < 0) k = keys.push(key) - 1;
      return k;
    });
    feedback.progress(0, 'Gruplar birleştiriliyor');
    const parts = geoDissolve(input, groups, v.multi);
    const counts = keys.map((_, k) => groups.filter((g) => g === k).length);
    let skipped = 0;
    let totals: (string | null)[][];
    try {
      totals = sums.map((name) => {
        const values = keys.map((_, k) => input.flatMap((e, i) => (groups[i] === k ? [Object.hasOwn(e.attrs, name) ? e.attrs[name] : null] : [])));
        const r = statisticMany(values, 'sum', keys.map(() => null));
        skipped += r.reduce((s, x) => s + x.skipped, 0);
        return r.map((x) => x.value ?? null);
      });
    } catch (e) {
      // Numbers too large or too long to add exactly: said as the core says it.
      return { refused: e instanceof Error ? e.message : String(e) };
    }
    if (skipped) feedback.warn(`${skipped} değer sayı olarak okunamadığı için toplanmadı.`);
    const add = parts.map((p) => {
      const attrs: Record<string, string> = {};
      if (group) attrs[group] = keys[p.group];
      attrs['Nesne sayısı'] = String(counts[p.group]);
      sums.forEach((name, j) => {
        const total = totals[j][p.group];
        if (total !== null) attrs[name] = total;
      });
      return newObject(p.shape, v.layer.id, attrs);
    });
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: group ? `${input.length} nesne ${keys.length} grupta birleştirildi; ${add.length} nesne yazıldı.` : `${input.length} nesne birleştirildi; ${add.length} nesne yazıldı.`,
    };
  },
});
