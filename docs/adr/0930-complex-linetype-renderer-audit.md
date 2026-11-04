# 0930 — Karmaşık çizgi tipi: gerçek kayıp noktaları ve ortak yürütme

Tarih: 4 Kasım 2026. Kapsam: CAD-ADV-11. Durum: araştırma ve başlangıç
kabulü ve ilk çıktı koruması; uygulama tamamlanmadı. CAD-ADV-10'un
kapanışından sonraki iştir.

## Dış davranış sözleşmesi

- [AutoCAD: çizgi tipinde metin](https://help.autodesk.com/view/ACDLT/2026/ENU/?caas=caas%2Fdocumentation%2FACD%2F2014%2FENU%2Ffiles%2FGUID-FEDCE7EB-4919-43AE-A54E-F3A293DD60CA-htm.html):
  metin stilinin yüksekliği ve `S` birlikte değerlendirilir; `X/Y` ofseti
  metnin `S` katsayısından ayrı, çizgi tipi ölçeğindedir. `U`, `R` ve `A`
  okunur, teğet ve mutlak yönlerdir. İşaretler tam çizilir; son işaretin
  yalnız bir parçası kırpılıp normal sonuç sayılmaz.
- [AutoCAD: çizgi tipinde şekil](https://help.autodesk.com/view/ACDLT/2026/ENU/?caas=caas%2Fdocumentation%2FACD%2F2014%2FENU%2Ffiles%2FGUID-AF0613E6-5C8B-47F0-800C-8B2524BF2015-htm.html):
  kaynak şekil dosyası, şekil adı, ölçek, yön ve ofset ayrı değerlerdir.
  KentOS'ta bulunamayan kaynak için sessiz boşluk yerine açık bulgu gerekir.
- [PLINEGEN 2026](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-352844E4-FBEA-44E0-B8F3-41C4187AD9C7.htm):
  köşelerde yeniden başlayan ve köşeleri kesintisiz geçen desenler ayrı
  davranışlardır. Bu seçenek daralan/genişleyen parçalara kendiliğinden
  uygulanmış kabul edilmez; eski polyline'ın davranışı gizlice değiştirilmez.
- [LTSCALE](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FCIV3D%2F2014%2FENU%2FfilesACD%2FGUID-47B3793A-37BF-45FC-94CB-670433ADD366-htm.html)
  genel desen katsayısıdır. Nesnenin katsayısı, model birimi ve paftanın
  ölçeği aynı sayı değildir. KentOS'un mevcut paper/drawing ayrımı korunur.

## Kodda doğrulanan dar boğazlar

| Aşama | Somut durum | Sonuç |
| --- | --- | --- |
| `shared/contracts/src/line_types.rs` | `MAX_PATTERN_ELEMENTS = 12` | Plandaki 12'den fazla öğe daha renderere gelmeden reddedilir. |
| `shared/formats/src/lin.rs` | Aynı üst sınırı kullanır. | Yalnız renderer sabitini değiştirmek LIN kabulünü genişletmez. |
| `native/style/src/line_types.rs::dashes_of` | Yazı/şekil atlanır; normalize dash/gap sekizde kesilir. | `lossy` bilgisi vardır ama eksik geometri üretilir. |
| Aynı fonksiyonun sürekli kolu | Gap yoksa `None`; kompleks kayıp bilgisi sonuçla taşınmaz. | Yalnız `Some(Dashes).lossy` sorgusu tüm kaybı bulamaz. |
| `native/style/src/batches.rs::dash8` | Derlenmiş liste ikinci kez sekizde kesilir. | Kaynağı düzeltmek tek başına yetmez; kütüphane sembolleri de etkilenir. |
| `native/style/src/uniform.rs::dash_values` | Sabit iki vec4, toplam ve mürekkep oranı ilk sekizden. | Yeni desenin gerçek periyodu kaybolur. |
| `native/style/src/batches.rs::dash_period` | Aynı kesilmiş periyotla origin fazı katlanır. | GPU'yu düzeltip bu hesabı bırakmak uzak koordinatlarda yanlış faz üretir. |
| `native/style/src/cpu.rs::dash_cover` | `[f64; 8]` aynı uniform düzenini okur. | GPU değişikliği CPU baskı önizlemesini kendiliğinden düzeltmez. |
| `shaders/wgsl/styled/common.wgsl::dashCover` | Sekiz uniform parçası; stroke ve hatch ortak kullanır. | Çizgi tipi değişikliğinde tarama regresyonu da sınanmalı. |
| `native/style/src/program.rs` | Kök nesne, katman, BYLAYER/BYBLOCK parça ve renderere ait görünüşü çözer. | Yeni denetim ikinci, basitleştirilmiş bir görünüş çözümleyicisi olmamalı. |
| `apps/desktop/src/line_types_window.rs` | Satır altında “yazı ve şekil: boşluk” / ilk dört çift bilgisi. | Bu küçük bilgi, gerçek çizim ve baskı kabulünün yerine geçmez. |
| `native/plot/src/preflight.rs` | Alan/font/görünür kaynak korumaları var; çizgi deseni kaybı için eşdeğer koruma bulunamadı. | Yanlış desenli teslimin sessiz başarı vermesi kapatılmalı. |

## Gerçek arayüzde başlangıç kanıtı

Repo dışındaki bağımsız KCAD referans yazarıyla iki deney hazırlandı:
12 dash/gap kaynak öğeli `LONG12` ve `HW` metin işaretli `WATER`.
Karşılaştırma çizgileri uygulamanın desen üreticisinden alınmadı; ilkinde
signed uzunluklar üzerinde doğrudan yürünerek açık mavi ayrı doğrular,
ikincisinde beklenen tekrarlara bağımsız metinler yerleştirildi.

Güncel uygulamada dosya gerçekten açıldı ve Home ile sığdırıldı. Beyaz
kaynak çizginin periyodu altındaki tam 12 öğeli mavi referansla uyuşmuyor;
`WATER` çizgisinde `HW` işaretleri yok. Bu bir düzeltme sonrası kabulü
değildir; başlangıç hatasının kanıtıdır. Deneydeki 16 öğeli ilk belgeyi
mevcut sözleşme reddetti; bu, üst sınırın ayrıca ele alınması gerektiğini
doğruladı. Geçerli 12 öğeli karşılaştırma 54 nesneyle açıldı.

Dosyalar/ekran repo dışında:
`/tmp/kentos-cad-adv08-09-2026-11-03/line-types-before.{json,kcad}` ve
`line-types-before-gui.png`. Üretim rendererinden beklenen sonuç türetilmedi.

## Uygulama sırası ve değişmeyecek sınırlar

1. **Kayıp bulgusu:** gerçek görünüş çözümünün seçtiği kaynak desenin
   kimliği ve eksik yetenekleri kaybolmadan derleme sonucuna taşınmalı.
   Kullanılmayan tanım, görünmeyen parça veya yerine özel sembol çizilen
   nesne yüzünden yanlış baskı engeli üretilmemeli. Görünür kök/blok parçası,
   layer state/viewport override, filtre, izolasyon, kat/faz ve kırpma
   kontrolleri rendererin kendi sonuçlarına dayanmalı. Aynı bulgu GUI,
   sorgu/Python/MCP ve çıktı ön denetiminde kullanılmalı. Açık kayıp varsa
   PDF ve doğrudan CPU/atlas çıktı yolları da reddetmeli.
2. **Tam desen tarifi:** bütün signed öğeler, sıfır uzunluklu noktalar ve
   aynı kümülatif konumdaki işaretler korunmalı. Tam periyot ve prefix
   mesafeleri Float64; kaynak paper/drawing birimi, genel/nesne ölçeği
   ayrı. Sadece MAX_DASHES artırılıp “tamam” denmeyecek. Sözleşme/KCAD/LIN
   sınırı yükseltilecekse eski okuyucunun ne yapacağı sürümlü tanımlanmalı;
   bağımsız KCAD okuyucu/yazarı ve Python/MCP şemaları birlikte doğrulanmalı.
3. **GPU/CPU/PDF yürütme:** kısa desenlerin mevcut hızlı yolu korunurken
   uzun desenler için ölçülmüş değişken boylu desen buffer/atlas düzeni
   kullanılmalı. Geometriyi her zoom'da milyonlarca ayrı dash nesnesine
   bölmek yok. Buffer kimliği/offset/count, tam periyot, cache invalidation,
   iptal ve bellek sınırı birlikte tasarlanmalı. Origin-relative FP32
   geometriye geçmeden önce faz tam Float64 periyoda katlanmalı. CPU,
   PDF ve shader aynı tarifin periyodunu kullanmalı.
4. **Yazı/şekil:** CAD-ADV-10'un sabit font/glyph planı kullanılmalı;
   ikinci bir metin motoru açılmamalı. Stil yüksekliği, S ve X/Y, U/R/A,
   eğri teğeti ve mirror/nonuniform dönüşüm ayrı sınanmalı. Şekil kaynağı
   bulunmadığında boşluk ya da benzer başka glyph ile teslim edilmemeli.
5. **Son kabul:** 16'dan fazla dash/gap, Türkçe ve birleşik karakterli
   işaret, mevcut kaynaklı/kaynaksız şekil; doğru/yay/kapalı path; ters yön,
   köşe, kısa parça, büyük koordinat, nonuniform/mirror blok. 1:1/1:20/1:100
   gerçek pafta ve GUI zoom testi; bağımsız konum/periyot hesabı, ekran,
   CPU önizleme ve PDF raster/kontur karşılaştırması. Temaya bağlı UI rengi
   atanmış baskı rengini değiştirmemeli. Eksik kaynak ret, round-trip ve
   salt okunur plan sorgusu da kabulün parçası.

## İlk uygulama: eksik desenle sessiz teslimi engelle

Kaynak desen kaybı `LineTypes` → simpleLine → `StrokeStyle` → gerçek
`StyledBatch` zincirinde taşınıyor. Gap içermeyen kompleks tanımın
`None`/continuous kolu da kayıp bilgisini korur. Ön denetim başka bir
görünüş çözümleyicisi kurmaz; `MapDrawer`ın ölçek/kat/faz seçenekleriyle
hazırladığı kaynağı aynı stil derleyicisinden geçirir. Sıradan kısa/built-in
desenler ucuz kaynak kapısı sayesinde ikinci kez derlenmez.

`linetype_unsupported` hata bulgusu mevcut tam baskı ön denetiminde ve
`file.preflight.sheets` Python/MCP sorgusunda görülür. PDF ve CPU önizleme
reddeder; doğrudan vektör/CPU/atlas yolları da ortak çıktı korumasından
geçer. Kullanılmayan tanım, built-in nesne override'ı, desteklenen özel
sembol, gizli/donmuş katman, görünüm dışında kalan batch ve XCLIP ile
çıkarılmış blok parçası için deneyler var. Denetim belgeyi değiştirmez.

**Sınır:** Bu ilk koruma batch sınır kutusunun viewportun eksenlere paralel
alanıyla kesişmesini kullanır. Döndürülmüş/çokgen viewport kırpmasında
yalnız kırpma dışında kalan batch için gereğinden ihtiyatlı ret olabilir;
bu henüz tam kırpma kabulü değildir. Uzun desen ve yazı/şekil çizimi
henüz düzeltilmedi; açık kayıp artık teslim başarısı sayılmaz.

Gerçek GUI testinde çıktı üretim hatasının yalnız günlüğe yazılıp PDF
penceresinin kapandığı da görüldü. Host artık üretim/yazma hatasında aynı
KentOS çıktı seçeneklerini kapsam ve DPI ile yeniden açar; standart hata
bandı nedenini gösterir. Bu davranış hem paylaşılan UI hem desktop host
testiyle kapsandı. Ayrı bir özel hata bileşeni eklenmedi.

**Doğrulama:** plot 18 birim + 64 entegrasyon testi geçti (3 test bilinçli
ignored); style-core 36, native-style 34 + batches 2, MCP kabul filtresi 2,
Python testi 1, çıktı hatası için UI/desktop testleri 1'er geçti. Desktop
strict Clippy ve derleme, GPU test hedefinin derlenmesi geçti; bu sonuncusu
GPU piksel kabulü sayılmaz. Python uzantısı yeniden derlendi.

Gerçek uygulama binary SHA-256:
`8fd737d460090516a9ac5872bf1304c1250870aa36af407b3bc39a3f2f0eeb9f`.
1:50 paftada PDF kaydı fareyle denendi: hedef PDF oluşmadı, neden standart
pencerede kaldı. Tam denetim LONG12 ve WATER için iki ayrı hata gösterdi.
Aynı tanımlar korunup nesnelere desteklenen built-in desen atanmış bağımsız
olumlu kontrol dosyası GUI'den başarıyla PDF verdi (13.241 bayt). Ekranlar
repo dışında `linetype-guard-pdf-failure-visible-gui.png`,
`linetype-guard-preflight-gui.png`, `linetype-supported-pdf-gui.png`;
olumlu çıktı `linetype-supported-guard-gui.pdf`, yukarıdaki deney dizininde.

CAD-ADV-11 açık; ortak tam desen/GPU buffer ve yazı/şekil uygulaması sürüyor.

## İkinci uygulama: tam çizgi/boşluk listesi

`dashes_of`, batch decode, uniform toplamı, CPU coverage ve hatch'in uzak
origin periyodu artık sekizde kesmez. `uniform::normalized_dashes` tek
normalizasyon noktasıdır: tek sayılı liste tam iki kez yinelenir. Kısa
desenler eski sekiz uniform slotunu ve shader fonksiyonunu kullanmayı
sürdürür. Uzun desenler katman yüklenirken FP32 olarak aynı tarifin
tamamından hazırlanır; periyot/ink share ve origin fazı önce Float64
hesaplanır. PDF stroke listesinin tamamı korunur, hatch vektörleştirmesi
de aynı tam normalizasyonu kullanır.

Native GPU, group 1 binding 1'de read-only storage sayfaları kullanır;
stroke/hatch'in boş `flags.zw` alanları offset/count taşır. Aynı FP32
tarif aynı slotu paylaşır, 256 KiB dolunca yeni sayfa açılır. Sayfa seçimi
batch'tedir; bellek muhasebesine bu veri de katılır. Kamera hareketi yeni
dash geometrisi veya yeni tarif yüklemesi istemez; mevcut katman/id
önbelleği korunur. Özel sembolün 4096'yı aşan ham listesi derlemede açık
hata ile reddedilir, kesilip başarılı sayılmaz. Bu sınır performans
benchmark'ının veya bütün kötü niyetli iş yükü kabulünün yerine geçmez.

[WGSL storage address space](https://www.w3.org/TR/WGSL/#storage-address-space)
ve [WebGPU limitleri](https://www.w3.org/TR/webgpu/#dom-supported-limits-maxstoragebufferspershaderstage)
bu bağlama türünün dayanağıdır. Uygulamada ayrıca mevcut wgpu 27.0.1 ve
Iced 0.14 compositor kodu kontrol edildi: native default/downlevel
profilleri sırasıyla 8/4 storage binding ve iki group sağlar. Web ürününün
ADR 0228 kapsamındaki duraklaması korunur: ortak browser v10 kaynak ve
bağlamaları değiştirilmedi; Rust `shader::source()` yalnız native ekini
birleştirir. Naga hem ortak hem native modülü doğrular.

**Doğrulama:** gerçek Apple M5/Metal testinde 12 ve 16 öğeli tarif, 9
öğenin 18 olarak tam yinelenmesi; stroke ve hatch; pozitif/negatif faz;
0,5/1/2/8 px/m ölçeklerinde bağımsız aralık hesabıyla GPU ve CPU örnekleri
karşılaştırıldı. Her ölçek/nesne için görünür iç örnek şartı var. Aynı
kare ikinci kez aynı görüntü ve aynı resident bellek hesabını verdi;
bu sayaç tek başına upload ölçümü diye sunulmaz. Genel gerçek GPU
regresyonu da geçti: örnek çizim, AA, pan hassasiyeti, tarama, kalınlık,
kompozisyon/maske ve 3D sahne. Style 36 birim + batches 2; plot 18 birim
ve 64 entegrasyonun ardından yeni 1:1/1:20/1:100 tam vektör reçetesi testi
ve altı çizgi tipi entegrasyonu geçti. MCP sorgu testleri 2, yeniden
derlenmiş Python üzerinden text/unused/long alt durumları geçti.

Gerçek GUI binary SHA-256:
`e40258e257ec46b09e0b26f8bf45cd8ace80814b2c50349c13679d26f02bb68d`.
Başlangıç dosyası tekrar açıldı; beyaz LONG12 deseni mavi bağımsız tam
geometriyle artık eşleşti, küçük fare zoom adımlarıyla iki kez
karşılaştırıldı. 1:50 paftada CPU önizleme ve PDF kaydı denendi. Poppler
ile PDF sayfası ve 300 dpi detay ayrıca görüntülendi; içerik akışındaki
dash operatöründe bütün 12 değer bulundu. GUI farklı kaydet sonucu
bağımsız KCAD kaynakla **bayt bayt aynı**. Çizgi tipleri yöneticisinin
tam önizlemesi ve yeni sınır açıklaması da gerçek pencerede doğrulandı.

Kanıtlar aynı repo dışı dizinde: `long-pattern-model-after-gui.png`,
`long-pattern-zoom-gui.png`, `long-pattern-deeper-zoom-gui.png`,
`long-pattern-preview-gui.png`, `long-pattern-manager-gui.png`,
`long-pattern-pdf-detail.png`; GUI çıktısı `long-pattern-full12-gui.pdf`,
kayıt `line-types-long-roundtrip-gui.kcad`.

**Açık sınırlar:** adlandırılmış KCAD/LIN tanımı halen en fazla 12 kaynak
öğesi kabul eder; kaynak üst sınırı/sürüm yükseltmesi henüz yapılmadı.
16 öğeli ilk deney doğrudan native batch tarifidir; aşağıdaki ek GUI
deneyi KCAD içindeki özel stil tarifini de kullanır. Yazı/şekil çizimi,
nonuniform/mirror blok, yay/kapalı path/köşe matrisi, tam viewport
kırpması ve büyük iş yükü ölçümleri açık. Bu nedenle CAD-ADV-11 bitmedi.

### PDF başlangıç fazı ve ek GUI kabulü

İncelemede `stroke_paths`ın mevcut `dash_offset` değerini kullanmadığı
doğrulandı ve aynı çalışma içinde düzeltildi. `MapStroke.dashOffset`
isteğe bağlı, kâğıt mm cinsinden bir alandır; eski girdide yoksa 0,
eski sıfır fazlı serileştirme değişmez. Rust→TS bağlaması yeniden üretildi;
bu mekanik sözleşme eşlemesidir, web ürününe özellik eklenmedi.

PDF kalem önbelleği artık deseni **ve fazı** birlikte tutar; save/restore
ikisini birlikte geri getirir. Tam (tek sayılıysa iki tekrar) periyot ve
pozitif/negatif faz Float64 hesaplanıp PDF'ye gitmeden pozitif eşdeğerine
katlanır. Böylece [PDF 1.7'nin negatif faz belirsizliği](https://pdfa.org/pdf-differences-github-repository-is-now-public/)
görüntüleyiciye bırakılmaz. Anlam, [Adobe ISO 32000-1 §8.4.3.6](https://developer.adobe.com/document-services/docs/assets/35e4369068f86065372c18787171a17e/PDF_ISO_32000-1.pdf)
ile aynıdır: faz, desen başlangıcından tüketilmiş mesafedir.

Önbellek/restore birim testi ve gerçek native PDF akışında 0/+1,25/−1,25 mm
fazı denetleyen entegrasyon testi geçti. Yedi çizgi tipi entegrasyonu ve
ortak sheet PDF testleri (13 geçti, 1 ignored), strict desktop Clippy ve
derleme de geçti. Mevcut PDF golden özeti değişmedi.

Ek gerçek GUI binary SHA-256:
`c4f9af84d09ae6db7d415f5693afd0ec399698c61bdd9e200403eebaaab64e54`.
Bağımsız KCAD yazarıyla **özel stil** içinde ham 12/+1,25 mm ve ham
16/−1,25 mm desenleri, ayrı hesaplanmış mavi referans doğrularıyla
hazırlandı. Uygulamada gerçekten açılıp karşılaştırıldı ve 1:50 PDF
kaydedildi. Poppler tam sayfa ve 300 dpi detayda aynı hizalamayı verdi;
bağımsız pypdf kontrolü 12/16 değer ve sırasıyla 1,25/42,75 mm pozitif
PDF fazını doğruladı (ikinci periyot 44 mm).

Kanıtlar repo dışında `line-types-phase.kcad`, `make-phase-fixture.py`,
`long-pattern-phase-model-gui.png`, `long-pattern-phase-sheet-gui.png`,
`long-pattern-phase-gui.pdf`, `long-pattern-phase-detail.png`.
Bu deney adlandırılmış `settings.lineTypes`ın 12 öğe sınırını yükseltmez.

## Yazı/şekil adımı için yeniden kullanım incelemesi

- `shared/style-core/src/style/compile.rs::emit_markers_along` ve
  `place.rs::place_along` mevcut mesafe/teğet altyapısıdır. Fakat mevcut
  markerLine, dönen her metni otomatik okunur tutar ve 30° üstü köşelerin
  çevresindeki bazı işaretleri atlar. CAD `R` ile `U` aynı davranış
  değildir; bu katmanı doğrudan ekleyip bütün LIN metinlerini desteklendi
  saymak yanlış olur. `A` yönü ile `X/Y` yerel ötelemesi ayrıca yürütülmeli.
- Aynı placement döngüsünde `MAX_MARKERS_PER_PATH` sınırı vardır. CAD
  tekrar sayısı bu sınıra geldiğinde görünmeyen işaretler için sessiz
  başarı verilemez; kaynak bütçesi, görünüm culling'i ve çıktı hata/iptali
  birlikte tasarlanmalı. Mevcut GIS markerLine davranışı değiştirilmemeli.
- `shared/contracts/src/annotation.rs::TextStyleDef` zaten font,
  yükseklik, genişlik katsayısı, eğiklik ve özgün `font_file` kaynağını
  tutar. `formats/src/dxf/styles.rs::text_style` bulunamayan font için
  proje fontunu seçip özgün dosya adını korur. LIN `PatternText.style`
  adı bu kaynakla ve importtaki yeniden adlandırmalarla eşleştirilmeli;
  yalnız seçilmiş `DrawingFont`u bulmak özgün kaynağın bulunduğunu kanıtlamaz.
- `apps/desktop/src/style/images.rs::outline` mevcut genel stil metnini
  Iced Advanced shaping ile üretir. `native/style/src/picture.rs::TextOutline`
  yalnız size/advance/outline taşır. Bu yol CAD-ADV-10'un dondurulmuş
  glyph/font planının karşılığı kabul edilmemeli; CAD işaretleri mevcut
  ortak CAD metin motorunun planını GPU/CPU/PDF'ye aynı biçimde vermeli.
- SHX shape adı/dosyası `PatternShape` içinde saklanır; bu kayıt çözülmüş
  bir shape kaynağı değildir. Kaynak gerçekten çözülemeden genel bir
  marker/harf veya boşluk konup mevcut çıktı koruması kaldırılmamalı.

**LIN sınırı hakkında kaynak düzeltmesi:**
[AutoCAD 2026 basit çizgi tipi belgesi](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FACDLT%2F2014%2FENU%2Ffiles%2FGUID-EF1DF0A9-2088-487C-8085-16FEE6425405-htm.html)
12 dash uzunluğu ve 80 karakterlik LIN satırını belirtir. Rendererimizin
sekizde kesmesi bu kurala uygun değildi ve kapatıldı. LIN'deki 12 sınırı,
tek başına keyfî bir renderer sınırı diye kaldırılmamalı. Planın daha
uzun tarif hedefi özel KentOS stilinde gerçek GUI/PDF ile sınandı.
Adlandırılmış yerel tanım ayrıca genişletilecekse KCAD sürümü, eski
okuyucunun açık reddi ve AutoCAD uyumlu LIN/DWG/DXF teslim politikası
birlikte tanımlanmalı; bu incelemede dosya şeması değiştirilmedi.


## Viewport birimi ve gerçek pafta ekranı düzeltmesi (4 Kasım 2026)

48 durumlu bağımsız çıktı testi, proje ölçeği 1:50 iken `drawing` deseninin
1:1 viewporta 50 kat küçük gitmesini önce kırmızı testle doğruladı.
`LineTypes::at_scale` artık çizim birimini **derlemenin gerçek viewport
ölçeğinde** kâğıt mm'ye çevirir. Native PDF/CPU ve masaüstü `SheetMaps`
aynı yolu kullanır; model görünüşünün kamera zoom'u bu ayarı değiştirmez.
Kâğıt desenleri yine sabit kâğıt mm'dir. mm/cm/m/ft × paper/drawing ×
1 ve 6 global/nesne çarpanı × 1:1/1:20/1:100 matrisi geçti. Masaüstünün
asıl stil önbelleğinde iki desen birimi, üç viewport ölçeği, aynı ölçeğin
önbellek tekrar kullanımı ve belgenin değişmemesi de ayrıca sınandı.

Gerçek GUI karşılaştırmasında ikinci bir hata bulundu: Iced 0.14
`iced_wgpu::geometry::dashed` yalnız desen sınırlarında ilerlediğinden
son yarım çizgiyi atlıyordu; bizim pafta `draw_paths` çağrısı da mesafe
fazını 0'a indiriyordu. Model GPU'su ve doğru PDF bu pafta ekranı
hatasının kanıtı değildi. **Önceki faz kabulü, pafta ekranının bütün faz
ve son parça davranışının tamamlandığı anlamında okunmamalı.**

Pafta artık tam mesafe fazıyla görünür çizgi parçalarını hazırlar. Kırpılan
başlangıcın tüketilmiş mesafesi korunur; düz çizginin son yarım deseni,
polyline köşesindeki devam ve kapalı halkanın dikiş birleşimi kaybolmaz.
Hesap, ortak Float64 kamera ve ortak Float64 kırpma ile yürür; yalnız
son, görünür noktalar Iced Float32'ye çevrilir. Bir milyar piksel uzaktaki
kesirli başlangıç için faz testi de vardır. Görünmeyen periyotlar tek
aritmetik adımla atlanır. Noktalı, dört pikselden sık veya 100.000 görünür
parçayı aşan tarif bütün haritayla mevcut CPU yoluna geçer; kısmi vektör
çizimi veya iki kez saydamlık birleştirmesi yapılmaz. Bu sınır bir hız
ölçümü değildir; büyük pafta performans matrisi ayrıca açıktır.

Doğrulama: plot 18 birim + 66 entegrasyon testi geçti (3 ignored);
masaüstü pafta 18 test geçti (7 isteğe bağlı GUI senaryosu ignored).
Strict masaüstü Clippy ve masaüstü derleme geçti. 48 durum yukarıdaki
entegrasyon testlerinden birinin iç matrisidir, 48 ayrı test diye sayılmaz.

Gerçek fare/klavye kabulü, binary SHA-256
`ed02fdb2d5476b4c1299f7d43149a3bd02c212580f68f78c2c163fd20bd90319`:

- Bağımsız KCAD yazarıyla 100/50/200/50/300/50 mm çizim deseni ve ayrı
  mavi referans doğruları oluşturulup 1:20 ve 1:100 paftalarda açıldı.
  Ölçek, zoom ve son yarım çizgi karşılaştırıldı.
- Gerçek GUI'den iki sayfalı PDF kaydedildi. pypdf'de her sayfanın altı
  dash değeri bağımsız dünya uzunluğu/viewport hesabına eşitti; Poppler
  tam sayfa çıktısı da incelendi.
- 12/+1,25 mm ve 16/−1,25 mm özel stil örnekleri 1:50 paftada yakından,
  1:20'de pafta çerçevesi içinde kırpılarak mavi referansla karşılaştırıldı.
  Başlangıç fazı ve son parça artık pafta ekranında da uyuşuyor.

Kanıtlar repo dışında `/tmp/kentos-cad-adv08-09-2026-11-03/` altında:
`pattern-unit-before-gui.png`, `pattern-unit-20-final-gui.png`,
`pattern-unit-100-final-gui.png`, `pattern-unit-both-gui.pdf`,
`pattern-phase-sheet-detail-final-gui.png`,
`pattern-phase-20-clipped-final-gui.png`, test günlükleri ve bağımsız
`make-unit-fixture.py`. Yazı/SHX işaretleri ve önceki açık matris bu
çalışmayla tamamlanmış sayılmadı.


## Uzak kümülatif path fazı (4 Kasım 2026)

İkinci gerçek GPU incelemesi, yakındaki bir path parçasının `s0` birikmiş
mesafesi çok büyükken Float32'nin kesirli fazı kaybettiğini kanıtladı.
Bağımsız .75 m periyot / 1.000.000.000,125 m başlangıç deneyinde Apple M5
Metal testi önce başarısızdı. Native CPU/export `StyledLayer::values` zaten
hi+lo toplamını kullanıyordu; GPU vertex girişi yalnız yüksek `mt.x`i
çarpıyordu. Çok büyük pozitif/negatif `dash_offset` da uniforma Float32
olarak gitmeden önce katlanmalıydı; ek kırmızı test bunu kapsadı.

Native GPU upload kopyasında **world-unit stroke** mesafesi, kaynak hi+lo
Float64 toplamından tam periyoda katlanır. Kaynak `StyledLayer`, belgenin
geometrisi ve PDF'nin tam mesafesi değiştirilmez. Ekran-piksel birimindeki
strokelerin mesafesine aynı metre modülü uygulanmaz. Kaynak fazı bir
periyottan küçük ve low stream hazırsa yeni kopya ayrılmaz; gereken
kopya yalnız katmanın upload'ında hazırlanır, kamera hareketinde değil.
Çizgi ve hatch'in kaynak `dash_offset`i de tam normalize periyotla Float64
katlanıp uniforma yazılır. Tek sayılı tarifin ikinci tekrarı hesaba katılır.

Doğrulama:

- Upload testi world/Px/alan ayrımını, diğer koordinatların ve kaynak
  high/low dizilerin değişmemesini, küçük fazda ödünç buffer yolunu sınar.
- Gerçek GPU/CPU testi uzak birikmiş mesafe ve 0 / büyük pozitif / büyük
  negatif kaynak fazını bağımsız yerel mesafe hesabıyla karşılaştırır.
  Apple M5 Metal'de geçti. Tam gerçek GPU regresyonu 5 geçti, 2 ölçüm
  testi ignored; render birim testleri 19 geçti.
- Native style 37 birim + 41 entegrasyon testi geçti (3 ignored).
  Strict masaüstü Clippy ve yeni derleme geçti. Native plot regresyonu da
  yeniden çalıştırıldı: 18 birim + 66 entegrasyon geçti (3 ignored).
- Önceki viewport düzeltmesinden sonra Python SDK yeniden derlendi;
  `test_sheet_line_types.py` içindeki tek testin üç alt durumu geçti.

Gerçek GUI için bağımsız KCAD yazarı 100 uzak yatay kenar ve sonra yerel
bir son parça oluşturdu. Decimal ile hesaplanan birikmiş mesafe
**1.005.000.999,625 m**, .75 m periyottaki faz **.625 m**. Geometrinin
koordinatları ±5.000.000 m içinde; büyük değer path'in toplam mesafesidir.
Kaynak polyline ve ayrı mavi referans 6,5 × 3 m adlı görünümde açıldı.
Önceki binary'de kaynak çizgi görünmüyordu; yeni binary'de 21 çizgi
parçasının sınırları mavi referanstan en fazla 1 piksel farklı. Fareyle
zoom tekrarında da karşılaştırıldı.

Yeni gerçek GUI binary SHA-256:
`935a2dc8aaf396cb7f1461373c108d62f735196d77f5ad7cd9d67cb4636bc413`.
Repo dışı kanıtlar: `make-remote-phase-fixture.py`,
`line-types-remote-phase.kcad`, `remote-phase-before-gui.png`,
`remote-phase-after-gui.png`, `remote-phase-zoom-after-gui.png` ve
`remote-phase-*.log` (aynı `/tmp/kentos-cad-adv08-09-2026-11-03/` dizini).

Bu düzeltme, tek bir çok uzun GPU segmentinin yerel interpolasyonuna veya
ekran-piksel birimindeki desenin dünya-mesafe fazına genel çözüm iddiası
olmaz. Yazı/SHX işaretleri, affine blok ve önceki açık kabul matrisi sürüyor.


## Karmaşık DXF kaynakları ve bağımsız kodek kontrolü (4 Kasım 2026)

Yalnız LTYPE'ın kullandığı STYLE kayıtları, TEXT/DIMENSION bulunmadığı için
önceden içe alma listesinden düşüyordu. Okuyucu artık bu referansları da
izler. Adsız SHX STYLE kayıtları adla değil handle ile ayrılır; farklı dosyalar
birbirini ezmez. Bulunamayan handle kaynakta tutulur, boş ada çevrilmez.
Sabit STYLE yüksekliğinde `S` boyutsuz çarpandır; değişken yükseklikte `S`
çizim uzunluğudur. X/Y her iki durumda çizim biriminden dönüştürülür.

Yazıcı karmaşık metni her zaman Standard'a bağlamak yerine gerçek STYLE
handle'ını ileri referansla yazar. Önceki handle dağılımı ve karmaşık metin
olmayan altın dosyaların baytları korunur. Stil bulunamazsa Standard'a
geçildiği raporda açıkça belirtilir. U/dik okunma 74 bit 8 ile korunur;
özellik kullanılan DXF'nin başlığı AC1027/R2013 olur, diğer dosyalar AC1021
kalır. SHX dışa yazımı bu dilimde tamamlanmadı; mevcut kayıp raporu sürer.

Açı kaynakları çelişkilidir: Autodesk'in [LTYPE sayfası](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-DXF/files/GUID-F57A316C-94A2-416C-8280-191E34B182AC.htm)
radyan derken [genel DXF grup kodu kuralı](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-DXF/files/GUID-3F0380A5-1C15-464D-BC66-2C5F094BCFB9.htm)
50–58 için DXF'de derece, uygulama API'sinde radyan ayrımını yapar.
[ezdxf'nin gerçek LIN derleyicisi](https://github.com/mozman/ezdxf/blob/master/src/ezdxf/tools/complex_ltype.py)
dereceyi grup 50'ye olduğu gibi taşır. Sabitlenmiş OpenCADCodec ff75fdd'nin
`src/io/dxf/writer/section_writer.rs::write_ltype_entry` kodu da iç radyanı
DXF derecesine çevirir ve U bitini R2013'e bağlar. Bu yüzden dosya açısı
derece kalır; kendi yazıcı-okuyucumuz arasında iki yanlış dönüşümle geçen
bir round-trip kabul kanıtı sayılmaz.

Doğrulama:

- Elle yazılmış `fixtures/formats/v1/complex-ltype-resources.dxf` derece
  açıları, ileri STYLE referansları, iki adsız SHX kaynağı, sabit/değişken
  yükseklik ve hiç TEXT içermeyen çizimi kapsar.
- Yazma testi gerçek stil handle'ını, ham DXF'de 90 dereceyi, değişmeyen
  kaynak girdiyi, kayıp stil raporunu ve U için sürüm yükseltmesini sınar.
- Bağımsız OpenCADCodec DXF ayrıştırıcısı 90 dereceyi iç PI/2 olarak okur.
  AC1027/AC1032 ikili DWG üzerinden desenler ve stil kaynakları yeniden
  karşılaştırıldı; eğiklikte derece/radyan dönüşümünün Float64 toleransı
  dışında değerler aynıdır. Bu gerçek AutoCAD testi değildir.
- Formats: 124 okuma/diğer entegrasyon + 52 DXF yazma testi geçti;
  9 mevcut ignored. DWG: tüm 6 entegrasyon testi geçti. Strict masaüstü
  Clippy ve yeni masaüstü derlemesi geçti. Python SDK yeniden derlendi,
  `test_sheet_line_types.py` üç alt durumu geçti.
- Yeni binary SHA-256 `9536fddd9a3471e1ba4f7deb1a5740b2750540c9570499685a2168c754fd282e`.
  Gerçek GUI'de DXF Aç ile açıldı; Yazı stilleri listesindeki Utility Text ve
  Variable Text seçildi, Barlow/Arimo kaynak dosyaları, sabit/boş yükseklik,
  12 derece eğiklik ve 0.8 genişlik görüldü. Kaydetmeden kapatıldı.
  `complex-resource-{fixed,variable}-style-gui.png` ve `complex-*.log`
  repo dışı önceki kanıt dizininde. Bu kontrol desen yazısını render etme
  kabulü değildir; CAD-ADV-11 hâlâ kısmen tamamdır.


## Paketli fontla yazılı çizgi deseni (4 Kasım 2026)

`native/style/line_types/text.rs`, kaynak yazı stilini ve çizim birimini
çözerek CAD-ADV-10'un ortak `shaping::placed` glyph planını bir kez hazırlar.
OS fontu veya Iced metin ölçümü kullanılmaz. Normalleştirilmiş konturlar,
0.0001 em eğri toleransıyla, nonzero dolgu kuralı üzerinden delikleri ve
ayrık parçaları korur. Stil genişliği, eğiklik ve font yüzü bu plandadır.
Kaynakta bulunmayan stil/SHX/font sessiz ikame edilmez; mevcut teslim
engeli sürer. Kaynak dosyanın font adı ve metni değişmez.

`style-core/style/cad_marks.rs` aynı planı gerçek path boyunca Float64
yerleştirir. Çizgi/boşluk toplamı tekrar mesafesidir; yazının kendi boyu
periyoda eklenmez. Köşede işaret atlanmaz, kapalı dikişte faz sıfırlanmaz.
R/A/U ayrı politikadır; X/Y çizginin eksenindedir, S ile çarpılmaz.
Dönme ekseni fontun gerçek nominal cap yüksekliğinin yarısıdır; font
yüksekliği ise mevcut KentOS model/em sözleşmesinde kalır. Bütün ithal
CAD yüksekliklerinin cap-height dönüşümü bu çalışmayla yapılmış sayılmaz.
Genel/nesne ölçeği fiziksel yerleşime birer kez uygulanır; kontur planı
tekrar ölçeklenmez. Sabit STYLE yüksekliği S'yi boyutsuz çarpan alır.

Yerleştirilmiş harfler GPU, CPU ve PDF'nin mevcut dolu geometri hattını
kullanır; ayrı backend yazı yerleşimleri oluşmaz. PDF'de bu çizgi tipi
harfleri vektör konturdur, bağımsız/seçilebilir TEXT nesnesi değildir.
Kaynak tarif KCAD içinde korunur; geçici konturlar belgeye yazılmaz.
Dolgu görünüşü kapatılınca çizginin harfi kaybolmaz; stroke görünürlüğü
uygulanır. Kaynak rengi, saydamlık ve çizim bandı mevcut yolu izler.

İş yükü kısmi başarılı bir önekle sınırlanmaz: 512 UTF-8 bayt/tek satır
kaynak metni, derlenmiş tarifte 32.768 kontur noktası, path'te 2.000
tekrar/500.000 nokta ve katmanda toplam 2.000.000 kontur noktası sınırı
vardır. Katman bütçesi tam path için atomik ayrılır; sığmayan path bütçe
harcamaz ve gerçek stroke'a kayıp bulgusu taşır. PDF/önizleme eksik teslimi
reddeder. Bu sınırlar bir performans kabulü veya görünür alana göre işaret
üretiminin tamamlanması değildir. Dalga/bulanıklıkla birlikte özel karmaşık
metin kaynağı da desteklenmiş sayılmaz; açık bulgu bırakır.

Doğrulama:

- Ortak yerleştirme testleri köşe, kapalı dikiş, ters yönlü R/A/U,
  nominal cap ekseni, X/Y, geçersiz faz, aşırı tekrar, görünüş filtresi
  ve toplam katman bütçesinin atomikliğini kapsar. Style-core 40 birim
  testi geçti.
- Native stil: 39 birim + 41 entegrasyon geçti, 3 mevcut ignored.
  O harfinin deliği, sabit STYLE/S, mm ve 1:20/1:50/1:100 ile genel ×
  nesne ölçeği fiziksel bağımsız beklentilerle sınandı. Eksik stil/font
  kaynakları bulgu üretir.
- Native plot genel regresyonu: 18 birim + 66 entegrasyon geçti, 3 ignored.
  Son bütçe değişikliğinden sonra çizgi tipi alt kümesinin 8 testi yeniden
  geçti. Buna görünür/katman/override/blok/boş tanım ayrımı ve çok yoğun
  yazıda salt okunur çıktı reddi dahildir. Strict masaüstü Clippy ve
  yeni uygulama derlemesi geçti.
- Style-core'un dondurulmuş JSON entegrasyonlarında 7 geçti, 2 başarısız:
  `one_symbol_on_one_object_as_frozen`, `whole_layers_as_frozen`.
  Değişikliksiz HEAD koduyla ayrı çalıştırmada aynı iki test, bayt olarak
  aynı beklenen/gerçek Float64 koordinat farklarını verdi. Altın dosyalar
  yenilenmedi; bunlar bu dilimin yeni hata veya başarılı testi sayılmadı.
- MCP eksik kaynak denetimi geçti. SDK son kodla tekrar derlendi;
  `test_sheet_line_types.py` içindeki 4 alt durum, plan/execute eşitliği,
  desteklenen metinde hazır çıktı, eksik stilde ret ve değişmeyen belgeyi
  doğruladı.
- Gerçek GUI'de önce HW çizgisi zoom ile, 1:20/1:50 paftada ve GUI'den
  kaydedilmiş iki sayfalı PDF'de incelendi. 1:50'deki ayrı Unicode-v1
  Barlow referanslarının dört kelimesiyle Poppler 600 dpi piksel sınır
  karşılaştırmasında x ve genişlik/yükseklik farkı 0 piksel. PDF'de iki
  sayfada da bitmap XObject yok. 1:20'de kâğıt desen aynı kâğıt boyunda,
  sabit model referansı farklı ölçekte görünür; bunlar karıştırılmaz.
- 73 nesneli ayrı dosyada elle hesaplanmış R0, R45, ters A0/U0 ve köşe
  referansları açıldı; zoom ve Kaydır/Esc gerçek fareyle kullanıldı.
  GUI KCAD Farklı kaydet sonucu, bağımsız yazılmış 14.534 baytlık kaynakla
  bayt bayt aynı; kaydedilen dosya GUI'den yeniden açıldı.

Son GUI binary SHA-256:
`740acd6faf7b8577cb6c40a49556e8344748ac76d56ce6da55ff13ace63ee772`.
Son başlatmada önceki QA DXF'sinin kurtarma kopyası için **Sonra** seçildi;
kopya silinmedi, yön testi dosyası tekrar zoom ile kontrol edildi.
Kanıtlar önceki repo dışı dizinde: `make-text-fixture.py`,
`make-text-directions-fixture.py`, `line-types-text*.kcad`,
`complex-text-*-gui.png`, `complex-text-both-gui.pdf`,
`complex-text-pdf-{page2,detail}.png`, `complex-text-pdf-pixel-check.json`,
`complex-text-*.log`. Ekranlar repoya eklenmedi.

**Açık:** gerçek SHX kaynağının yüklenmesi/çizimi ve dışa yazımı, affine
blok/variable-width tam görünüşü, yay/dikiş/yoğun sahne performans matrisi,
tam viewport clip denetimi ve önceki açık hassasiyet sınırları. Bu dilim
CAD-ADV-11'in tamamlanması değildir; `[~]` korunur.


## Geniş çoklu çizginin gerçek desenli bandı (4 Kasım 2026)

`native/style/program.rs` kök ve blok içindeki geniş çoklu çizginin
`leader_symbols_of` kolunda adlandırılmış tarifi uygulamıyordu. Şeklin
`MIXED` kaydı yalnız genişliksiz kenarları çizgi, geniş kenarları tam dolu
bant yaptığından, yalnız çizgi sembolüne dash eklemek de yeterli değildi.
Aynı adlandırılmış tarif atlaması kılavuz ve tablo çizgilerinde düzeltildi.

- Ortak `style-core/style/cad_band.rs` gerçek genişlikli yolu doldurmadan
  önce mürekkep aralıklarına ayırır. Yaylar kiriş uzunluğuyla değil
  `r × |sweep|` ile kesilir; alt yayın bulge değeri açıdan, uç genişlikleri
  özgün kenarın parametresinden gelir. Kaynak çoklu çizgi değiştirilmez.
- Mürekkep köşeyi geçerse aynı alt yolun parçası kalır; mevcut bant
  yordamının gönyesi korunur. Kapalı dikişte bitiş ve başlangıç mürekkebi
  birleştirilir. Sıfır boşluk sahte kapak üretmez. Genişliksiz kenarlar
  bölünmüş mürekkep aralıklarında mevcut çizgi kalınlığını kullanır.
- Renk, saydamlık, çizim bandı ve çizgi görünürlüğü mevcut sink üzerinden
  gider. Alan dolgusu kapatıldığında geometrik çizgi mürekkebi kaybolmaz.
  Sürekli çizginin eski bant yolu korunur; isteğe bağlı katman alan dolgusu
  geniş çizginin içine sızmaz. Metin işaretleri mevcut ortak derleyiciden
  tam merkez yolu boyunca yerleştirilir.
- Bir yol 20.000 desen geçişi/kenar ve 500.000 kontur noktasıyla sınırlı;
  katman bütçesiyle birlikte aşım açık `linetype_unsupported` bulgusudur.
  Nokta öğeli geniş bant henüz desteklenmiyor ve sessiz sürekli bant
  olarak teslim edilmiyor. Ön denetimin hızlı kapısı geniş yolların bu
  çalışma anı hatasını atlamaz. Özel katman efektli semboller bu dar
  geometrik bant kolunun kapsamı değildir.
- [AutoCAD polyline belgesi](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-392BF13C-D9E7-47A8-8E07-435296332279.htm)
  geometrik genişlik ve lineweight ayrımını, köşe üzerinden desen
  üretimini açıklıyor. Bu dilim KentOS'un mevcut kesintisiz faz politikasını
  uygular; yeni bir `PLINEGEN` özelliğinin tamamlandığı iddia edilmiyor.

Doğrulama:

- Ortak çekirdekte 44 birim testi; native stil/plot paketlerinde 172 test
  geçti, 6 mevcut test atlandı. Yeni bağımsız alan integrali testi
  `w(x)=1+x` genişliğini 1:1/1:20/1:100, model/plot ve dolgu açık/kapalı
  karşılaştırır. Analitik yarım daire kesimi, genişlik interpolasyonu,
  köşe/dikiş ve aşırı iş yükü ayrıca sınanır. İç içe döndürülmüş,
  ölçeklenmiş ve aynalı blok kökteki aynı geometriyle eşleşir; bu tam
  nonuniform/affine kabulü değildir.
- Geçerli kısa/uzun/yazılı/built-in geniş desen PDF üretir; eksik kaynak,
  SHX, noktalı bant ve yoğun tarif reddedilir; belge değişmez. Yeniden
  derlenen Python SDK'sında aynı ön denetimin plan/execute sonuçları altı
  senaryoda eşit, kaynak baytları ve çıktı klasörü değişmez. Masaüstü
  strict Clippy ve derleme geçti.
- Gerçek GUI'de aynı bağımsız 70 nesneli dosya eski ve yeni binary ile
  açıldı: eski kesintisiz beyaz bant, yeni kesik ve daralan/genişleyen
  parçalar. Yakınlaştırma, 1:20 ve 1:50 pafta, iki sayfalı GUI PDF'si
  incelendi. PDF'de bitmap yok; içerik yollarının doğrudan ölçümünde iki
  paftada da 5 mm mürekkep / 8 mm periyot, en büyük fark 0,000015 mm.
  Mavi geometrik referans 1:50'de eşleşir; sabit model geometrisi olduğu
  için 1:20'de kâğıt desenine göre 2,5 kat büyümesi beklenir.
- Son GUI binary SHA-256:
  `334cd67e60cf46fd5541ec92bfc07ab4ed7a671317d1c860d9a3a8e9b25af848`.
  `wide-pattern-{before,after,detail,final,sheet20,sheet50}-gui.png`,
  `wide-pattern-both-gui.pdf`, `wide-pattern-pdf-dimensions.json` ve test
  günlükleri `/tmp/kentos-cad-adv08-09-2026-11-03` içinde; repoya alınmaz.
  Son pencere tekrarından önce benim yapmadığım bir viewport düzenlemesi
  görüldü; ayrı `wide-pattern-preserved-window-change.kcad` olarak korundu,
  son binary temiz kaynakla yeniden açılarak kontrol edildi.

SHX kaynağını gerçekten çizme, tam affine blok, noktalı geniş bant,
PLINEGEN sözleşmesi, metin işaretinin analitik yay fazı ve geniş veri
performansı açık. CAD-ADV-11 tamamlanmadı.


## Çizgi tipi yöneticisinin gerçek önizlemesi (4 Kasım 2026)

[AutoCAD Linetype Manager](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-19A63E71-A551-4767-88FA-A6F54034C9DE.htm)
listede görünüş örneği, seçili tanım ve ayrıntıları birlikte gösterir.
KentOS yöneticisinin eski özel Canvas'ı yalnız dash/gap çiziyor, her
kompleks tanım için desteklenmiyor diyordu; artık desteklenen yazılı
desenle çelişiyordu.

`apps/desktop/src/line_types_window.rs` mevcut KentOS `Styles.thumbs`
shader bileşenini kullanır: kaynak `LineTypes.apply` → ortak stil
derleyicisi → sahnenin GPU çizim hattı. Ayrı bir harf veya faz hesabı yok.
Seçili desteklenen desenin büyük örneği gösterilir. Kaynak konturları
pencere açılırken veya `cad.linetypes.edit` sonrasında hazırlanır;
farenin her hareketinde yeniden font çözümleme yapılmaz. Çizgi tipi
örneği saç teliyle çizilir; çok küçük drawing-unit desende gösterim
kalınlığı örneği doldurmaz. Projedeki lineweight değiştirilmez. Liste yüksekliği öğe sayısına göre
küçülür; uzun listeler mevcut kaydırma bileşenini kullanır.

Kaynak gerçekten çizilemiyorsa listede standart uyarı ikonu ve “Kaynak
sorunu”, seçimde tam kaynak dosyası/şekil açıklaması vardır. Eksik şeklin
yalnız çizgi parçaları başarılı bir tam örnek gibi gösterilmez. Genel
yazı/şekil desteklenmiyor iddiası kaldırılır. Veri ve komut davranışı
korunur; yeni Python/MCP komutu eklenmez.

- Desktop birim testi gerçek `preview::preview` sonucunda yazının fill
  batch'ini, 13 mm periyodu, saç teli ve missing.shx kayıp bilgisini
  doğrular. Son Clippy ve derleme geçti.
- Gerçek GUI'de eski yanlış uyarı kaydedildi, yeni listede HW harfleri ve
  seçili büyük örnek görüldü. LIN'den `QA_MISSING_SHX` yüklenince liste
  güncellendi; seçimde `missing.shx` ve FENCE açıklaması doğrulandı.
  GUI'den farklı kaydedilen `line-types-preview.kcad` yeni binary ile
  yeniden açıldı; hem yazılı örnek hem kaynak uyarısı yeniden kontrol edildi.
- Son binary SHA-256:
  `c4ff9d7db8aa0e9638e2173fa0f4cd909eb4ec2120bc91fac62cc127ffd13065`.
  `linetype-preview-before-gui.png`, `linetype-preview-{text,source}-compact-gui.png` ve
  `linetype-preview-{tests-final,clippy-final,desktop-compact}.log` aynı
  repo dışı kanıt klasöründe. Dar pencere için gerçek köşe sürükleme
  denemesi araçta `windowNotFoundAtPosition` verdi; dar pencere kabulü
  yapılmış sayılmaz.
