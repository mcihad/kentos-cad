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
