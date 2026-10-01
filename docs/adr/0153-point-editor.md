# ADR 0153: Nokta editörü

- **Durum:** kabul edildi (2026-10-02). Sıra sahibin kararıdır: TODOS.md §16.0'ın dördüncü işi `HYB-04`. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** TODOS.md `HYB-04` (ilgili: `HYB-03`, `HYB-06`, `GIS-01`), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ADR 0152 (ölçü noktası: ad, kod, kot; Artır), ADR 0048 (koordinat listesi al ve ver), ADR 0058 ve 0114 (alt panel, Koordinat listesi), ADR 0142 (köşe kotu), ADR 0068 (tutamaçla köşe taşıma), ADR 0149 (gösterim kuralı).

## Bağlam

Ölçü noktalarıyla çalışan bir çizimde yüzlerce, binlercesi olur. Mühendis onları bir tabloda görmek ister: adına göre sıralı, süzülmüş; yanlış okunan koordinatı ya da kodu orada düzeltir, iki kez ölçülen noktayı ayıklar, numaraları düzene sokar.

KentOS'ta bugün noktalar yalnız tek tek düzenlenir (Öznitelikler). Alt paneldeki Koordinat listesi seçili nesnelerin köşelerini salt okunur gösterir. Sıralama, süzme, toplu adlandırma ve çift nokta ayıklama yoktur.

Netcad bunu Nokta Editörü ile yapar: süzgeçle açılan tablo (Sıra, Nokta Adı, Y, X, Z, Kod, Pafta, Tabaka); Ekle, Düzenle, Sil; Sırala, Çift Noktaları Ayıkla (ilki ya da ortalama), Yeniden Adlandır, Sıralı Numara Ver, Tabakalandır; dosyadan yükleme ve saklama; çıkarken “Bağlı Çizgileri Güncelle” ve “Çizgi Kotlarını Güncelle”. ArcGIS ve QGIS'te karşılığı öznitelik tablosu ve XY alanlarıdır.

## Karar

### 1. Yer

