import type { Command } from '../core/commands';
import type { AppContext } from './context';

/**
 * Nokta bulutu (docs/adr/0207): the desktop's for now (the owner's decision, 8 October); the web's side is being made.
 * The web shows the same buttons where the desktop has them (CBS's Veri and CAD's Ekle, beside Raster; İşlemler's
 * point cloud tools under the panel's ▾), dimmed with the note, and running one says the work is the desktop's. The
 * desktop's catalog takes these commands from the inventory and runs them (apps/desktop/src/catalog.rs `PORTED`).
 */
const WAITING = 'Yalnız masaüstünde; web için hazırlanıyor';

/** The panel's buttons. */
export const POINT_CLOUDS = ['pointcloud.add', 'pointcloud.style', 'pointcloud.query', 'pointcloud.vpcSave'];

/** İşlemler's point cloud tools, under the panel's ▾ (the desktop's crates/native/processing builtin/pointcloud). */
export const POINT_CLOUD_TOOLS = [
  'processing.run.pointcloud.areaStats',
  'processing.run.pointcloud.thin',
  'processing.run.pointcloud.ground',
  'processing.run.pointcloud.classify',
  'processing.run.pointcloud.clip',
  'processing.run.pointcloud.merge',
  'processing.run.pointcloud.tile',
  'processing.run.pointcloud.rasterize',
  'processing.run.pointcloud.boundary',
];

type Spec = Omit<Command, 'run' | 'pending' | 'pendingNote' | 'category'>;

