// The label engine's commands (docs/adr/0212 §4; the desktop's `labelling.rs`): Etiketler (the layer's labelling
// window, loaded on first use, CLAUDE.md §20), Sabit etiketleri vurgula and Yerleşmeyen etiketleri göster (the user's
// view preferences: the drawing does not change).

import type { LayerNode } from '../model/layers';
import type { AppContext } from './context';

const failed = (ctx: AppContext) => (e: Error) => ctx.log.error(`Pencere yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`);

/** The layer a command means: the one the tree's menu named, else the active one; a group or a service layer is none. */
function layerAtHand(ctx: AppContext, args: unknown): LayerNode | undefined {
  const layers = ctx.doc.layers;
  const node = layers.get(typeof args === 'string' ? args : layers.active.value);
  return node?.type === 'layer' && !node.service ? node : undefined;
}

export function registerLabelCommands(ctx: AppContext): void {
  const { prefs } = ctx;
  ctx.commands.registerAll([
    {
      id: 'layer.labels',
      title: 'Etiketler…',
      short: 'Etiketler',
      category: 'Katman',
      icon: 'layerLabels',
      description:
        'Katmanın etiketlemesi: tek etiket, kurallı sınıflar (her biri kendi koşulu ve stiliyle) ya da etiketsiz; etiketin metni (şablon ya da ifade), yerleşimi (noktada çevresinde, çizgide kıvrık ya da eş yükselti, alanda parsel, sınır …), biçimi (hale, zemin, gölge, çağrı çizgisi), sığdırması (yığma, kısaltma, küçültme) ve önceliği; katmanın nesneleri öbür etiketlere engel olabilir.',
      aliases: ['ETIKETLER', 'ETIKETLEME', 'LABELING', 'LABELS'],
      run: (args) => {
        const layer = layerAtHand(ctx, args);
        if (!layer) return void ctx.log.warn('Etiketleme katmanın olur: etkin katman bir grup ya da servisten çizilen bir katman. Nesneleri olan bir katmanı etkin yapın.');
        void import('../ui/labels/LabelsDialog').then((m) => m.openLabels(ctx, layer.id), failed(ctx));
      },
    },
    {
      id: 'view.pinnedLabels',
      title: 'Sabit etiketleri vurgula',
      short: 'Sabit etiketler',
      category: 'Görünüm',
      icon: 'labelsPinned',
      description: 'Elle taşınmış, döndürülmüş ya da sabitlenmiş etiketlerin çevresine kesikli bir çerçeve çizer. Çizim değişmez.',
      aliases: ['SABITETIKETLER', 'PINNEDLABELS'],
      run: () => prefs.pinnedLabels.set(!prefs.pinnedLabels.value),
      isChecked: () => prefs.pinnedLabels.value,
      watch: [prefs.pinnedLabels],
    },
    {
      id: 'view.unplacedLabels',
      title: 'Yerleşmeyen etiketleri göster',
      short: 'Yerleşmeyenler',
      category: 'Görünüm',
      icon: 'labelsUnplaced',
      description: 'Boş yer bulamadığı için yazılmayan etiketleri en iyi yerlerinde kırmızı çizer: Etiketi taşı ile bir yere konabilirler. Çizim değişmez.',
      aliases: ['YERLESMEYENETIKETLER', 'UNPLACEDLABELS'],
      run: () => prefs.unplacedLabels.set(!prefs.unplacedLabels.value),
      isChecked: () => prefs.unplacedLabels.value,
      watch: [prefs.unplacedLabels],
    },
  ]);
}
