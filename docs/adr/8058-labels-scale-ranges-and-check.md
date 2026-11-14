# ADR 8058: Etiketler — kaybolma, görünür ölçekler (1:N), Denetle ve her yerden ölçek

- **Durum:** uygulandı (14 Kasım 2026). Faz 3'ün ADIM 19'u (ADR 8057 §8) sürerken sahibin isteğiyle araya girdi;
  bitince ADIM 19'a dönülür.
- **Sahibin sözü (14 Kasım):** "Etiketlerle ilgili çok ciddi problemler var, zoom yaparken de bazen kayboluyor ve
  etiketlerin çok daha iyi hale getirilmesi gerekiyor, denetle ölçeği de çalışmıyor olabilir, ayrıca QGIS belli bir
  zoom seviyesinde görünme ve gizlenme ayarları var etiketlerin buna benzer bir yapı olmalıdır, bunları yaptıktan
  sonra diğer maddeye geçelim." · "Ölçek bir çok yerde işimize yarayabilir o yüzden ölçek her yerden erişilebilir
  olmalı."
- **Bağlam:** ADR 0212 (etiket motoru), 0533 §6 (`cad.labels.report`, Denetle), 0636 §7 (Denetle ölçeği), 0213 §3
  (görünümün ölçeğine göre kurulan stil), 0137 ve 8056 (ajan bağlantısı, MCP), 8057 §8 (eklenti işlevi etikette).
  Web ADR 0228'den beri duraklatılmış: aşağıdakiler masaüstünde, Python'da ve MCP'dedir, web geri dönüşte yetişir.
- **Örnek veri:** bütün gerçek pencere sınamaları yapay `taks-ornegi.kcad` ile (parseller 101/1–3, 102/1, beş yapı).

## §1. Yakınlaşırken kaybolma: dört neden, üç düzeltme

Keşif (dosya:satır) beş nedeni ayırdı; üçü hataydı ve düzeltildi, ikisi tasarımdır ve artık görünür:

1. **Eklenti işlevli etiket hiç görünmüyordu** (`eb63df1a2`, `4f56a681e`, `06cf82c37`, `e0e565a98`): etiketin çizim
   anahtarında metnin sürümü yoktu, eski yerleşim ve tuvalin önbelleği kalıyordu; Python hazır değilken gelen boş
   yanıtlar oturum boyu önbellekte duruyordu. `Spatial::texts_version()` anahtara girdi; yanıt yalnız süreç gerçekten
   yanıtladıysa saklanır. Gerçek pencerede `ext.….ada-parsel(ada, parsel)` etiketleri kayıttan ~3 sn sonra çıktı.
2. **Kaybolmasın 240 px'ten büyük yazıyı atıyordu** (`8089583bc`): adı kaybolmamak olan ayar, yakınlaşınca büyüyen
   her çizim yazısını siliyordu. Yalnız Gerçek boy 5–240 px sınırını tutar.
3. **Görünüm hareket ederken uzaklaşınca kenarlar boş kalıyordu** (`c2740dd4b`): hareket sırasında son yerleşim yalnız
   sorulan kutunun içinde ve ölçeğin 1,25 katı yakınında yeniden kullanılır; dışarı çıkınca yeniden yerleşir.
