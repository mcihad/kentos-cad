# 0914 — CAD yazısının yerleşimi ve canlı alanların teslim doğruluğu

Tarih: 4 Kasım 2026. Kapsam: CAD-ADV-10. Durum: yerel 2D CAD kabulü tamamlandı. Destek sınırları aşağıdaki güncel matriste tanımlıdır.

İlk bölümler uygulama sırasındaki tarihsel inceleme kayıtlarıdır; o bölümlerdeki
“henüz” ifadeleri son durum değildir. Son durum için güncel destek matrisi ve
“Son kabul” bölümünü okuyun.

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

> **11 Kasım 2026 eki — DXF yüksekliği doğrulandı ([ADR 8046 §3](8046-dwg-dxf-layer-profile-and-texts.md)).**
> Trimble Tekla'nın DWG aktarımı em'i büyük harf yüksekliğine çevirir ve yüksekliğin AutoCAD'de aynı göründüğünü
> söyler; BricsCAD, QCAD ve ezdxf de 40'ı büyük harf yüksekliği okur. Tek bir sabit değil, **her yazı tipinin kendi
> büyük harf oranı** (Arial 1466/2048, Barlow 0,700 …) kullanılır: TrueType yazının 40'ı `em × oran` yazılır, okunurken
> bölünür; SHX'te dönüşüm yoktur. KentOS'un model/em davranışı değişmedi; dönüşüm yalnız DXF/DWG sınırındadır.

## Açık profil entegrasyonu — 4 Kasım 2026, devam ediyor

Önceki hazırlık kayıtlarının ardından `layout.engine: "unicode-v1"` açık
profili eklendi. Alan yoksa legacy ölçü ve satır düzeni korunur. Dosya
açılması, kaydedilmesi veya font listesinin değişmesi profil yükseltmez.
KCAD şema 163 bu bilgiyi kök ve blok yazılarında taşır; önceki şemalar
motor alanını reddeder. Motor dışında düzen bilgisi olmayan yazı da
legacy sayılıp alanını kaybedemez. Genel arayüzde varsayılan etkinleştirme
ve açık, tek Undo ile profil dönüşümü henüz yapılmadı.

- Ortak `shaping::placed` planı gerçek font konturunu, eğri üzerindeki
  grapheme/ligatür/kesir gruplarını, alt çizgileri ve fontun gerçek taşmasını
  içeren seçim/maske kutusunu üretir. Koordinatlar yazı noktasına göredir;
  kamera çıkarımı f64 yapılır, çizime geçerken f32'ye çevrilir.
- Yerel sahne kaynak metni UI font motorunda yeniden dizmez. Hazırlanmış
  konturlar nesne sürümü, font ve blok açılımı kimliğiyle önbelleklenir.
  Yakınlaştırma planı bozmaz; değişiklik ve Undo sürümü geçersizleştirir.
  Önbellek 128 yazı ve toplam bir milyon kontur işlemiyle sınırlıdır.
- Açık profilin ölçüm sorguları desteklenmeyen metin için hata verir;
  sahnedeki hata işaretinin kutusunu gerçek yazı ölçüsü olarak döndürmez.
  Eğriye oturtma/düzleştirme, okunurluk ve maske hesabı aynı kümeleri kullanır.
- PDF aynı GID'leri ve em dönüşüm matrislerini alır. Yirmi dokuz yüzün
  sabit paketli byte'larından gerçek font alt kümesi, açık CID→GID tablosu
  ve ToUnicode üretilir. Aynı glyph'in farklı kaynak kümeleri ayrı CID
  alabilir; font byte'ları alt küme adına da katılır. Sistem fontu yoktur.
- PDF'de gerçek yazı nesneleri korunur; yalnız kontur çizip görünmez
  sahte metin eklenmez. `ActualText` mantıksal kaynak metni taşır; kesir
  yazımındaki `#` görünür `/`, tolerans ayıracı `^` boşluk olarak aktarılır.
  Tek yazının PDF replacement metni 16.382 UTF-16 birimiyle sınırlıdır;
  aşınca metin kesilmez, yazıyı bölmeyi isteyen açık hata verilir.
- Desteklenmeyen profil, font/GID, sıfır veya sonlu olmayan dönüşüm ve
  aşırı kaynak/yerleşim bütçesi PDF'de reddedilir. Paketli tamamlayıcı
  fontların gerçekten kullanıldığı yerler denetimde bilgi olarak gösterilir.

Kabul kanıtları: 44 çekirdek metin testi, 15 uygulama denetim testi,
13 PDF testi (bir dosya üretme testi ayrı/ignored) geçti. Gerçek GUI'de
yedi yazı açıldı; birleşik karakter silme, onay, Undo, yakınlaştırma ve
Farklı kaydet denendi. Bağımsız KCAD okuyucusu, Undo sonrası kaydedilen
dosyanın özgün dosyayla **byte düzeyinde aynı** olduğunu doğruladı.
Bağımsız pypdf + FontTools okuyucusu 29 yüzün 328 CID konturunu paketli
orijinalleriyle karşılaştırdı; GUI'den kaydedilen örneğin PDF'sindeki beş
yüz ve 60 CID de eşleşti. Sonraki gerçek GUI kabulünde 1:20 ve 1:50 iki
pafta açıldı, arayüzden birlikte PDF'ye kaydedildi. Bu çıktının beş yüz ve
64 CID konturu da doğrulandı. Bağımsız PDF okuyucusu ilk glyph matrisindeki
boyut oranını tam 2,5 buldu; ilk baseline konumları bağımsız kâğıt hesabına
0,00002 mm toleransla uydu. KCAD'in 235 testi, yerel kitap PDF yolunun
sekiz testi (ayrı dosya testi ignored) ve ortak PDF'nin 13 testi tekrar geçti. Bunlar yeni profilin
bütün kabulü değildir.

