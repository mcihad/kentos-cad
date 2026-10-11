import { entityGeometry, type NewEntity } from '../../../model/entities';
import { generatePoles } from '../../../model/ops/generate';
import { defineTool } from '../../types';
import { outputLayer } from '../convert/shared';
import { AREA_KINDS } from '../geometry/shared';
import { lengthText } from '../proximity/shared';
import { GENERATE_SCOPES, inputNotes } from './shared';

/**
 * Ulaşılmazlık kutbu (docs/adr/0219 §6; QGIS Pole of inaccessibility): each area's point farthest from its edges,
 * holes too, within the precision (Mapbox's polylabel, the label engine's), the part's whose pole is farthest in a
 * multi-part area, a circle's centre; a point with the area's attributes and Uzaklık.
 */

export const pole = defineTool({
  id: 'generate.pole',
  label: 'Ulaşılmazlık kutbu',
  category: 'generation',
  icon: 'poleOfInaccessibility',
  description: 'Her alanın kenarlarından en uzak iç noktasını, kenara uzaklığıyla yeni katmana nokta olarak yazar.',
  help: [
    'Örnekler: parsel numaralarını ve etiketleri alanın en geniş yerine koymak, ince uzun ya da L biçimli alanların içinde kalan bir nokta bulmak.',
    'Nokta alanın içindedir; delikler de kenar sayılır. Çok parçalı alanda kutbu en uzak olan parçanınki alınır, dairede merkezi. Hassasiyet kutbun en uzak noktaya ne kadar yakın olacağıdır; küçüldükçe süre uzar.',
    'Nokta alanın özniteliklerini ve kenara uzaklığını (Uzaklık) taşır.',
  ].join('\n\n'),
  keywords: ['kutup', 'ulaşılmazlık', 'polylabel', 'iç nokta', 'merkez', 'etiket noktası', 'pole of inaccessibility'],
  aliases: ['KUTUP', 'POLYLABEL'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: AREA_KINDS, scopes: GENERATE_SCOPES, description: 'Kutbu bulunacak alanlar.' },
    { name: 'precision', label: 'Hassasiyet', type: 'number', unit: 'm', min: 0.001, default: 0.1, description: 'Kutbun gerçek en uzak noktaya olabilecek uzaklığı.' },
    outputLayer('Kutuplar', '#E54666'),
  ] as const,
  outputs: [
    { name: 'points', label: 'Yazılan noktalar', type: 'features' },
    { name: 'count', label: 'Nokta sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Kutuplar hesaplanıyor');
    const r = generatePoles(inputs.map(entityGeometry), v.precision);
    inputNotes(feedback, inputs, r.curved);
    const add: NewEntity[] = [];
    let empty = 0;
    r.list.forEach((q, i) => {
      if (!q) return void (empty += 1);
      add.push({ kind: 'point', layerId: v.layer.id, attrs: { ...inputs[i].attrs, Uzaklık: lengthText(ctx.units, q.d) }, p: q.p });
    });
    if (empty) feedback.warn(`${empty} nesnenin alanı yok; kutbu yazılmadı.`);
    return { changes: { add }, outputs: { count: add.length }, summary: `${inputs.length} nesneden ${add.length} nokta yazıldı.` };
  },
});
