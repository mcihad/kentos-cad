import type { NewEntity } from '../../model/entities';
import { defineTool, type DefaultsContext } from '../types';

/**
 * Kenar uzunluklarını yaz: every edge of the chosen areas, paths and lines
 * gets its length as text, centred on the edge and turned to read along
 * it, outside (or inside) the shape. An edge shared by two parcels is
 * written once. Where each label goes and which edges are shared come from
 * the geometry core (docs/adr/0008, S4).
 */
export const edgeLengths = defineTool({
  id: 'annotation.edgeLengths',
  label: 'Kenar uzunluklarını yaz',
  category: 'annotation',
  icon: 'edgeLengths',
  description: 'Alan, çoklu çizgi ve çizgilerin her kenarına uzunluğunu yazar; ortak kenarlar bir kez yazılır.',
  help: [
    'Yazı kenarın ortasına, kenar boyunca okunur biçimde konur: kapalı alanlarda dışa (ya da içe), açık çizgilerde sola (ya da sağa).',
    'Yay kenarlarında yay boyu yazılır. Komşu parsellerin ortak kenarı bir kez yazılır; bunu kapatırsanız her alan kendi kenarını yazar.',
  ].join('\n\n'),
  keywords: ['kenar', 'uzunluk', 'ölçü', 'yazı', 'edge', 'length', 'label', 'parsel'],
  aliases: ['KENARYAZ', 'KENARUZUNLUK'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: ['polygon', 'polyline', 'line'], description: 'Kenar uzunlukları yazılacak alanlar, çoklu çizgiler ve çizgiler.', default: { scope: 'selection' } },
    { name: 'decimals', label: 'Ondalık basamak', type: 'number', integer: true, min: 0, max: 6, default: (c: DefaultsContext) => c.lengthDecimals },
    {
      name: 'side',
      label: 'Yazının yeri',
      type: 'enum',
      options: [
        { value: 'outside', label: 'Dışa', hint: 'Kapalı alanın dışına; açık çizgide soluna' },
        { value: 'inside', label: 'İçe', hint: 'Kapalı alanın içine; açık çizgide sağına' },
      ],
      default: 'outside',
    },
    { name: 'textHeight', label: 'Yazı yüksekliği', type: 'number', min: 0.1, max: 50, default: 2, unit: 'mm', description: 'Kâğıt üzerinde, çizim ölçeğine göre.' },
    { name: 'prefix', label: 'Önek', type: 'string', default: '', allowEmpty: true, maxLength: 12, advanced: true },
    { name: 'suffix', label: 'Sonek', type: 'string', default: '', allowEmpty: true, maxLength: 12, advanced: true, description: 'Örneğin “ m”.' },
    { name: 'minLength', label: 'En kısa kenar', type: 'number', min: 0, default: 0, unit: 'm', advanced: true, description: 'Bundan kısa kenarlar yazılmaz.' },
    { name: 'shared', label: 'Ortak kenarlara bir kez yaz', type: 'boolean', default: true },
    { name: 'layer', label: 'Hedef katman', type: 'layer', default: { newName: 'Kenar ölçüleri' }, description: 'Bu adda katman yoksa oluşturulur.', newLayerStyle: { color: 'fg-dim' } },
  ] as const,
  outputs: [
    { name: 'labels', label: 'Kenar yazıları', type: 'features' },
    { name: 'count', label: 'Yazı sayısı', type: 'number' },
  ],
  preview: (v) => `${v.prefix}${(12.3456).toFixed(v.decimals)}${v.suffix}`,
  run: (v, ctx) => {
    const height = (v.textHeight / 1000) * ctx.units.plotScale;
    // Lines, polylines and polygons have edges; one key per edge whichever way it runs (1 mm grid: parcels share exact corners).
    const { labels, skipped } = ctx.geometry.edgeLengths(
      v.input.entities.map((e) => e.id),
      height,
      v.minLength,
      v.side,
      v.shared,
    );
    const add: NewEntity[] = labels.map((l) => ({
      kind: 'text',
      layerId: v.layer.id,
      p: l.p,
      text: `${v.prefix}${l.length.toFixed(v.decimals)}${v.suffix}`,
      height,
      rotation: l.rotation,
      attrs: { Tür: 'Kenar ölçüsü', 'Uzunluk (m)': l.length.toFixed(3) },
    }));
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${v.input.entities.length} nesneye ${add.length} kenar uzunluğu yazıldı${skipped ? `; ${skipped} ortak kenar bir kez yazıldı` : ''}.`,
    };
  },
});