CPU baskı önizlemesi/raster map çıktısı da hazırlanmış konturları ve satır
renklerini kullanır; ikinci kez legacy fontla şekillendirme yapmaz. Gerçek
GUI dosyasının iki paftasının 144 dpi önizlemesi üretildi. 1:50 önizlemesi
bağımsız Poppler PDF rasteriyle aynı 2382 × 1684 pikselde karşılaştırıldı:
tüm mürekkep sınırları bir piksel içinde; bir piksel komşuluğunda eşleşmeyen
CPU pikseli 0/6726, PDF pikseli 1/7084. Anti-aliasing motorları farklıdır;
bu sonuç byte düzeyinde raster eşitliği veya bütün metin çeşitleri kabulü
olarak yorumlanmaz. Deneme dosyaları repo dışında kalır.

Ek otomatik çıktı testinde renkli kesir/alt çizgi, karışık yazı ve eğri
metin, 900 milyon/400 milyon metre referans ve 17° döndürülmüş iki pafta
(1:50/1:100) kullanıldı. Her yönde renkli mürekkebin en az %99'u bir piksel
komşuluğunda bağımsız PDF rasteriyle eşleşti; kırmızı kesir ve mavi temel
harfler ayrıca sayıldı. Belge bu kontrollerle değişmez. Güncel gerçek
GUI'de 1:50 baskı önizlemesi açıldı; font fallback bilgisi hata rengi yerine
standart vurgu rengiyle gösterildi. Aynı bilgi PDF ön denetiminde de bilgi
düzeyinde tutulur.

**Açık kabul:** PDF görseli ve mantıksal ActualText doğru olsa da kullanılan
Poppler sürümü bazı karışık RTL replacement metnini çıkarırken ters
sıralıyor; tüm okuyucularda arama/kopyalama kabulü tamamlanmış sayılmaz.
PDFium 156.0.8076.0 ile Arapça ve İbranice sözcüklerin ayrı ayrı araması
ve kopyası doğru; bütün karışık satırın run sırası kaynak sırayla aynı değil.
Bu nedenle yalnız sözcük araması tam paragraf kopyası kabulü sayılmaz.
Ligatür içi parçalı renk/alt çizgi, bütün field taşıyıcıları ve açık profil
dönüşümü sürüyor. Python SDK ve komut kataloğu güncellendi; yeniden derlenen
Python bağlamasıyla altı test, MCP üzerinden beş test geçti. Kaynak metin,
profil ve salt okunur denetim aynı sözleşmeyle korunur.
Web çizimi ADR0228 uyarınca TODOS.md §0.2'ye ertelendi. CAD-ADV-10 bu nedenle açık kalır. Ekranlar ve deneme dosyaları
repo dışında tutulur.