4. **Ölçek eşikleri** (tasarım): türlerin ve şablonun varsayılanları: nokta etiketi 2 px/m (≈1:1890'dan yakında),
   çizgi 1,6 px/m (≈1:2362), şablonun Ada etiketi en yakın 5 px/m (≈1:756'dan yakında gizlenir). Değiştirilmedi;
   pencere artık bunları 1:N gösterir (§3), kullanıcı neden gizlendiğini görür.
5. **En küçük nesne** (tasarım): alan etiketi ekranda 26 px'ten küçük nesneye yazılmaz (`min(w, h)` ölçüsü,
   `placing.rs`). Ayrıca "Çakışmasın" ile büyüyen etiket yer bulamayınca düşer. İkisi açık iş (§6).

## §2. Tek ölçek kuralı: `kentos_geometry_core::scale`

`a98165dba`, `43626594d`. Görünüm ölçeğini px/m tutar; paydası N = 1 / (px/m × 0,00026458 m), 96 dpi'lık bir ekran
pikselinin kâğıttaki boyu. Durum çubuğu, kamera, araçlar, etiketler ve ajanlar aynı işlevleri okur: `METRES_PER_PX`,
`denominator`, `px_per_metre`, `visible_at`, `PRECISION`.

`visible_at(n, en_uzak, en_yakın)` aralığı QGIS'in okuduğu gibi okur ("Minimum (hariç)", "Maksimum (dahil)"): en uzak
ölçeğin kendisinde gösterilmez (1:10 000 → 1:9 999'dan yakında), en yakın ölçeğin kendisinde gösterilir. Bir ölçek
uca `PRECISION` (göreli 1e-9) kadar yakınsa uçtadır: yazılan 1:2510 px/m'den geri 1:2510,0000000000005 okunur ve durum
çubuğundan 1:10 000'e alınan görünüm, ikisi nasıl hesaplanmış olursa olsun 1:10 000 ucundadır.

Etiketin sınıfı (`Class::shown_at`) bu kurala geçti; önce iki uç da gösteriliyordu, fark yalnız tam en uzak uçtaki
görünümdedir. **Stil kurallarının aralığı iki ucu da çizer** (`StyledBatch::in_scale`, web'in `inScale`'i); dokunulmadı.

## §3. Etiket penceresi: Görünür ölçekler 1:N ve Şimdiki

`43626594d`. Öncelik sekmesinin "Ölçek aralığı (px/m)" satırı "Görünür ölçekler" oldu: **En uzak 1:… [Şimdiki] En
yakın 1:… [Şimdiki]**. Saklama px/m kaldı (sözleşme ve dosya değişmedi); pencere 1:N gösterir ve okur ("1:10.000",
"10000", "10.000"). **Şimdiki** görünümün ölçeğini, görünüm aralığın içinde kalacak tam N olarak yazar: en uzak uç
kendisi gösterilmediği için görünümün N'inin bir üstü (1:435,4 → 1:436), en yakın uç N'i ya da altı. Yuvarlanmış N,
görünümü sınırın hemen dışında bırakabilir ve Şimdiki'ye basmak etiketleri gizlerdi.

Gerçek pencere: En uzak = Şimdiki (1:435), Kaydet; 1:587'de parsel etiketleri gizlendi, 1:322'de geri geldi.

## §4. Denetle

`10a2ab171`. Dört kusur düzeldi; önce/sonra gerçek pencerede görüntülendi:

1. **Penceredeki taslak sayılır:** Denetle çizimdeki etiketlemeyi sayıyordu; kaydedilmemiş şablonla "0 etiket
   yerleşti" diyordu (4 yerleşir). Pencere etiketlemesini yazmadan planlar (`layers_labels::plan`), rapor o katmanı
   taslağıyla sayar (`labels_report::Draft`, `label_store`'un `draft`'ı).
2. **Uzak görünüm kâğıt ölçeğinde hata vermez:** 1:21 481'den "çizim ölçeğinde (1:1000)" sayım yalnız "Pencere bu
   ölçekte 100 milyon pikselden büyük" diyordu. Pencere, görünümün motorun sayfasına sığan ortasını (`MOST_PIXELS`'in
   %98'i) sayar ve söyler: "ortasındaki 3422 × 2005 m'lik kısım sayıldı".
3. **Çizim yazıları engeldir:** rapor etiketleri hiç yazı yokmuş gibi yerleştiriyordu (`place_labels(.., &[], ..)`);
   ekran ve kâğıt yazıları önce yerleştirip etiketleri üstlerinden uzak tutar (ADR 0212 §3.5). Rapor artık
   `labels_shown_parts`'tan geçer: kâğıtta yazıların gerçek boyu, ekranda host'unki (`Draft::texts`, masaüstünün
   `label_size()`), host yoksa projenin kendi kuralı. 25 m'lik bir yazının altındaki parselin etiketi önce
   "yerleşti", şimdi "yerleşemedi" sayılır.
4. **Seçilen Denetle ölçeği hatırlanır** (oturum boyu, `GisUiState::labelling_report_at`).

`cad.labels.report`'un sözleşmesi değişmedi; ortak durumları yazı tutmaz ve yeşildir (kaynak testleriyle 43).
MCP ve Python için de rapor artık yazıları engel sayar.

## §5. Ölçek her yerden

- **Ajanlar, konsol, MCP, Python** (`53bd64ff2`): masaüstünün ajan bağlantısı ve Python konsolu öndeki çizim için
  `view` yanıtlar (`view_answer`): `scale` (1:N'nin N'i, durum çubuğundaki gibi), `pxPerMetre`, `window` (görünen
  kutu; döndürülmüşse dönük alanı saran), `center`, `twist` (derece, doğudan saat yönünün tersine) ve `plotScale`. Arka
  sekmedeki çizim `not_in_front`, açık olmayan `document_closed`. Python: `Document.view()` bir `View` verir;
  betikte açılan çizimin görünümü yoktur (None). MCP: `drawing.view`; sunucunun açtığı çizimde `no_view` ve çizim
  ölçeğinin yeri. Uygulama içi asistan aynı yolu kullanır (kompakt profilde de listelidir).
- **Katman stili › Kurallar** (`89e1c2ec4`): kuralın her 1:N ucunun yanında **Şimdiki ölçek** düğmesi. Kuralın
  aralığı iki ucu da çizdiği için en yakın uç görünümün N'i ya da altı, en uzak N'i ya da üstüdür (1:435,4 → 1:435 ve
  1:436). Gerçek pencere: Yapı katmanı, en uzak = Şimdiki (1:435); 1:587'de yapılar çizilmedi, 1:322'de çizildi.

## §6. Ertelenenler (planlandı, yapılmadı)

1. **Katmanın kendi ölçek aralığı** (QGIS'in katman görünürlüğü). Yeri "Katman çizimi" (`LayerCompose`, QGIS'in
   Görüntüleme sekmesinin karşılığı): `farthestScale` / `nearestScale` (1:N). Çizim: katmanın parçalarının
   `min_scale`/`max_scale`'i katman aralığıyla daraltılır, `in_scale` kare başında gizler (yeniden kurma yok); etiket:
   `label_parts`'a ölçekli katman yüklemi; seçim ve kenet: etkin görünümün ölçeğinde. `cad.layers.compose`
   sözleşmesi, ortak durumlar, Python tipleri ve katalog değişir; sahip istemedi ve Kurallar biçimi bugün ölçeğe göre
   gizleyebiliyor.
2. **İfadede görünüm ölçeği** (`$görünüm_ölçeği`): etiket metinleri belge değişince bir kez değerlendirilir; görünüm
   ölçeğine bağlı metin her ölçek adımında yeniden değerlendirme ister. Stil tarafında `needs.scale` yolu hazır.
3. En küçük nesne testinin `min(w, h)` ölçüsü ve "Çakışmasın" + büyüme ile düşen etiket (§1.5).
4. Web: etiket formu hâlâ px/m; web'in `cad.labels.report`'u yazıları engel saymaz (ADR 0228: web geri dönüşte).

## Uygularken verilen kararlar (planda yazılı değildi)

1. Şimdiki'nin yuvarlaması uç kuralına göre: etikette en uzak ⌊N⌋+1, en yakın ⌊N⌋; kuralda en yakın ⌊N⌋, en uzak ⌈N⌉.
2. Uç toleransı göreli 1e-9 (`scale::PRECISION`).
3. Denetle'nin sığmayan görünümü ortasından, sayfanın %98'iyle sayması.
4. Raporda yazı boyu: kâğıtta gerçek boy, ekranda host'un ayarı, komut yolunda projenin türüne göre kural.
5. `view` yanıtının alanları ve `not_in_front` / `document_closed` / `no_view` ayrımı.
6. Katman ölçek aralığının ertelenmesi (§6.1).
