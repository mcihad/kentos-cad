import type { Command } from '../core/commands';
import type { AppContext } from './context';

/**
 * Raster katmanları (docs/adr/0204): Raster ekle, Raster stili and Raster
 * oturt. Their windows load on first use (CLAUDE.md §20); reading, colouring
 * and resampling are the formats core's (crates/shared/formats/src/raster),
 * in the raster workers (io/rasterWorker.ts).
 */
export function registerRasterCommands(ctx: AppContext): void {
  const failed = (e: unknown) => ctx.log.error(`Raster penceresi yüklenemedi: ${e instanceof Error ? e.message : String(e)}. Bağlantınızı denetleyip komutu yeniden çalıştırın.`);
  const list: Command[] = [
    {
      id: 'raster.add',
      title: 'Raster ekle…',
      short: 'Raster ekle',
      category: 'Raster',
      icon: 'rasterAdd',
      description:
        'GeoTIFF, TIFF, dünya dosyalı PNG ve JPEG ya da NetCDF (değişkeni ve dilimiyle) ekler: boyu, bantları, türü, konumu, sistemi ve nodata değeri gösterilir; sistemi projeninkinden başkaysa eklenmez, söylemiyorsa projenin sisteminde olduğu onaylanır, konumu yoksa görünümün ortasına oturtulmamış eklenir. Dosyanın adıyla yeni katmana tek adımda yazılır; büyük raster karolarla ve önizleme piramidiyle çizilir.',
      aliases: ['RASTER', 'RASTEREKLE', 'GEOTIFF', 'ORTOFOTO', 'RASTERYUKLE', 'RASTERYÜKLE'],
      run: () => void import('../ui/raster/RasterAddDialog').then((m) => m.openRasterAdd(ctx)).catch(failed),
    },
    {
      id: 'mesh.add',
      title: 'Mesh ekle…',
      short: 'Mesh ekle',
      category: 'Raster',
      icon: 'meshAdd',
      description:
        "Hidrodinamik modellerin üçgen ve dörtgen ağlarını ekler: UGRID ağlı NetCDF ya da 2DM ve ASCII DAT dosyaları (tek bir UGRID NetCDF'e yazılır). Veri seti, zaman ya da öbür boyutların değeri, hücre boyu ve ağ çizgileri seçilir; mesh her katta ağdan örneklenen sanal raster olarak çizilir, zaman sürgüsünü izleyebilir.",
      aliases: ['MESH', 'MESHEKLE', 'UGRID', '2DM', 'NETCDF'],
      run: () => void import('../ui/raster/MultidimDialog').then((m) => m.openMeshAdd(ctx)).catch(failed),
    },
    {
      id: 'raster.style',
      title: 'Raster stili…',
      short: 'Raster stili',
      category: 'Raster',
      icon: 'rasterStyle',
      description:
        'Seçili rasterin görünüşünü değiştirir: renkli, gri, paletli, renk rampası, gölgeli kabartma ya da rampa ve gölge; bantlar, gerdirme (bantların istatistikleriyle), rampa, ışığın doğrultusu ve yüksekliği, nodata, saydamlık ve örnekleme. Çizim değişiklikleri hemen gösterir; Uygula tek adımda yazar.',
      aliases: ['RASTERSTILI', 'RASTERSTİLİ', 'HILLSHADE', 'GOLGELIKABARTMA', 'GÖLGELİKABARTMA', 'RAMPA'],
      run: () => void import('../ui/raster/RasterStyleDialog').then((m) => m.openRasterStyle(ctx)).catch(failed),
    },
    {
      id: 'raster.georef',
      title: 'Raster oturt…',
      short: 'Raster oturt',
      category: 'Raster',
      icon: 'rasterGeoref',
      description:
        'Rasteri kontrol noktalarıyla yerine oturtur: noktanın pikseli rasterin üstünde, hedefi çizimde gösterilir ya da yazılır. Helmert, afin, projektif, polinom 2 ve 3, ince plaka; artıklar ve m0 her değişiklikte. Helmert ve afin rasterin dönüşümünü değiştirir; ötekiler rasteri yeniden örnekleyip GeoTIFF olarak indirir ve gösterir.',
      aliases: ['RASTEROTURT', 'OTOREG', 'GEOREF', 'GEOREFERANS', 'GEOREFERENCE'],
      run: () => void import('../ui/raster/RasterGeorefDialog').then((m) => m.openRasterGeoref(ctx)).catch(failed),
    },
  ];
  ctx.commands.registerAll(list);
}