PDF kaynakları: [pdf-writer 0.15.0](https://docs.rs/pdf-writer/0.15.0/pdf_writer/),
[PDF Association, ActualText](https://pdfa.org/download-area/publications/Tagged-PDF-Best-Practice-Guide.pdf),
[PDFium RTL ActualText kabul örneği](https://pdfium.googlesource.com/pdfium.git/+/refs/heads/chromium/8077/testing/resources/actual_text_rtl.in).
Bu kontroller PDF/A veya PDF/UA uygunluk sertifikası değildir.

## Tablo hücrelerindeki canlı alanlar — 4 Kasım 2026

Kayıtlı tablo hücresi, kaynak nesne/proje alanından salt okunur olarak
hesaplanan güncel değerle karşılaştırılır. Eksik kaynak ayrıca bildirilir;
bir yazıdan okunan hücrede, o yazının sınırlı kaynak zinciri de denetlenir.
Hücrenin kendisi kaynak yazının eski önbelleğiyle eşleşse bile kaynak zinciri
güncel değilse temiz kabul edilmez. Bulgudaki ilk kimlik tablo, devamındaki
kimlikler gerçek kaynaklardır; işaret hücrenin döndürülmüş merkezine gelir.
512'den çok alan taşıyan tablo açık `table_field_unverified` verir.

Dosya/çizelge kaynağı olmayan fakat canlı hücre alanları olan tablolar da
mevcut `cad.tables.edit` / `refresh` üzerinden açıkça yenilenebilir.
Kaynağı ve alanı olmayan tablo eski `no_source` davranışını korur. Masaüstü
“Tabloyu güncelle” aynı komutu kullanır; kilitli katman ve tek Undo kuralı
korunur. Kaynak yazı zincirinin onarımı otomatik yapılmaz; denetimde Kaynağa
git ile incelenmesi istenir. Güncel olmayan/doğrulanamayan alan ve kaynak
zinciri, yerel PDF ile CPU baskı önizlemesini engeller.

17 uygulama denetim testi, tablolara ait mevcut komut örnekleri, 10 kitap
çıktı testi (bir dosya testi ayrıca ignored), altı MCP ve yedi Python testi
geçti. Python bağlaması yeniden derlendi. Bu değişiklik yeni komut veya
kalıcı şema getirmez; mevcut sözleşmenin salt okunur denetimini ve açık
refresh kapsamını tamamlar. Gerçek GUI'de eski hücre → açık yenileme → temiz denetim → tek Undo →
aynen eski bulgu dönüşü denendi. Undo sonrası Farklı kaydet dosyası bağımsız
KCAD okuyucusuyla okundu; özgün dosyayla byte düzeyinde aynıydı. Önizleme
reddinin açıklaması ilk testte modalın arkasında kalıyordu; bu, standart
KentOS `Banner::error` ile baskı seçeneklerine taşındı. Yenileme sonrası
uyarı kalktı ve aynı paftanın önizlemesi açıldı. Hata/retry durum testi de
geçti; ekranlar ve belgeler repo dışında. Tabloların ortak Unicode glyph profiline
geçişi ve blok içi alanların denetimi ayrıca açıktır.

## İç içe blok yazısının gerçek kaynağı — 4 Kasım 2026

Blok açılımındaki anonim yazının kalıcı nesne UUID'si yoktur. Yerleştirmenin
UUID'si bulgunun sahibidir; kaynak kimlikleri gerçek belge nesneleridir.
Çekirdek parçanın yalnız dahili köken kaydı, değişmez tanım kimliği ve o
tanımdaki özgün parça sırasını taşır. Dinamik görünürlük filtrelediğinde bu
sıra değişmez. İç içe açılım, dönüşüm ve XCLIP mevcut çizim yoluyla yapılır;
font/field bilgisi render geometrisinden tahmin edilmez. Kaynak tanımdaki
yazı, gerçek yerleştirilmiş şekliyle salt okunur değerlendirilir. Sahte UUID,
geçici belgeye nesne ekleme ve açılışta otomatik alan yenileme yoktur.

- `block_text_field_stale`, `block_text_field_broken` ve
  `block_text_field_unverified` bağımsız yazıyla aynı sınırlı kaynak grafiğini
  okur. Sahip kimliğinin kök yazı olmadığı açıkça ayrılır; mevcut yazı
  döngüsü ve 512 kaynak sınırı korunur. Font kapsamı da gerçek parçadan okunur.
- Denetim, blok kütüphanesini sorgu boyunca bir kez hazırlar. Yalnız yazı/alan
  parçalarını en çok 64 parçalık geçici öbeklerde yerleştirir; ilgisiz
  geometriler kopyalanmaz. Temiz parçalar dahil iş/iptal kontrol noktaları
  vardır; iptal edilmiş sorgu kısmi sonucu tamamlanmış gibi döndürmez.
  Kütüphanenin mevcut düzleştirme/dinamik önbellek hazırlığı ayrıca atomik
  kalır; bu hazırlığın tümüne gerçek zamanlı iptal garantisi verilmez.
- Normal denetim gizli katman kapılarını atlar; `includeHidden` bunları
  kapsar. XCLIP ile gerçekten çizilmeyen parça yine dışarıda kalır.
  PDF, hazırlanmış sahnenin gerçek parça listesini, viewport katman
  geçersiz kılmalarını ve yazı anchor kırpmasını kullanır. Güncel
  olmayan/doğrulanamayan görünür blok alanı PDF ve CPU önizlemesini engeller.
- UI'da işaret, yerleştirilmiş yazının konumuna gelir; tanım adı ve özgün
  parça sırası gösterilir. Kaynağa git yaşayan kaynaklara gider. Bağımsız
  yazının yenilemesi blok yerleştirmeye uygulanmaz. Tanımın açık düzenlenmesi
  istenir; patlatma veya tüm örnekleri habersiz değiştirme yapılmaz.
- Mevcut dosya/blok komutları tanım içinde tabloyu yasaklar; bu kapsam
  tabloyu bloğa koyma özelliği eklemez. Bellekte doğrulanmamış bir tablo
  parçasına rastlanırsa hücre denetimi savunmacı olarak çalışır; bu durumun
  birim/PDF testi, geçerli KCAD blok-tablosu desteği sayılmaz.

Aynı okuma Python'da `cad.drawing.inspect(doc, checks=['annotations'],
uids=[insert_uid])`; MCP'de `cad.drawing.inspect` için
`{"drawing": drawing_id, "checks": ["annotations"], "uids": [insert_uid]}`
girdisidir. İlk `uids` öğesi gerçek yerleştirmedir; `points` model
koordinatındadır. Denetim kaydetme/Undo içeriğini değiştirmez. Yeni komut,
kalıcı şema veya yeni kullanıcı ayarı yoktur. 25 uygulama denetimi, 14
çekirdek birim + 12 blok entegrasyonu, dokuz kitap çıktı testi (bir dosya
üretme testi ignored), yedi MCP ve yeniden derlenen bağlamayla sekiz
Python testi geçti. İki çekirdek blok entegrasyonu ayrıca mevcut ignored
kalır. Gerçek GUI'de içteki eski yazıya yakınlaşma, tanım/parça açıklaması
ve baskı önizlemesinin standart hata şeridi doğrulandı. Bu işlemlerden sonra
Farklı kaydet dosyası bağımsız KCAD okuyucusuyla aynı model ve aynı byte'ları
verdi. Ham ekran/çizim belgeleri repo dışında tutulur.

[Autodesk UPDATEFIELD](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-2B47D62E-19D0-4CA5-9AAC-1946132D2F62.htm)
seçilen nesnelerin alanını açıkça güncelleyen işlemdir.
[Blok/xref bağlamsal alan açıklaması](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Core/files/GUID-D32A29E7-7B98-4ADB-BF32-5CC24DBCC8C3.htm)
son önbellek ile yerleştirme bağlamının ayrılabildiğini belirtir. KentOS
kararı, açılışı yenileme saymamak ve doğrulanmamış önbelleği teslimde
sessizce güncel kabul etmemektir; AutoCAD'in bütün bağlamsal alan türlerinin
uygulandığı iddia edilmez.

## Açık yerleşim seçimi ve çizim üzerinde taslak — 4 Kasım 2026

Mevcut paragraf düzenleyicisinde **Eski görünüm / Unicode** seçimi vardır.
Öznitelikler'in **Yazı yerleşimi** menüsü de tek yazıyı aynı düzenleyicide
önizlemeye açar; menü seçimi belgeye yazmaz. Mevcut tek satır çift tıklama
alışkanlığı korunur. Çoklu seçim için habersiz toplu dönüşüm eklenmez.
Yeni yazıda da açık seçim yapılabilir; eski/yeni dosyanın varsayılanı bu
adımla değiştirilmez.

- Önizleme mevcut sahne çizimini ve ortak glyph konturlarını kullanır.
  Eğri üzerindeki yazının gerçek baseline'ı da taslağa taşınır. Onay,
  mevcut `cad.entities.edit` / `properties` komutudur; tek **Değiştir**
  geri alma adımıdır. Esc/Vazgeç salt okunurdur.
- Yalnız profil değiştiğinde baş/son boşluklar, özgün scalar aralıkları,
  field'lar, girinti, sekme, font, maske, genişlik ve eğri korunur.
  Mevcut profili yeniden onaylamak boş işlem/boş Undo üretmez.
- Desteklenmeyen glyph/biçim standard KentOS hata şeridinde görünür;
  Tamam kapanır, klavye de aynı denetimden geçer. Taslak kaybolmadan
  düzeltilebilir veya iptal edilebilir. Eski field'ı güncellemek profil
  seçiminin yan etkisi değildir; eski/doğrulanamayan alan önce açıkça
  yenilenmelidir. Özgün görünüşe dönmek için metni bozmak gerekmez.
- Kaydetmeden önce belge oturumu, kalıcı hedef UUID'si ve belge sürümü
  yeniden kontrol edilir. Başka işlem çizimi değiştirdiyse eski taslak
  üzerine yazmaz; kilitli hedef yazılmaz. Yeni metin işlemlerinin mevcut
  araç tamamlama yolu korunur.
- Glyph hazırlığı editör oturumu, içerik sürümü ve gerçek fontla tek
  önbellektir; imleç/seçim hareketi tekrar şekillendirme yapmaz. Sahnenin
  önizleme anahtarı büyük kontur ağacının Debug çıktısını her karede
  üretmez. Taslak kimliği kalıcı belgeye eklenmez.
- Kısa sahnede yalnız düzenleyici gövdesi kayar; Tamam/Vazgeç sabit
  erişilebilir kalır. Bütün kontroller mevcut KentOS düğme, property
  seçici, tipografi ve Banner bileşenleridir. macOS kısayol yazısı gerçek
  Command+Enter davranışını gösterir; diğer platformlarda Ctrl+Enter'dır.

81 düzenleyici/öznitelik/inline regresyon testi (beş mevcut görsel test ignored),
dokuz Python ve sekiz MCP testi geçti. Bunlar
boşluk/alan/eğri korumasını, salt okunur taslağı, eksik glyph reddini,
başka belge değişikliğine karşı korumayı, önbelleği ve tek Undo'yu kapsar.
Gerçek uygulamada koyu/açık tema, kısa ve geniş sahne, çift tıklama ve property menüsünden
önizleme, açık onay, eksik glyph reddi ve Esc denendi. Bağımsız KCAD
okuyucusu GUI dönüşümünde yalnız ilk yazının `layout.engine` alanının
değiştiğini; tek GUI Undo sonrası dosyanın özgün byte'larla aynı olduğunu
kanıtladı. Ham ekran/çizimler repo dışındadır. Tablonun ortak glyph
profiline geçişi, bütün harf biçimi sınırları ve okuyucular arası PDF kopyası
bu dilimle tamamlanmış sayılmaz.

[Autodesk'in yerinde MTEXT düzenleme açıklaması](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-3740EC59-FB75-47FD-B042-03D6AC095D4F.htm)
biçimi mevcut editörde düzenleme, açık bitirme ve Esc ile vazgeçme
akışını destekler. KentOS kararı mevcut düzenleyiciyi genişletmek ve
kalıcı metrik değişikliğini önizleme/onaydan ayırmaktır; yeni bir arayüz
veya gizli font yükseltme yolu açılmaz.

## Okunur yap ve ölçülen kutunun korunması — 4 Kasım 2026

[AutoCAD TORIENT](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-3A9CCCD9-7B1D-4C83-944C-2D66F5FF32DF.htm)
yazıyı orta noktası çevresinde 180 derecelik adımlarla çevirir; nesnenin
çizimdeki genel konumu korunur. KentOS'un mevcut `Okunur yap` kuralında
Unicode profili de artık eski nominal em kutusu yerine ortak glyph
planının ölçülen çerçevesinin ortası çevresinde döner. Bu çerçeve harfin
gerçek taşmasını, fallback yüzünü ve eğiklik dönüşümünü içerir. Eski
yerleşimin dönüşüm formülü değiştirilmez.

- Beş ters açı, üç hiza, aksan/teknik sembol/RTL ve yalnız boşluk içeren
  yazılar 5.000.000/4.400.000 koordinatında karşılaştırılır. Dönüşten sonra
  karşılıklı köşeler iki nanometre içinde aynı yerde olmalıdır. İkinci
  çağrı yeniden değişiklik üretmez; glyph kimlikleri korunur. Düzeltmeden
  önce bu test, eğik aksanlı yazının kaydığını göstermiştir.
- Tek satırın bütün hizaları arasında ve çok satırın düşey hizasında
  glyph/alt çizgi dünya koordinatları değişmez. Paragrafın yatay hizası
  değişince kısa satırların sağa/ortaya dizilmesi mevcut bilinçli davranıştır;
  bütün harflerin aynı yerde kalması bu işlem için yanlış bir kabul olur.
- `textReadable` ve `textRealign` JSON/WASM çağrılarındaki `width` kestirimi
  yalnız eski profile aittir. Açık Unicode profilinde gerçek plan ölçülür;
  istemci eski bir genişlik göndererek motoru devre dışı bırakamaz.
- `cad.shapes.edit/readable` bütün seçimi yazmadan önce doğrular. Eksik
  glyph veya eski/doğrulanamayan field varsa açıklamalı, atomik ret verir.
  Alan önbelleği çevirmeden sonra kendiliğinden yenilenip önceden hesaplanan
  kutuyu değiştiremez; kullanıcı önce açık alan yenilemesi yapmalıdır.
  Bu koruma mevcut genel field takip mekanizmasını değiştirmez.
- Native test, güncel alanı olan yazının yalnız konumu/yönünün değiştiğini,
  field aralıkları, biçimi ve maskesinin korunduğunu; eski alanın ve bozuk
  profilin validate/plan/execute aşamalarında Undo üretmediğini denetler.
  Python ve MCP aynı mevcut komuta bağlanır; ikinci dönüşüm yolu eklenmez.

## Aynalama, miras alınan font ve blok önbelleği — 4 Kasım 2026

[AutoCAD MIRROR](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-595277C8-9B87-4CFB-A3AF-769537A22F3D.htm)
kaynakları koruma/silme ve yazı yönünü koruma kararlarını ayrı tutar.
KentOS'un ADR 0292'deki mevcut politikası korunur; ters glyph bayrağı
olmayan `mirrored` kipini tam AutoCAD MIRRTEXT=1 eşdeğeri saymıyoruz.

Unicode yazının okunur aynasında nominal hiza tablosu tek başına gerçek
harf kutusunu doğru yere koymuyordu. `transform_shape_in`, gerçek kaynak
çerçevesinin merkezini aynalama matrisiyle taşır ve yeni okunur yazının
ölçülen merkezini o konuma oturtur. Yalnız açık Unicode profili etkilenir;
eski metin dönüşümleri ve donmuş web politikası aynı kalır. Eğri yazısının
mevcut eğri dönüşümü bu düz yazı düzeltmesine sokulmaz.

- Fontu olmayan yazı projenin seçtiği paketli fontla ölçülür. Araç komutu,
  seçim önizlemesi ve paketli store kopyası aynı fontu iletir. Oda modülü
  dönüşümleri de fontu belge üzerinden geçirir; nesneye yeni bir font
  yazılarak miras alma davranışı bozulmaz.
- İç içe ve dinamik bloklar çizimin fontuyla hazırlanır. Nadir proje-fontu
  değişiminde düzleştirilmiş parçalar tekrar kaynak sayılmaz; saklanan
  özgün parçalar, öznitelikler, dinamik tarif ve küçük PDF-altlık metadata'sı
  üzerinden yeniden hazırlanır. Sırf bunun için bütün statik kaynak
  geometrisinin ikinci kalıcı kopyası tutulmaz. Varyant önbelleği yenilenir.
- Sahne, çıktı, denetim, nesne/parça seçimi ve kaynağa bağlı ölçü/kılavuz
  okuyucuları aynı fontlu blok konumunu kullanır. Font değişikliği model
  sürümü ilerlemese de sahne store'u fontu kontrol eder; eski glyph planı
  geçerli kabul edilmez.
- Aynalama ile Okunur yap aynı alan/profil ön denetimini kullanır:
  eski alanın otomatik yenilenmesi ölçülen konumu değiştiremez. Ret bütün
  seçime aittir ve validate/plan/execute yazmaz.
- Gerçek ölçülen merkez; dört açı, yatay/düşey/eğik eksen, aksan ve karışık
  yönlü paragrafla sınanır. İki kez iç içe aynalanan blokta Arimo/Overpass/
  Barlow değişimi, yeni hazırlanmış blokla aynı parçayı verir; font mirası
  ve altlık saydamlığı korunur. Native plan/execute eşitliği ve tek Undo
  ayrıca sınanır. Nokta bulutu testindeki eski “bilinmeyen 35” beklentisi,
  35'in artık `Tag` olması nedeniyle gerçek atanmamış 65535 ile düzeltilir;
  paket okuyucusunun üretim kodu değiştirilmez.

Gerçek GUI kabulünde önce ikinci eksen noktası beklenirken Esc denendi:
çizim dört nesne ve temiz kayıt olarak kaldı. Yeniden aynalama başlatılıp
fareyle eksen onaylandı; kaynak dört nesne kaldı, dört bağımsız kopya oluştu.
Harici HarfBuzz/FontTools ölçüsünden hazırlanmış iki referans çerçevesinin
köşeleri ve iki yazının gerçek merkezleri aynı yansıma eksenine 4 nm içinde
uydu. Tek Ctrl+Z sonrası Farklı kaydet dosyası özgün KCAD byte'larıyla aynı.
Önizleme, onay ve Undo ekranları repo dışında tutulur. Mevcut aynalama
önizlemesi yazı için yer işaretini gösterir; canlı glyph önizlemesi iddia edilmez.

Bu dilimde tam geometry-core takımı 973 geçti/31 ignored; native uygulama
459 geçti/37 ignored; interaction 72, MCP denetimi 10, Python 12 testi geçti.
Sıkı masaüstü Clippy ve güncel masaüstü derlemesi başarılı. Bu kayıt,
CAD-ADV-10'un kalan toplu metin ve genel kabul koşullarını kapatmaz.

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

## Güncel destek matrisi — yerel 2D CAD

Aşağıdaki matris eski hazırlık kayıtlarından daha yenidir. Destek açık
`unicode-v1` profili içindir; “bütün Unicode yazıları” veya eski metinleri
kendiliğinden yeniden akıtma sözü değildir.

| Yol/içerik | Davranış ve kabul sınırı |
| --- | --- |
| Türkçe, birleşik aksan, teknik semboller | Paketli gerçek glyph ve kontrollü fallback; aynı plan model/viewport, hit-test kutusu, CPU önizleme ve PDF'de kullanılır. Bağımsız font/contour ve raster karşılaştırmaları yukarıda. |
| Arapça ve İbranice karışık metin | Paragraf bidi çözümü, gerçek satır seviyeleri, bağlamsal shaping ve mantıksal kaynak aralıkları korunur. RTL sözcük araması ve bütün paragrafın kopya sırası ayrı kabul edilir; okuyucunun geometrik sıra tahmini kaynak metnin sırası yerine geçmez. |
| Girinti, sekme, statik sütun, kesir/tolerans, üst/alt simge | Ortak fiziksel akış, kesir çubuğu ve gerçek ink kutusu; düzenleme ve kâğıt boyu GUI/PDF ile denendi. Scalar aralıklar dosyada değişmez. |
| Eğri boyunca metin | Grapheme/ligatür/kesir grubu bütün olarak eğriye taşınır. Düz metnin merkez düzeltmesi eğriyi yeniden yorumlamaz. Eğride kopyalama/seçim, okuyucunun geometrik tahminine de bağlıdır. |
| Renk/alt çizgi | Glyph kümesinin tamamında destekli. Ligatürün yalnız bazı bileşenlerine ayrı boya/kırpma desteklenmez; açık `MixedClusterPaint` hatası, editörde onay koruması ve çıktı reddi vardır. Ligatürü bozarak yeni ölçü üretilmez. |
| Paketli fontta bulunmayan küme, ör. CJK/emoji | Sessiz `?`, işletim sistemi fontu veya rastgele metrik yok. Profil işlemi/teslim açıklamalı ret verir. Font bulunması ile layout desteği ayrı denetlenir. |
| Kök/blok yazısı | Açık profil kaydedilir; miras fontu blok açılımı/denetim/çıktıda aynı. Tanım/parça kökeni korunur; alanın kaynağı, eski önbelleği ve doğrulanamaması ayrıdır. |
| Kök tablo ve bağlı kaynak yazılar | Hücre alanları açık yenilenebilir, tek Undo; kaynak zinciri de denetlenir. Tablo hücreleri henüz açık Unicode metin profili taşımaz; eski metrik/karakter yolu korunur. |
| Blok tanımındaki alan | Kaynak/önbellek tanım/parça kökeniyle görünür; eski veya doğrulanamayan değer teslimi engeller. Otomatik patlatma/anonim kimlik üretme yok. Tanımın toplu alan yenilemesi bu sözleşmede yoktur. |
| Kılavuz, ölçü, üretilen etiketler ve pafta metni | Mevcut yerleşim/ölçek davranışı korunur; Unicode Text profiline gizlice geçirilmez. Desteklenmeyen gerçek çıktı karakteri ön denetimde açık hata olur. Bu yolların yeni profile toplu dönüşümü teslim edilmiş sayılmaz. |
| Eski dosya/yazı | `engine` yokluğu eski profildir; aç/kaydet, font mirası ve inceleme yerleşimi değiştirmez. Kullanıcı editörde açık önizleyip tek Undo ile profil seçebilir. |
| Web | ADR0228 uyarınca ürün arayüzü ertelenmiştir. Paylaşılan saf çekirdek/kalıcı veri sözleşmesi korunur; masaüstü kabulü web kabulü diye sunulmaz. |

## Toplu metin işlemlerinin korumaları — 4 Kasım 2026

[UAX #29](https://unicode.org/reports/tr29/tr29-49.html) görünür karakterin
scalar'dan farklı olabildiğini açıklar.
[AutoCAD TXT2MTXT](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-1E68C8B2-520F-4084-BD20-51DC4A32A7E5.htm)
birleştirme/biçim ayarını açık bir işlem olarak sunar. KentOS'ta bu
inceleme üç somut açığı gösterdi:

1. `replace` scalar eşleşmesi `g` + aksanın yalnız `g` parçasını
   değiştirip aksanı geride bırakabiliyordu. Yazılacak eşleşmeler extended
   grapheme sınırında değilse bütün işlem açıklamalı `invalid_input` olur.
   Arama sonucu/kayıtlı run aralıkları scalar kalır; gizli normalizasyon yok.
2. Sekmeli paragrafın tab parçaları satır sanılıyordu. Ayırma artık ortak
   layout'un fiziksel derinliğini ve gerçek baseline ofsetini kullanır;
   aynı satırdaki ikinci hücre satır sayısını artırmaz. Döndürülmüş,
   alt hizalı üç satır bağımsız sayısal konumlarla sınanır.
3. Unicode ile eski profil veya farklı font/paragraf biçimleri
   birleştirilirken ikinci yazının biçimi sessizce ilkine dönüşebiliyordu.
   Unicode içeren uyumsuz seçim reddedilir; önce biçim açıkça eşitlenir.
   Uyumlu seçimde run ve alan aralıkları taşınır, tek Undo korunur.

Önerilen bütün Unicode sonuçlar yazmadan önce ortak planla doğrulanır.
Buna numaralama, font/biçim kopyası, wildcard bul/değiştir ve property/stil
komut yolu da dahildir. Eski veya doğrulanamayan alanı olan yazının konum,
boy, birleştirme ve ayırması önce açık alan yenilemesi ister. Denetim ve
plan belgeye yazmaz; toplu ret önceki sağlam hedefi de değiştirmez.
Yeni dokuz native testi, 50 çekirdek metin testi ve tam native takımın
468 testi geçti (37 ayrı/ignored); MCP 11, yeniden derlenen Python bağlamasıyla
13 sözleşme testi de geçti. GUI kabulü aşağıda kaydedildi.

## PDF'nin aranabilir karakter kutusu — 4 Kasım 2026

Gerçek Preview aramasında `العربية` bulunamadı. İnceleme, dondurulmuş
Noto Sans Arabic'in bazı kaynak harflerini sıfır advance'lı ayrı nokta
glyph'inden önce ürettiğini gösterdi. İlk çizilen glyph'e Unicode anlamı
vermek karakterin seçilebilir kutusunu noktanın konumuna bağlıyordu.

Artık kümenin Unicode metni advance'ı en büyük gövde glyph'ine aittir;
diğer glyph'ler boş continuation taşır. Tümü sıfır advance ise ilk glyph
korunur. Harf sırası, GID, kontur, dönüşüm matrisi ve boya değişmez;
`ActualText` yine mantıksal kaynak metnidir. Deneyde yalnız bu eşleme
değişince Preview aynı sözcüğü buldu. C HarfBuzz 15'in bağımsız yedi gövde
GID'siyle birim regresyon geçti. Ardından üretim derlemesinden gerçek GUI
PDF'si üretildi ve aynı sözcük Preview'da bulundu; değiştirilmiş deney
dosyası bu son kabulün yerine kullanılmadı.

PDF Association'ın [mantıksal içerik sırası açıklaması](https://pdfa.org/techniques-for-accessible-pdf/order-of-marked-content-sequences-within-a-tag-correctly-set/UA1_Tpdf-G4_02/)
ve [ActualText rehberi](https://pdfa.org/download-area/publications/Tagged-PDF-Best-Practice-Guide.pdf)
görsel sıra ile mantıksal kaynağı ayırır. PDFium ve PDFKit sürümlerinin
karışık paragrafta çıkarım/kopyalama sırası farklı olabilir. Kaynak ters
çevrilerek belli bir okuyucuya uydurulmaz; PDF/UA, PDF/A veya her okuyucuda
tam paragraf kopyası uygunluğu iddia edilmez.

## Son kabul — 4 Kasım 2026

CAD-ADV-10'un yerel 2D CAD kapsamı aşağıdaki kanıtlarla tamamlandı. Açık
Unicode profili, korunan legacy davranışı ve açıklamalı ret verilen içerik
birlikte teslim edilir. Matrisin dışında kalan içerik için sessiz başarı
veya başka font motoruna kaçış yoktur.

- Gerçek uygulamada yedi Unicode yazılı dosya açıldı: Türkçe/birleşik aksan,
  teknik sembol, Arapça/İbranice/Latin karışık yön, güncel proje field'ı,
  kesir/tolerans, sekme/iki sütun ve eğri boyunca yazı. Aksanlı görünür
  karakter tek Backspace ile silindi; yalnız bu metin değişti. Tek GUI
  Undo ve kayıt özgün KCAD byte'larını geri verdi.
- Aynı yedi yazı arayüzden kaynaklar kaldırılarak okunur biçimde aynalandı.
  Kayıt bağımsız KCAD okuyucusuyla açıldı: profil, metin, run/field, font,
  boy ve genişlik faktörü korundu. Önceki bağımsız referans çerçeveli
  aynalama kabulü merkez/köşe ölçüsünü 4 nm içinde doğruladı. Yeni karma
  dosyada da tek GUI Undo sonrası kayıt özgün byte'larla aynıydı.
- Aynalanmış kayıttan arayüzde 1:50 pafta ve baskı önizlemesi açıldı,
  PDF kaydedildi. Bağımsız FontTools/pypdf denetiminde beş fontun 64 CID
  konturu paketli özgün yüzlerle eşleşti. Mantıksal `ActualText` yedi
  kaynak metni korudu; kesir/toleransın kaynak işaretçileri görünür
  kesir/boşluk karşılıklarıyla açıkça karşılaştırıldı.
- Gerçek Preview aramasında `العربية`, `עברית` ve `İstanbul` ayrı ayrı
  bir eşleşme verdi. Bağımsız PDFium araması bu üç sözcüğü ve `Türkçe`yi
  tam kaynak sözcükle geri verdi. Karışık paragrafın bütününü kopyalarken
  okuyucuların run sırası tahmini hâlâ farklı olabilir. Bu sınırlama
  destek matrisinde kalır; sözcük araması bütün okuyucularda tam paragraf
  kopyası, PDF/UA veya PDF/A uygunluğu diye adlandırılmaz.
- GUI'de kaydedilen aynı dosyanın 144 dpi CPU önizlemesi ve bağımsız
  Poppler rasteri karşılaştırıldı: sınırlar en çok 1 px farklı;
  1 px komşuluğunda eşleşmeyen CPU mürekkebi 0/6776, PDF mürekkebi
  6/7114. Aranabilir karakter sahipliği düzeltmesinin öncesi/sonrası
  gerçek GUI PDF rasterleri byte düzeyinde aynıydı; boya geometrisi değişmedi.
- Gerçek GUI'de üç sekmeli paragraf satırlara ayrıldı. Önceki sürümde
  kaydığı görülen satırlar artık bağımsız referans baseline'larında:
  yeni üst noktalar `(4, 1.36)`, `(4, .86)`, `(4, .36)`. Uyumlu iki metin
  birleştirildi; yalnız beklenen yeni paragraf oluştu. Her iki işlemde
  tek Undo sonrası dosya özgün byte'larla aynıydı.
- Gerçek Bul ve değiştir penceresinde yalnız `g` arayarak aksanı ayırma
  girişimi açıklamalı standart KentOS Banner ile reddedildi. Ret sonrası
  kayıt aynı byte'lar; bütün `Öğç` → `İstanbul` değişiminde yalnız metin
  değişti, Unicode profili korundu, tek Undo aynı byte'ları geri verdi.
  Son derlemede başarılı işlem yalnız başarı bilgisini gösterir; sonuçlar
  tüketildi diye ayrıca “eşleşme yok” uyarısı göstermez. Bu son görünüm
  de gerçek pencerede yeniden doğrulandı.
- Son doğrulamalar: 50 çekirdek metin, tam uygulama 468 (37 ayrı/ignored),
  interaction 72, plot birim 18, kitap 9 (bir ayrı dosya testi ignored),
  ayrıca GUI kaydından çıktı üreten ignored test açıkça çalıştırıldı;
  MCP 11 ve Python 13 geçti. Sıkı masaüstü Clippy, masaüstü derlemesi,
  üretilmiş SDK/katalog kontrolü ve diff kontrolü temiz. Derlemede mevcut
  ts-rs `deny_unknown_fields` uyarıları sürer; bunlar Clippy ihlali değildir.

Ekranlar, geçici KCAD/PDF'ler, bağımsız okuyucu/raster raporları ve loglar
repo dışında `/tmp/kentos-cad-adv08-09-2026-11-03/` altında tutulur. Son GUI
ikili özeti: `bc4339425cf8f280bbcc502d6b39d00e3fb3caa09a57075aa23f7841c0fcca45`.
PDF kabulü aynı çıktı kodunu içeren önceki derlemede yapıldı; son derlemenin
tek ilave değişikliği bul/değiştir penceresindeki gereksiz boş sonuç uyarısıdır.

Tablo/kılavuz/üretilen etiketlerin yeni profile toplu geçişi, blok tanımı
alanlarının toplu yenilemesi, ligatür bileşenlerine ayrı boya ve paketli
fontların dışında kalan yazı sistemleri teslim edilmiş gibi işaretlenmez.
Bunların mevcut davranışları, denetimleri ve açık ret sınırları matriste
yer alır. CAD-ADV-10'un motorları değerlendirme ve açık destek sözleşmesi
koşulu bunları otomatik dönüştürme yetkisi vermez. Web ürün çalışması
ADR0228 kapsamında ertelenmiş durumdadır.
