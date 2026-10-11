// Bilgi kartı ve eylemler and Konumlu fotoğraflar (docs/adr/0225 §3–§5; the desktop's `layer_info.rs` and
// `exchange/photo_import.rs`): their windows load on first use (CLAUDE.md §20).

import type { AppContext } from './context';

const failed = (ctx: AppContext) => (e: Error) => ctx.log.error(`Pencere yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`);

export function registerObjectInfoCommands(ctx: AppContext): void {
  ctx.commands.registerAll([
    {
      id: 'layer.info',
      title: 'Bilgi kartı ve eylemler…',
      short: 'Bilgi kartı',
      category: 'Katman',
      icon: 'layerInfo',
      description:
        'Katmanın nesnelerinin bilgi kartını (Bilgi aracının gösterdiği: başlık, alanlar, metin, ekler) ve eylemlerini (nesneden açılan adresler ve dosyalar) düzenler. Başlık, metin ve hedef şablondur: {Ada}, {yuvarla($alan, 2)}, {$enlem}.',
      aliases: ['BILGIKARTI', 'BALONICERIGI', 'EYLEMLER', 'POPUP'],
      run: (args) => {
        void import('../ui/layers/LayerInfoDialog').then((m) => m.openLayerInfo(ctx, typeof args === 'string' ? args : undefined), failed(ctx));
      },
    },
    {
      id: 'file.import.photos',
      title: 'Konumlu fotoğraflar…',
      short: 'Konumlu fotoğraflar',
      category: 'Dosya',
      icon: 'photoImport',
      description:
        'JPEG fotoğrafların EXIF’indeki konumdan nokta katmanı yapar: her fotoğraf bir nokta, adı, tarihi, yönü, yüksekliği ve kamerası öznitelik, fotoğrafın kendisi ek. Konum WGS 84’ten projenin sistemine Koordinat dönüştür’ün yoluyla çevrilir.',
      aliases: ['KONUMLUFOTOGRAF', 'FOTOGRAFAL', 'GEOTAG'],
      run: () => void import('../ui/io/PhotoImportDialog').then((m) => m.openPhotoImport(ctx), failed(ctx)),
    },
  ]);
}