export function registerPointCloudCommands(ctx: AppContext): void {
  const desktopOnly = (c: Spec): Command => ({
    ...c,
    category: 'Nokta bulutu',
    pending: true,
    pendingNote: WAITING,
    run: () => ctx.log.warn(`“${c.short ?? c.title.replace(/…$/, '')}” şimdilik yalnız KentOS CAD masaüstünde çalışır; web sürümü hazırlanıyor.`),
  });
  const list: Spec[] = [
    {
      id: 'pointcloud.add',
      title: 'Nokta bulutu ekle…',
      short: 'Nokta bulutu',
      icon: 'pointCloudAdd',
      description:
        'LAS, LAZ, COPC ya da metin (XYZ, PTS, TXT, CSV) nokta bulutunu, birkaç dosyayı tek sanal bulut ya da ayrı nesneler olarak, sanal bulut dosyasını (.vpc) ya da HTTP adresindeki bulutu ekler: biçimi, nokta sayısı, kapsamı ve sistemi gösterilir; sistemi projeninkinden başkaysa eklenmez. COPC hemen, öbürleri dizini bu cihazda bir kez hazırlanınca kat kat çizilir. Dosyanın adıyla yeni katmana tek adımda yazılır.',
      aliases: ['NOKTABULUTU', 'BULUTEKLE', 'LIDAR', 'LAZ', 'COPC'],
    },
    {
      id: 'pointcloud.style',
      title: 'Nokta bulutu stili…',
      short: 'Bulut stili',
      icon: 'pointCloudStyle',
      description:
        'Seçili nokta bulutunun görünüşünü değiştirir: dosyanın renkleri, sınıflar, yükseklik ya da yoğunluk rampası (Otomatik aralıkla), dönüşler ya da tek renk; gösterilen sınıflar, noktanın boyu (piksel ya da metre) ve biçimi, saydamlık. Çizim değişiklikleri hemen gösterir; Uygula tek adımda yazar.',
      aliases: ['BULUTSTILI', 'NOKTABULUTUSTILI'],
    },
    {
      id: 'pointcloud.query',
      title: 'Nokta bulutu XYZ sor',
      short: 'XYZ sor',
      icon: 'pointCloudQuery',
      description:
        'Tıklanan yere ekranda 8 piksel içindeki en yakın noktayı tam çözünürlükte okur: Y, X, Z, sınıf, yoğunluk, dönüş, renk, GPS zamanı ve kaynak kimliği iletiye yazılır, nokta çizimde işaretlenir. Esc bitirir; çizime bir şey yazılmaz.',
      aliases: ['BULUTSOR', 'NOKTABULUTUSOR', 'BULUTXYZ'],
    },
    {
      id: 'pointcloud.vpcSave',
      title: 'Sanal bulut olarak kaydet…',
      short: 'Sanal bulut kaydet',
      icon: 'pointCloudVpc',
      description:
        "Seçili nokta bulutunun dosyalarını sanal bulut (.vpc, QGIS ve PDAL'ın biçimi) olarak yazar: her dosyanın yolu, nokta sayısı ve kapsamı, sistemi EPSG kodluysa WGS 84'teki çerçevesi.",
      aliases: ['VPC', 'SANALBULUT', 'VPCKAYDET'],
    },
    {
      id: 'processing.run.pointcloud.areaStats',
      title: 'Nokta bulutu alan sorgusu…',
      icon: 'pointCloudArea',
      description:
        'Seçili her alanın içindeki bulut noktalarını sayar: yoğunluk, kotların en küçüğü, en büyüğü, ortalaması ve standart sapması, sınıfların sayıları; tablo olarak verir.',
      aliases: ['BULUTALANSOR', 'ALANSORBULUT'],
    },
    {
      id: 'processing.run.pointcloud.thin',
      title: 'Seyrelt…',
      icon: 'pointCloudThin',
      description:
        "Nokta bulutunu seyreltir: her hücrede merkezine en yakın nokta, birbirine yarıçaptan yakın olmayan noktalar ya da her n'inci nokta kalır; sonuç LAS, LAZ ya da COPC olarak yazılır.",
      aliases: ['SEYRELT', 'BULUTSEYRELT'],
    },
    {
      id: 'processing.run.pointcloud.ground',
      title: 'Zemin süzgeci…',
      icon: 'pointCloudGround',
      description:
        "Nokta bulutunda zemini ayırır (SMRF): zemin noktaları 2 sınıfına alınır, zemin olmayan 2'ler 1 olur, öbür sınıflar kalır; sonuç LAS, LAZ ya da COPC olarak yazılır.",
      aliases: ['ZEMINSUZ', 'YUZEYFILTRE', 'ZEMIN'],
    },
    {
      id: 'processing.run.pointcloud.classify',
      title: 'Yüksekliğe göre sınıfla…',
      icon: 'pointCloudClassify',
      description:
        'Zemin noktalarından (2) yüzeyi kurar, noktaları zeminden yüksekliklerine göre düşük (3), orta (4) ve yüksek bitki (5) sınıflarına alır; sonuç LAS, LAZ ya da COPC olarak yazılır.',
      aliases: ['YUKSEKLIKSINIF', 'BITKISINIF'],
    },
    {
      id: 'processing.run.pointcloud.clip',
      title: 'Bulutu kırp…',
      icon: 'pointCloudClip',
      description:
        'Nokta bulutunun seçili alanların içinde (ya da dışında) kalan noktalarını LAS, LAZ ya da COPC olarak yazar; alanların delikleri, parçaları ve yayları kesindir.',
      aliases: ['BULUTKIRP'],
    },
    {
      id: 'processing.run.pointcloud.merge',
      title: 'Bulutları birleştir…',
      icon: 'pointCloudMerge',
      description:
        'Seçili nokta bulutlarının (sanal bulutun bütün dosyalarının) noktalarını tek LAS, LAZ ya da COPC dosyasında toplar.',
      aliases: ['BULUTBIRLESTIR'],
    },
    {
      id: 'processing.run.pointcloud.tile',
      title: 'Karola…',
      icon: 'pointCloudTile',
      description:
        'Nokta bulutunu verilen boyda karolara böler: her karo kendi dosyasıdır (<ad>_<x>_<y>), isterseniz karoların sanal bulutu (.vpc) da yazılır.',
      aliases: ['KAROLA', 'BULUTKAROLA'],
    },
    {
      id: 'processing.run.pointcloud.rasterize',
      title: 'Rasterleştir…',
      icon: 'pointCloudRaster',
      description:
        'Nokta bulutundan yükseklik rasteri (GeoTIFF) üretir: hücrenin en düşük, en yüksek ya da ortalama kotu, nokta sayısı ya da IDW; zemin noktalarından sayısal arazi modeli (DTM).',
      aliases: ['RASTERLESTIR', 'BULUTDEM', 'DTMURET'],
    },
    {
      id: 'processing.run.pointcloud.boundary',
      title: 'Sınır çıkar…',
      icon: 'pointCloudBoundary',
      description:
        'Nokta bulutunun kapladığı yeri alan olarak çizer: verilen boydaki hücrelerden en az şu kadar nokta düşenler birleşir, delikleriyle tek çok parçalı alan olur.',
      aliases: ['BULUTSINIR', 'SINIRCIKAR'],
    },
  ];
  ctx.commands.registerAll(list.map(desktopOnly));
}
