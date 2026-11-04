# 0914 — CAD yazısının yerleşimi ve canlı alanların teslim doğruluğu

Tarih: 4 Kasım 2026. Kapsam: CAD-ADV-10. Durum: uygulama ve kabul çalışması sürüyor; plan maddesi henüz tamamlanmadı.

## Araştırma ve mevcut yolların karşılaştırılması

- [Unicode UAX #29, revizyon 49](https://unicode.org/reports/tr29/tr29-49.html) kullanıcıya görünen karakter sınırlarını tanımlar. Kalıcı biçim/field aralıklarının scalar indeksleriyle düzenleme imlecinin görünür karakter birimi ayrı sözleşmelerdir. Standart Unicode 18 sürümündedir; mevcut Iced bağımlılığının `unicode-segmentation 1.13.3` tabloları Unicode 17'dir. Unicode 18 uyumluluğu iddia edilmez.
- [unicode-segmentation 1.13.3](https://docs.rs/unicode-segmentation/1.13.3/unicode_segmentation/) genişletilmiş grapheme sınırlarını verir. Iced/Cosmic Text'in zaten kullandığı kilitli sürüm doğrudan kullanılır; elle aksan/emoji aralığı listesi oluşturulmaz.
- [HarfBuzz: shaping concepts](https://harfbuzz.github.io/shaping-concepts.html) kod noktalarından fontun glyph yerleşiminin üretilmesini açıklar. Bu, paragraf kırma ve sütun yerleşiminin tamamı değildir.
- [HarfBuzz: clusters](https://harfbuzz.github.io/working-with-harfbuzz-clusters.html) şekillendirme sonucunda kaynak aralıklarının birleşebildiğini ve yönle yeniden sıralanabildiğini belirtir. Bir glyph'i bir scalar saymak doğru değildir.
- [Unicode UAX #9, revizyon 52](https://unicode.org/reports/tr9/tr9-52.html) karışık yönlerde mantıksal saklama sırasıyla görsel çizim sırasını ayırır. Dosya metni görsel sıraya çevrilerek saklanmaz.

Kodun mevcut durumu:

| Yol | Ölçüm / yerleşim | Çizim |
| --- | --- | --- |
| Model geometri, hit-test, MTEXT satır/sütun ve biçim parçaları | `geometry-core/text`, 352 karakterlik legacy advance tabloları; scalar aralıklar | Ortak `TextPlace` ve paragraph kayıtları |
| Masaüstü model/viewport | Aynı kayıtlar; bazı anchor/maskelerde ayrıca gerçek Iced ölçümü | `labels.rs`, Iced Advanced/Cosmic Text; paketli font yoksa işletim sistemi fallback'i devreye girebilir |
| Yerel PDF model yazısı | Ortak geometri kayıtları, ayrıca sheet font metrikleri | `sheet/pdf/fonts.rs` scalar → CID; legacy advance'lar; OpenType shaping yapmıyor |
| CPU harita/SVG yazısı | Ayrı label/shape girdileri | `plot/images.rs::outline`, Rustybuzz 0.20.1 ve aynı paketli TTF'ler |
| Düzenleme kutusu | Iced Content; byte imleci ↔ scalar aralık dönüşümü | Standart KentOS arayüzünün mevcut paragraf düzenleyicisi |

Bu yollar eşdeğer kabul edilmez. Özellikle birleşik aksan genişliği, teknik sembol kapsamı, kerning/ligatür ve çift yönlü yazı için ekranın düzgün görünmesi PDF'nin de doğru olduğunu kanıtlamaz.

## Uygulanan ilk düzeltmeler

1. Paragraf editöründe Backspace/Delete, seçilmiş aralık veya imlecin dokunduğu genişletilmiş grapheme kümesini bütün olarak siler. Cosmic Text 0.15'in Backspace'i scalar sildiği için denetim KentOS'un editör adaptöründedir; dış bağımlılık kaynakları değiştirilmez. Satır birleştirme ve boş sınır işlemleri korunur. Dosyanın `runs`/`fields` aralıkları scalar indeksli kalır; metin normalizasyonuyla dosya içeriği dönüştürülmez.
2. Font çözümleyicisi aile önekini metrik kanıtı saymaz. Arial Narrow/Black, Helvetica Neue, Barlow Condensed, Overpass Mono ve Courtney gibi adlar artık başka ailenin `own/equivalent` sonucu olmaz. Bilinen aile ve stil etiketleri, Windows/Unix dosya yolunun son bileşeni üzerinden eşlenir. Stilde saklanan özgün dosya adı ve mevcut çizim fontu değiştirilmez. Yeni DXF okuması da ortak çözümleyiciyi kullanır.
3. `cad.drawing.inspect` içindeki açıklama denetimi yazının özgün fontunu, canlı field sonucu ile saklanan önbelleğinin farkını ve kaynak hatalarını okur. `text_field_stale`, `text_field_broken`, `text_field_unverified`, `text_font_substituted` bulguları masaüstü, Python ve MCP'nin aynı komut sonucudur. Dosyayı açmak veya denetlemek güncelleme işlemi değildir.
4. Bağlı yazı kaynakları da denetlenir: bir yazının kendi önbelleği doğrudan kaynağıyla eşitken, o kaynağın alanı eskimiş olabilir. Görünür yazının gizli kaynakları kapsam dışında bırakılmaz. Bir kaynak zinciri en fazla 512 nesneyle sınırlıdır; döngü tespiti yığıt tüketmeyen, iki geçişli yönlü grafik taramasıdır. Sınır aşılınca güncel sonucu varmış gibi davranılmaz.
5. CAD denetimindeki alan onarımı mevcut `text.fieldsUpdate` / `cad.fields.edit` işlemini kullanır. İlgili yaşayan yazı kaynaklarıyla birlikte açıkça yenilenir; işlem mevcut geri alma sözleşmesini kullanır. Kilitli kaynağın yazılması sessizce atlanmaz.
6. Yerel pafta ön denetimi aynı açıklama okuyucusunu kullanır. Güncel olmayan veya doğrulanamayan field bulunan teslim PDF'si `text_field_stale`/`text_field_unverified` ile reddedilir. Önce açık yenileme ve kontrol gerekir. Gerçekten bozuk alan `####` olarak yenilendiyse durum uyarı olarak görünür. Paketli/eşdeğer font dosyasını adlandırmak tek başına “eksik font” uyarısı üretmez.
7. Model/viewport yazılarının gerçek parça listesi de PDF karakter kapsamı için denetlenir; yalnız pafta üzerindeki metin öğelerini denetlemek yeterli değildir. `map_text_unsupported`, PDF hattının desteklemediği kod noktalarını font/yüz ve viewport ile gösterir ve soru işaretine sessiz dönüşümü engeller. Bu koruma, karakterlerin doğru üretiminin yerine geçmez; ortak yerleşim tamamlanana kadar yanlış teslimi önler.

## Ortak Unicode çekirdeği — entegrasyon öncesi doğrulama

`geometry-core/text/shaping.rs` aynı paketli yüzlerden gerçek glyph kimliği,
byte/scalar kaynak aralığı, görsel sıra, advance/offset, sentetik eğiklik ve
ink kutusu üretir. Rustybuzz 0.20.1, unicode-bidi 0.3.18 ve
unicode-segmentation 1.13.3 sürümleri sabittir; saf Rust çekirdek dosya,
ağ veya işletim sistemi font listesine erişmez. Bu modül henüz model
geometrisinin, ekranın veya PDF'nin legacy yerleşimi yerine kullanılmaz.

- `Paragraph` çift yönlü çözümü paragrafın bütünü için bir kez yapar;
  gerçek satır aralıklarında UAX #9 L1/L2 uygulanır. Bir satırın sayı veya
  parantezle başlaması paragraf yönünü yeniden belirlemez. Byte/scalar
  aralıkları satırın kesilmiş metnine değil özgün paragrafa bağlıdır.
- Biçim aralıkları sıralı ve çakışmasız olmalı, grapheme kümesini bölmemelidir.
  Font/kalınlık değişiminde aynı satırın önceki/sonraki metni shaping
  bağlamı olarak verilir; gerçek satır sınırının dışına bağlanılmaz.
  Gereksiz aynı biçim sınırları kerning/ligatürü parçalamaz.
- En fazla 65.536 UTF-8 byte, 4.096 biçim aralığı ve byte sınırının dört katı
  glyph kabul edilir. Font seçimi her grapheme için yeniden yapılmaz;
  biçim başına çözülür, aralık araması sıralı listede ikili aramadır.
  Desteklenmeyen küme hata döndürür; `?` üretmez.
- Var olan 22 TTF yüzünün byte'ları değiştirilmeden ortak katalogda kullanılır.
  Bunlara Noto Sans, Noto Sans Arabic, Noto Sans Hebrew ve Noto Sans Math
  ailelerinden yedi sabit yüz eklenir. Kullanıcının yedi çizim ailesi veya
  eski stil eşlemesi bu nedenle değişmez. Kaynak Google Fonts commit'i
  `2fe1e8c6b137fb13041a7cfca6fdcd8a910ea4dc`, indirilen dosya hash'leri,
  FontTools 4.67.0 üretim parametreleri ve OFL lisansları
  `geometry-core/data/text-fonts/manifest.json` ile saklanır.
  `scripts/fonts/text_fallbacks.py --check` çevrimdışı paket doğrulamasıdır;
  `--build --check --source-dir ...` hash'i denetlenmiş kaynaktan tekrar
  üretimi karşılaştırır. Uygulama çalışırken font indirilmez.
- Bağımsız referans `scripts/fixtures/text_shaping_cases.py`, KentOS çağırmadan
  C HarfBuzz 15.0.0 ve FriBidi 1.0.17 çalıştırır: 29 yüzün glyph/offset/advance
  sonucu, 18 karışık yön, 12 paragraf bağlamından kesilmiş satır ve iki
  biçimli metin örneği. Biçim sınırındaki Arapça birleşmesi de harici
  HarfBuzz pre/post-context sonucuyla karşılaştırılır. Yedi Rust testi geçti.
  Unicode tablo sürümleri farklı olduğundan tüm Unicode 18 uyumluluğu iddia edilmez.

### Fiziksel akış ve kontur doğrulaması — ikinci hazırlık bölümü

`shaping/paragraph.rs`, aynı gerçek glyph satırlarını ölçerek girinti,
sekme, kutuya sözcük bazlı sığdırma ve statik sütun yerleşimini üretir.
Girdi yüksekliği CAD model yüksekliğidir; glyph koordinatları em, satır
baseline'ları model birimidir. Width factor yalnız baseline doğrultusuna
uygulanır. Ölçülen advance kutusuyla gerçek ink kutusu ayrı kalır.
Satır kırıldıktan sonra yönü yeniden tahmin edilmez. CR/LF/CRLF kaynak
indeksleri ve sondaki boş paragraf korunur. Boşluğa bağlanmış aksan da
bir kümedir; sarma veya satır sonu temizliği kümeyi bölemez.

Ölçümde artan aday aralığı ve ikili arama kullanılır; her kısa satır için
kalan bütün paragrafın yarısını tekrar tekrar shape etmekten kaçınılır.
Yedi aile ve beş kutu genişliği, basit doğrusal sözcük taramasıyla
karşılaştırılır. 6.000 satırlık birleşik aksan örneği de sınanır. Bu bir
GPU/sahne performansı kabulü değildir; sahne önbelleği/gerçek çizim
performansı entegrasyon sırasında ölçülecek.

- Harici C HarfBuzz + FriBidi ile dört fiziksel akış örneğinde satır
  aralıkları, global byte/scalar aralıkları, glyph'ler, ofsetler, advance,
  girinti/tab/sütun baseline'ları karşılaştırıldı. Referans üretimi KentOS
  çağırmaz; hızlandırılmış aramadan ayrı doğrusal bir sarma algoritmasıdır.
- `outline` aynı yerleştirilmiş glyph'in gerçek konturunu verir. 13 Latin,
  birleşik aksan, Arapça/İbranice ve teknik sembol konturu bağımsız
  FontTools BasePen verisiyle eşleşti. Kontur kapatmanın örtük düz kenarı
  karşılaştırmada açık çizgi olarak temsil edilir; beklenen geometri
  KentOS çıktısından türetilmez.
- Kesir için gereken atomik satır içi nesne temeli eklendi. Nesnenin özgün
  metni değişmeden kalır; yalnız bidi çözümünde U+FFFC yer tutucusu kullanılır.
  Çözülen seviyeler özgün kaynak aralıklarına taşınır. Bu,
  [Unicode 18, bölüm 23.8.4](https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-23/)
  içindeki nesne yer tutucusunun kullanımıdır. 15 harici FriBidi örneğiyle
  görsel sıra, nesne sınırında kesilen Arapça birleşmesiyle de harici
  HarfBuzz glyph sonucu karşılaştırılır. Bir kesrin kaynak metninin
  içindeki boşluklar satır bölme fırsatı olmaz. Bu temel, kesir içeriğinin
  ve üst/alt simgenin son renderer entegrasyonunun tamamlandığı anlamına gelmez.
- Tüm 29 runtime font kimliği ve font byte özeti `unicode-v1` manifestinde
  sabitlendi; ilk 22 yüzün UI varlıklarından gelmesi bu denetimi atlatmaz.
  Eski dosyayı yeniden akıtan bir font güncellemesi, mevcut manifesti yeniden
  üreterek kabul edilmez: eski profil byte'ları korunarak yeni profil gerekir.
  Kontrol hem Python kaynak doğrulamasında hem Rust testinde yapılır.

Bu aşamada Unicode LS/PS ve diğer özel paragraf kontrol kodları glyph
satırına sessizce gömülmez; desteklenmeyen sınır hata döndürür. Tam UAX #14
sarma veya bütün Unicode yazıları için uygunluk iddia edilmez. Yeni profilin
kalıcı KCAD ayrımı, açık dönüşümü, rich-run adaptörü, gerçek sahne/hit-test ve
aranabilir PDF entegrasyonu henüz tamamlanmamıştır.

Ek kod incelemesi: mevcut alan denetimi belge içindeki bağımsız yazılar ve
onların kaynak zincirini kapsar. Blok tanımlarındaki anonim yazı geometrileri,
tablo/kılavuz alan yolları ve bunların açık yenileme/teslim davranışı ayrıca
incelenecek; yalnız bağımsız yazı kabulü bütün field ekosisteminin kabulü değildir.

### Rich-run ve yazı sistemi hazırlığının güncel durumu

- `shaping/rich.rs` üst/alt simgeyi gerçek glyph boyu ve baseline kaymasıyla,
  üç kesir/tolerans biçimini atomik yerleşimle üretir. Beş bağımsız C
  HarfBuzz örneği, Arapça rakamlar ve kesir içindeki boşluklar dahil geçti.
  Alt çizgi ve kesir çubukları da ink kutusuna katılır; renk metni her satır
  için çoğaltılmak yerine özgün run'a referans verir.
- Renk veya alt çizgi değişimi glyph'leri yeniden shape ederek kerning'i
  değiştirmez. Bir ligatürün tamamına renk uygulama yerleşimi değiştirmeden
  çalışır. Bir ligatürün içindeki bileşenleri ayrı boyama/kırpma yolu henüz
  tamamlanmadı; `MixedClusterPaint` bunu açıkça reddeder. Mevcut legacy
  metinler bu adaptörden geçirilmez.
- [UAX #24 §5](https://unicode.org/reports/tr24/) uyarınca bir grapheme'in
  yazı sistemi birlikte çözülür; ortak işaretler bağlamla, eşleşen parantezler
  çevreleyen yazıyla ilişkilendirilir. Latin/Yunanca/Kiril karışımları dört
  açık ISO 15924 script etiketli C HarfBuzz örneğiyle eşleşti. Unicode
  blok aralıklarından alfabe tahmini yapılmaz. Yeni doğrudan bağımlılık
  `unicode-script 0.5.8` Unicode 17 tablolarını kullanır.
- Güncel 20 entegrasyon testi ve iki yazı sistemi birim testi geçti;
  eski metin testleriyle `--test all text` toplam 39 testi geçti.
  Altı bin satırın alt çizgileri ve 601 scalar'lık tek birleşik karakter
  kümesi de sınırlı metadata işiyle işlendi. Bunlar GPU/sahne kabulü değildir.
- Mevcut GUI yolu Cosmic Text 0.15/Harfrust kullanır; Rustybuzz ile aynı
  motor olduğu varsayılmaz. Yeni ortak glyph planının gerçek ekranda ve
  PDF'de kullanılması hâlâ gerekiyor.

Kapatılmadan önce kalan entegrasyon koşulları: kalıcı profil ayrımı ve
tek geri alınabilir açık dönüşüm; güvenli glyph hata/onarım davranışı;
sahne önbelleği ve yerel koordinatta GPU çizimi; eğri boyunca yazıda
aksanın/ligatürün ve kesrin tek grup tutulması; glyph kimlikli font alt
kümesi ve mantıksal okunabilir PDF metni (`#`/`^` gösterim semantiği dahil);
blok/tablo/kılavuz alan denetimi; Python/MCP eşdeğerliği ve gerçek GUI kabulü.
Hiçbiri henüz tamamlandı sayılmaz.

Yükseklik konusunda otomatik yeniden yorumlama yapılmaz.
[Autodesk'in TrueType açıklaması](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-LT/files/GUID-7BC6132F-BF07-4A49-B8A8-4FCB524EB736.htm)
nominal yüksekliği büyük harf ve ayrılan aksan alanıyla birlikte açıklar;
[BricsCAD Height](https://developer.bricsys.com/bricscad/help/en_US/V25/DevRef/source/Properties_Height.htm)
büyük harf yüksekliği ifadesini kullanır. Bu açıklamalardan bütün fontlar
ve ithal dosyalar için tek bir cap-height dönüşümü çıkarılmaz. Mevcut KentOS
model/em davranışı korunur; DWG/DXF yüksekliği ve gerçek kâğıt ölçüsü,
font metrikleri ve karşılaştırılabilir gerçek çıktı ile ayrıca doğrulanacak.

## MTEXT aktarımındaki veri kaybı

Sayısal geometri paketinin önceki `kind=10` yazı kaydı `flow` alanını
atıyordu. Yeni genişletilmiş `kind=37`, aynı Float64 geometri kaydının
sonuna yalnız paragraf düzeninin JSON string indeksini ekler. Girinti,
sekme, sütun ve kesir biçimleri taşınırken koordinatlar JSON'a çevrilmez;
negatif sıfır dahil Float64 bitleri korunur. Düzeni olmayan eski yazı
paketi yine `kind=10` olarak üretilir. Web adaptörü de aynı kaydı okur/yazar;
üst/alt bitlerinin birlikte kullanıldığı kesir biçimini ayrıca korur.
Kimlik/ölçek/öteleme karşılaştırması doğrudan çekirdek dönüşümüyle aynıdır;
15 WASM paket testi geçti. Bu, kalıcı KCAD şemasında değişiklik değildir.

Web uygulamasının tam `tsc --noEmit` kontrolünde 35 mevcut hata vardır;
bu kontrol başarılı diye kaydedilmez. Dosyaları değiştirmeyen ikinci bir
TypeScript Compiler API denetiminde yalnız değişen `pack.ts` ve
`pack.test.ts` girdileri HEAD içeriğiyle karşılaştırıldı: aynı 35 hata,
aynı dosya/konum/mesaj. Bu düzeltme yeni tür hatası eklemiyor; tam web
uygulamasının önceki tür borcu bu maddeyle çözülmüş sayılmaz.

## Yerleşim değişikliğinin koruma koşulları

Tam Unicode yerleşimine geçiş, eski dosyaların otomatik yeniden akıtılması şeklinde yapılmayacak. Yeni ortak motorun en az şu bilgileri birlikte vermesi gerekir: kaynak scalar/byte aralığı, grapheme sınırları, mantıksal/görsel yön, seçilen paketli yüz, glyph kimliği, advance/offset, baseline ve ink kutusu. Satır kırma, biçim parçaları, PDF ve hit-test aynı sonucu kullanmalı.

Eski advance profili ile yeni şekillendirilmiş profil kalıcı olarak ayırt edilmeli; profil değiştirme açık bir işlem ve tek geri alma adımı olmalı. Sadece PDF'ye Rustybuzz eklemek model kutularını düzeltmez. Sadece model ölçümünü değiştirmek ise eski MTEXT satır sonlarını ve alan metinlerinin yerleşimini değiştirir. Bu nedenle bu ilk düzeltmeler metrik tablosunu, eski dosyanın fontunu veya satır kırma kuralını değiştirmez.

## Destek/kabul matrisi — henüz tamamlanmayanlar açık

| İçerik | Düzenleme | Model/çıktı eşdeğerliği kabulü |
| --- | --- | --- |
| Türkçe Latin harfleri | Mevcut scalar dönüşümü korunuyor | Tek satır, MTEXT, aynalama, kâğıt boyu ve satır sonları karşılaştırılacak |
| Birleşik aksan / ZWJ / bayrak | Küme bütünlüğü testleri; geri/ileri silme | Shaping ve ölçülen kutu ortaklaştırılmadan tam kabul yok |
| Teknik semboller | Simge menüsü korunuyor | Paketli font kapsamı ve fallback ölçümü doğrulanacak; sistem fontu görünüşü kanıt değildir |
| Karışık LTR/RTL | Mantıksal metin korunuyor | Ortak bidi/shaping/glyph yerleşimi ve aranabilir PDF kanıtı gerekiyor |
| Girinti/tab/sütun/kesir | Mevcut veri ve düzenleyici korunuyor | Yeni profile geçişte satır/range ve yerleşim regresyonları gerekiyor |
| Field | Kaynak/önbellek ve hata ayrımı; açık yenileme | Güncel olmayan değer PDF teslimini engeller; gerçek arayüzde yenileme/tek geri alma ve baskı denetimi doğrulandı |

Gerçek GUI, PDF ve otomatik testlerin kabul kaydı plan maddesinin altında tutulacak. Ham ekran/PDF/deneme çizimleri repo dışındadır. Henüz çalıştırılmamış bir kontrol bu belgede tamamlanmış sayılmaz.
