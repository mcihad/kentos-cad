# ADR 0162: Bitişik alan ve çakışma denetimi

- **Durum:** kabul edildi (2026-10-02). `HYB-08`. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** TODOS.md `HYB-08` (ilgili: `HYB-06`, `GIS-04`), ADR 0065 (alan işlemleri), ADR 0143 (çok parçalı alan), ADR 0151 (Toplu alan), ADR 0160 (Topolojik düzenleme), ADR 0161 (İzle), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ArcGIS Pro Create adjoining polygons (Autocomplete Polygon), QGIS Overlapping control (Avoid overlap).

## Bağlam

Yeni parsel çoğu zaman var olan parsellerin arasına çizilir. Ortak sınır komşununkiyle bire bir aynı olmalıdır: araya giren boşluk ya da bindirme alanları bozar, Toplu alan (ADR 0151) ve topoloji denetimi (`GIS-04`) hata bulur.

KentOS'ta bugün iki yol var:

- köşeleri komşunun köşelerine kenetleyerek çizmek;
- İzle ile (ADR 0161) ortak sınırı komşunun çizgileri boyunca çizmek.

İkisi de ortak sınırın her köşesini kullanıcıya çizdirir.

Öbür programlar:

- **ArcGIS Pro, Autocomplete Polygon:** yalnız yeni sınır çizilir. İlk ve son parça komşu alanların içinde ya da sınırındadır. Bitirince komşuya taşan parçalar kırpılır, ortak sınır komşudan alınır, komşulara köşe eklenir.
- **QGIS, Overlapping control:** üç kip vardır: çakışmaya izin ver, etkin katmanda önle, katman başına ayarla. Önleme açıkken yeni alan komşularına taşırılarak çizilir; taşan kısım kesilir, yeni alan komşunun sınırına oturur. Tümüyle komşuların içinde kalan alan yazılmaz, hata söylenir.
- **Netcad:** karşılığı bulunmadı.

## Karar

### 1. Çakışma kipi

**Çakışma** bir çizim yardımcısıdır: Topoloji gibi durum çubuğundadır. Oturum ayarıdır (`drafting.overlap`, ADR 0023), üç değeri vardır:

| Değer | Ad | Komşu sayılan alanlar |
|---|---|---|
| `allow` (ilk değer) | Serbest | yok; yeni alan olduğu gibi yazılır |
| `layer` | Kendi katmanında önle | yeni alanın yazılacağı katmandakiler |
| `layers` | Seçili katmanlarda önle | seçilen katmanlardakiler |

- **Seçili katmanlar** oturumun durumudur, ayar değildir (ayarlar liste tutmaz). Çizim değişince var olmayan katmanlar düşer. Liste boşken bu kipte hiçbir şey kırpılmaz; araç bunu söyler.
- **Komşu alan:** çakışma katmanlarının görünen alanları: kapalı alan (delikleri ve parçalarıyla), daire, tam elips, kapalı eğri, kapalı çoklu çizgi (alan işlemlerinin alan saydıkları, `ops::areas`). Gizli katmandakiler sayılmaz. Komşu değişmez; kilitli olması önemli değildir.
- **Durum çubuğu:** “Çakışma” hücresi. Tık, Serbest ile son önleme kipi arasında geçer (ilk önleme kipi: kendi katmanında). Hücre önleme kipindeyken basılıdır. Sağ tık menüsü:
  - üç kip, radyo düğmesiyle;
  - **Katmanlar ▸** alt menüsü: katmanlar renk örnekleriyle, seçilenler işaretli, gizliler “gizli” notuyla. Bir katmanı işaretlemek kipi Seçili katmanlarda önle'ye getirir.
- **Komutlar:** `draft.overlap` (aç/kapat), `draft.overlap.allow`, `draft.overlap.layer`, `draft.overlap.layers`. Araçlar › Çizim yardımcıları'nda da bulunurlar.

### 2. Kırpma

Kip açıkken yeni alan yazılmadan önce komşuların birleşimi ondan çıkarılır (alan işlemlerinin farkı, ADR 0065). Sonuç:

- **Değişmedi:** yeni alan olduğu gibi yazılır.
- **Boş:** hiçbir şey yazılmaz. İleti: “Yeni alan komşu alanların içinde kalıyor; alan eklenmedi.”
- **Bir parça** (delikli olabilir): alan olarak yazılır.
- **Birden çok parça:** tek, çok parçalı alan olarak yazılır (ADR 0143).

