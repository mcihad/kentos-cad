/**
 * The toolbox tree. Tools name a category id; a category may sit under a
 * parent ("cadastre.numbering" under "cadastre"). Empty categories are not
 * shown, so the list can name what is coming before the tools exist.
 */
export interface ProcessingCategory {
  readonly id: string;
  readonly label: string;
  readonly parent?: string;
  readonly icon?: string;
  /** One line under the name in the toolbox. */
  readonly description?: string;
}

export const PROCESSING_CATEGORIES: readonly ProcessingCategory[] = [
  { id: 'points', label: 'Nokta işlemleri', icon: 'point', description: 'Nokta üretme, numaralandırma ve nokta listeleri' },
  { id: 'annotation', label: 'Yazı ve etiket', icon: 'text', description: 'Ölçü, uzunluk ve öznitelik yazıları' },
  { id: 'attributes', label: 'Öznitelik', icon: 'table', description: 'Öznitelik hesaplama ve düzenleme' },
  { id: 'cadastre', label: 'Kadastro', icon: 'parcel', description: 'Parsel, ada ve tapu işlemleri' },
  { id: 'geometry', label: 'Geometri', icon: 'polygon', description: 'Sadeleştirme, tampon, onarım ve dönüşümler' },
  { id: 'analysis', label: 'Analiz', icon: 'measure', description: 'Ölçüm, istatistik ve raporlar' },
  // docs/adr/0215: how near objects are to one another.
  { id: 'proximity', label: 'Yakınlık', icon: 'nearestFeature', description: 'En yakın nesne, uzaklık matrisi, en yakın merkez, komşu alanlar, en kısa çizgi' },
  { id: 'network', label: 'Ağ analizi', icon: 'networks', description: 'En yakın tesis, maliyet matrisi ve hizmet alanları' },
  { id: 'conversion', label: 'Dönüştürme', icon: 'explode', description: 'Nesne türleri arasında dönüşüm' },
  { id: 'selection', label: 'Seçim', icon: 'select', description: 'Özniteliğe ve konuma göre seçim' },
  // docs/adr/0231: the DEM's surface (the desktop's `pointcloud` category is its own, docs/adr/0207 §7).
  { id: 'surface', label: 'Yüzey analizi', icon: 'hillshade', description: 'Eğim, bakı, kabartma, eğrilik, güneşlenme ve eş yükselti eğrileri' },
  // docs/adr/0232: surfaces from points, densities.
  { id: 'interpolation', label: 'İnterpolasyon', icon: 'idw', description: 'Noktalardan yüzey: IDW, doğal komşu, spline, kriging, TIN' },
  { id: 'density', label: 'Yoğunluk', icon: 'kernelDensity', description: 'Noktaların ve çizgilerin yoğunluğu' },
  // docs/adr/0233: map algebra, masks, mosaics, statistics.
  { id: 'rasterOps', label: 'Raster işlemleri', icon: 'rasterCalculator', description: 'Hesaplayıcı, sınıflandırma, maskeyle kırpma, mozaik, yeniden örnekleme' },
  { id: 'rasterStats', label: 'Raster istatistiği', icon: 'zonalStats', description: 'Bölgesel, komşuluk ve hücre istatistikleri, histogram' },
  // docs/adr/0234: vectors burnt into cells, regions, lines and points out of them; scanned sheets digitized.
  { id: 'rasterVector', label: 'Raster ve vektör', icon: 'rasterize', description: 'Rasterleştirme; rasterden alan, çizgi ve nokta' },
  { id: 'scannedMap', label: 'Taranmış harita', icon: 'captureLine', description: 'Çizgi yakalama, alan kapatma, eğrilere kot verme' },
  // docs/adr/0235: the water's way over a DEM.
  { id: 'hydrology', label: 'Hidroloji', icon: 'streams', description: 'Çukur doldurma, akış yönü ve birikimi, havzalar, dere ağı, nemlilik indisi' },
  // docs/adr/0236: how far, and how dear, every cell is from the sources.
  { id: 'distance', label: 'Uzaklık ve maliyet', icon: 'costPath', description: 'Uzaklık yüzeyi, birikimli maliyet, en düşük maliyetli yol, maliyet koridoru' },
];
