import type { NewEntity } from '../../../model/entities';
import type { NetworkFound, NetworkNearest } from '../../../model/networkAnswers';
import { defineTool, type Shown } from '../../types';
import { newObject } from '../geometry/shared';
import { costValue, missingNote, nameOf, placesOf, runNetwork } from './shared';

/**
 * Maliyet matrisi (docs/adr/0209 §7; ArcGIS's OD Cost Matrix, pgRouting's pgr_dijkstraCostMatrix): from each origin
 * the cost along the network to each destination, the cheapest first, within an upper bound and the nearest `count`
 * when given. The table (Başlangıç, Varış, Sıra and the chosen cost) goes to the clipboard or a CSV; straight lines from
 * each origin to its destinations are written when asked.
 */
export const networkOdMatrix = defineTool({
  id: 'network.odMatrix',
  label: 'Maliyet matrisi',
  category: 'network',
  icon: 'odMatrix',
  description: 'Başlangıç noktalarından varış noktalarına ağ boyunca maliyetleri tablo olarak çıkarır; isteğe bağlı düz çizgilerle.',
  help: [
    'Ağ projenin yol ya da şebeke ağıdır (Ağlar); maliyet uzunluk ya da ağın süresi ve maliyetleridir. Tek yönler ve kapalı yollar gözetilir.',
    'Her başlangıçtan her varışa maliyet, ucuzdan pahalıya sıralanır; En yakın verilirse yalnız o kadar varış, Üst sınır verilirse ondan ucuzlar yazılır. Ulaşılamayan varış yazılmaz.',
    'Tablo Başlangıç, Varış, Sıra ve maliyet sütunlarıdır; panoya ya da CSV’ye alınır. Düz çizgiler açıkken her satır başlangıçtan varışa bir çizgidir.',
  ].join('\n\n'),
  keywords: ['maliyet matrisi', 'od matrix', 'başlangıç varış', 'uzaklık matrisi', 'süre matrisi', 'ağ'],
  aliases: ['MALIYETMATRISI', 'ODMATRIX', 'ODCOSTMATRIX'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'network', label: 'Ağ', type: 'network', prefers: 'road', description: 'Maliyetlerin bulunacağı ağ ve maliyet.' },
    { name: 'origins', label: 'Başlangıçlar', type: 'features', kinds: ['point'], scopes: ['layer', 'selection', 'visible', 'all'] },
    { name: 'destinations', label: 'Varışlar', type: 'features', kinds: ['point'], scopes: ['layer', 'selection', 'visible', 'all'] },
    { name: 'count', label: 'En yakın', type: 'number', integer: true, min: 1, max: 10000, optional: true, default: null, placeholder: 'Hepsi', description: 'Her başlangıç için en yakın kaç varış; boş: hepsi.' },
    { name: 'cutoff', label: 'Üst sınır', type: 'number', optional: true, default: null, min: 0, placeholder: 'Yok', description: 'Maliyeti bundan büyük varış yazılmaz.' },
    { name: 'reach', label: 'Arama uzaklığı', type: 'number', unit: 'm', min: 0.001, max: 100000, default: 100, advanced: true, description: 'Noktalar ağa bu uzaklıktan yakınsa ağın en yakın yerine oturur.' },
    { name: 'lines', label: 'Düz çizgiler', type: 'boolean', default: false, description: 'Her satır için başlangıçtan varışa düz çizgi yazılır.' },
    { name: 'layer', label: 'Çizgilerin katmanı', type: 'layer', default: { newName: 'Maliyet matrisi' }, newLayerStyle: { color: '#00897B', lineType: 'continuous', lineWeight: 0.18 }, visibleWhen: (v: Shown) => v.lines === true },
  ] as const,
  outputs: [
    { name: 'table', label: 'Maliyet matrisi', type: 'table' },
    { name: 'lines', label: 'Çizgiler', type: 'features' },
  ],
  run: (v, ctx, feedback) => {
    const n = runNetwork(v.network, ctx, feedback);
    try {
      const origins = placesOf(n.net, v.origins.entities, v.reach);
      const targets = placesOf(n.net, v.destinations.entities, v.reach);
      missingNote(origins.missing, 'başlangıç', v.reach, feedback);
      missingNote(targets.missing, 'varış', v.reach, feedback);
      if (!origins.found.length || !targets.found.length) return { refused: 'Ağın üstünde başlangıç ya da varış yok; noktaların ağa yakın olduğunu ya da Arama uzaklığını denetleyin.' };
      feedback.progress(0, 'Maliyetler hesaplanıyor');
      const rows = JSON.parse(
        n.net.nearest(JSON.stringify(origins.found.map((p) => p.at)), JSON.stringify(targets.found.map((p) => p.at)), v.reach, v.count ?? -1, v.cutoff ?? Number.NaN, n.cost, false, '[]', false),
      ) as NetworkNearest;
      if (!Array.isArray(rows)) return { refused: 'Noktalar ağda bulunamadı.' };
      const cost = n.costNames[n.cost];
      const table: string[][] = [];
      const add: NewEntity[] = [];
      (rows as readonly (readonly NetworkFound[])[]).forEach((row, i) =>
        row.forEach((f, rank) => {
          const from = origins.found[i];
          const to = targets.found[f.target];
          const value = costValue(f.cost, n.cost, ctx.units.lengthDecimals);
          table.push([nameOf(from.entity), nameOf(to.entity), String(rank + 1), value]);
          if (v.lines && v.layer)
            add.push(
              newObject({ kind: 'line', a: { x: from.at[0], y: from.at[1] }, b: { x: to.at[0], y: to.at[1] } } as never, v.layer.id, {
                Başlangıç: nameOf(from.entity),
                Varış: nameOf(to.entity),
                Sıra: String(rank + 1),
                Ağ: n.def.name,
                Maliyet: cost,
                [cost]: value,
              }),
            );
        }),
      );
      return {
        ...(add.length && { changes: { add } }),
        outputs: { table: { columns: ['Başlangıç', 'Varış', 'Sıra', cost], rows: table } },
        summary: `${origins.found.length} başlangıçtan ${targets.found.length} varışa ${table.length} satır (“${n.def.name}” ağında ${cost}).`,
      };
    } finally {
      n.net.free();
    }
  },
});
