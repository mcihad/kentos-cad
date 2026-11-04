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
