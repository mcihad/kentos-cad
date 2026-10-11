import { entityGeometry, type NewEntity } from '../../../model/entities';
import { sampleAlong } from '../../../model/ops/sampling';
import { defineTool } from '../../types';
import { outputLayer, zsOf } from '../convert/shared';
import { AREA_KINDS } from '../geometry/shared';
import { bearingText, lengthText } from '../proximity/shared';
import { LINE_KINDS, SAMPLE_SCOPES, curvedNote, pointOn, written } from './shared';

/**
 * Hat boyunca nokta (docs/adr/0220 §5; QGIS Points along geometry, ArcGIS Generate Points Along Lines): points along
 * each path of the lines and of the areas' boundaries, every so far or at equal parts, with their distance, bearing
 * and elevation, from the core (`ops::sampling::along`).
 */

const byParts = (v: { mode?: unknown }) => v.mode === 'parts';

export const alongLines = defineTool({
  id: 'sample.alongLines',
  label: 'Hat boyunca nokta',
  category: 'sampling',
  icon: 'pointsAlongLine',
  description: 'Çizgilerin ve alan sınırlarının üstüne aralıkla ya da eşit parçalara noktalar koyar; noktalar uzaklık ve semt alır.',
  help: [
    'Örnekler: bir enerji hattına 50 metrede bir direk yeri, bir yolun 20 metrelik enkesit yerleri, bir parsel sınırına eşit aralıklı noktalar koymak.',
    'Aralıkla iken noktalar yolun başından Başlangıç payı ötede başlar, Aralık kadar ilerler, sondan Bitiş payı kadar önce durur; Uçları da açıksa yolun başı ve sonu da eklenir. Eşit parçalara iken yol Parça sayısı kadar eşit parçaya bölünür. Kapalı yolun (alan sınırı, daire) sonu başıdır.',
    "Nokta nesnenin özniteliklerini, yolun başından Uzaklık'ı ve oradaki gidiş doğrultusunu Semt olarak (köşede sonraki kenarınki) alır; birden çok yolu olan nesnede Parça. Kot köşelerin kotlarından doğrusal taşınır. Yaylar kesin; elips ve eğri 0,1 mm'lik kirişlerle.",
  ].join('\n\n'),
  keywords: ['hat', 'boyunca', 'nokta', 'aralık', 'direk', 'enkesit', 'points along lines', 'parçala', 'böl'],
  aliases: ['HATNOKTA', 'ARALIKNOKTA'],
  targets: ['client', 'worker'],
  parameters: [
    {
      name: 'input',
      label: 'Çizgiler',
      type: 'features',
      kinds: [...LINE_KINDS, ...AREA_KINDS],
      scopes: SAMPLE_SCOPES,
      description: 'Üstüne nokta konacak çizgiler ve sınırlarına nokta konacak alanlar.',
    },
    {
      name: 'mode',
      label: 'Yerleşim',
      type: 'enum',
      options: [
        { value: 'interval', label: 'Aralıkla', hint: 'Belli uzaklıkta bir' },
        { value: 'parts', label: 'Eşit parçalara', hint: 'Yolu eşit parçalara böler' },
      ],
      default: 'interval',
    },
    { name: 'step', label: 'Aralık', type: 'number', unit: 'm', min: 0.001, default: 10, visibleWhen: (v: { mode?: unknown }) => !byParts(v), description: 'İki nokta arası yol boyunca uzaklık.' },
    { name: 'start', label: 'Başlangıç payı', type: 'number', unit: 'm', min: 0, default: 0, visibleWhen: (v: { mode?: unknown }) => !byParts(v), description: 'İlk noktanın yolun başından uzaklığı.' },
    { name: 'end', label: 'Bitiş payı', type: 'number', unit: 'm', min: 0, default: 0, visibleWhen: (v: { mode?: unknown }) => !byParts(v), description: 'Son noktanın yolun sonuna en az uzaklığı.' },
    { name: 'parts', label: 'Parça sayısı', type: 'number', integer: true, min: 1, default: 10, visibleWhen: byParts, description: 'Yolun bölüneceği eşit parça sayısı.' },
    { name: 'ends', label: 'Uçları da', type: 'boolean', default: true, description: 'Yolun başı ve sonu da nokta olur.' },
    outputLayer('Hat noktaları', '#F76B15'),
  ] as const,
  outputs: [
    { name: 'points', label: 'Yazılan noktalar', type: 'features' },
    { name: 'count', label: 'Nokta sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Noktalar yerleştiriliyor');
    const mode = v.mode === 'parts' ? 'parts' : 'interval';
    const got = sampleAlong(inputs.map(entityGeometry), inputs.map(zsOf), mode, v.step ?? 10, v.start ?? 0, v.end ?? 0, v.parts ?? 10, v.ends);
    if ('refused' in got) return { refused: got.refused };
    curvedNote(feedback, inputs);
    const r = got.points;
    const add: NewEntity[] = [];
    r.at.forEach((o, i) => {
      const attrs: Record<string, string> = { ...inputs[o].attrs };
      attrs.Uzaklık = lengthText(ctx.units, r.s?.[i] ?? 0);
      attrs.Semt = bearingText(ctx.units, r.bearing?.[i] ?? 0);
      const part = r.path?.[i] ?? 0;
      if (part > 0) attrs.Parça = String(part);
      add.push(pointOn(v.layer.id, attrs, { x: r.xy[2 * i], y: r.xy[2 * i + 1] }, r.z?.[i]));
    });
    return written(add, `${inputs.length} nesneden ${add.length} nokta yazıldı.`);
  },
});
