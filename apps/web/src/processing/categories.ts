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
  { id: 'conversion', label: 'Dönüştürme', icon: 'explode', description: 'Nesne türleri arasında dönüşüm' },
  { id: 'selection', label: 'Seçim', icon: 'select', description: 'Özniteliğe ve konuma göre seçim' },
];
