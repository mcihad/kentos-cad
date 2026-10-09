import { fixed } from '../../../core/displayNumber';
import type { EntityGeometry } from '../../../model/entities';
import { lineGeometry, refusalWords, type NetworkServiceArea } from '../../../model/networkAnswers';
import { parseNumber } from '../../../tools/coordinateInput';
import { defineTool, type Shown } from '../../types';
import { newObject } from '../geometry/shared';
import { costValue, missingNote, nameOf, placesOf, runNetwork } from './shared';

/**
 * Hizmet alanları (docs/adr/0209 §6; ArcGIS's Service Area, QGIS's service area from layer): the facilities of a
 * layer and the network within each break of the chosen cost, as Hizmet alanı draws it: the areas (the reached lines'
 * buffers, Kenar payı wide; discs or rings; merged or one each) and, when asked, the lines.
 */

/** The breaks as written (“5 10 15”): rising numbers above zero, at most ten; null when they are not. */
export function breaksOf(text: string): number[] | null {
  const values = text.split(/[\s;]+/).filter(Boolean).map(parseNumber);
  const ok = values.length > 0 && values.length <= 10 && values.every((v, i) => v !== null && v > 0 && Number.isFinite(v) && (i === 0 || v > values[i - 1]!));
  return ok ? (values as number[]) : null;
}

/** A number as a label shows it (“0–300”). */
const plain = (v: number): string => fixed(v, 6).replace(/\.?0+$/, '');