Kırpılınca araç ayrıca söyler: “Çakışma önlendi: n komşu alanla örtüşen kısım çıkarıldı.” Alan iletisi yazılan alanı verir.

- **Köşeler:** yeni alanın kendi köşeleri ve örtüşen komşuların köşeleri bit bit korunur. Yalnız sınırların kesiştiği noktalar hesaplanır.
- **Yalnız değen komşu** (kenarı ya da köşesi yeni alanın sınırında) kırpmaya katılmaz, köşe de eklemez. Köşe bağlama §4'ün işidir.
- **Yaylar** yay kalır.
- **Kot:** çizim araçlarının alanında kot yoktur; kırpılmış alanda da yoktur.

**Uygulandığı araçlar:** yeni alanın sınırını çizenler:

- Kapalı alan;
- Parsel oluştur (komşu: parsel katmanı ya da seçilenler);
- Dikdörtgen ve döndürülmüş dikdörtgen;
- Düzgün çokgen;
- Alan hesapla'nın Alan olarak çiz'i;
- Bitişik alan (§3).

**Uygulanmadığı yerler:**

- Daire, Elips, Halka: analitik tanım korunur, sessizce çokgene indirilmez (CLAUDE.md §0).
- Revizyon bulutu: açıklama nesnesidir.
- İçine tıklayarak alan ve Toplu alan: bölge var olan çizgilerden gelir.
- Yapıştırma, kopya, dizi, içe aktarma ve düzenlemeler: çakışma denetimi `GIS-04`'ün işidir.

Önizleme kırpılmamış alanı gösterir (QGIS ve ArcGIS gibi); kırpma yazarken yapılır.

### 3. Bitişik alan aracı

- **Ad ve yer:** “Bitişik alan”. Şeritte Giriş › Çizim panelinde, Kapalı alan'ın yanında; kendi ikonuyla.
- **Çizim:** açık bir yol, Çoklu çizgi gibi çizilir: Yay, Uzunluk, İzle, Akış, Geri, kenet ve dinamik giriş geçerlidir. İlk ve son nokta komşu alanların içinde ya da sınırında olmalıdır.
- **Bölge:** yol ile komşu alanların sınırlarının kapattığı, komşuların dışında kalan ve sınırında yolun bir parçası bulunan bölgelerin birleşimi; komşuların içinde kalan adalar delik olur. Bölge birden çok parçaysa çok parçalı tek alan olur.
  - Sınırdaki parça iki yüzü ayırmalıdır: bölgenin içinde sarkan yol ucu sayılmaz.
  - Yolun iki yanı da kapalıysa (komşularla çevrili bir boşluğu kesen yol) iki yan da dolar. Boşluğun bir yanını almak için o yanı Kapalı alan ile çakışma önlenerek çizilir.
