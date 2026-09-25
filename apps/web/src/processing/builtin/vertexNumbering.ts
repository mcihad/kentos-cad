import type { NewEntity } from '../../model/entities';
import { defineTool, type Shown } from '../types';
import { formatNumber, nameCorners, numberedPoints } from './numbering';

/**
 * Köşe noktalarını numarala: every corner of the chosen areas (and paths)
 * gets a point named in a fixed format — "P00001", "P00002", … — walked
 * clockwise or counter-clockwise from a chosen start corner. Corners
 * shared by neighbouring parcels get one number; points already on the
 * target layer keep theirs and numbering continues after them.
 */
export const vertexNumbering = defineTool({
  id: 'points.numberVertices',
  label: 'Köşe noktalarını numarala',
  category: 'points',
  icon: 'numberVertices',
  description: 'Alanların köşelerine belirlediğiniz biçimde numaralı nokta koyar (P00001, P00002 …); yön ve başlangıç köşesi seçilir.',
  help: [
    'Her alanın köşeleri seçilen yönde, seçilen başlangıç köşesinden itibaren numaralanır; delikli alanlarda önce dış halka, sonra delikler.',
    'Birden çok alan varsa sıra başlangıç köşelerine göredir: kuzeybatıdakiler önce. “Seçilen noktaya en yakın” başlangıçta noktaya yakın olan alan önce gelir.',
    'Komşu parsellerin ortak köşesi tek numara alır. Hedef katmanda aynı biçimde numaralanmış noktalar varsa onların adı korunur ve numara kaldığı yerden devam eder.',
  ].join('\n\n'),
  keywords: ['numara', 'köşe', 'nokta', 'isim', 'vertex', 'number', 'label', 'parsel'],
  aliases: ['KOSENUMARA', 'KNUM', 'NUMARALA'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: ['polygon', 'polyline'], description: 'Köşeleri numaralanacak kapalı alanlar ve çoklu çizgiler.', default: { scope: 'selection' } },
    {
      name: 'direction',
      label: 'Yön',
      type: 'enum',
      options: [
        { value: 'cw', label: 'Saat yönünde' },
        { value: 'ccw', label: 'Saat yönünün tersine' },
      ],
      default: 'cw',
    },
    {
      name: 'start',
      label: 'Başlangıç köşesi',
      type: 'enum',
      description: 'Numaralamanın başlayacağı köşe.',
      options: [
        { value: 'northwest', label: 'Kuzeybatı', hint: 'Kuzeybatı yönüne en yakın köşe' },
        { value: 'north', label: 'En kuzey', hint: 'Kuzeyi en büyük köşe' },
        { value: 'first', label: 'İlk çizilen', hint: 'Nesnenin ilk köşesi' },
        { value: 'point', label: 'Seçilen noktaya en yakın', hint: 'Haritada gösterdiğiniz noktaya en yakın köşe' },
      ],
      default: 'northwest',
    },
    { name: 'startPoint', label: 'Başlangıç noktası', type: 'point', description: 'Her alanda bu noktaya en yakın köşeden başlanır.', visibleWhen: (v: Shown) => v.start === 'point' },
    { name: 'prefix', label: 'Önek', type: 'string', default: 'P', allowEmpty: true, maxLength: 12, description: 'Numaranın başındaki yazı.' },
    { name: 'length', label: 'Toplam uzunluk', type: 'number', integer: true, min: 1, max: 30, default: 6, description: 'Önek dahil karakter sayısı: P00001 için 6.' },
    { name: 'pad', label: 'Doldurma karakteri', type: 'string', default: '0', allowEmpty: true, maxLength: 1, description: 'Önek ile rakamlar arasını doldurur; boş bırakılırsa doldurulmaz.' },
    { name: 'first', label: 'İlk numara', type: 'number', integer: true, min: 0, default: 1 },
    { name: 'step', label: 'Artış', type: 'number', integer: true, min: 1, default: 1, advanced: true },
    { name: 'shared', label: 'Ortak köşelere tek numara', type: 'boolean', default: true, description: 'Komşu alanların aynı yerdeki köşeleri tek nokta olur.' },
    { name: 'tolerance', label: 'Aynı nokta toleransı', type: 'number', min: 0, max: 1, default: 0.001, unit: 'm', advanced: true, visibleWhen: (v: Shown) => v.shared === true },
    {
      name: 'output',
      label: 'Oluşturulacak',
      type: 'enum',
      options: [
        { value: 'points', label: 'Nokta', hint: 'Adı etiket olarak görünen nokta nesnesi' },
        { value: 'text', label: 'Yazı', hint: 'Köşenin dışına yazı' },
        { value: 'both', label: 'Nokta ve yazı' },
      ],
      default: 'points',
    },
    { name: 'textHeight', label: 'Yazı yüksekliği', type: 'number', min: 0.1, max: 50, default: 2, unit: 'mm', description: 'Kâğıt üzerinde, çizim ölçeğine göre.', visibleWhen: (v: Shown) => v.output !== 'points' },
    {
      name: 'layer',
      label: 'Hedef katman',
      type: 'layer',
      default: { newName: 'Köşe noktaları' },
      description: 'Bu adda katman yoksa oluşturulur.',
      newLayerStyle: { color: '#E5C07B', point: { symbol: 'cross', size: 7 }, label: { placement: 'beside', size: 10.5 } },
    },
  ] as const,
  outputs: [
    { name: 'points', label: 'Numaralı noktalar', type: 'features' },
    { name: 'count', label: 'Yeni numara sayısı', type: 'number' },
  ],
  validate: (v) => (v.pad && v.prefix.length >= v.length ? 'Toplam uzunluk önekten uzun olmalı; yoksa doldurma yapılamaz.' : null),
  preview: (v) => {
    const f = { prefix: v.prefix, length: v.length, pad: v.pad };
    return `${formatNumber(v.first, f)}, ${formatNumber(v.first + v.step, f)}, ${formatNumber(v.first + 2 * v.step, f)} …`;
  },
  run: (v, ctx) => {
    const format = { prefix: v.prefix, length: v.length, pad: v.pad };
    // Areas and paths have corners; the core walks their rings (a polygon's holes after its outer ring).
    const shapes = v.input.entities.filter((e) => e.kind === 'polygon' || e.kind === 'polyline');
    if (!shapes.length) return { summary: 'Numaralanacak alan yok.' };
    const layer = v.layer;
    const existing = v.shared || !layer.isNew
      ? ctx.doc.byLayer(layer.id).flatMap((e) => (e.kind === 'point' ? [{ p: e.p, name: e.label ?? e.attrs.Nokta ?? '' }] : []))
      : [];
    const named = numberedPoints(existing);
    const walk = { dir: v.direction, start: v.start, point: v.startPoint ?? null, tolerance: v.tolerance ?? 0.001, shared: v.shared };
    const found = ctx.geometry.numberCorners(
      shapes.map((e) => e.id),
      walk,
      v.shared ? named.map((e) => e.p) : [],
    );
    const corners = nameCorners(found, named, { format, first: v.first, step: v.step });
    const created = corners.filter((c) => c.created);
    const height = ((v.textHeight ?? 2) / 1000) * ctx.units.plotScale;
    // Outside the corner, centred on its bisector (placed by the core from the name's width in the drawing's typeface).
    const texts = v.output !== 'points' ? ctx.geometry.cornerTexts(created, created.map((c) => c.name), height, ctx.units.drawingFont) : [];
    const add: NewEntity[] = [];
    created.forEach((c, i) => {
      const attrs = { Nokta: c.name, Tür: 'Köşe noktası' };
      if (v.output !== 'text') add.push({ kind: 'point', layerId: layer.id, p: { ...c.p }, label: c.name, attrs });
      if (v.output !== 'points') add.push({ kind: 'text', layerId: layer.id, p: texts[i], text: c.name, height, rotation: 0, attrs });
    });
    const first = created[0]?.name;
    const last = created[created.length - 1]?.name;
    // Reused numbers: from points already on the layer, or from a neighbour numbered in this run.
    const before = new Set(existing.map((e) => e.name));
    const kept = new Set(corners.filter((c) => !c.created && before.has(c.name)).map((c) => c.name)).size;
    const shared = new Set(corners.filter((c) => !c.created && !before.has(c.name)).map((c) => c.name)).size;
    const notes = [shared ? `${shared} ortak köşe komşularıyla tek numara aldı` : '', kept ? `${kept} köşe mevcut numarasını korudu` : ''].filter(Boolean);
    return {
      changes: { add },
      outputs: { count: created.length },
      summary: created.length
        ? `${shapes.length} nesnede ${created.length} köşe numaralandı: ${first} – ${last}${notes.length ? `; ${notes.join('; ')}` : ''}.`
        : 'Yeni numara gerekmedi: bütün köşelerin numarası zaten var.',
    };
  },
});
