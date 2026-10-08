import type { Command } from '../core/commands';
import type { AppContext } from './context';

/**
 * Hesap menüsü: surveying computations (poligon, kutupsal alım, aplikasyon,
 * önden ve geriden kestirme). Their windows load on first use (CLAUDE.md
 * §20); the computations are the geometry core's (crates/shared/geometry-core
 * /src/survey), checked against an independent reference (§23.4).
 */
export function registerCalcCommands(ctx: AppContext): void {
  const failed = (e: unknown) => ctx.log.error(`Hesap penceresi yüklenemedi: ${e instanceof Error ? e.message : String(e)}. Bağlantınızı denetleyip komutu yeniden çalıştırın.`);
  const list: Command[] = [
    {
      // Karne editörü (docs/adr/0169 §6): an instrument's file or a text book, its stations reduced.
      id: 'calc.fieldbook',
      title: 'Karne editörü…',
      short: 'Karne',
      category: 'Hesap',
      icon: 'fieldBook',
      description:
        'Total station karnesini açar: Leica GSI, Sokkia SDR, Topcon GTS-7, Nikon RAW ve Trimble JobXML içeriğinden tanınır, CSV/TXT karnenin sütunları eşlenir. İstasyonlar ve gözlemleri görülür, gözlem kullanılmaz ya da nokta adı düzeltilir; iki durum eşlenip ortalanır, indeks hatası, yatay uzunluk ve kot farkı (yer eğriliği ve refraksiyonla) hesaplanır, toleransı aşan fark uyarı rengindedir.',
      aliases: ['KARNE', 'KARNEEDITORU', 'KARNEEDİTÖRÜ', 'GSI', 'FIELDBOOK'],
      run: () => void import('../ui/calc/FieldBookDialog').then((m) => m.openFieldBook(ctx)).catch(failed),
    },
    {
      // Cihaza gönder (docs/adr/0169 §4; the owner's name, 4 October): the selected points as an instrument's coordinate file.
      id: 'field.send',
      title: 'Cihaza gönder…',
      short: 'Cihaza gönder',
      category: 'Hesap',
      icon: 'fieldSend',
      description:
        'Seçili noktaları ölçüm cihazına koordinat dosyası olarak yazar: Leica GSI-16 ve GSI-8, Topcon GTS-7, Trimble JobXML, Nikon RAW ya da CSV. Ad, kod ve kot taşınır; biçimin taşıyamadığı nokta nedeniyle söylenir, kırpılmaz. Aplikasyon penceresinden ve Nokta editöründen de açılır.',
      aliases: ['CIHAZAGONDER', 'CİHAZAGÖNDER', 'ALETEGONDER', 'GONDER', 'GÖNDER'],
      run: () => {
        const points = [...ctx.selection.ids.value].map((id) => ctx.doc.get(id)).filter((e) => e?.kind === 'point');
        if (!points.length) {
          ctx.log.warn('Seçili nokta yok: cihaza gönderilecek noktaları seçin ya da Nokta editöründen veya Aplikasyon penceresinden gönderin.');
          return;
        }
        void import('../ui/io/FieldSendDialog')
          .then((m) =>
            m.openFieldSend(
              ctx,
              points.flatMap((e) => {
                const p = e ? m.fieldPoint(e) : null;
                return p ? [p] : [];
              }),
              `Çizimde seçili ${points.length} nokta`,
            ),
          )
          .catch(failed);
      },
    },
    {
      id: 'calc.traverse',
      title: 'Poligon hesabı…',
      short: 'Poligon',
      category: 'Hesap',
      icon: 'surveyTraverse',
      description:
        'Bilinen bir noktadan kırılma açıları ve kenarlarla yeni poligon noktalarını hesaplar: bağlı, kapalı ya da açık. Açı kapanma hatası açılara eşit, koordinat kapanma hatası kenarlara uzunluklarıyla orantılı dağıtılır; noktalar çizime eklenir.',
      aliases: ['POLIGON', 'POLİGON', 'TRAVERSE'],
      run: () => void import('../ui/calc/TraverseDialog').then((m) => m.openTraverse(ctx)).catch(failed),
    },
    {
      id: 'calc.polar',
      title: 'Kutupsal alım…',
      short: 'Kutupsal alım',
      category: 'Hesap',
      icon: 'surveyPolar',
      description:
        'Bilinen bir istasyondan yatay açı okumaları ve uzunluklarla alınan noktaları hesaplar (takeometri). Eğik uzunluk başucu açısıyla yataya indirilir, istasyon kotu verilirse nokta kotları da hesaplanır.',
      aliases: ['KUTUPSAL', 'TAKEOMETRI', 'ALIM'],
      run: () => void import('../ui/calc/PolarDialog').then((m) => m.openPolar(ctx)).catch(failed),
    },
    {
      id: 'calc.stakeout',
      title: 'Aplikasyon…',
      short: 'Aplikasyon',
      category: 'Hesap',
      icon: 'surveyStakeout',
      description: 'Bir istasyondan bilinen noktaların aplikasyon değerlerini verir: semt, yatay uzunluk ve bakılan noktadan saat yönünde dönülecek açı. Rapor panoya kopyalanır.',
      aliases: ['APLIKASYON', 'APLİKASYON', 'SEMTMESAFE'],
      run: () => void import('../ui/calc/StakeoutDialog').then((m) => m.openStakeout(ctx)).catch(failed),
    },
    {
      id: 'calc.forward',
      title: 'Önden kestirme…',
      short: 'Önden kestirme',
      category: 'Hesap',
      icon: 'surveyForward',
      description: 'İki bilinen noktada yeni noktaya ölçülen açılardan yeni noktanın koordinatını hesaplar.',
      aliases: ['ONDEN', 'ÖNDEN', 'ONDENKESTIRME'],
      run: () => void import('../ui/calc/IntersectionDialog').then((m) => m.openIntersection(ctx, 'forward')).catch(failed),
    },
    {
      id: 'calc.resection',
      title: 'Geriden kestirme…',
      short: 'Geriden kestirme',
      category: 'Hesap',
      icon: 'surveyResection',
      description: 'Yeni noktada üç bilinen noktaya ölçülen iki açıdan durulan noktanın koordinatını hesaplar; tehlike dairesine yakınsa uyarır.',
      aliases: ['GERIDEN', 'GERİDEN', 'GERIDENKESTIRME'],
      run: () => void import('../ui/calc/IntersectionDialog').then((m) => m.openIntersection(ctx, 'resection')).catch(failed),
    },
    {
      // Yatay ağ dengelemesi (docs/adr/0203): directions and distances adjusted by least squares, with their statistics.
      id: 'calc.network',
      title: 'Yatay ağ dengelemesi…',
      short: 'Yatay ağ',
      category: 'Hesap',
      icon: 'surveyNetwork',
      description:
        'Doğrultu ve kenar ölçülerinden oluşan yatay ağı en küçük kareler yöntemiyle dengeler: sabit ya da ağırlıklı bilinen noktalar, istasyon başına yöneltme, ağırlıklar Proje ayarları › Ölçme’nin önsel doğruluklarından. Noktaların koordinatları, standart sapmaları ve hata elipsleri; gözlemlerin düzeltmeleri, denetlenebilirliği ve uyuşumsuz ölçü testi; m₀ ve model testi. Yeni noktalar çizime yazılır.',
      aliases: ['AGDENGELE', 'AĞDENGELE', 'YATAYAG', 'DENGELEME', 'NETWORK'],
      run: () => void import('../ui/calc/NetworkDialog').then((m) => m.openNetwork(ctx)).catch(failed),
    },
    {
      // Kot ağı dengelemesi (docs/adr/0203 §5): height differences adjusted by least squares.
      id: 'calc.levelNetwork',
      title: 'Kot ağı dengelemesi…',
      short: 'Kot ağı',
      category: 'Hesap',
      icon: 'surveyLevel',
      description:
        'Geometrik nivelman ya da trigonometrik kot farklarından oluşan kot ağını en küçük kareler yöntemiyle dengeler: sabit ya da ağırlıklı reperler, ağırlıklar hat uzunluğundan. Kotlar ve standart sapmaları, gözlemlerin düzeltmeleri, uyuşumsuz ölçü testi ve model testi; çizimdeki aynı adlı noktaların kotları yazılır.',
      aliases: ['KOTAGI', 'KOTAĞI', 'NIVELMAN', 'NİVELMAN', 'LEVELNETWORK'],
      run: () => void import('../ui/calc/NetworkDialog').then((m) => m.openLevel(ctx)).catch(failed),
    },
    {
      // Vektör oturtma (docs/adr/0156): a drawing or a layer fitted to another system by control points.
      id: 'transform.fit',
      title: 'Vektör oturtma…',
      short: 'Oturt',
      category: 'Koordinat',
      icon: 'vectorFit',
      description:
        'Çizimi ya da bir katmanı ortak noktalarla başka bir sisteme oturtur: Helmert, afin ya da projektif dönüşüm en küçük kareler ile; her çiftin artığı ve m0 görünür, kötü çift çıkarılınca çözüm yenilenir. Çiftler yazılır, yapıştırılır, çizimden seçilir ya da iki katmandaki aynı adlı noktalardan eşlenir; Uygula tek adımda yazar.',
      aliases: ['OTURT', 'OTURTMA', 'DONUSUM', 'DÖNÜŞÜM', 'HELMERT', 'AFIN', 'AFİN'],
      run: () => void import('../ui/calc/FitDialog').then((m) => m.openFit(ctx)).catch(failed),
    },
    {
      // Koordinat dönüştür (docs/adr/0167 §4): points between the coordinate systems, one or a list.
      id: 'crs.transform',
      title: 'Koordinat dönüştür…',
      short: 'Dönüştür',
      category: 'Koordinat',
      icon: 'crsTransform',
      description:
        'Bir noktayı ya da listeyi bir koordinat sisteminden ötekine dönüştürür (TM, UTM, coğrafi DMS ve ondalık derece; TUREF, ED50, WGS 84): kaynak projenin, hedef ikinci sistemindir; her sonuç doğruluğunu ve dayandığı EPSG dönüşümünü söyler. Nokta yazılır ya da çizimden seçilir, liste yapıştırılır; değerler panoya kopyalanır ya da CSV olarak kaydedilir. Çizime bir şey yazılmaz.',
      aliases: ['DONUSTUR', 'DÖNÜŞTÜR', 'KOORDINATDONUSTUR', 'DATUM', 'CONVERT'],
      run: () => void import('../ui/calc/ConvertDialog').then((m) => m.openConvert(ctx)).catch(failed),
    },
    {
      // Kenar eşleme (docs/adr/0159): the line ends of two sheets put together across their common edge.
      id: 'transform.edgematch',
      title: 'Kenar eşleme…',
      short: 'Kenar eşle',
      category: 'Koordinat',
      icon: 'edgematch',
      description:
        'Komşu paftaların kenarında buluşmayan çizgileri birleştirir: arama uzaklığı ve açı toleransı içinde çizginin devamı olan uçlar eşlenir, her bağ tabloda görülür ve çıkarılabilir. Uçlar komşunun ucunda, ortada ya da pafta sınırında buluşur; uç taşınır, parça eklenir ya da kayma çizgi boyunca dağıtılır. Uygula tek adımda yazar.',
      aliases: ['KENARESLE', 'KENAREŞLE', 'KENARESLEME', 'EDGEMATCH'],
      run: () => void import('../ui/calc/EdgematchDialog').then((m) => m.openEdgematch(ctx)).catch(failed),
    },
  ];
  ctx.commands.registerAll(list);
}