export const networkServiceAreas = defineTool({
  id: 'network.serviceAreas',
  label: 'Hizmet alanları',
  category: 'network',
  icon: 'serviceAreas',
  description: 'Tesislerden ağ boyunca verilen uzaklık ya da sürelerde ulaşılan alanları çizer: aralık başına alan, isteğe bağlı ulaşılan yollarla.',
  help: [
    'Ağ projenin yol ya da şebeke ağıdır (Ağlar); Aralıklar maliyetin biriminde artan sayılardır (Uzunluk’ta metre, Süre’de dakika): “5 10 15”.',
    'Alan, ulaşılan yolların Kenar payı kadar tamponudur; Disk her aralığın bütün ulaşılanını, Halka bir öncekinden farkını yazar. Birleşik açıkken bütün tesislere aralık başına tek alan, kapalıyken her tesise ayrı alan yazılır.',
    'Yön Tesisten (tesisten çıkış) ya da Tesise (tesise varış; tek yönlü yollarda farklıdır).',
    'Alanlara Ağ, Maliyet, Tesis, Başlangıç ve Bitiş; çizgilere Aralık (“0–5”) yazılır.',
  ].join('\n\n'),
  keywords: ['hizmet alanı', 'service area', 'erişim', 'izokron', 'isochrone', 'ulaşım süresi', 'ağ', 'itfaiye'],
  aliases: ['HIZMETALANLARI', 'SERVICEAREAS'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'network', label: 'Ağ', type: 'network', prefers: 'road', description: 'Alanların bulunacağı ağ ve maliyet.' },
    { name: 'facilities', label: 'Tesisler', type: 'features', kinds: ['point'], scopes: ['layer', 'selection', 'visible', 'all'], description: 'Okul, sağlık ocağı, itfaiye istasyonu gibi noktalar.' },
    { name: 'breaks', label: 'Aralıklar', type: 'string', default: '500 1000 1500', placeholder: '5 10 15', description: 'Maliyetin biriminde artan sayılar, aralarında boşluk; en çok 10.' },
    {
      name: 'direction',
      label: 'Yön',
      type: 'enum',
      options: [
        { value: 'from', label: 'Tesisten', hint: 'Tesisten çıkılarak ulaşılan yerler' },
        { value: 'to', label: 'Tesise', hint: 'Tesise bu sürede varılan yerler' },
      ],
      default: 'from',
    },
    {
      name: 'shape',
      label: 'Biçim',
      type: 'enum',
      options: [
        { value: 'disc', label: 'Disk', hint: 'Her aralığın bütün ulaşılanı' },
        { value: 'ring', label: 'Halka', hint: 'Bir önceki aralıktan farkı' },
      ],
      default: 'disc',
    },
    { name: 'merged', label: 'Birleşik', type: 'boolean', default: true, description: 'Bütün tesislere aralık başına tek alan; kapalıyken her tesise ayrı.' },
    { name: 'trim', label: 'Kenar payı', type: 'number', unit: 'm', min: 0.01, max: 10000, default: 50, description: 'Alanın ulaşılan yollardan uzaklığı.' },
    { name: 'reach', label: 'Arama uzaklığı', type: 'number', unit: 'm', min: 0.001, max: 100000, default: 100, advanced: true, description: 'Tesisler ağa bu uzaklıktan yakınsa ağın en yakın yerine oturur.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Hizmet alanı' }, newLayerStyle: { color: '#1976D2', lineType: 'continuous', lineWeight: 0.25, fill: '#1976D233' }, description: 'Bu adda katman yoksa oluşturulur.' },
    { name: 'lines', label: 'Ulaşılan yollar', type: 'boolean', default: false, description: 'Aralıklarda ulaşılan yollar da yazılır.' },
    { name: 'linesLayer', label: 'Yolların katmanı', type: 'layer', default: { newName: 'Hizmet alanı çizgileri' }, newLayerStyle: { color: '#1565C0', lineType: 'continuous', lineWeight: 0.35 }, visibleWhen: (v: Shown) => v.lines === true },
  ] as const,
  outputs: [
    { name: 'areas', label: 'Alanlar', type: 'features' },
    { name: 'count', label: 'Alan sayısı', type: 'number' },
  ],
  validate: (v) => (breaksOf(v.breaks) ? null : '“Aralıklar” artan, sıfırdan büyük sayılar olmalı (en çok 10), aralarında boşluk: 5 10 15.'),
  run: (v, ctx, feedback) => {
    const breaks = breaksOf(v.breaks)!;
    const n = runNetwork(v.network, ctx, feedback);
    try {
      const facilities = placesOf(n.net, v.facilities.entities, v.reach);
      missingNote(facilities.missing, 'tesis', v.reach, feedback);
      if (!facilities.found.length) return { refused: 'Ağın üstünde tesis yok; noktaların ağa yakın olduğunu ya da Arama uzaklığını denetleyin.' };
      feedback.progress(0, 'Hizmet alanları hesaplanıyor');
      const ring = v.shape === 'ring';
      const answer = JSON.parse(
        n.net.area(JSON.stringify(facilities.found.map((p) => p.at)), JSON.stringify(breaks), v.reach, n.cost, v.direction === 'to', !v.merged, '[]', v.trim, ring, true),
      ) as NetworkServiceArea;
      if ('error' in answer) return { refused: refusalWords(answer, `${v.reach} m`) };
      const cost = n.costNames[n.cost];
      const all = facilities.found.map((f) => nameOf(f.entity)).join(', ');
      const facility = (i: number | null | undefined) => (i === null || i === undefined ? all : nameOf(facilities.found[i].entity));
      const number = (x: number) => costValue(x, n.cost, ctx.units.lengthDecimals);
      const add = answer.areas
        .filter((a) => a.shape)
        .map((a) =>
          newObject(a.shape as EntityGeometry, v.layer.id, {
            Ağ: n.def.name,
            Maliyet: cost,
            Tesis: facility(a.facility),
            Başlangıç: number(ring && a.band > 0 ? breaks[a.band - 1] : 0),
            Bitiş: number(breaks[a.band]),
          }),
        );
      const areas = add.length;
      if (v.lines && v.linesLayer)
        for (const l of answer.lines)
          add.push(
            newObject(lineGeometry(l.line), v.linesLayer.id, { Ağ: n.def.name, Maliyet: cost, Tesis: facility(l.facility), Aralık: `${plain(l.band > 0 ? breaks[l.band - 1] : 0)}–${plain(breaks[l.band])}` }),
          );
      if (!areas) feedback.warn('Hiçbir aralıkta ulaşılan yer yok.');
      return {
        changes: { add },
        outputs: { count: areas },
        summary: `${facilities.found.length} tesisin ${breaks.length} aralıkta ${areas} hizmet alanı yazıldı${v.lines ? ` (${add.length - areas} yol)` : ''}; “${n.def.name}” ağında ${cost}.`,
      };
    } finally {
      n.net.free();
    }
  },
});
