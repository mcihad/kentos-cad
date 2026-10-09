import type { NewEntity } from '../../../model/entities';
import { lineGeometry, type NetworkFound, type NetworkNearest } from '../../../model/networkAnswers';
import { defineTool, type Shown } from '../../types';
import { newObject } from '../geometry/shared';
import { costValue, missingNote, nameOf, placesOf, runNetwork } from './shared';

/**
 * En yakın tesis (docs/adr/0209 §7; ArcGIS's Closest Facility, QGIS's shortest path to the nearest point): for each
 * incident the `count` cheapest facilities along the network, within an upper bound when one is given; from the
 * incident to the facility or the other way. Each way is written to the output layer as a polyline with its incident,
 * facility, rank and costs; the same rows are the table. Equal costs: the facility first in the list first.
 */
export const networkClosestFacility = defineTool({
  id: 'network.closestFacility',
  label: 'En yakın tesis',
  category: 'network',
  icon: 'closestFacility',
  description: 'Her olay noktası için ağ boyunca en yakın tesisleri (okul, sağlık ocağı, itfaiye) bulur; yollarını ve maliyetlerini yazar.',
  help: [
    'Ağ projenin yol ya da şebeke ağıdır (Ağlar); maliyet uzunluk ya da ağın süresi ve maliyetleridir. Tek yönler ve kapalı yollar gözetilir.',
    'Olaylar ve tesisler noktalardır; ağa Arama uzaklığından uzak olan nokta alınmaz ve söylenir.',
    'Her olayın en yakın tesisleri Tesis sayısı kadar, maliyeti küçükten büyüğe yazılır; Üst sınır verilirse ondan pahalı tesis alınmaz. Eşit maliyette listede önce gelen tesis önce gelir.',
    'Her yol çıktı katmanına çoklu çizgi olarak yazılır: Olay, Tesis, Sıra, Ağ, Maliyet ve her maliyetin toplamıyla; aynı satırlar tabloda.',
  ].join('\n\n'),
  keywords: ['en yakın tesis', 'closest facility', 'en yakın', 'itfaiye', 'okul', 'hastane', 'ağ', 'rota', 'yol'],
  aliases: ['ENYAKINTESIS', 'CLOSESTFACILITY'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'network', label: 'Ağ', type: 'network', prefers: 'road', description: 'Yolların bulunacağı ağ ve maliyet.' },
    { name: 'incidents', label: 'Olaylar', type: 'features', kinds: ['point'], scopes: ['layer', 'selection', 'visible', 'all'], description: 'Tesislerin aranacağı noktalar (adres, olay yeri).' },
    { name: 'facilities', label: 'Tesisler', type: 'features', kinds: ['point'], scopes: ['layer', 'selection', 'visible', 'all'], description: 'Okul, sağlık ocağı, itfaiye istasyonu gibi noktalar.' },
    { name: 'count', label: 'Tesis sayısı', type: 'number', integer: true, min: 1, max: 10, default: 1, description: 'Her olay için en yakın kaç tesis.' },
    { name: 'cutoff', label: 'Üst sınır', type: 'number', optional: true, default: null, min: 0, placeholder: 'Yok', description: 'Maliyeti bundan büyük tesis alınmaz (maliyetin biriminde: m, dk ya da alanın birimi).' },
    {
      name: 'direction',
      label: 'Yön',
      type: 'enum',
      options: [
        { value: 'toFacility', label: 'Olaydan tesise', hint: 'Olaydan çıkılır (hasta tesise gider)' },
        { value: 'fromFacility', label: 'Tesisten olaya', hint: 'Tesisten çıkılır (itfaiye olaya gider)' },
      ],
      default: 'toFacility',
    },
    { name: 'reach', label: 'Arama uzaklığı', type: 'number', unit: 'm', min: 0.001, max: 100000, default: 100, advanced: true, description: 'Noktalar ağa bu uzaklıktan yakınsa ağın en yakın yerine oturur.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'En yakın tesis' }, newLayerStyle: { color: '#7B1FA2', lineType: 'continuous', lineWeight: 0.5 }, description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'routes', label: 'Yollar', type: 'features' },
    { name: 'table', label: 'En yakın tesisler', type: 'table' },
  ],
  preview: (v: Shown) => (v.cutoff ? `En yakın ${v.count} tesis, en çok ${v.cutoff}` : `En yakın ${v.count} tesis`),
  run: (v, ctx, feedback) => {
    const n = runNetwork(v.network, ctx, feedback);
    try {
      const incidents = placesOf(n.net, v.incidents.entities, v.reach);
      const facilities = placesOf(n.net, v.facilities.entities, v.reach);
      missingNote(incidents.missing, 'olay', v.reach, feedback);
      missingNote(facilities.missing, 'tesis', v.reach, feedback);
      if (!incidents.found.length || !facilities.found.length) return { refused: 'Ağın üstünde olay ya da tesis yok; noktaların ağa yakın olduğunu ya da Arama uzaklığını denetleyin.' };
      feedback.progress(0, 'En yakın tesisler aranıyor');
      const rows = JSON.parse(
        n.net.nearest(
          JSON.stringify(incidents.found.map((p) => p.at)),
          JSON.stringify(facilities.found.map((p) => p.at)),
          v.reach,
          v.count,
          v.cutoff ?? Number.NaN,
          n.cost,
          v.direction === 'fromFacility',
          '[]',
          true,
        ),
      ) as NetworkNearest;
      if (!Array.isArray(rows)) return { refused: 'Noktalar ağda bulunamadı.' };
      const decimals = ctx.units.lengthDecimals;
      const cost = n.costNames[n.cost];
      const add: NewEntity[] = [];
      const table: string[][] = [];
      let none = 0;
      (rows as readonly (readonly NetworkFound[])[]).forEach((row, i) => {
        if (!row.length) none++;
        row.forEach((f, rank) => {
          const values = n.costNames.map((_, c) => costValue(f.totals?.[c], c, decimals));
          const incident = nameOf(incidents.found[i].entity);
          const facility = nameOf(facilities.found[f.target].entity);
          table.push([incident, facility, String(rank + 1), ...values]);
          if (f.line && f.line.pts.length >= 2) {
            const attrs: Record<string, string> = { Olay: incident, Tesis: facility, Sıra: String(rank + 1), Ağ: n.def.name, Maliyet: cost };
            n.costNames.forEach((name, c) => (attrs[name] = values[c]));
            add.push(newObject(lineGeometry(f.line), v.layer.id, attrs));
          }
        });
      });
      if (none) feedback.warn(`${none} olaya ${v.cutoff ? 'üst sınır içinde ' : ''}ulaşılabilen tesis yok.`);
      return {
        changes: { add },
        outputs: { table: { columns: ['Olay', 'Tesis', 'Sıra', ...n.costNames], rows: table } },
        summary: `${incidents.found.length} olay için ${table.length} tesis bulundu; ${add.length} yol “${n.def.name}” ağında ${cost} ile yazıldı.`,
      };
    } finally {
      n.net.free();
    }
  },
});
