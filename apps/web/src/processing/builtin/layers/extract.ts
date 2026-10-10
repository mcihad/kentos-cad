import { defineTool } from '../../types';
import { copyOf, MODE_OPTIONS } from './shared';

/**
 * Seçilenleri çıkar (docs/adr/0217 §4; QGIS Extract selected features): the objects copied, or moved, into the result
 * layer; a new result layer takes the look and fields of the first object's layer and goes right above it. Moving
 * leaves the objects already there as they are. One undo step.
 */

export const extractSelected = defineTool({
  id: 'layers.extract',
  label: 'Seçilenleri çıkar',
  category: 'layers',
  icon: 'extractSelected',
  description: 'Seçili nesneleri yeni ya da var olan bir katmana kopyalar ya da taşır.',
  help: [
    'Örnekler: bir sorgunun seçtiği parselleri ayrı bir katmanda toplamak; elle seçilen yapıları bir iş katmanına taşımak.',
    'Yeni sonuç katmanı ilk nesnenin katmanının görünüşünü ve alanlarını alır ve onun hemen üstüne açılır. Kopyala nesneleri yerinde bırakır; Taşı onları sonuç katmanına geçirir. Kilitli katmandaki nesneler alınmaz.',
  ].join('\n\n'),
  keywords: ['çıkar', 'seçili', 'seçilen', 'katman', 'kopyala', 'taşı', 'extract', 'extract selected features', 'export selection'],
  aliases: ['SECILENLERICIKAR'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', scopes: ['selection', 'layer', 'visible', 'all'], writes: true, description: 'Çıkarılacak nesneler; kilitli katmandakiler alınmaz.' },
    { name: 'mode', label: 'İşlem', type: 'enum', options: MODE_OPTIONS, default: 'copy' },
    { name: 'output', label: 'Sonuç', type: 'layer', default: { newName: 'Seçilenler' }, newLayerStyle: { color: '#0090FF' }, above: 'input' },
  ] as const,
  outputs: [
    { name: 'objects', label: 'Çıkarılan nesneler', type: 'features' },
    { name: 'count', label: 'Nesne sayısı', type: 'number' },
  ],
  run: async (v) => {
    const inputs = v.input.entities;
    const out = v.output;
    const like = inputs[0]?.layerId;
    if (v.mode === 'move') {
      const moved = inputs.filter((e) => e.layerId !== out.id);
      return {
        changes: { update: moved.map((e) => ({ id: e.id, patch: { layerId: out.id } })) },
        outputs: { objects: moved.map((e) => e.id), count: moved.length },
        summary: `${moved.length} nesne “${out.name}” katmanına taşındı.`,
        ...(like !== undefined && { like }),
      };
    }
    return {
      changes: { add: inputs.map((e) => copyOf(e, out.id)) },
      outputs: { count: inputs.length },
      summary: `${inputs.length} nesne “${out.name}” katmanına kopyalandı.`,
      ...(like !== undefined && { like }),
    };
  },
});