Nokta editörü alt panelin yeni sekmesidir: **Noktalar** (Komut geçmişi, Koordinat listesi ve Uyarılar'ın yanında).

- `point.editor` komutu (“Nokta editörü”) paneli bu sekmede açar. Takma adlar: NOKTAEDITORU, NE, NOKTALAR.
- Şeritte Harita › Koordinatlar'da, Koordinat listesi'nin yanında; komut aramasında ve takma adlarıyla her yerde.
- Gerekçe: tablo geniş sütunlar ister ve çizimle birlikte kullanılır. Alt panel tam genişliktedir, boyu sürüklenerek büyür ve çizimi örtmez (ArcGIS'in öznitelik tablosu da altta açılır). Pencere olsaydı satırdan çizime geçmek için her seferinde kapanırdı.

### 2. Tablo

- **Satırlar:** çizimdeki bütün nokta nesneleri. Gizli ve kilitli katmandakiler de listelenir; kilitli olanların hücreleri düzenlenmez.
- **Sütunlar:** Sıra, Ad, Y, X, Z, Kod, Katman.
  - Sıra satırın listedeki yeridir (1, 2, 3…).
  - Ad noktanın etiketidir, Kod `Kod` özniteliğidir, Z noktanın kotudur (ADR 0152 §1).
- **Sayılar:** projenin ondalıklarıyla, ADR 0149'un gösterim kuralıyla yazılır. Düzenlemeye başlanınca hücre değerin tamamını gösterir (en kısa geri dönen yazım). Değiştirmeden Enter'a basmak değeri değiştirmez.
- **Sıralama:** sütun başlığına bir tık artan, ikinci tık azalan, üçüncü tık çizim sırasına döner.
  - Ad ve Kod doğal sırayla dizilir: P2, P10'dan önce; 101/2, 101/10'dan önce (çekirdeğin `natural_cmp`'i).
  - Sayılar sayı olarak dizilir. Boş değerler her iki yönde de sondadır.
- **Süzgeç:**
  - Arama kutusu adda ya da kodda arar. Türkçe kuralla büyük küçük harf ayırmaz; `*` herhangi bir dizidir, `*` yoksa içinde geçen aranır.
  - Katman seçimi: Bütün katmanlar ya da biri.
  - Yalnız seçililer.
  - Sayaç: `128 / 1 204 nokta`.
- **Seçim:**
  - Satıra tık noktayı çizimde seçer; Ctrl ekler ya da çıkarır, Shift aralığı seçer.
  - Çizimde seçilen noktaların satırları seçili görünür; ilki görünür yere kaydırılır.
  - Göster (araç çubuğunda ve satırın menüsünde) seçili noktalara yakınlaştırır. Satırın Sıra hücresine çift tık da yakınlaştırır.

### 3. Yerinde düzenleme

Ad, Y, X, Z ve Kod düzenlenir; Sıra ve Katman düzenlenmez (katmanı Katmana taşı değiştirir).

- **Başlatma:** hücreye çift tık. (F2 alt paneli açıp kapatır; düzenlemeyi başlatmaz.)
- **Bitirme:**
  - Enter yazar ve düzenleme, düzenlemeden önceki sırayla alttaki satırın aynı hücresine geçer; son satırda biter.
  - Tab yazar ve sağdaki düzenlenen hücreye geçer (Kod'dan sonra alttaki satırın Ad'ı); Shift+Tab soldakine.
  - Başka yere tıklamak yazar ve bitirir. Esc vazgeçer.
- **Değer:** Ad ve Kod baştaki ve sondaki boşluklar atılarak yazılır. Y, X ve Z nokta girişinin sayı dilbilgisiyle okunur (`parse_number`).
- **Yanlış değer:** sayı olmayan Y, X ya da Z yazılmaz; hücre düzenlemede kalır ve nedeni söylenir (“Y bir sayı olmalı.”). Boş Y ya da X da sayı değildir.
- **Değişmeyen değer** hiçbir şey yazmaz.
- **Her hücre bir geri alma adımıdır: “Nokta düzenle”.**
  - Ad: `cad.entities.set` (etiket); boş ad adı kaldırır.
  - Kod: `cad.entities.set` (`Kod` özniteliği); boş kod özniteliği kaldırır.
  - Y, X: `cad.entities.edit`, `properties` işlemi (noktanın yeri).
  - Z: `cad.entities.edit`, `elevation` işlemi; boş Z kotu kaldırır.
- **Kilitli katman:** noktanın katmanı kilitliyse komutun reddi söylenir; hiçbir şey yazılmaz.
- **Bağlı çizgiler izler** (tablonun araç çubuğunda kutu; açık; oturum boyunca):
  - Nokta taşınınca, eski yerinde (1 µm) köşesi olan çizgi, çoklu çizgi ve alanların o köşeleri de taşınır.
  - Z değişince o köşeler noktanın yeni kotunu alır; kot kaldırılırsa onlarınki de kalkar.
  - Hepsi noktanın adımındadır (aynı komutun değişiklikleri). Yaylı kenarın bükümü korunur (tutamacın kuralı, ADR 0068).
  - Bağlı çizgilerden biri kilitli katmandaysa düzenleme bütünüyle reddedilir (`cad.entities.edit`'in kuralı). İleti nedeni ve çaresini söyler: “Bağlı çizgilerden biri kilitli katmanda; katmanın kilidini açın ya da Bağlı çizgiler izler'i kapatın.”
- **Aynı ad:** başka bir noktanın adı verilirse yazılır, ama uyarılır: ““101” adında başka bir nokta da var.” (`#ad` ikisini ayırt edemez, ADR 0152 §4.)

### 4. Satır ekleme ve silme

- **Satır ekle:** tablonun sonunda taslak satır açılır; sıralama ne olursa olsun en alttadır. Düzenleme Ad'dan başlar.
  - Ad, Y, X, Z ve Kod yazılır; Tab ve Shift+Tab taslağın hücreleri arasında yazmadan gezer.
  - Taslaktaki bir hücrede Enter, Y ve X geçerliyse noktayı `cad.point.create` ile etkin katmana, geçerli renkle yazar (adım “Ekle”). Sonra yeni bir taslak açılır: adı yazılanın Artır'la bir sonrakidir (sayıyla bitmeyen ad boş kalır), düzenleme Y'dedir. Böylece bir koordinat listesi satır satır yazılır.
  - Y ya da X eksikse Enter eksik olanı söyler (“Y ve X yazılmalı.”, “X yazılmalı.”) ve taslak kalır; sayı olmayan değer düzenlemedeki gibi söylenir.
  - Esc taslağı bırakır. Nokta aracının Ad, Kod ve Kot'u kullanılmaz: satırda ne yazıyorsa odur.
- **Sil:** seçili satırların noktaları tek adımda silinir (`cad.entities.delete`, Sil'in kuralıyla: kilitli katmandakiler kalır ve söylenir). Seçim tablo ile çizim arasında ortak olduğundan Delete tuşu da aynı işi yapar.
- **Ortak durumlar:** hücre düzenlemeleri, bağlı çizgiler ve taslak `fixtures/point-editor/v1/edits.json`'dadır (`scripts/fixtures/point_edit_cases.py`, kurallardan); iki platform aynı sonuçları, iletileri ve adım adlarını verir.

### 5. Toplu işlemler

İşlemler ▾ menüsünde. Seçili satırlara uygulanır; seçim yoksa süzülen bütün satırlara. Her biri tek geri alma adımıdır ve adımı işlemin adını taşır.

- **Yeniden adlandır:** Önek ekle (ör. `P.`) ya da Önek kaldır. Öneki olmayan ad değişmez.
- **Sıralı numara ver:** başlangıç adı yazılır (1, P100, 101/1). Satırlar tablodaki sırayla adı alır; her biri Yazı'nın Artır kuralıyla bir sonrakini verir (ADR 0152 §6). Sayıyla bitmeyen başlangıç reddedilir: “Başlangıç adı sayıyla bitmeli.”
- **Katmana taşı:** seçilen katmana (`cad.entities.set`).
- **Çift noktaları ayıkla:**
  - **Ölçüt:** Aynı ad, ya da Aynı yer (tolerans metre, varsayılan 0,001 m).
    - Aynı adda adlar baştaki ve sondaki boşluklar atılarak tam karşılaştırılır; adsız noktalar çift sayılmaz.
    - Aynı yerde noktalar çizim sırasıyla gezilir; her nokta, ilk noktası toleransla değen ilk gruba katılır, yoksa yeni grup açar. Zincirleme olmaz: grubun her noktası ilk noktaya tolerans içindedir.
  - **Tutulan:** İlki, Sonuncusu (çizim sırasıyla) ya da Ortalaması.
    - Ortalamada tutulan, grubun ilk noktasıdır. Yeri grubun yerlerinin ortalamasına taşınır; kotu, kotu olanların ortalamasıdır (hiçbirinin yoksa kotsuz). Adı ve kodu ilkininkidir.
  - **Önce sayılar söylenir:** “6 grupta 15 nokta; 9 nokta silinecek.” Çiftleri göster tabloyu gruplara süzer.
  - **Ayıkla:** fazlalar silinir, ortalamada tutulan taşınır (Bağlı çizgiler izler açıksa onlar da izler). Tek adım “Çift noktaları ayıkla”.
- **Dışa aktar:** tablodaki (süzülen) noktalar ADR 0048'in koordinat listesi verme penceresiyle yazılır.
- **İçe aktar:** ADR 0048'in koordinat listesi alma penceresi.
  - Çizimde aynı adlı nokta varsa içe aktarma yine ekler. Sonra Çift noktaları ayıkla, Aynı ad ile, Netcad'in seçeneklerini verir: Sonuncusu dosyadakini, İlki çizimdekini tutar, Ortalaması ikisinin ortalamasıdır.

### 6. Hesap (ortak çekirdek)

- **`text::natural_cmp`:** doğal sıra; rakam dizileri sayı olarak (öndeki sıfırlar eşitse kısası önce), öbür karakterler Türkçe büyük küçük harf katlamasıyla, sonra özgün metinle.
- **`ops::duplicate_points`** (WASM `duplicatePoints`):
  - Girdi: noktalar (yer, kot, ad) çizim sırasıyla, ölçüt (ad ya da yer ve tolerans), tutulan (ilki, sonuncusu, ortalaması).
  - Çıktı: gruplar, her grubun tutulanı ve onun yeni yeri ve kotu, silinenler.
  - Ortalama, grubun ilk noktasına göre farkların ortalamasıyla alınır: büyük koordinatlarda yuvarlama hatası birikmez.
- **`ops::follow_point`** (WASM `followPoint`): bir nesnenin `from`'a 1 µm içindeki köşelerini `to`'ya taşır, kot verildiyse o köşelere kotu yazar. Çizgi, çoklu çizgi ve alan (delikleri ve parçalarıyla); öbür türler değişmez.
- **Bağımsız başvurular:** `scripts/fixtures/point_editor_cases.py` doğal sırayı, çift gruplarını (kesirlerle ortalama) ve bağlı köşeleri kurallardan yazar (`fixtures/point-editor/v1/cases.json`).

### 7. Komutlar

Yeni sözleşme yoktur:

| İş | Komut |
|---|---|
| Ad, Kod, Katmana taşı | `cad.entities.set` |
| Y, X, Z, bağlı çizgilerle | `cad.entities.edit` (`properties`, `elevation`) |
| Satır ekle | `cad.point.create` |
| Sil | `cad.entities.delete` |
| Yeniden adlandır, Sıralı numara ver, Çift noktaları ayıkla | Bu komutların tek işlemde birleşimi, adımın adıyla |

Python ve MCP aynısını betiğin tek adımlık grubuyla yapar.

### 8. Kapsam dışı

- Modelden kot (yüzey modeli gelince, `GIS-` maddeleri), kolon ve eksen işlemleri.
- Koordinatları ondalığa yuvarlama: kaynak koordinatı değiştirir; ADR 0149'un gösterim kuralı ayrıdır.
- Rapor yazdırma (§16.4), ikinci projeksiyon sütunları (`HYB-11`), pafta sütunu (pafta nesnesi yok).
- Genel öznitelik tablosu (`GIS-01`).

### 9. İş sırası

1. **Çekirdek:** `natural_cmp`, `duplicate_points`, `follow_point`; WASM; bağımsız başvuru ve ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Doğal sıra:** `text::natural::natural_cmp`; WASM'da `naturalOrder` (adların sırası, dizinleriyle).
   - **Tablo:** `ops::point_editor::point_table` (WASM `pointTable`): süzgeç ve sıralama iki platformda tek yerden. Arama `text::edit::search`'tür (Türkçe katlama, `*` joker).
   - **Çift noktalar ve bağlı köşeler:** `duplicate_points` (WASM `duplicatePoints`), `follow_point` (WASM `followPoint`).
   - **Başvuru:** `point_editor_cases.py` 197 durum yazar (19 sıra, 114 tablo, 34 çift, 30 bağlı köşe; elle kurulanlar başlangıçta ve TM koordinatlarında, gerisi rastgele). Çekirdek (`tests/point_editor.rs`) ve web WASM'ı (`model/ops/pointEditor.test.ts`) aynı; bozulan kuralları yakalar.
2. **Tablo:** Noktalar sekmesi, sütunlar, sıralama, süzgeç, seçimin eşlenmesi, Göster; tablonun kuralları iki platformda aynı durumlarla, resimler.

   *(2 Ekim: tamam.)*
   - **Sekme:** web'de `ui/bottom/PointTable.ts`, masaüstünde `points/`. Süzgeç ve sıralama çekirdeğin `point_table`'ından.
   - **Sorgu:** arama, katman, Yalnız seçililer ve sıralama oturum boyunca tutulur.
   - **Katman listesi:** yalnız nokta taşıyan katmanlar, katman listesinin sırasıyla ve nokta sayılarıyla. Seçilen katman nokta taşımaz olursa bütün katmanlar gösterilir.
   - **Seçim:** ilk seçili satır değişince bir kez görünür yere kaydırılır; kullanıcı kaydırırsa öyle kalır.
   - **Komut:** `point.editor` Harita › Koordinatlar'da; ikonu kendi çizimi (satırlar ve Nokta'nın artı halkası).
   - **Yerleşim:** alt panelin kalıcı sekmesi `points` olabilir (`fixtures/shell/v1/layout.json`).
   - **Sınama ve resimler:** başlık döngüsü, tıklamayla seçim ve satırın okunuşu iki platformda aynı durumlarla (`PointTable.test.ts`, `points/tests.rs`). Masaüstünde sekmenin sorguyu izlemesi ve çift tıkla yakınlaştırma da sınanır. Resimler `fixtures/interaction/v1/point-editor.kcad` üzerinde: web `shots.mjs pointeditor`, masaüstü `points::tests::screens`.
3. **Düzenleme:** hücreler, Bağlı çizgiler izler, Satır ekle, Sil; ortak durumlar ve iz.

   *(2 Ekim: tamam.)*
   - **Kurallar:** hücrenin yazılması ve taslak satır web'de `ui/bottom/pointEdit.ts`, masaüstünde `points/edit.rs`. İki platform `fixtures/point-editor/v1/edits.json`'un 35 durumunu (24 hücre, 11 taslak) geçer: belge, ileti, adımın adı, bağlı çizgilerin köşeleri ve kotları. Durumları `scripts/fixtures/point_edit_cases.py` kurallardan yazar; bozulan kuralları yakalar.
   - **Gezinme:** Enter, Tab ve Shift+Tab'ın gideceği hücre yazmadan önceki sırayla bulunur (`nextCell`, `next_cell`; iki platformda aynı durumlar).
   - **Alan:** web'de hücrenin içinde `input`; masaüstünde `points/cell.rs`'in alanı (Esc ve başka yere basma onda yakalanır, Tab uygulamanın tuşudur). Açılınca değer seçilidir.
   - **Araç çubuğu:** iki grup: solda arama, katman, Yalnız seçililer ve Bağlı çizgiler izler; sağda sayaç, Satır ekle, Sil ve Göster. Dar panelde sağ grup ikinci satıra geçer, düğme kesilmez.
   - **Sınama ve resimler:** tarayıcıda `shots.mjs pointeditor`'ın sahneleri klavye ve fareyle sınar: çift tıkla açma ve değerin tamamı, Enter'la yazma ve alttaki satıra geçme, iki satır ekleme ve sonraki ad, iki satır silme ve tek adımda geri alma. Masaüstünde `points::tests` aynı akışları sınar. Etkileşim izi yoktur: izler çizim alanının araçları içindir, tablo alt paneldedir.
4. **Toplu işlemler:** Yeniden adlandır, Sıralı numara ver, Katmana taşı, Çift noktaları ayıkla, Dışa aktar, İçe aktar; ortak durumlar ve iz.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Ölçü noktaları tek tabloda görülür, sıralanır, süzülür ve yerinde düzeltilir; çizgiler noktaları izler.
- İki kez ölçülen noktalar ve dağınık numaralar tek işlemde düzene girer.
- Yeni komut sözleşmesi yoktur; her değişiklik var olan komutlarla, geri alınabilir adımlarla yazılır.

## Doğrulama

- **Hesap:** doğal sıra, çift grupları ve ortalama, bağlı köşeler bağımsız başvuruyla iki platformda.
- **Tablo:** ortak tablo modeli (sıralama, süzgeç, sayaç) iki platformda; düzenleme ve toplu işlemler ortak durumlarla.
- **Arayüz:** resimler iki temada, 1440×900 ve 1100×650.
