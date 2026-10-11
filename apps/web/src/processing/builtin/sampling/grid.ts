import { entityGeometry, type NewEntity } from '../../../model/entities';
import { sampleGrid, type GridShape } from '../../../model/ops/sampling';
import { defineTool } from '../../types';
import { outputLayer } from '../convert/shared';
import { GEO_KINDS } from '../geometry/shared';
import { lengthText } from '../proximity/shared';
import { SAMPLE_SCOPES, written } from './shared';

/**
 * Ağ oluştur (docs/adr/0220 §2; QGIS Create grid, ArcGIS Create Fishnet and Generate Tessellation, Netcad Karelaj
 * Üret): rectangular, hexagonal or triangular cells, or a rectangular grid's lines, over the objects' box, all of them
 * or those that meet the objects, from the core's lattice (`ops::sampling::grid`).
 */

const shapeIs = (v: { shape?: unknown }, keys: readonly string[]) => keys.includes(typeof v.shape === 'string' ? v.shape : 'rect');

export const grid = defineTool({
  id: 'sample.grid',
  label: 'Ağ oluştur',
  category: 'sampling',
  icon: 'fishnet',
  description: 'Nesnelerin kutusunu dikdörtgen, altıgen ya da üçgen hücrelerle ya da ağın çizgileriyle kaplayıp yeni katmana yazar.',
  help: [
    'Örnekler: bir mahalleyi sayım için altıgen hücrelere bölmek, pafta için karelaj çizgileri çizmek, örnekleme çerçevesi için balık ağı kurmak.',
    'Koordinatlara otur açıkken ağ koordinat başlangıcına bağlı genel kafese oturur: dikdörtgenler aralığın katlarında başlar, ayrı üretilen ağlar birbirine uyar. Kapalıyken ağ kapsamın sol alt köşesinden başlar. Komşu hücreler köşelerini paylaşır.',
    "Hücreler Satır ve Sütun alır (satırlar yukarıdan, sütunlar soldan); yatay çizgiler Satır ve Kuzey, düşey çizgiler Sütun ve Doğu. Nesnelere değenler seçiliyse yalnız kapsamın nesnelerine değen ya da onları kesen hücreler yazılır (Konuma göre seç'in Kesişen'i). En çok 1 000 000 parça yazılır.",
  ].join('\n\n'),
  keywords: ['ağ', 'balık ağı', 'karelaj', 'altıgen', 'üçgen', 'hücre', 'grid', 'fishnet', 'tessellation', 'hexagon'],
  aliases: ['AG', 'AGOLUSTUR', 'FISHNET'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Kapsam', type: 'features', kinds: GEO_KINDS, scopes: SAMPLE_SCOPES, description: 'Ağın kaplayacağı nesneler; ağ onların kutusunu kaplar.' },
    {
      name: 'shape',
      label: 'Biçim',
      type: 'enum',
      options: [
        { value: 'rect', label: 'Dikdörtgen', hint: 'Doğu ve kuzey aralıklı hücreler' },
        { value: 'hexagon', label: 'Altıgen', hint: 'Düz üstlü düzgün altıgenler' },
        { value: 'triangle', label: 'Üçgen', hint: 'Eşkenar üçgenler' },
        { value: 'lines', label: 'Çizgi', hint: 'Dikdörtgen ağın çizgileri' },
      ],
      default: 'rect',
    },
    { name: 'dx', label: 'Doğu aralığı', type: 'number', unit: 'm', min: 0.001, default: 100, visibleWhen: (v: { shape?: unknown }) => shapeIs(v, ['rect', 'lines']), description: 'Hücrenin doğu–batı boyu.' },
    { name: 'dy', label: 'Kuzey aralığı', type: 'number', unit: 'm', min: 0.001, default: 100, visibleWhen: (v: { shape?: unknown }) => shapeIs(v, ['rect', 'lines']), description: 'Hücrenin kuzey–güney boyu.' },
    { name: 'side', label: 'Kenar', type: 'number', unit: 'm', min: 0.001, default: 100, visibleWhen: (v: { shape?: unknown }) => shapeIs(v, ['hexagon', 'triangle']), description: 'Altıgenin ya da üçgenin kenarı.' },
    { name: 'snap', label: 'Koordinatlara otur', type: 'boolean', default: true, description: 'Ağ aralığın katlarına oturur; kapalıyken kapsamın sol alt köşesinden başlar.' },
    {
      name: 'keep',
      label: 'Hücreler',
      type: 'enum',
      options: [
        { value: 'all', label: 'Kutunun hepsi', hint: 'Kapsamın kutusunu kaplayan bütün hücreler' },
        { value: 'meeting', label: 'Nesnelere değenler', hint: 'Kapsamın nesnelerine değen ya da onları kesen hücreler' },
      ],
      default: 'all',
      visibleWhen: (v: { shape?: unknown }) => !shapeIs(v, ['lines']),
    },
    outputLayer('Ağ', '#8E4EC6'),
  ] as const,
  outputs: [
    { name: 'cells', label: 'Yazılan parçalar', type: 'features' },
    { name: 'count', label: 'Parça sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    const kind = v.shape as GridShape;
    feedback.progress(0, 'Ağ kuruluyor');
    const got = sampleGrid(inputs.map(entityGeometry), kind, v.dx ?? 100, v.dy ?? 100, v.side ?? 100, v.snap, v.keep === 'meeting');
    if ('refused' in got) return { refused: got.refused };
    const { rows, cols, counts, xy } = got.grid;
    const layerId = v.layer.id;
    const add: NewEntity[] = [];
    let at = 0;
    for (let i = 0; i < counts.length; i++) {
      const n = counts[i];
      const pts = [];
      for (let k = 0; k < n; k++) pts.push({ x: xy[2 * (at + k)], y: xy[2 * (at + k) + 1] });
      at += n;
      if (kind === 'lines') {
        const attrs: Record<string, string> = rows[i]
          ? { Satır: String(rows[i]), Kuzey: lengthText(ctx.units, pts[0].y) }
          : { Sütun: String(cols[i]), Doğu: lengthText(ctx.units, pts[0].x) };
        add.push({ kind: 'line', layerId, attrs, a: pts[0], b: pts[1] } as NewEntity);
      } else {
        add.push({ kind: 'polygon', layerId, attrs: { Satır: String(rows[i]), Sütun: String(cols[i]) }, pts } as NewEntity);
      }
    }
    return written(add, `${add.length} ${kind === 'lines' ? 'çizgi' : 'hücre'} yazıldı.`);
  },
});