- **Komşular:** çakışma kipinin katmanlarının (§1) görünen alandaki alanları; Serbest'te araç kendi katmanını kullanır. Önizleme ve yazma aynı komşularla yapılır: görünen ne ise yazılan odur. Görünümden taşan bölge için görünüm uzaklaştırılır (Zincir gibi, ADR 0161).
- **Önizleme:** imleç sonraki nokta sayılarak bölge canlı doldurulur. Kart yolun uzunluğunu ve bölgenin alanını verir.
- **Bitirme:** Enter ya da sağ tık. Bölge yoksa hiçbir şey yazılmaz, yol kalır. İleti: “Yol komşu alanlarla kapalı bir bölge oluşturmuyor: ilk ve son noktayı komşu alanların içine ya da sınırına koyun.”
- **Yazma:** etkin katmana, etkin renk ve kalınlıkla, tek “Bitişik alan” adımında (`cad.entities.create`'in `adjoin` işlemi).

### 4. Topoloji açıkken komşulara köşe

Topolojik düzenleme (ADR 0160) açıkken yeni alan komşularıyla köşe köşe bağlanır:

- yeni alanın bir köşesi komşunun bir kenarının üstündeyse (1 µm) o köşe komşuya da eklenir;
- komşunun bir köşesi yeni alanın bir kenarının üstündeyse yeni alana da eklenir.

Kurallar ADR 0160'ınkilerdir: görünen ve kilitsiz komşular, kot kenardan doğrusal. Kilitli komşu değişmez, sayısıyla söylenir. Hepsi yeni alanın adımındadır.

Uygulandığı araçlar §2'ninkilerdir, kip Serbest olsa da.

### 5. Ortak çekirdek

**`ops::adjoin`** (WASM `AdjoinWork`):

- **`avoid(area, neighbours)`:** alan işlemlerinin farkı (`overlay`, `FirstNotOthers`). Komşular kutuları yeni alanın kutusuna değenlerdir.
- **`fill(path, neighbours)`:** yolun ve komşu sınırlarının düzenlemesi (`arrangement::build`, alan işlemlerininki), yüz yürüyüşü (Tarama'nın ve Toplu alan'ınki) ve §3'ün kuralı tek geçişte: seçilen yüzlerle seçilmeyenleri ayıran parçalar halkalara dizilir.
  - Komşular verilenlerin hepsidir.
  - Önizleme her imleç hareketinde hesaplanır. Görünen komşuların kenarları 2 000'den çoksa yalnız tıklarda ve Enter'da hesaplanır (ölçüm: 1 596 kenarda 2,7 ms, 4 092'de 7,2 ms, yerli; süre çoğunlukla ortak düzenlemenin kurulumunda).
- **`junctions(area, neighbours)`:** §4'ün köşeleri.
- **Bağımsız başvuru:** `scripts/fixtures/adjoin_cases.py`, KentOS kodu olmadan.
  - Düz kenarlarda kesin kesirlerle kesişim, yüz yürüyüşü, fark ve birleşim.
  - Yaylarda 50 basamaklı `mpmath` ile alan ve kesişim noktaları.

  Durumlar: bir ve iki komşuyla kırpma, değişmeyen alan, tümüyle içte kalan alan, komşuyu içine alan alan (delik), iki parçaya bölünen alan, üst üste binen ortak kenar, yalnız köşede değme, yaylı komşu, büyük koordinatlar; Bitişik alan'da iki komşu arası boşluk, derin girinti (uzak komşuyla kapanan), komşuya girip çıkan yol, kapanmayan yol, kendini kesen yol, ada; köşe bağlama.

### 6. İş sırası

1. Çekirdek: `ops::adjoin`, WASM, bağımsız başvuru ve ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Kırpma:** `Neighbours::avoid` (WASM `AdjoinWork.avoid`, işlem `adjoinAvoid`). Kutusu yeni alanınkine değen komşulardan örtüşenler alan işlemlerinin kesişimiyle bulunur; yalnız onlar `subtract_areas` ile çıkarılır. Örtüşen yoksa alan olduğu gibi döner.
   - **Bölge:** `geom::overlay::adjoin_faces` (`Neighbours::fill`, WASM `AdjoinWork.fill`, işlem `adjoinFill`).
     - Komşular alan kaynağı, yol çizgi kaynağıdır; tek düzenleme kurulur, her parça iki yönde yürünür.
     - Yüzler önce yola bitişiklikle (ucuz), sonra kutu süzgeçli iç testiyle seçilir.
     - Seçilenleri öbürlerinden ayıran parçalar overlay'in halka dizimiyle (`rings`, `assemble`) alanlara döner.
   - **Başvuru:** `scripts/fixtures/adjoin_cases.py` (`fixtures/adjoin/v1/cases.json`): 54 kırpma ve 46 doldurma.
     - Elle durumlar başlangıçta ve TM koordinatlarında; yaylı iki durum elle (mpmath); 20'şer rastgele durum (parsel blokları).
     - Girdi köşeleri bit bit, öbürleri 1e-9 m, kabarıklıklar 1e-12, alanlar göreli 1e-9 içinde.
   - **Sonuç:** çekirdek (yerli) ve web (WASM) başvuruyla aynı; tutulan komşular (`AdjoinWork`) işlemlerle aynı. Kuralı bozan üç deneme durumları düşürür: sarkan uç bitişik sayılır, komşunun içi tutulur, yalnız değen komşu da kırpar.
   - **Ölçüm** (yerli, release; ızgara parseller arasında bir boşluk ve onu kapatan yol): 396 kenarda 0,6 ms, 1 596'da 2,7 ms, 4 092'de 7,2 ms, 8 096'da 17,8 ms. Sürenin çoğu düzenlemenin kurulumunda (`arrangement::build`); önizleme bütçesi bu yüzden 2 000 kenardır.
2. Çakışma kipi iki platformda: ayar, komutlar, durum çubuğu hücresi ve menüsü, araçlarda kırpma; ortak iz; resimler.

   *(2 Ekim: tamam.)*
   - **Ayar ve durum:** `drafting.overlap` (oturum, `allow`, `layer`, `layers`); seçili katmanlar ve hücrenin döneceği son kip oturumun durumudur (web `DraftingSettings.overlapLayers`, `overlapLast`; masaüstü `App::overlap_layers`, `overlap_last`; araçlar `Context::overlap_layers` ile görür).
   - **Komutlar:** `draft.overlap` (aç/kapat) ve üç kipin radyo komutları; Araçlar › Çizim yardımcıları ve Çakışma alt menüsü. Dört ikon: iki alan üst üste (Serbest), kırpılmış alan ve eski kenarı kesik (Çakışmayı önle), bir tabaka (kendi katmanı), iki tabaka (seçili katmanlar).
   - **Durum çubuğu:** Çakışma hücresi; sağ tık menüsü üç kip ve Katmanlar ▸ (renk örnekleri, gizli notu). Web `ui/statusbar/overlapMenu.ts`, masaüstü `view.rs`'in `overlap_menu`'su.
   - **Araçlar:** ortak yardımcı (web `tools/overlap.ts`, masaüstü `kentos_interaction::overlap`): komşular görünen nesnelerden kutusu yeni alanınkine değenler, çekirdeğin `adjoinAvoid`'iyle kırpılır; kırpılan alan `cad.entities.create` ile tek nesne (delikleri ve parçalarıyla). Kapalı alan, Parsel oluştur (parsel katmanına), Dikdörtgen, Döndürülmüş dikdörtgen, Düzgün çokgen, Daire dilimi (`writeArea` / `write_area`) ve Alan olarak çiz (kendi adımında). Daire dilimi de yeni alanın sınırını çizdiği için listeye girdi.
   - **Sınama:**
     - İki platformda aynı elle hesaplanmış birim testleri (web `tools/overlap.test.ts`, masaüstü `tests/overlap.rs`): Serbest, kendi katmanı, başka katmanın komşusu yalnız seçilince, örtülen alan, seçili katmansız uyarı, gizli katman, iki parça, Dikdörtgen, Parsel oluştur, Alan olarak çiz.
     - Ortak iz `overlap.json` (`overlap.kcad`); köşeler yazılarak verilir, ileti alanları tam karşılaştırılır. Masaüstünde hücre ve menüsü `overlap_tests.rs`'te, web'de yerleşim denetiminin `status-overlap` görünümlerinde.
     - Kullanım senaryosu `usage-overlap.json`.
3. Bitişik alan iki platformda: araç, ikon, önizleme, `cad.entities.create`'in `adjoin` işlemi; ortak iz; resimler.
4. Topoloji açıkken komşulara köşe iki platformda; ortak iz; resimler.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

### 7. Kapsam dışı

- **Düzenlemelerde çakışma denetimi:** tutamaç, taşıma ve Esnet çakışmayı denetlemez; topoloji doğrulaması `GIS-04`'tür.
- **Katman listesinin projeye kaydı:** liste oturumdadır. Projeyle saklanması `.kcad` şeması ister; ihtiyaç olunca ayrı karar.
- **Çizgi katmanlarına yaslanma:** komşu yalnız alandır. Yol kenarı gibi çizgilere İzle ile yaslanılır.

## Sonuçlar

- Komşular arasına yeni alan yalnız yeni sınırı çizilerek eklenir; ortak sınır komşununkidir, boşluk ve bindirme kalmaz.
- Komşulara taşırılarak çizilen alan kırpılır.
- Kip Serbest'ken ve Bitişik alan kullanılmazken bugünkü çizim aynen sürer.

## Doğrulama

- **Çekirdek:** bağımsız başvuruya göre iki platformda kırpma, bölge ve köşe bağlama.
- **Arayüz:** ortak izlerle iki platformda; resimler iki temada, 1440×900 ve 1100×650.
