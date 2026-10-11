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
  // docs/adr/0217: layers split by values, merged with a field map, objects extracted, duplicates deleted, ordered copies.
  { id: 'layers', label: 'Katman', icon: 'layers', description: 'Özniteliğe göre bölme, katmanları birleştirme, seçilenleri çıkarma, yinelenenleri silme, sıralama' },
  { id: 'cadastre', label: 'Kadastro', icon: 'parcel', description: 'Parsel, ada ve tapu işlemleri' },
  { id: 'geometry', label: 'Geometri', icon: 'polygon', description: 'Sadeleştirme, tampon, onarım ve dönüşümler' },
  { id: 'analysis', label: 'Analiz', icon: 'measure', description: 'Ölçüm, istatistik ve raporlar' },
  // docs/adr/0215: how near objects are to one another.
  { id: 'proximity', label: 'Yakınlık', icon: 'nearestFeature', description: 'En yakın nesne, uzaklık matrisi, en yakın merkez, komşu alanlar, en kısa çizgi' },
  // docs/adr/0238: where objects lie and how their values sit among their neighbours.
  { id: 'spatialStats', label: 'Mekânsal istatistik', icon: 'statsHotSpot', description: 'Merkezler ve yayılım, en yakın komşu, Moran I, sıcak noktalar, kümeleme' },
  { id: 'network', label: 'Ağ analizi', icon: 'networks', description: 'En yakın tesis, maliyet matrisi ve hizmet alanları' },
  // docs/adr/0218: points from objects, lines from points and tables, a path's piece, rings and areas, densified edges.
  { id: 'conversion', label: 'Dönüştürme', icon: 'conversion', description: 'Nesneden nokta, noktalardan ve tablolardan çizgi, çizginin parçası, alan ve çizgi çevirme, köşe ekleme' },
  // docs/adr/0219: cells and triangles of points, bounding geometries, concave hulls, poles of inaccessibility.
  { id: 'generation', label: 'Geometri üretimi', icon: 'generation', description: 'Voronoi alanları, Delaunay üçgenleri, sınırlayıcı geometri, içbükey zarf, ulaşılmazlık kutbu' },
  // docs/adr/0220: grids, regular and random points, points along lines, subdivided objects.
  { id: 'sampling', label: 'Örnekleme', icon: 'sampling', description: 'Ağ oluştur, düzenli ve rastgele noktalar, hat boyunca nokta, alt bölümlere ayırma' },
  // docs/adr/0221: smoothing, squaring, buildings, aggregation, parts, elimination, coverage simplification.
  {
    id: 'generalization',
    label: 'Genelleştirme',
    icon: 'generalization',
    description: 'Yumuşat, dik açılandır, bina sadeleştir, alanları kümele, küçük parçaları ayıkla, küçükleri komşuya kat, ortak kenarları koruyarak sadeleştir',
  },
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
  // docs/adr/0237: criteria brought to one scale, weighed together, checked against known events.
  { id: 'suitability', label: 'Uygunluk analizi', icon: 'weightedOverlay', description: 'Bulanık üyelik ve çakıştırma, ağırlıklı toplam ve çakıştırma, AHP, ROC' },
  // docs/adr/0242: multi-band images: bands, indices, classes, their accuracy, change, fusion.
  {
    id: 'remoteSensing',
    label: 'Uzaktan algılama',
    icon: 'spectralIndex',
    description: 'Bant birleştirme, spektral indisler, sınıflandırma, doğruluk, değişim ve görüntü birleştirme',
  },
  // docs/adr/0243: NetCDF slices and meshes read along lines, at points over time, and worked into new datasets.
  { id: 'multidim', label: 'Çok boyutlu veri', icon: 'multidimData', description: 'Kesit, zaman serisi ve mesh hesaplayıcı' },
];
